use telegram_bot::types::{UserId, MessageId};
use telegram_bot::types::User as TUser;
use std::cmp::Ordering;

#[derive(Serialize, Deserialize, Debug)]
pub struct Image {
    pub id: MessageId,
    pub user_id: UserId,
    pub date: i64,
}

impl Image {
    pub fn new(id: MessageId, user_id: UserId, date: i64) -> Image {
        Image {
            id: id,
            user_id: user_id,
            date: date,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User(pub UserId, pub UserContent);

#[derive(Serialize, Deserialize, Debug, Eq)]
pub struct UserContent {
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub count: i64,
}

impl From<TUser> for User {
    fn from(user: TUser) -> Self {
        let content = UserContent {
            first_name: user.first_name,
            last_name: user.last_name,
            username: user.username,
            count: 0,
        };
        User(user.id, content)
    }
}

impl Ord for UserContent {
    fn cmp(&self, user: &UserContent) -> Ordering {
        self.count.cmp(&user.count)
    }
}

impl PartialOrd for UserContent {
    fn partial_cmp(&self, user: &UserContent) -> Option<Ordering> {
        self.count.partial_cmp(&user.count)
    }
}

impl PartialEq for UserContent {
    fn eq(&self, other: &UserContent) -> bool {
        self.count == other.count
    }
}
