extern crate futures;
extern crate telegram_bot;
extern crate tokio_core;
extern crate hyper;
extern crate hyper_tls;
extern crate image;
extern crate img_hash;
extern crate rocksdb;
extern crate chrono;

mod message;

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

fn main() {
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
            println!("{:?}", cfs_str);
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
                    rc_db.clone())
        }
        Ok(())
    });
    core.run(future).unwrap();
}
