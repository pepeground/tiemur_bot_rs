use telegram_bot::{Api, Message, MessageKind, CanReplySendMessage, CanGetFile};
use tokio_core::reactor::Handle;
use futures::Future;
use std::env;

pub fn process(message: Message, api: Api, handle: &Handle) {
    let clone = message.clone();
    match message.kind {
        MessageKind::Photo { ref data, .. } => {
            let future = api.send(data[0].get_file())
                .map_err(|e| e.to_string())
                .and_then(|file| {
                    file.get_url(&env::var("TELEGRAM_TOKEN").unwrap())
                        .ok_or("No file path".to_owned())
                })
                .and_then(move |url| {
                    api.send(clone.text_reply(url))
                        .map_err(|e| e.to_string())
                });
            handle.spawn({
                future.map_err(|_| ()).map(|_| ())
            })
        }
        _ => println!("{:?}", message),
    }
}
