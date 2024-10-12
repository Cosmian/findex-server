use std::sync::{mpsc, Arc};

use actix_cors::Cors;
use actix_identity::IdentityMiddleware;
use actix_web::{
    dev::ServerHandle,
    middleware::Condition,
    web::{self, Data, JsonConfig, PayloadConfig},
    App, HttpServer,
};
use openssl::{
    ssl::{SslAcceptor, SslAcceptorBuilder, SslMethod, SslVerifyMode},
    x509::store::X509StoreBuilder,
};
use tracing::info;

use crate::{
    config::{self, JwtAuthConfig, ServerParams},
    core::FindexServer,
    findex_server_bail,
    middlewares::{extract_peer_certificate, AuthTransformer, JwksManager, JwtConfig, SslAuth},
    result::FResult,
    routes::get_version,
    FServer,
};

/// Starts the Findex server based on the provided configuration.
///
/// The server is started using one of three methods:
/// 1. Plain HTTP,
/// 2. HTTPS with PKCS#12,
///
/// The method used depends on the server settings specified in the `ServerParams` instance provided.
///
/// # Arguments
///
/// * `server_params` - An instance of `ServerParams` that contains the settings for the server.
/// * `server_handle_transmitter` - An optional sender channel of type `mpsc::Sender<ServerHandle>` that can be used to manage server state.
///
/// # Errors
///
/// This function will return an error if any of the server starting methods fails.
pub async fn start_findex_server(
    server_params: ServerParams,
    findex_server_handle_tx: Option<mpsc::Sender<ServerHandle>>,
) -> FResult<()> {
    // Log the server configuration
    info!("Findex Server configuration: {:#?}", server_params);
    match &server_params.http_params {
        config::HttpParams::Https(_) => {
            start_https_findex_server(server_params, findex_server_handle_tx).await
        }
        config::HttpParams::Http => {
            start_plain_http_findex_server(server_params, findex_server_handle_tx).await
        }
    }
}

/// Start a plain HTTP Findex server
///
/// This function will instantiate and prepare the Findex server and run it on a plain HTTP connection
///
/// # Arguments
///
/// * `server_params` - An instance of `ServerParams` that contains the settings for the server.
/// * `server_handle_transmitter` - An optional sender channel of type `mpsc::Sender<ServerHandle>` that can be used to manage server state.
///
/// # Errors
///
/// This function returns an error if:
/// - The Findex server cannot be instantiated or prepared
/// - The server fails to run
async fn start_plain_http_findex_server(
    server_params: ServerParams,
    server_handle_transmitter: Option<mpsc::Sender<ServerHandle>>,
) -> FResult<()> {
    // Instantiate and prepare the Findex server
    let findex_server = Arc::new(FServer::instantiate(server_params).await?);

    // Prepare the server
    let server = prepare_findex_server(findex_server, None).await?;

    // send the server handle to the caller
    if let Some(tx) = &server_handle_transmitter {
        tx.send(server.handle())?;
    }

    info!("Starting the HTTP Findex server...");
    // Run the server and return the result
    server.await.map_err(Into::into)
}

/// Start an HTTPS Findex server using a PKCS#12 certificate file
///
/// # Arguments
///
/// * `server_params` - An instance of `ServerParams` that contains the settings for the server.
/// * `server_handle_transmitter` - An optional sender channel of type `mpsc::Sender<ServerHandle>` that can be used to manage server state.
///
/// # Errors
///
/// This function returns an error if:
/// - The path to the PKCS#12 certificate file is not provided in the config
/// - The file cannot be opened or read
/// - The file is not a valid PKCS#12 format or the password is incorrect
/// - The SSL acceptor cannot be created or configured with the certificate and key
/// - The Findex server cannot be instantiated or prepared
/// - The server fails to run
async fn start_https_findex_server(
    server_params: ServerParams,
    server_handle_transmitter: Option<mpsc::Sender<ServerHandle>>,
) -> FResult<()> {
    let config::HttpParams::Https(p12) = &server_params.http_params else {
        findex_server_bail!("http/s: a PKCS#12 file must be provided")
    };

    // Create and configure an SSL acceptor with the certificate and key
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
    if let Some(pkey) = &p12.pkey {
        builder.set_private_key(pkey)?;
    }
    if let Some(cert) = &p12.cert {
        builder.set_certificate(cert)?;
    }
    if let Some(chain) = &p12.ca {
        for x in chain {
            builder.add_extra_chain_cert(x.to_owned())?;
        }
    }

    if let Some(verify_cert) = &server_params.authority_cert_file {
        // This line sets the mode to verify peer (client) certificates
        builder.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
        let mut store_builder = X509StoreBuilder::new()?;
        store_builder.add_cert(verify_cert.clone())?;
        builder.set_verify_cert_store(store_builder.build())?;
    }

    // Instantiate and prepare the Findex server
    let findex_server = Arc::new(FServer::instantiate(server_params).await?);
    let server = prepare_findex_server(findex_server, Some(builder)).await?;

    // send the server handle to the caller
    if let Some(tx) = &server_handle_transmitter {
        tx.send(server.handle())?;
    }

    info!("Starting the HTTPS Findex server...");

    // Run the server and return the result
    server.await.map_err(Into::into)
}

