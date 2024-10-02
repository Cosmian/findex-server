mod api;
mod common;

use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use api::health;
use common::env::Config;
use env_logger::Env;
use log::info;
use std::{io::Result, sync::Mutex};

#[actix_web::main]
async fn main() -> Result<()> {
    let config = Config::from_env();
    env_logger::Builder::new()
        .filter(None, config.log_level)
        .init();

    info!("Loaded env, starting Http server ...");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            // .app_data(findex_data.clone())
            .service(health)
    })
    .bind((config.host, config.port))?
    .run()
    .await
}
