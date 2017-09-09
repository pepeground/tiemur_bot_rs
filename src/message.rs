use telegram_bot::{Api, Message, MessageKind, CanReplySendMessage, CanGetFile};
use tokio_core::reactor::Handle;
use futures::Future;

pub fn process(message: Message, api: Api, handle: &Handle) {
    let clone = message.clone();
    match message.kind {
        MessageKind::Photo { ref data, .. } => {
            let future = api.send(data[0].get_file())
                .map_err(|e| e.to_string())
                .and_then(|file| Ok(file))
                .and_then(|file| file.file_path.ok_or("No file path".to_owned()))
                .and_then(move |file_path| {
                    api.send(clone.text_reply(file_path))
                        .map_err(|e| e.to_string())
                });
            handle.spawn({
                future.map_err(|_| ()).map(|_| ())
            })
        }
        _ => println!("{:?}", message),
    }
}
