use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use telegram_bot::{Api,CanForwardMessage, CanReplySendMessage, CanGetFile, ToChatRef};
use telegram_bot::types::{Message, MessageKind,  Chat,  ChatRef};
use tokio_core::reactor::Handle;
use futures::{Future, Stream};
use hyper::Client;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use img_hash::{ImageHash, HashType};
use image::load_from_memory;
use rocksdb::{DB, Options, IteratorMode};
use chrono::{DateTime, NaiveDateTime, Utc, Duration};

pub fn process(message: Rc<Message>,
               api: Api,
               handle: &Handle,
               client: Client<HttpsConnector<HttpConnector>>,
               db: Rc<RefCell<DB>>) {
    let clone = message.clone();

    let id = &message.chat.id().to_string();
    let cf_handle;
    {
        cf_handle = db.borrow().cf_handle(&id);
    }
    let cf = match cf_handle {
        Some(cf) => cf,
        None => db.borrow_mut().create_cf(&id, &Options::default()).unwrap(),
    };

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
                .and_then(|res| res.body().concat2().map_err(|e| e.to_string()))
                .and_then(|body| {
                    let image = load_from_memory(&body[..]);
                    let hash = ImageHash::hash(&image.unwrap(), 8, HashType::Gradient);
                    Ok(hash)
                })
                .and_then(move |hash| {
                    let bytes = &hash.bitv.to_bytes()[..];
                    let find = db.borrow()
                        .iterator_cf(cf, IteratorMode::End)
                        .unwrap()
                        .find(|&(ref key, ref _value)| key.as_ref() == bytes);
                    match find {
                        Some(a) => Ok(a),
                        None => {
                            let _ = db.borrow().put_cf(cf, bytes, b"value");
                            Err("new record".to_owned())
                        }
                    }
                })
                .and_then(move |_record| {
                    let (text, chat_ref) = build_text(clone.clone());
                    if let Some(chat_ref) = chat_ref {
                        api.spawn(clone.forward(chat_ref))
                    }
                    api.send(clone.text_reply(text))
                        .map_err(|e| e.to_string())
                });
            handle.spawn({
                future.map_err(|_| ()).map(|_| ())
            })
        }
        _ => println!("{:?}", message),
    }
}


fn build_text(message: Rc<Message>) -> (String, Option<ChatRef>) {
    let first_name: &str = match message.from.as_ref() {
        Some(from) => &from.first_name,
        None => "",
    };

    let naive_time = NaiveDateTime::from_timestamp(message.date, 0);
    let message_time = DateTime::<Utc>::from_utc(naive_time, Utc);
    let now = Utc::now();
    let diff = now.signed_duration_since(message_time);
    let time_ago = distance_of_time_in_words(diff);

    let username = match message.chat {
        Chat::Supergroup(ref supergroup) => supergroup.username.as_ref(),
        _ => None,
    };

    let id = match message.chat {
        Chat::Supergroup(ref supergroup) => Some(supergroup.id.to_chat_ref()),
        Chat::Group(ref group) => Some(group.id.to_chat_ref()),
        _ => None,
    };

    match username {
        Some(ref username) => {
            (format!("Ебать ты Темур! It happened {}, author: {} Proof: https://t.me/{}/{}",
                    time_ago,
                    first_name,
                    username,
                    message.id), None)
        }
        None => {
            (format!("Ебать ты Темур! It happened {}, author: {}",
                    time_ago,
                    first_name), id)
        }
    }
}

fn distance_of_time_in_words(diff: Duration) -> String {
    let diff_num_tuple = (diff.num_weeks(), diff.num_days(), diff.num_hours(), diff.num_minutes());
    match diff_num_tuple {
        (0, 0, 0, 0) => format!("less than a minute"),
        (0, 0, 0, 1) => format!("a minute ago"),
        (0, 0, 0, a) => format!("{} minutes ago", a),
        (0, 0, 1, _) => format!("an hour ago"),
        (0, 0, a, _) => format!("{} hours ago", a),
        (0, 1, _, _) => format!("a day ago"),
        (0, a, _, _) => format!("{} days ago", a),
        (1, _, _, _) => format!("a week ago"),
        (a, _, _, _) => format!("{} weeks ago", a),
    }
}
