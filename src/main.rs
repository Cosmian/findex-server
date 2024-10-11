mod api;
mod common;
mod routes;
mod services;
mod middlewares;
mod error;

use std::sync::Arc;

use actix_identity::IdentityMiddleware;
use tokio;
use error::FindexServerError;
use actix_web::{
    middleware::Logger, web::{self, Data}, App, Error, HttpServer
};
use common::Config;
use log::{debug, info};
use actix_cors::Cors;
use crate::middlewares::{ LoginTransformerFactory, JwksManager, JwtConfig};



use routes::health_get;

#[derive(Debug, Clone)]
pub struct IdpConfig {
    pub jwt_issuer_uri: String,
    pub jwks_uri: Option<String>,
    pub jwt_audience: Option<String>,
}

impl From<FindexServerError> for std::io::Error {
    fn from(error: FindexServerError) -> Self {
        // Convert your custom error to std::io::Error
        std::io::Error::new(std::io::ErrorKind::Other, error.to_string())
    }
}


type FindexServerResult<T> = Result<T, FindexServerError>;

#[actix_web::main]
async fn main() -> FindexServerResult<()> {
    let config = web::Data::new(Config::from_env());
    let config_for_bind = config.clone();
    env_logger::Builder::new()
        .filter(None, config.log_level)
        .init();

    info!("Loaded env, starting Http server ...");
    debug!("debugging!");

    // let idp_config_google = IdpConfig {
    //     jwt_issuer_uri: "https://accounts.google.com".to_string(),
    //     jwks_uri: Some("https://www.googleapis.com/oauth2/v3/certs".to_string()),
    //     jwt_audience: Some("cosmian_kms".to_string()),
    // };
//
    let idp_config = Some(IdpConfig {
        jwt_issuer_uri: "https://findex-server.eu.auth0.com/".to_string(),
        jwks_uri: Some("https://findex-server.eu.auth0.com/.well-known/jwks.json".to_string()),
        jwt_audience: Some("https://findex-server/".to_string()),
    });

    let jwt_config_for_middleware= if let Some(identity_provider_configurations) = idp_config {
        let jwks_manager = Arc::new(JwksManager::new(vec![identity_provider_configurations.jwks_uri.unwrap().clone()]).await?);
        let jwt_config = JwtConfig {
            jwt_issuer_uri: identity_provider_configurations.jwt_issuer_uri.clone(),
            jwks: jwks_manager.clone(),
            jwt_audience: identity_provider_configurations.jwt_audience.clone(),
        };
        Some(Arc::new(Vec::<JwtConfig>::from_iter([jwt_config])))
    } else {
        None
    };

    HttpServer::new(move || {
        App::new()
            .wrap(LoginTransformerFactory::new(
                jwt_config_for_middleware.clone()
                // Some(Arc::new(Vec::<JwtConfig>::from_iter([])))
            ))
            .wrap(IdentityMiddleware::default())
            .wrap(Cors::permissive())
            .wrap(Logger::default())
            // .app_data(findex_data.clone())
            .route("/health", web::get().to(health_get))
            .app_data(config.clone())
    })
    .bind((config_for_bind.get_ref().host.clone(), config_for_bind.port))?
    // .map_err(FindexServerError::from)? // Convert std::io::Error to FindexServerError
    .run()
    .await?;

    Ok(())
}
