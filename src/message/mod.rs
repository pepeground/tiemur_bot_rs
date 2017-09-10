mod response;

use std::env;
use std::rc::Rc;
use std::cell::RefCell;

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

use types::Image;

pub fn process(message: Rc<Message>,
               api: Api,
               handle: &Handle,
               client: Client<HttpsConnector<HttpConnector>>,
               db: Rc<RefCell<DB>>) {
    let clone = message.clone();
    let clone1 = message.clone();

    let id = &message.chat.id().to_string();
    let cf_handle;
    {
        cf_handle = db.borrow().cf_handle(&id);
    }
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
                    let bytes = &hash.bitv.to_bytes()[..];
                    let find = db.borrow()
                        .iterator_cf(cf, IteratorMode::End)
                        .unwrap()
                        .find(|&(ref key, ref _value)| key.as_ref() == bytes);
                    match find {
                        Some((_key, value)) => {
                            let row: Image = deserialize(&*value).unwrap();
                            Ok(row)
                        }
                        None => {
                            let user = clone.from.clone().unwrap();
                            let image = Image::new(clone.id, user.id, 1);
                            let value = serialize(&image, Infinite).unwrap();
                            let _ = db.borrow().put_cf(cf, bytes, &value);
                            Err("new record".to_string())
                        }
                    }
                })
                .and_then(move |record| {
                    let text = response::build(record, &clone1.chat);
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
