#![deny(
    nonstandard_style,
    refining_impl_trait,
    future_incompatible,
    keyword_idents,
    let_underscore,
    rust_2024_compatibility,
    unreachable_pub,
    unused,
    unsafe_code,
    clippy::all,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    clippy::pedantic,
    clippy::cargo,
    clippy::nursery,

    // restriction lints
    clippy::unwrap_used,
    clippy::get_unwrap,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_in_result,
    clippy::assertions_on_result_states,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::renamed_function_params,
    clippy::verbose_file_reads,
    clippy::str_to_string,
    clippy::string_to_string,
    clippy::unreachable,
    clippy::as_conversions,
    clippy::print_stdout,
    clippy::empty_structs_with_brackets,
    clippy::unseparated_literal_suffix,
    clippy::map_err_ignore,
    clippy::redundant_clone,
    // clippy::use_debug,
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::cargo_common_metadata,
    clippy::multiple_crate_versions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::redundant_pub_crate,
    clippy::cognitive_complexity
)]

mod api;
mod common;
mod error;
mod middlewares;
mod routes;
mod services;

use std::sync::Arc;

use crate::middlewares::{JwksManager, JwtConfig, LoginTransformerFactory};
use actix_cors::Cors;
use actix_identity::IdentityMiddleware;
use actix_service::ServiceFactory;
use actix_web::{
    body::{BoxBody, EitherBody, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    middleware::Logger,
    web::{self, Data, ServiceConfig},
    App, Error, HttpServer,
};
use common::Config;
use error::FindexServerError;
use log::{debug, info};
use tokio;

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

fn config_app(config: Data<Config>) -> Box<dyn Fn(&mut ServiceConfig)> {
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.wrap(IdentityMiddleware::default())
            // .wrap(Cors::permissive())
            // .wrap(Logger::default())
            // .app_data(findex_data.clone())
            // .route("/health", web::get().to(health_get))
            .app_data(config.clone())

        // cfg.app_data(config.cData<Config>)
        //     .service(web::resource("/notes").route(web::get().to(notes)));
    })
}

fn create_app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
        Error = Error,
        InitError = (),
    >,
> {
    App::new()
        .wrap(IdentityMiddleware::default())
        .wrap(Cors::permissive())
        .wrap(Logger::default())
}

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

    let jwt_config_for_middleware = if let Some(identity_provider_configurations) = idp_config {
        let jwks_manager = Arc::new(
            JwksManager::new(vec![identity_provider_configurations
                .jwks_uri
                .unwrap()
                .clone()])
            .await?,
        );
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
        // .wrap(LoginTransformerFactory::new(
        //     jwt_config_for_middleware.clone()
        //     // Some(Arc::new(Vec::<JwtConfig>::from_iter([])))
        // ))
        // .wrap(IdentityMiddleware::default())
        // .wrap(Cors::permissive())
        // .wrap(Logger::default())
        // // .app_data(findex_data.clone())
        // .route("/health", web::get().to(health_get))
        // .app_data(config.clone())
    })
    .bind((config_for_bind.get_ref().host.clone(), config_for_bind.port))?
    // .map_err(FindexServerError::from)? // Convert std::io::Error to FindexServerError
    .run()
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use actix_web::{http::header::ContentType, test, App};

    use super::*;

    #[actix_web::test]
    async fn test_index_get() {
        let app = test::init_service(App::new().service(index)).await;
        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
