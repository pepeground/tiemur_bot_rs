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
use rocksdb::{DB, Options, ColumnFamily};
use types::TypedDBWithCF;

pub fn process(message: Rc<Message>,
               api: Api,
               handle: &Handle,
               client: Client<HttpsConnector<HttpConnector>>,
               db: Rc<RefCell<DB>>) {
    let message_clone = message.clone();

    let mut cf_name = message.chat.id().to_string();
    let image_cf = db_handle(&mut db.borrow_mut(), &cf_name);
    cf_name.push_str("_users");
    let user_cf = db_handle(&mut db.borrow_mut(), &cf_name);

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
                    let borrow = db.borrow();
                    let image_db = TypedDBWithCF::new(&borrow, image_cf);
                    let user_db = TypedDBWithCF::new(&borrow, user_cf);
                    response::find_tiemur(user_db, image_db, hash, message_clone)
                })
                .and_then(move |(message, image, user)| {
                    let text = response::build(&image, &user, &message.chat);
                    api.send(message.text_reply(text))
                        .map_err(|e| e.to_string())
                });
            handle.spawn({
                future.map_err(|_| ()).map(|_| ())
            })
        }
        MessageKind::Text { ref data, .. } => {
            let borrow = db.borrow();
            let user_db = TypedDBWithCF::new(&borrow, user_cf);
            match data.as_ref() {
                "/tiemur_stats" |
                "/tiemur_stats@TiemurBot" => {
                    let text = response::top_tiemurs(user_db);
                    let future = api.send(message_clone.text_reply(text))
                        .map_err(|e| e.to_string());
                    handle.spawn({
                        future.map_err(|_| ()).map(|_| ())
                    })
                }
                _ => (),
            }
        }
        _ => (),
    }
}

fn db_handle(db: &mut DB, cf_name: &str) -> ColumnFamily {
    match db.cf_handle(cf_name) {
        Some(cf) => cf,
        None => db.create_cf(cf_name, &Options::default()).unwrap(),
    }
}
