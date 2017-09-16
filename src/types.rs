use std::cmp::Ordering;
use std::marker::PhantomData;
use telegram_bot::types::{UserId, MessageId};
use telegram_bot::types::User as TUser;
use rocksdb::{DB, ColumnFamily, Error, IteratorMode, DBIterator};
use bincode::{serialize, deserialize, Infinite};
use serde::Serialize;
use serde::de::DeserializeOwned;

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

pub struct TypedDBWithCF<'a, K, V> {
    db: &'a DB,
    cf: ColumnFamily,
    phantom_key: PhantomData<K>,
    phantom_value: PhantomData<V>,
}

impl<'a, K, V> TypedDBWithCF<'a, K, V>
    where K: Serialize,
          V: Serialize + DeserializeOwned
{
    pub fn new(db: &'a DB, cf: ColumnFamily) -> Self {
        TypedDBWithCF {
            db: db,
            cf: cf,
            phantom_key: PhantomData,
            phantom_value: PhantomData,
        }
    }

    pub fn put(&self, key: &K, value: &V) -> Result<(), Error> {
        let key = serialize(key, Infinite).unwrap();
        let value = serialize(value, Infinite).unwrap();
        self.db.put_cf(self.cf, &key, &value)
    }

    pub fn get(&self, key: &K) -> Result<Option<V>, Error> {
        let key = serialize(&key, Infinite).unwrap();
        let value = self.db.get_cf(self.cf, &key);
        value.map(|a| a.map(|b| deserialize(&b).unwrap()))
    }

    pub fn iterator(&self, mode: IteratorMode) -> Result<TypedIterator<K, V>, Error> {
        self.db.iterator_cf(self.cf, mode).map(|a| TypedIterator::new(a))
    }
}

pub struct TypedIterator<K, V> {
    db_iterator: DBIterator,
    phantom_key: PhantomData<K>,
    phantom_value: PhantomData<V>,
}

impl<K, V> TypedIterator<K, V> {
    pub fn new(iterator: DBIterator) -> Self {
        TypedIterator {
            db_iterator: iterator,
            phantom_key: PhantomData,
            phantom_value: PhantomData,
        }
    }

    fn convert((k, v): (Box<[u8]>, Box<[u8]>)) -> (K, V)
        where K: DeserializeOwned,
              V: DeserializeOwned
    {
        let key = deserialize(&*k).unwrap();
        let value = deserialize(&*v).unwrap();
        (key, value)
    }
}

impl<K, V> Iterator for TypedIterator<K, V>
    where K: DeserializeOwned,
          V: DeserializeOwned
{
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.db_iterator.next().map(Self::convert)
    }
}
