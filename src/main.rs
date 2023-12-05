mod api;
mod findex_backend;

use actix_web::{middleware::Logger, web::Data, App, HttpServer};
use api::fetch;
use cloudproof_findex::BackendConfiguration;
use env_logger::Env;
use findex_backend::SqliteFindexBackend;
use std::{io::Result, sync::Mutex};

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
    // Instantiate Findex Backend
    let findex_config = BackendConfiguration::Sqlite(
        "./data/entry.sql".to_string(),
        "./data/chain.sql".to_string(),
    );
    let findex = Mutex::new(SqliteFindexBackend::new(findex_config).unwrap());
    let findex_data = Data::new(findex);
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(findex_data.clone())
            .service(fetch)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
