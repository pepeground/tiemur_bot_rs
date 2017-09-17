extern crate tiemur_bot_rs;
extern crate rocksdb;

use tiemur_bot_rs::types::{UserContent, TypedDBWithCF};
use rocksdb::{DB, Options};

#[test]
fn test() {
    let db_path = "/tmp/test_tiemur_bot_db";
    let cfs = DB::list_cf(&Options::default(), &db_path);
    let mut db = match cfs {
        Ok(cfs) => {
            let cfs_str: Vec<_> = cfs.iter().map(|a| a.as_str()).collect();
            DB::open_cf(&Options::default(), &db_path, &cfs_str).unwrap()
        }
        Err(_) => DB::open_default(&db_path).unwrap(),
    };
    let user_cf = match db.cf_handle("user") {
        Some(cf) => cf,
        None => db.create_cf("user", &Options::default()).unwrap(),
    };
    let user_db = TypedDBWithCF::<i64, UserContent>::new(&db, user_cf);
    let user = UserContent {
        first_name: "Test".to_string(),
        last_name: None,
        username: None,
        count: 5,
    };
    let _ = user_db.put(&1, &user);
    let user_get = user_db.get(&1).unwrap().unwrap();
    assert_eq!(user_get, user);
}
