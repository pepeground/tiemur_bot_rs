mod error;
pub use self::error::{Error, ErrorKind};

use std::cmp::Ordering;
use std::marker::PhantomData;
use telegram_bot::types::{UserId, ChatId, MessageId, User};
use bincode::{serialize, deserialize, Infinite};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sled::{Tree, TreeIter};

#[derive(Serialize, Deserialize, Debug)]
pub struct ImageKey {
    pub chat_id: ChatId,
    pub bytes: Vec<u8>,
}

impl ImageKey {
    pub fn new(chat_id: ChatId, bytes: Vec<u8>) -> Self {
        Self {
            chat_id: chat_id,
            bytes: bytes,
        }
    }
}

impl From<ChatId> for ImageKey {
    fn from(chat_id: ChatId) -> Self {
        Self::new(chat_id, Vec::new())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImageData {
    pub id: MessageId,
    pub user_id: UserId,
    pub date: i64,
}

impl ImageData {
    pub fn new(id: MessageId, user_id: UserId, date: i64) -> Self {
        Self {
            id: id,
            user_id: user_id,
            date: date,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserKey {
    pub chat_id: ChatId,
    pub user_id: Option<UserId>,
}

impl UserKey {
    pub fn new(id: ChatId, user_id: Option<UserId>) -> Self {
        Self {
            chat_id: id,
            user_id: user_id,
        }
    }
}

impl From<ChatId> for UserKey {
    fn from(chat_id: ChatId) -> Self {
        Self::new(chat_id, None)
    }
}

#[derive(Serialize, Deserialize, Debug, Eq)]
pub struct UserData {
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub count: i64,
}

impl From<User> for UserData {
    fn from(user: User) -> Self {
        UserData {
            first_name: user.first_name,
            last_name: user.last_name,
            username: user.username,
            count: 1,
        }
    }
}

impl Ord for UserData {
    fn cmp(&self, user: &UserData) -> Ordering {
        self.count.cmp(&user.count)
    }
}

impl PartialOrd for UserData {
    fn partial_cmp(&self, user: &UserData) -> Option<Ordering> {
        self.count.partial_cmp(&user.count)
    }
}

impl PartialEq for UserData {
    fn eq(&self, other: &UserData) -> bool {
        self.count == other.count
    }
}

pub struct TypedDB<'a, K, V> {
    db: &'a Tree,
    phantom_key: PhantomData<K>,
    phantom_value: PhantomData<V>,
}

impl<'a, K, V> TypedDB<'a, K, V>
where
    K: Serialize,
    V: Serialize + DeserializeOwned,
{
    pub fn new(db: &'a Tree) -> Self {
        Self {
            db: db,
            phantom_key: PhantomData,
            phantom_value: PhantomData,
        }
    }

    pub fn set(&self, key: &K, value: &V) {
        let key = serialize(key, Infinite).unwrap();
        let value = serialize(value, Infinite).unwrap();
        self.db.set(key, value)
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let key = serialize(&key, Infinite).unwrap();
        self.db.get(&key).map(|a| deserialize(&a).unwrap())
    }

    pub fn cas(&self, key: &K, old: Option<&V>, new: Option<&V>) -> Result<(), Option<V>> {
        let key = serialize(key, Infinite).unwrap();
        let old = match old {
            Some(value) => Some(serialize(value, Infinite).unwrap()),
            None => None,
        };
        let new = match new {
            Some(value) => Some(serialize(value, Infinite).unwrap()),
            None => None,
        };
        self.db.cas(key, old, new).map_err(|e| {
            e.map(|a| deserialize(&a).unwrap())
        })
    }

    pub fn iter(&self) -> TypedIterator<'a, K, V> {
        TypedIterator::new(self.db.iter())
    }

    pub fn scan(&self, key: &K) -> TypedIterator<'a, K, V> {
        let key = serialize(key, Infinite).unwrap();
        TypedIterator::new(self.db.scan(&key))
    }
}

pub struct TypedIterator<'a, K, V> {
    db_iterator: TreeIter<'a>,
    phantom_key: PhantomData<K>,
    phantom_value: PhantomData<V>,
}

impl<'a, K, V> TypedIterator<'a, K, V> {
    pub fn new(iterator: TreeIter<'a>) -> Self {
        TypedIterator {
            db_iterator: iterator,
            phantom_key: PhantomData,
            phantom_value: PhantomData,
        }
    }

    fn convert((k, v): (Vec<u8>, Vec<u8>)) -> (Result<K, Error>, Result<V, Error>)
    where
        K: DeserializeOwned,
        V: DeserializeOwned,
    {
        let key = deserialize(&k).map_err(|e| e.into());
        let value = deserialize(&v).map_err(|e| e.into());
        (key, value)
    }
}

impl<'a, K, V> Iterator for TypedIterator<'a, K, V>
where
    K: DeserializeOwned,
    V: DeserializeOwned,
{
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        let tuple = self.db_iterator.next().map(Self::convert);
        match tuple {
            Some((Ok(key), Ok(value))) => Some((key, value)),
            Some((Err(_key), __)) => None,
            Some((_, Err(_value))) => None,
            None => None,
        }
    }
}
