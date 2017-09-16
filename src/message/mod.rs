mod response;

use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use telegram_bot::{Api, CanReplySendMessage, CanGetFile};
use telegram_bot::types::{Message, MessageKind};
use tokio_core::reactor::Handle;
use futures::{Future, Stream, IntoFuture};
use hyper::Client;
use hyper::client::HttpConnector;
use hyper::error::UriError;
use hyper_tls::HttpsConnector;
use img_hash::{ImageHash, HashType};
use image::load_from_memory as load;
use rocksdb::{DB, Options, ColumnFamily};
use types::{TypedDBWithCF, Error};

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
                .map_err(|e| -> Error { e.into() })
                .and_then(|file| {
                    file.get_url(&env::var("TELEGRAM_TOKEN").unwrap())
                        .ok_or("No file path".to_string().into())
                })
                .and_then(move |url| {
                    detect_tiemur(url, client, db, image_cf, user_cf, message_clone, api)
                });
            handle.spawn({
                future.map_err(|_| ()).map(|_| ())
            })
        }
        MessageKind::Text { ref data, .. } => {
            match data.as_ref() {
                "/tiemur_stats" |
                "/tiemur_stats@TiemurBot" => {
                    let borrow = db.borrow();
                    let user_db = TypedDBWithCF::new(&borrow, user_cf);
                    let text = response::top_tiemurs(user_db);
                    let future = api.send(message_clone.text_reply(text));
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

fn detect_tiemur(url: String,
                 client: Client<HttpsConnector<HttpConnector>>,
                 db: Rc<RefCell<DB>>,
                 image_cf: ColumnFamily,
                 user_cf: ColumnFamily,
                 message: Rc<Message>,
                 api: Api)
                 -> Box<Future<Item = Message, Error = Error>> {
    let future = url.parse()
        .map_err(|e: UriError| -> Error { e.into() })
        .into_future()
        .and_then(move |url| client.get(url).map_err(From::from))
        .and_then(|res| res.body().concat2().map_err(From::from))
        .and_then(|ref body| load(body).map_err(From::from))
        .and_then(|ref image| Ok(ImageHash::hash(image, 8, HashType::Gradient)))
        .and_then(move |ref hash| {
            let borrow = db.borrow();
            let image_db = TypedDBWithCF::new(&borrow, image_cf);
            let user_db = TypedDBWithCF::new(&borrow, user_cf);
            response::find_tiemur(&user_db, &image_db, hash, message)
        })
        .and_then(move |(ref message, ref image, ref user)| {
            let text = response::build(image, user, &message.chat);
            api.send(message.text_reply(text)).map_err(From::from)
        });
    Box::new(future)
}
