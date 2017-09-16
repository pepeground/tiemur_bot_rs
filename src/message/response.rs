use std::collections::BinaryHeap;
use std::rc::Rc;
use chrono::{DateTime, NaiveDateTime, Utc, Duration};
use telegram_bot::types::{Chat, UserId, Message};
use types::{Image, User, UserContent, TypedDBWithCF};
use rocksdb::IteratorMode;
use img_hash::ImageHash;

pub fn build(image: &Image, user: &UserContent, chat: &Chat) -> String {
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


pub fn find_tiemur(user_db: TypedDBWithCF<UserId, UserContent>,
                   image_db: TypedDBWithCF<Vec<u8>, Image>,
                   hash: ImageHash,
                   message: Rc<Message>)
                   -> Result<(Rc<Message>, Image, UserContent), String> {
    let bytes = hash.bitv.to_bytes();
    let find = image_db.iterator(IteratorMode::End)
        .unwrap()
        .find(|&(ref key, ref _value)| key == &bytes);
    let telegram_user = message.from.clone().ok_or("user empty".to_string())?;
    let mut user: User = telegram_user.into();
    let user_row = user_db.get(&user.0).unwrap();
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
            Err("new record".to_string())
        }
    }
}

pub fn top_tiemurs(user_db: TypedDBWithCF<UserId, UserContent>) -> String {
    let mut users: BinaryHeap<_> = user_db.iterator(IteratorMode::End)
        .unwrap()
        .map(|(_key, value)| value)
        .take(5)
        .collect();
    let top = vec![users.pop(), users.pop(), users.pop(), users.pop(), users.pop()];
    top.into_iter().map(|u| u.map_or("".to_string(), |u| u.first_name)).collect()
}
