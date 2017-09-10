use telegram_bot::types::{UserId, MessageId};
use telegram_bot::types::User as TUser;

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
pub struct User {
    #[serde(skip, default="Hack::hack")]
    pub id: UserId,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub count: i64,
}

impl From<TUser> for User {
    fn from(user: TUser) -> Self {
        User {
            id: user.id,
            first_name: user.first_name,
            last_name: user.last_name,
            username: user.username,
            count: 0
        }
    }
}

trait Hack {
    fn hack() -> Self;
}

impl Hack for UserId {
    fn hack() -> Self {
        0.into()
    }
}
