mod response;

use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use telegram_bot::{Api, CanGetFile};
use telegram_bot::types::{Message, MessageKind, MessageEntityKind};
use tokio_core::reactor::Handle;
use futures::Future;
use hyper::Client;
use hyper_rustls::HttpsConnector;
use types::Error;
use sled::Tree;

const EXTENSIONS: [&str; 9] = [
    ".jpg",
    ".jpeg",
    ".png",
    ".gif",
    ".bmp",
    ".ico",
    ".tiff",
    ".webp",
    ".ppm",
];

#[cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]
pub fn process(
    message: &Rc<Message>,
    api: Api,
    handle: &Handle,
    client: Client<HttpsConnector>,
    user_db: Rc<RefCell<Tree>>,
    image_db: Rc<RefCell<Tree>>,
) {
    let message_clone = message.clone();
    response::insert_new_chat(message, &image_db.borrow(), &user_db.borrow());

    match message.kind {
        MessageKind::Photo { ref data, .. } => {
            let future = api.send(data[0].get_file())
                .map_err(|e| -> Error { e.into() })
                .and_then(|file| {
                    file.get_url(&env::var("TELEGRAM_TOKEN").unwrap())
                        .ok_or_else(|| "No file path".to_string().into())
                })
                .and_then(move |url| {
                    response::detect_tiemur(&url, client, image_db, user_db, message_clone, api)
                });
            let future = Box::new(future);
            handle.spawn({
                future.map_err(|e| error!("{:?}", e)).map(|_| ())
            })
        }
        MessageKind::Text {
            ref data,
            ref entities,
        } => {
            entities.iter().for_each(|entity| match entity.kind {
                MessageEntityKind::BotCommand => {
                    let command = data.chars()
                        .skip(entity.offset as usize)
                        .take(entity.length as usize)
                        .collect::<String>();
                    match command.as_ref() {
                        "/tiemur_stats" |
                        "/tiemur_stats@TiemurBot" => {
                            let future = response::top_tiemurs(&user_db, &api, message);
                            handle.spawn({
                                future.map_err(|e| error!("{:?}", e)).map(|_| ())
                            })
                        }
                        _ => (),
                    }
                }
                MessageEntityKind::Url => {
                    let url = data.chars()
                        .skip(entity.offset as usize)
                        .take(entity.length as usize)
                        .collect::<String>();
                    if !EXTENSIONS.iter().any(|&a| url.ends_with(a)) {
                        return;
                    }
                    let future = response::detect_tiemur(
                        &url,
                        client.clone(),
                        image_db.clone(),
                        user_db.clone(),
                        message_clone.clone(),
                        api.clone(),
                    );
                    handle.spawn({
                        future.map_err(|e| error!("{:?}", e)).map(|_| ())
                    })
                }
                _ => (),
            })
        }
        _ => (),
    }
}
