mod response;

use std::env;
use std::rc::Rc;

use telegram_bot::{Api, CanGetFile};
use telegram_bot::types::{Message, MessageKind, MessageEntityKind};
use futures::{Future, future};
use hyper::{Uri, Client};
// use hyper::error::UriError;
use hyper_rustls::HttpsConnector;
use types::{TiemurFuture, Error};

#[cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]
pub fn process(
    message: &Rc<Message>,
    api: Api,
    client: Client<HttpsConnector>,
) -> Vec<TiemurFuture<()>> {
    let message_clone = message.clone();
    response::insert_new_chat(message);

    match message.kind {
        MessageKind::Photo { ref data, .. } => {
            let future = api.send(data[0].get_file())
                .map_err(|e| -> Error { e.into() })
                .and_then(|file| {
                    file.get_url(&env::var("TELEGRAM_TOKEN").unwrap())
                        .ok_or_else(|| "No file path".to_string().into())
                })
                .and_then(|url| url.parse::<Uri>().map_err(|a| a.into()))
                .and_then(move |url| {
                    response::detect_tiemur(url, client, message_clone, api)
                }).map(|_| ());
            vec![Box::new(future)]
        }
        MessageKind::Text {
            ref data,
            ref entities,
        } => {
            if entities.is_empty() {
                return vec![Box::new(future::ok(()))];
            }
            entities.iter().map(|entity| match entity.kind {
                MessageEntityKind::BotCommand => {
                    let command = data.chars()
                        .skip(entity.offset as usize)
                        .take(entity.length as usize)
                        .collect::<String>();
                    match command.as_ref() {
                        "/tiemur_stats" |
                        "/tiemur_stats@TiemurBot" => {
                            let future = response::top_tiemurs(&api, message).map(|_| ()).from_err();
                            Box::new(future)
                        }
                        _ => Box::new(future::ok(())) as TiemurFuture<_>
                    }
                }
                MessageEntityKind::Url => {
                    let url = data.chars()
                        .skip(entity.offset as usize)
                        .take(entity.length as usize)
                        .collect::<String>().parse::<Uri>();
                    match url {
                        Ok(url) => {
                            let future = response::detect_tiemur(
                                url,
                                client.clone(),
                                message_clone.clone(),
                                api.clone(),
                            ).map(|_| ());
                            Box::new(future) as TiemurFuture<_>
                        }
                        Err(err) => {
                            let future = future::err(err.into());
                            Box::new(future) as TiemurFuture<_>
                        }
                    }
                }
                _ => Box::new(future::ok(())) as TiemurFuture<_>
            }).collect()
        }
        _ => vec![Box::new(future::ok(()))]
    }
}
