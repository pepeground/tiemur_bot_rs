use config::CONFIG;
use types::{ImageKey, ImageData, UserKey, UserData, UrlKey, TypedDB};
use sled::{Config, Tree};
use std::path::Path;

type ImageDB<'a> = TypedDB<'a, ImageKey, Option<ImageData>>;
type UrlDB<'a> = TypedDB<'a, UrlKey, ImageData>;
type UserDB<'a> = TypedDB<'a, UserKey, Option<UserData>>;

fn db_path(db_name: &str) -> String {
    let path = Path::new(&CONFIG.sled_db_dir).join(db_name);
    path.as_path().to_str().unwrap().to_string()
}

lazy_static! {
    static ref IMAGE_TREE: Tree = Config::default().path(db_path("image.db")).tree();
    static ref USER_TREE: Tree = Config::default().path(db_path("user.db")).tree();
    static ref URL_TREE: Tree = Config::default().path(db_path("url.db")).tree();
    pub static ref IMAGE_DB: ImageDB<'static> = ImageDB::new(&IMAGE_TREE);
    pub static ref USER_DB: UserDB<'static> = UserDB::new(&USER_TREE);
    pub static ref URL_DB: UrlDB<'static> = UrlDB::new(&URL_TREE);
}
