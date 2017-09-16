use std::collections::BinaryHeap;
use std::rc::Rc;
use std::cell::RefCell;

use chrono::{DateTime, NaiveDateTime, Utc, Duration};
use telegram_bot::{Api, CanReplySendMessage, TelegramFuture};
use telegram_bot::types::{Chat, UserId, Message};
use types::{Image, User, UserContent, TypedDBWithCF, Error};
use rocksdb::{IteratorMode, DB, ColumnFamily};
use img_hash::{ImageHash, HashType};
use hyper::Client;
use hyper::client::HttpConnector;
use hyper::error::UriError;
use hyper_tls::HttpsConnector;
use futures::{Future, IntoFuture, Stream};
use image::load_from_memory;

pub fn detect_tiemur(url: String,
                     client: Client<HttpsConnector<HttpConnector>>,
                     db: Rc<RefCell<DB>>,
                     image_cf: ColumnFamily,
                     user_cf: ColumnFamily,
                     message: Rc<Message>,
                     api: Api)
                     -> Box<Future<Item = Message, Error = Error>> {
    let future = url.parse()
        .map_err(|e: UriError| -> Error { e.into() })
        .into_future()
        .and_then(move |url| client.get(url).map_err(From::from))
        .and_then(|res| res.body().concat2().map_err(From::from))
        .and_then(|ref body| load_from_memory(body).map_err(From::from))
        .and_then(|ref image| Ok(ImageHash::hash(image, 8, HashType::Gradient)))
        .and_then(move |ref hash| {
            let borrow = db.borrow();
            let image_db = TypedDBWithCF::new(&borrow, image_cf);
            let user_db = TypedDBWithCF::new(&borrow, user_cf);
            find_tiemur(&user_db, &image_db, hash, message)
        })
        .and_then(move |(ref message, ref image, ref user)| {
            let text = build(image, user, &message.chat);
            api.send(message.text_reply(text)).map_err(From::from)
        });
    Box::new(future)
}

fn find_tiemur(user_db: &TypedDBWithCF<UserId, UserContent>,
               image_db: &TypedDBWithCF<Vec<u8>, Image>,
               hash: &ImageHash,
               message: Rc<Message>)
               -> Result<(Rc<Message>, Image, UserContent), Error> {
    let bytes = hash.bitv.to_bytes();
    let find = image_db.iterator(IteratorMode::End)?
        .find(|&(ref key, ref _value)| key == &bytes);
    let telegram_user = message.from.clone().ok_or("user empty".to_string())?;
    let mut user: User = telegram_user.into();
    let user_row = user_db.get(&user.0)?;
    if user_row.is_none() {
        let _ = user_db.put(&user.0, &user.1);
    }
    match find {
        Some((_key, image)) => {
            if let Some(user_row) = user_row {
                user.1 = UserContent { count: user_row.count + 1, ..user.1 };
                let _ = user_db.put(&user.0, &user.1);
            }
            Ok((message, image, user.1))
        }
        None => {
            let image = Image::new(message.id, user.0, message.date);
            let _ = image_db.put(&bytes, &image);
            Err("new record".to_string().into())
        }
    }
}

fn build(image: &Image, user: &UserContent, chat: &Chat) -> String {
    let first_name = &user.first_name;

    let naive_time = NaiveDateTime::from_timestamp(image.date, 0);
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
                    image.id)
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

pub fn top_tiemurs(db: Rc<RefCell<DB>>,
                   user_cf: ColumnFamily,
                   api: Api,
                   message: Rc<Message>)
                   -> TelegramFuture<Message> {
    let borrow = db.borrow();
    let user_db = TypedDBWithCF::<UserId, UserContent>::new(&borrow, user_cf);
    let mut users: BinaryHeap<_> = user_db.iterator(IteratorMode::End)
        .unwrap()
        .map(|(_key, value)| value)
        .collect();
    let top = vec![users.pop(), users.pop(), users.pop(), users.pop(), users.pop()];
    let mut text = "Топ Темуров:".to_string();
    for user in top {
        match user {
            Some(u) => {
                text.push_str("\n");
                text.push_str(&u.first_name);
                text.push_str(" => ");
                text.push_str(&u.count.to_string());
            }
            _ => (),
        }
    }
    api.send(message.text_reply(text))
}