/**
 * This function prepares a server for the application. It creates an `HttpServer` instance,
 * configures the routes for the application, and sets the request timeout. The server can be
 * configured to use OpenSSL for SSL encryption by providing an `SslAcceptorBuilder`.
 *
 * # Arguments
 *
 * * `findex_server`: A shared reference to the `Findex server` instance to be used by the application.
 * * `builder`: An optional `SslAcceptorBuilder` to configure the SSL encryption for the server.
 *
 * # Returns
 *
 * Returns a `Result` type that contains a `Server` instance if successful, or an error if
 * something went wrong.
 *
 * # Errors
 *
 * This function can return the following errors:
 * - `FindexServerError::ServerError` - If there is an error in the server configuration or preparation.
 */
pub(crate) async fn prepare_findex_server(
    findex_server: Arc<FindexServer>,
    builder: Option<SslAcceptorBuilder>,
) -> FResult<actix_web::dev::Server> {
    // Prepare the JWT configurations and the JWKS manager if the server is using JWT for authentication.
    let (jwt_configurations, _jwks_manager) = if let Some(identity_provider_configurations) =
        &findex_server.params.identity_provider_configurations
    {
        // Prepare all the needed URIs from all the configured Identity Providers
        let all_jwks_uris: Vec<_> = identity_provider_configurations
            .iter()
            .map(|idp_config| {
                JwtAuthConfig::uri(&idp_config.jwt_issuer_uri, idp_config.jwks_uri.as_deref())
            })
            .collect();

        let jwks_manager = Arc::new(JwksManager::new(all_jwks_uris).await?);

        let built_jwt_configurations = identity_provider_configurations
            .iter()
            .map(|idp_config| JwtConfig {
                jwt_issuer_uri: idp_config.jwt_issuer_uri.clone(),
                jwks: jwks_manager.clone(),
                jwt_audience: idp_config.jwt_audience.clone(),
            })
            .collect::<Vec<_>>();

        (Some(Arc::new(built_jwt_configurations)), Some(jwks_manager))
    } else {
        (None, None)
    };

    // Determine if Client Cert Auth should be used for authentication.
    let use_cert_auth = findex_server.params.authority_cert_file.is_some();

    // Determine the address to bind the server to.
    let address = format!(
        "{}:{}",
        findex_server.params.hostname, findex_server.params.port
    );

    // Create the `HttpServer` instance.
    let server = HttpServer::new(move || {
        // Create an `App` instance and configure the passed data and the various scopes
        let app = App::new()
            .wrap(IdentityMiddleware::default())
            .app_data(Data::new(findex_server.clone())) // Set the shared reference to the `Findex server` instance.
            .app_data(PayloadConfig::new(10_000_000_000)) // Set the maximum size of the request payload.
            .app_data(JsonConfig::default().limit(10_000_000_000)); // Set the maximum size of the JSON request payload.

        // The default scope serves from the root / the KMIP, permissions, and tee endpoints
        let default_scope = web::scope("")
            .wrap(AuthTransformer::new(jwt_configurations.clone())) // Use JWT for authentication if necessary.
            .wrap(Condition::new(use_cert_auth, SslAuth)) // Use certificates for authentication if necessary.
            // Enable CORS for the application.
            // Since Actix is running the middlewares in reverse order, it's important that the
            // CORS middleware is the last one so that the auth middlewares do not run on
            // preflight (OPTION) requests.
            .wrap(Cors::permissive())
            .service(get_version);

        app.service(default_scope)
    })
    .client_disconnect_timeout(std::time::Duration::from_secs(30)) // default: 5s
    .tls_handshake_timeout(std::time::Duration::from_secs(18)) // default: 3s
    .keep_alive(std::time::Duration::from_secs(30)) // default: 5s
    .client_request_timeout(std::time::Duration::from_secs(30)) // default: 5s
    .shutdown_timeout(180); // default: 30s

    Ok(match builder {
        Some(cert_auth_builder) => {
            if use_cert_auth {
                // Start an HTTPS server with PKCS#12 with client cert auth
                server
                    .on_connect(extract_peer_certificate)
                    .bind_openssl(address, cert_auth_builder)?
                    .run()
            } else {
                // Start an HTTPS server with PKCS#12 but not client cert auth
                server.bind_openssl(address, cert_auth_builder)?.run()
            }
        }
        _ => server.bind(address)?.run(),
    })
}
