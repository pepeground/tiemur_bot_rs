mod response;

use std::env;
use std::rc::Rc;
use std::cell::RefCell;
use std::mem::transmute;

use telegram_bot::{Api, CanReplySendMessage, CanGetFile};
use telegram_bot::types::{Message, MessageKind};
use tokio_core::reactor::Handle;
use futures::{Future, Stream};
use hyper::Client;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use hyper::error::UriError;
use img_hash::{ImageHash, HashType};
use image::load_from_memory as load;
use rocksdb::{DB, Options, IteratorMode};
use bincode::{serialize, deserialize, Infinite};

use types::{Image,User};

pub fn process(message: Rc<Message>,
               api: Api,
               handle: &Handle,
               client: Client<HttpsConnector<HttpConnector>>,
               db: Rc<RefCell<DB>>) {
    let clone = message.clone();
    let clone1 = message.clone();

    let id = &message.chat.id().to_string();
    let cf_handle = db.borrow().cf_handle(&id);
    let cf = match cf_handle {
        Some(cf) => cf,
        None => db.borrow_mut().create_cf(&id, &Options::default()).unwrap(),
    };

    match message.kind {
        MessageKind::Photo { ref data, .. } => {
            let future = api.send(data[0].get_file())
                .map_err(|e| e.to_string())
                .and_then(|file| {
                    file.get_url(&env::var("TELEGRAM_TOKEN").unwrap())
                        .ok_or("No file path".to_owned())
                })
                .and_then(|url| url.parse().map_err(|e: UriError| e.to_string()))
                .and_then(move |url| client.get(url).map_err(|e| e.to_string()))
                .and_then(|res| res.body().concat2().map_err(|e| e.to_string()))
                .and_then(|body| load(&body[..]).map_err(|e| e.to_string()))
                .and_then(|image| Ok(ImageHash::hash(&image, 8, HashType::Gradient)))
                .and_then(move |hash| {
                    let db = db.borrow();
                    let bytes = &hash.bitv.to_bytes()[..];
                    let find = db.iterator_cf(cf, IteratorMode::End)
                        .unwrap()
                        .find(|&(ref key, ref _value)| key.as_ref() == bytes);
                    let telegram_user = clone.from.clone().unwrap();
                    let mut user: User = telegram_user.into();
                    let key: [u8; 8] = unsafe { transmute(user.id) };
                    let user_row = db.get_cf(cf, &key).unwrap();
                    if user_row.is_none() {
                        let value = serialize(&user, Infinite).unwrap();
                        let _ = db.put_cf(cf, &key, &value);
                    }
                    match find {
                        Some((_key, value)) => {
                            let image: Image = deserialize(&*value).unwrap();
                            if let Some(user_row) = user_row {
                                let row: User = deserialize(&*user_row).unwrap();
                                user = User{
                                    count: row.count + 1,
                                    ..user
                                };
                                let value = serialize(&user, Infinite).unwrap();
                                let _ = db.put_cf(cf, &key, &value);
                            }
                            Ok((image, user))
                        }
                        None => {
                            let image = Image::new(clone.id, user.id, 1);
                            let value = serialize(&image, Infinite).unwrap();
                            let _ = db.put_cf(cf, bytes, &value);
                            Err("new record".to_string())
                        }
                    }
                })
                .and_then(move |(image, user)| {
                    let text = response::build(&image, &user, &clone1.chat);
                    api.send(clone1.text_reply(text))
                        .map_err(|e| e.to_string())
                });
            handle.spawn({
                future.map_err(|_| ()).map(|_| ())
            })
        }
        _ => println!("{:?}", message),
    }
}
