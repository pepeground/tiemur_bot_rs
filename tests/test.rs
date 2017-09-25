extern crate tiemur_bot_rs;
extern crate sled;

use tiemur_bot_rs::types::{UserData, TypedDBWithCF};
use sled::Config;

#[test]
fn test() {
    let db_path = "/tmp/test_tiemur_bot_db/test".to_string();
    let user_db = Config::default().path(db_path).tree();
    let user_db = TypedDBWithCF::<i64, UserData>::new(&user_db);
    let user = UserData {
        first_name: "Test".to_string(),
        last_name: None,
        username: None,
        count: 5,
    };
    let _ = user_db.put(&1, &user);
    let user_get = user_db.get(&1).unwrap().unwrap();
    assert_eq!(user_get, user);
}
