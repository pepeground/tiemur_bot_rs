extern crate futures;
extern crate telegram_bot;
extern crate tokio_core;

use std::env;
use futures::Stream;
use tokio_core::reactor::Core;
use telegram_bot::{Api, UpdateKind, CanReplySendMessage};

fn main() {
    let token = env::var("TELEGRAM_TOKEN").unwrap();

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let api = Api::configure(token.as_str()).build(&handle);

    let future = api.stream().for_each(|update| {
        if let UpdateKind::Message(message) = update.kind {
            api.spawn(message.text_reply("Hello World"));
        }
        Ok(())
    });
    core.run(future).unwrap();
}
