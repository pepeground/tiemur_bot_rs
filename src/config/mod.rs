extern crate envy;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub telegram_token: String,
    pub sled_db_dir: String,
    pub dist_ratio: f32,
}

fn config() -> Config {
    match envy::from_env() {
        Ok(config) => config,
        Err(error) => panic!("{:#?}", error)
    }
}

lazy_static! {
    pub static ref CONFIG: Config = config();
}
