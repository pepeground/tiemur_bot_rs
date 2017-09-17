extern crate futures;
extern crate telegram_bot;
extern crate tokio_core;
extern crate hyper;
extern crate hyper_tls;
extern crate image;
extern crate img_hash;
extern crate rocksdb;
extern crate chrono;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bincode;
#[macro_use]
extern crate log;
extern crate env_logger;

mod message;
pub mod types;

use std::env;
use futures::Stream;
use tokio_core::reactor::Core;
use telegram_bot::{Api, UpdateKind};
use hyper::Client;
use hyper_tls::HttpsConnector;
use message::process;
use rocksdb::{DB, Options};
use std::rc::Rc;
use std::cell::RefCell;

pub fn start() {
    let _ = env_logger::init().unwrap();
    let token = env::var("TELEGRAM_TOKEN").unwrap();
    let db_path = env::var("DB_PATH").unwrap();

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let api = Api::configure(token.as_str()).build(&handle);
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);

    let cfs = DB::list_cf(&Options::default(), &db_path);
    let db = match cfs {
        Ok(cfs) => {
            let cfs_str: Vec<_> = cfs.iter().map(|a| a.as_str()).collect();
            DB::open_cf(&Options::default(), &db_path, &cfs_str).unwrap()
        }
        Err(_) => DB::open_default(&db_path).unwrap(),
    };

    let rc_db = Rc::new(RefCell::new(db));

    let future = api.stream().for_each(|update| {
        if let UpdateKind::Message(message) = update.kind {
            let rc_message = Rc::new(message);
            process(rc_message,
                    api.clone(),
                    &handle,
                    client.clone(),
                    rc_db.clone()).map_err(|e| e.to_string())?
        }
        Ok(())
    });
    core.run(future).unwrap();
}
