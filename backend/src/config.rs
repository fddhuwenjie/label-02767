use std::env;

#[allow(dead_code)]
pub struct Config {
    pub database_url: String,
    pub secret_key: String,
    pub admin_username: String,
    pub admin_password: String,
}

#[allow(dead_code)]
impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "mysql://root:password@localhost:3306/rocket_db".to_string()),
            secret_key: env::var("SECRET_KEY")
                .unwrap_or_else(|_| "super_secret_key_change_in_production".to_string()),
            admin_username: env::var("ADMIN_USERNAME")
                .unwrap_or_else(|_| "admin".to_string()),
            admin_password: env::var("ADMIN_PASSWORD")
                .unwrap_or_else(|_| "admin123".to_string()),
        }
    }
}

lazy_static::lazy_static! {
    #[allow(dead_code)]
    pub static ref CONFIG: Config = Config::from_env();
}
