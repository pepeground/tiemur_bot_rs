mod response;

use std::env;
use std::rc::Rc;
use std::cell::{RefCell, RefMut};
use std::collections::BinaryHeap;

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
use rocksdb::{DB, Options, IteratorMode, ColumnFamily};
use bincode::{serialize, deserialize, Infinite};

use types::{Image,User,UserContent};

pub fn process(message: Rc<Message>,
               api: Api,
               handle: &Handle,
               client: Client<HttpsConnector<HttpConnector>>,
               db: Rc<RefCell<DB>>) {
    let clone = message.clone();
    let clone1 = message.clone();


    let mut cf_name = message.chat.id().to_string();

    let image_cf = db_handle(db.borrow_mut(), &cf_name);
    cf_name.push_str("_users");
    let user_cf = db_handle(db.borrow_mut(), &cf_name);

    match message.kind {
        MessageKind::Photo { ref data, .. } => {
            let future = api.send(data[0].get_file())
                .map_err(|e| e.to_string())
                .and_then(|file| {
                    file.get_url(&env::var("TELEGRAM_TOKEN").unwrap())
                        .ok_or("No file path".to_string())
                })
                .and_then(|url| url.parse().map_err(|e: UriError| e.to_string()))
                .and_then(move |url| client.get(url).map_err(|e| e.to_string()))
                .and_then(|res| res.body().concat2().map_err(|e| e.to_string()))
                .and_then(|body| load(&body[..]).map_err(|e| e.to_string()))
                .and_then(|image| Ok(ImageHash::hash(&image, 8, HashType::Gradient)))
                .and_then(move |hash| {
                    let db = db.borrow();
                    let bytes = &hash.bitv.to_bytes()[..];
                    let find = db.iterator_cf(image_cf, IteratorMode::End)
                        .unwrap()
                        .find(|&(ref key, ref _value)| key.as_ref() == bytes);
                    let telegram_user = clone.from.clone().ok_or("user empty".to_string())?;
                    let mut user: User = telegram_user.into();
                    let key = serialize(&user.0, Infinite).unwrap();
                    let user_row = db.get_cf(user_cf, &key).unwrap();
                    if user_row.is_none() {
                        let value = serialize(&user.1, Infinite).unwrap();
                        let _ = db.put_cf(user_cf, &key, &value);
                    }
                    match find {
                        Some((_key, value)) => {
                            let image: Image = deserialize(&*value).unwrap();
                            if let Some(user_row) = user_row {
                                let row: UserContent = deserialize(&*user_row).unwrap();
                                user.1 = UserContent{
                                    count: row.count + 1,
                                    ..user.1
                                };
                                let value = serialize(&user.1, Infinite).unwrap();
                                let _ = db.put_cf(user_cf, &key, &value);
                            }
                            Ok((image, user.1))
                        }
                        None => {
                            let image = Image::new(clone.id, user.0, clone.date);
                            let value = serialize(&image, Infinite).unwrap();
                            let _ = db.put_cf(image_cf, bytes, &value);
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
        MessageKind::Text { ref data, .. } => {
            match data.as_ref() {
                "/tiemur_stats" | "/tiemur_stats@TiemurBot" => {
                    let mut users: BinaryHeap<_> = db.borrow()
                        .iterator_cf(user_cf, IteratorMode::End)
                        .unwrap()
                        .map(|(_key,value)| -> UserContent {deserialize(&*value).unwrap()})
                        .collect();
                    let text = users.pop().map_or("".to_string(), |a| a.first_name);
                    let future = api.send(clone1.text_reply(text))
                        .map_err(|e| e.to_string());
                    handle.spawn({
                        future.map_err(|_| ()).map(|_| ())
                    })
                }
                _ => ()
            }
        }
        _ => ()
    }
}

fn db_handle(mut db: RefMut<DB>, cf_name: &str) -> ColumnFamily {
    match db.cf_handle(cf_name) {
        Some(cf) => cf,
        None => db.create_cf(cf_name, &Options::default()).unwrap(),
    }
}
