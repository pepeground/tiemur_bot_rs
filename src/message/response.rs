use std::collections::BinaryHeap;
use std::rc::Rc;
use std::cell::RefCell;

use chrono::{DateTime, NaiveDateTime, Utc, Duration};
use telegram_bot::{Api, CanReplySendMessage, TelegramFuture};
use telegram_bot::types::{Chat, Message};
use types::{ImageKey, ImageData, UserKey, UserData, TypedDB, Error};
use img_hash::{ImageHash, HashType};
use hyper::Client;
use hyper::error::UriError;
use hyper_rustls::HttpsConnector;
use futures::{Future, IntoFuture, Stream};
use image::load_from_memory;
use sled::Tree;

type ImageDB<'a> = TypedDB<'a, ImageKey, Option<ImageData>>;
type UserDB<'a> = TypedDB<'a, UserKey, Option<UserData>>;

pub fn insert_new_chat(message: &Message, image_db: &Tree, user_db: &Tree) {
    let key: ImageKey = message.chat.id().into();
    let _ = ImageDB::new(&image_db).cas(&key, None, Some(&None));
    let key: UserKey = message.chat.id().into();
    let _ = UserDB::new(&user_db).cas(&key, None, Some(&None));
}

pub fn detect_tiemur(url: String,
                     client: Client<HttpsConnector>,
                     image_db: Rc<RefCell<Tree>>,
                     user_db: Rc<RefCell<Tree>>,
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
            let user_borrow = user_db.borrow();
            let image_borrow = image_db.borrow();
            let image_db = ImageDB::new(&image_borrow);
            let user_db = UserDB::new(&user_borrow);
            find_tiemur(&user_db, &image_db, hash, message)
        })
        .and_then(move |(ref message, ref image, ref user)| {
            let text = build(image, user, &message.chat);
            api.send(message.text_reply(text)).map_err(From::from)
        });
    Box::new(future)
}

fn find_tiemur(user_db: &UserDB,
               image_db: &ImageDB,
               hash: &ImageHash,
               message: Rc<Message>)
               -> Result<(Rc<Message>, ImageData, UserData), Error> {
    let bytes = hash.bitv.to_bytes();
    let find = image_db.iter()
        .find(|&(ref key, ref _value)| &key.bytes == &bytes);
    let telegram_user = message.from.clone().ok_or("user empty".to_string())?;
    let user_id = telegram_user.id;
    match find {
        Some((_key, Some(image))) => {
            let user_key = UserKey::new(message.chat.id(), Some(user_id));
            let mut user_data: Option<UserData> = Some(telegram_user.into());
            user_data = match user_db.cas(&user_key, None, Some(&user_data)) {
                Err(Some(Some(user_row))) => {
                    user_data.as_mut().unwrap().count = user_row.count + 1;
                    let _ = user_db.set(&user_key, &user_data);
                    user_data
                }
                Ok(_) | Err(_) => user_data,
            };
            Ok((message, image, user_data.unwrap()))
        }
        Some((_, None)) | None => {
            let image = ImageData::new(message.id, user_id, message.date);
            let key = ImageKey::new(message.chat.id(), bytes);
            image_db.set(&key, &Some(image));
            Err("new record".to_string().into())
        }
    }
}

fn build(image: &ImageData, user: &UserData, chat: &Chat) -> String {
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

pub fn top_tiemurs(user_db: Rc<RefCell<Tree>>, api: Api, message: Rc<Message>) -> TelegramFuture<Message> {
    let borrow = user_db.borrow();
    let user_db = UserDB::new(&borrow);
    let iterator = user_db.iter();
    let mut users: BinaryHeap<_> = iterator.map(|(_key, value)| value)
        .collect();
    let top = vec![users.pop(), users.pop(), users.pop(), users.pop(), users.pop()];
    let mut text = "Топ Темуров:".to_string();
    for user in top {
        match user {
            Some(Some(u)) => {
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
