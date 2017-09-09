extern crate futures;
extern crate telegram_bot;
extern crate tokio_core;

mod message;

use std::env;
use futures::Stream;
use tokio_core::reactor::Core;
use telegram_bot::{Api, UpdateKind};
use message::process;

fn main() {
    let token = env::var("TELEGRAM_TOKEN").unwrap();

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let api = Api::configure(token.as_str()).build(&handle);

    let future = api.stream().for_each(|update| {
        if let UpdateKind::Message(message) = update.kind {
            process(message, &api)
        }
        Ok(())
    });
    core.run(future).unwrap();
}
