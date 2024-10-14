use dotenvy::dotenv;
use log::LevelFilter;
use std::env;

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 9999;

#[derive(Clone)]
pub(crate) struct Config {
    pub host: String,
    pub port: u16,
    pub log_level: LevelFilter,
}

impl Config {
    pub(crate) fn from_env() -> Self {
        dotenv().ok();
        // let port: String = env::var("PORT").unwrap_or_else(|_| String::from("9999"));
        // let log_level: String = env::var("RUST_LOG").unwrap_or_else(|_| String::from("info"));

        Self {
            host: env::var("HOST").unwrap_or_else(|_| String::from(DEFAULT_HOST)),
            port: env::var("PORT").unwrap_or_else(|_| DEFAULT_PORT.to_string()).parse().unwrap_or(DEFAULT_PORT),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| String::from("info")).parse().unwrap_or(LevelFilter::Info),
        }
    }
}
