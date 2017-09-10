use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use telegram_bot::{Api, CanReplySendMessage, CanGetFile};
use telegram_bot::types::{Message, MessageKind, Chat, MessageId, UserId};
use tokio_core::reactor::Handle;
use futures::{Future, Stream};
use hyper::Client;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use hyper::error::UriError;
use img_hash::{ImageHash, HashType};
use image::load_from_memory as load;
use rocksdb::{DB, Options, IteratorMode};
use chrono::{DateTime, NaiveDateTime, Utc, Duration};
use bincode::{serialize, deserialize, Infinite};

#[derive(Serialize, Deserialize)]
struct ImageRow {
    id: MessageId,
    user_id: UserId,
    date: i64,
}

pub fn process(message: Rc<Message>,
               api: Api,
               handle: &Handle,
               client: Client<HttpsConnector<HttpConnector>>,
               db: Rc<RefCell<DB>>) {
    let clone = message.clone();
    let clone1 = message.clone();

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
                .and_then(|url| url.parse().map_err(|e: UriError| e.to_string()))
                .and_then(move |url| client.get(url).map_err(|e| e.to_string()))
                .and_then(|res| res.body().concat2().map_err(|e| e.to_string()))
                .and_then(|body| load(&body[..]).map_err(|e| e.to_string()))
                .and_then(|image| Ok(ImageHash::hash(&image, 8, HashType::Gradient)))
                .and_then(move |hash| {
                    let bytes = &hash.bitv.to_bytes()[..];
                    let find = db.borrow()
                        .iterator_cf(cf, IteratorMode::End)
                        .unwrap()
                        .find(|&(ref key, ref _value)| key.as_ref() == bytes);
                    match find {
                        Some((_key, value)) => {
                            let row: ImageRow = deserialize(&*value).unwrap();
                            Ok(row)
                        }
                        None => {
                            let row = build_row(clone);
                            let value = serialize(&row, Infinite).unwrap();
                            let _ = db.borrow().put_cf(cf, bytes, &value);
                            Err("new record".to_string())
                        }
                    }
                })
                .and_then(move |record| {
                    let text = build_respone(record, &clone1.chat);
                    api.send(clone1.text_reply(text))
                        .map_err(|e| e.to_string())
                });
            handle.spawn({
                future.map_err(|_| ()).map(|_| ())
            })
        }
        _ => println!("{:?}", message),
    }
}

fn build_row(message: Rc<Message>) -> ImageRow {
    let user = message.from.clone().unwrap();
    ImageRow {
        id: message.id,
        user_id: user.id,
        date: message.date,
    }
}

fn build_respone(image_record: ImageRow, chat: &Chat) -> String {
    let first_name = image_record.user_id;

    let naive_time = NaiveDateTime::from_timestamp(image_record.date, 0);
    let message_time = DateTime::<Utc>::from_utc(naive_time, Utc);
    let now = Utc::now();
    let diff = now.signed_duration_since(message_time);
    let time_ago = distance_of_time_in_words(diff);

    let username = match *chat {
        Chat::Supergroup(ref supergroup) => supergroup.username.as_ref(),
        _ => None,
    };

    match username {
        Some(ref username) => {
            format!("Ебать ты Темур! It happened {}, author: {} Proof: https://t.me/{}/{}",
                    time_ago,
                    first_name,
                    username,
                    image_record.id)
        }
        None => {
            format!("Ебать ты Темур! It happened {}, author: {}",
                    time_ago,
                    first_name)
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
