extern crate futures;
extern crate telegram_bot;
extern crate tokio_core;
extern crate hyper;
extern crate hyper_tls;
extern crate image;
extern crate img_hash;

mod message;

use std::env;
use futures::Stream;
use tokio_core::reactor::Core;
use telegram_bot::{Api, UpdateKind};
use hyper::Client;
use hyper_tls::HttpsConnector;
use message::process;

fn main() {
    let token = env::var("TELEGRAM_TOKEN").unwrap();

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let api = Api::configure(token.as_str()).build(&handle);
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);

    let future = api.stream().for_each(|update| {
        if let UpdateKind::Message(message) = update.kind {
            process(message, api.clone(), &handle, client.clone())
        }
        Ok(())
    });
    core.run(future).unwrap();
}
