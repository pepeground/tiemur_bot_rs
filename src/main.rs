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
#[macro_use]
extern crate lazy_static;

mod message;
pub mod types;
pub mod db;

use std::env;
use futures::{Stream, Future, future};
use tokio_core::reactor::Core;
use telegram_bot::{Api, UpdateKind};
use hyper::Client;
use hyper_rustls::HttpsConnector;
use std::rc::Rc;

#[cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]
fn main() {
    env_logger::init().unwrap();
    let token = env::var("TELEGRAM_TOKEN").unwrap();

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let api = Api::configure(token.as_str()).build(&handle);
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle))
        .build(&handle);

    let future = api.stream().for_each(|update| {
        if let UpdateKind::Message(message) = update.kind {
            let rc_message = Rc::new(message);
            let process_message = message::process(
                message,
                api.clone(),
                client.clone(),
            );
            let select_all = future::select_all(process_message).map_err(|e| error!("{:?}", e.0)).map(|_| ());
            handle.spawn(select_all);
        }
        Ok(())
    });
    core.run(future).unwrap();
}
