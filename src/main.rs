extern crate futures;
extern crate telegram_bot;
extern crate tokio_core;
extern crate hyper;
extern crate hyper_rustls;
extern crate image;
extern crate img_hash;
extern crate chrono;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bincode;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
extern crate sled;

mod message;
pub mod types;

use std::env;
use std::path::Path;
use futures::{Stream, Future, future};
use tokio_core::reactor::Core;
use telegram_bot::{Api, UpdateKind};
use hyper::Client;
use hyper_rustls::HttpsConnector;
use std::rc::Rc;
use sled::Config;

#[cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]
fn main() {
    env_logger::init().unwrap();
    let token = env::var("TELEGRAM_TOKEN").unwrap();
    let db_path = env::var("SLED_DB").unwrap();

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let api = Api::configure(token.as_str()).build(&handle);
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle))
        .build(&handle);

    let mut path = Path::new(&db_path).to_path_buf();
    path.push("image_db.db");
    let image_path = path.as_path().to_str().unwrap().to_string();
    let image_db = Config::default().path(image_path).tree();
    path.pop();
    path.push("user_db.db");
    let user_path = path.as_path().to_str().unwrap().to_string();
    let user_db = Config::default().path(user_path).tree();
    let ref_user_db = Rc::new(user_db);
    let ref_image_db = Rc::new(image_db);

    let future = api.stream().for_each(|update| {
        if let UpdateKind::Message(message) = update.kind {
            let rc_message = Rc::new(message);
            let process_message = message::process(
                &rc_message,
                api.clone(),
                client.clone(),
                ref_user_db.clone(),
                ref_image_db.clone(),
            );
            let select_all = future::select_all(process_message).map_err(|e| error!("{:?}", e.0)).map(|_| ());
            handle.spawn(select_all);
        }
        Ok(())
    });
    core.run(future).unwrap();
}
