use telegram_bot::types::{UserId, MessageId};

#[derive(Serialize, Deserialize)]
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
