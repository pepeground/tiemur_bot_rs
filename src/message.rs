use telegram_bot::{Api, Message, MessageKind, CanReplySendMessage, CanGetFile};
use tokio_core::reactor::Handle;
use futures::{Future, Stream};
use std::env;
use hyper::Client;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use img_hash::{ImageHash, HashType};
use image::load_from_memory;

pub fn process(message: Message,
               api: Api,
               handle: &Handle,
               client: Client<HttpsConnector<HttpConnector>>) {
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
                    client.get(url.parse().unwrap()).map_err(|e| e.to_string())
                })
                .and_then(|res| {
                    res.body().concat2().map_err(|e| e.to_string())
                })
                .and_then(|body| {
                    let image = load_from_memory(&body[..]);
                    let hash = ImageHash::hash(&image.unwrap(), 8,
                                               HashType::Gradient);
                    Ok(hash)
                })
                .and_then(move |hash| {
                    api.send(clone.text_reply(hash.to_base64()))
                        .map_err(|e| e.to_string())
                });
            handle.spawn({
                future.map_err(|_| ()).map(|_| ())
            })
        }
        _ => println!("{:?}", message),
    }
}
