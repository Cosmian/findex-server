use dotenv::dotenv;
use log::LevelFilter;
use std::env;

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub log_level: LevelFilter,
}

impl Config {
    pub fn from_env() -> Config {
        dotenv().ok();
        let host: String = env::var("HOST").unwrap_or_else(|_| String::from("0.0.0.0"));
        let port: String = env::var("PORT").unwrap_or_else(|_| String::from("9999"));
        let log_level: String = env::var("RUST_LOG").unwrap_or_else(|_| String::from("info"));

        Config {
            host,
            port: port.parse().unwrap(),
            log_level: log_level.parse().unwrap_or(LevelFilter::Info),
        }
    }
}
