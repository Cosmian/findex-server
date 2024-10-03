mod api;
mod common;
mod routes;
mod services;
mod middlewares;

use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpServer,
};
use common::Config;
use log::info;
use actix_cors::Cors;
use middlewares::AuthTransformer;


use std::{io::Result, sync::Mutex};

use routes::health_get;

#[actix_web::main]
async fn main() -> Result<()> {
    let config = web::Data::new(Config::from_env());
    let config_for_bind = config.clone();
    env_logger::Builder::new()
        .filter(None, config.log_level)
        .init();

    info!("Loaded env, starting Http server ...");

    HttpServer::new(move || {
        App::new()
            .wrap(AuthTransformer::new(
                true, true
            ))
            .wrap(Cors::permissive())
            .wrap(Logger::default())
            // .app_data(findex_data.clone())
            .route("/health", web::get().to(health_get))
            .app_data(config.clone())
    })
    .bind((config_for_bind.get_ref().host.clone(), config_for_bind.port))?
    .run()
    .await
}
