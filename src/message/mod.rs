mod response;

use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use telegram_bot::{Api, CanGetFile};
use telegram_bot::types::{Message, MessageKind};
use tokio_core::reactor::Handle;
use futures::Future;
use hyper::Client;
use hyper_rustls::HttpsConnector;
use rocksdb::{DB, Options, ColumnFamily};
use types::Error;

pub fn process(message: Rc<Message>,
               api: Api,
               handle: &Handle,
               client: Client<HttpsConnector>,
               db: Rc<RefCell<DB>>)
               -> Result<(), Error> {
    let message_clone = message.clone();

    let mut cf_name = message.chat.id().to_string();
    let image_cf = db_handle(&mut db.borrow_mut(), &cf_name)?;
    cf_name.push_str("_users");
    let user_cf = db_handle(&mut db.borrow_mut(), &cf_name)?;

    match message.kind {
        MessageKind::Photo { ref data, .. } => {
            let future = api.send(data[0].get_file())
                .map_err(|e| -> Error { e.into() })
                .and_then(|file| {
                    file.get_url(&env::var("TELEGRAM_TOKEN").unwrap())
                        .ok_or("No file path".to_string().into())
                })
                .and_then(move |url| {
                    response::detect_tiemur(url, client, db, image_cf, user_cf, message_clone, api)
                });
            handle.spawn({
                future.map_err(|e| error!("{:?}", e)).map(|_| ())
            })
        }
        MessageKind::Text { ref data, .. } => {
            match data.as_ref() {
                "/tiemur_stats" |
                "/tiemur_stats@TiemurBot" => {
                    let future = response::top_tiemurs(db, user_cf, api, message_clone);
                    handle.spawn({
                        future.map_err(|e| error!("{:?}", e)).map(|_| ())
                    })
                }
                _ => (),
            }
        }
        _ => (),
    };
    Ok(())
}

fn db_handle(db: &mut DB, cf_name: &str) -> Result<ColumnFamily, Error> {
    match db.cf_handle(cf_name) {
        Some(cf) => Ok(cf),
        None => Ok(db.create_cf(cf_name, &Options::default())?),
    }
}
