use std::{
    env,
    path::{Path, PathBuf},
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

use actix_server::ServerHandle;
use cosmian_findex_client::{
    ClientError, RestClient, RestClientConfig, client_bail, client_error,
    reexport::cosmian_http_client::HttpClientConfig,
};
use cosmian_findex_server::{
    config::{
        ClapConfig, DBConfig, DatabaseType, HttpConfig, HttpParams, JwtAuthConfig, ServerParams,
    },
    findex_server::start_findex_server,
};
use tokio::sync::OnceCell;
use tracing::{info, trace};

use crate::test_jwt::{AUTH0_TOKEN, get_auth0_jwt_config};

const REDIS_DEFAULT_URL: &str = "redis://localhost:6379";
const SQLITE_DEFAULT_URL: &str = "sqlite-data.db";

/// In order to run most tests in parallel,
/// we use that to avoid to try to start N Findex servers (one per test)
/// with a default configuration.
/// Otherwise, we get: "Address already in use (os error 98)"
/// for N-1 tests.
pub(crate) static ONCE: OnceCell<TestsContext> = OnceCell::const_new();
pub(crate) static ONCE_SERVER_WITH_AUTH: OnceCell<TestsContext> = OnceCell::const_new();

fn redis_db_config() -> DBConfig {
    let url: String = if let Ok(host_env) = env::var("REDIS_HOST") {
        format!("redis://{host_env}:6379")
    } else if let Ok(url_env) = env::var("REDIS_URL") {
        url_env
    } else {
        REDIS_DEFAULT_URL.to_owned()
    };
    trace!("TESTS: using redis with findex on {url}");
    DBConfig {
        database_type: DatabaseType::Redis,
        clear_database: false,
        database_url: url,
    }
}

fn sqlite_db_config(sqlite_url_var_env: &str) -> DBConfig {
    let url = env::var(sqlite_url_var_env).unwrap_or_else(|_| SQLITE_DEFAULT_URL.to_owned());
    trace!("TESTS: using sqlite with findex on {url}");
    DBConfig {
        database_type: DatabaseType::Sqlite,
        clear_database: false,
        database_url: url,
    }
}

pub fn get_db_config() -> DBConfig {
    env::var_os("FINDEX_TEST_DB").map_or_else(redis_db_config, |v| match v.to_str().unwrap_or("") {
        "redis-findex" => redis_db_config(),
        "sqlite-findex" => sqlite_db_config("FINDEX_SQLITE_URL"),
        _ => redis_db_config(),
    })
}

/// Start a test Findex server in a thread with the default options:
/// No TLS, no certificate authentication
pub async fn start_default_test_findex_server() -> &'static TestsContext {
    trace!("Starting default test server");
    ONCE.get_or_try_init(|| {
        start_test_server_with_options(
            get_db_config(),
            6668,
            AuthenticationOptions {
                use_jwt_token: false,
                use_https: false,
                use_client_cert: false,
                use_api_token: false,
                do_not_send_client_certificate: false,
                do_not_send_api_token: false,
                do_not_send_jwt_token: false,
            },
        )
    })
    .await
    .unwrap()
}
/// TLS + certificate authentication
pub async fn start_default_test_findex_server_with_cert_auth() -> &'static TestsContext {
    trace!("Starting test server with cert auth");
    ONCE_SERVER_WITH_AUTH
        .get_or_try_init(|| {
            start_test_server_with_options(
                get_db_config(),
                6660,
                AuthenticationOptions {
                    use_jwt_token: false,
                    use_https: true,
                    use_client_cert: true,
                    use_api_token: false,
                    do_not_send_client_certificate: false,
                    do_not_send_api_token: false,
                    do_not_send_jwt_token: false,
                },
            )
        })
        .await
        .unwrap()
}

pub struct TestsContext {
    pub owner_client_conf: RestClientConfig,
    pub user_client_conf: RestClientConfig,
    pub server_handle: ServerHandle,
    pub thread_handle: JoinHandle<Result<(), ClientError>>,
}

impl TestsContext {
    pub async fn stop_server(self) -> Result<(), ClientError> {
        self.server_handle.stop(false).await;
        self.thread_handle
            .join()
            .map_err(|_e| client_error!("failed joining the stop thread"))?
    }
}

#[derive(Default)]
pub struct AuthenticationOptions {
    // Authentication methods to enable on the server
    pub use_jwt_token: bool,
    pub use_https: bool,
    pub use_client_cert: bool,
    pub use_api_token: bool,

    //TODO check the KMS est Server equivalent to replicate how this used in testing authentication scenarios
    // Client credential configuration (all false by default)
    pub do_not_send_client_certificate: bool, // True = don't send client certificate even when required
    pub do_not_send_api_token: bool,          // True = do not send an API token
    pub do_not_send_jwt_token: bool,          // True = do not send an JWT token
}

/// Start a Findex server in a thread with the given options
pub async fn start_test_server_with_options(
    db_config: DBConfig,
    port: u16,
    authentication_options: AuthenticationOptions,
) -> Result<TestsContext, ClientError> {
    cosmian_logger::log_init(None);
    let server_params = generate_server_params(db_config, port, &authentication_options)?;

    // Create a (object owner) conf
    let owner_client_conf = generate_owner_conf(&server_params)?;
    let user_client_conf = generate_user_conf(&owner_client_conf)?;
    let findex_client = RestClient::new(owner_client_conf.clone())?;

    info!(
        "Starting Findex test server at URL: {} with server params {:?}",
        owner_client_conf.http_config.server_url, &server_params
    );

    let (server_handle, thread_handle) = start_test_findex_server(server_params);

    // wait for the server to be up
    wait_for_server_to_start(&findex_client)
        .await
        .expect("server timeout");

    Ok(TestsContext {
        owner_client_conf,
        user_client_conf,
        server_handle,
        thread_handle,
    })
}

/// Start a test Findex server with the given config in a separate thread
fn start_test_findex_server(
    server_params: ServerParams,
) -> (ServerHandle, JoinHandle<Result<(), ClientError>>) {
    let (tx, rx) = mpsc::channel::<ServerHandle>();

    let thread_handle = thread::spawn(move || {
        // allow others `spawn` to happen within the Findex server future
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(start_findex_server(server_params, Some(tx)))
            .map_err(|e| ClientError::Default(e.to_string()))
    });
    trace!("Waiting for test Findex server to start...");
    let server_handle = rx
        .recv_timeout(Duration::from_secs(25))
        .expect("Can't get test Findex server handle after 25 seconds");
    trace!("... got handle ...");
    (server_handle, thread_handle)
}

/// Wait for the server to start by reading the version
async fn wait_for_server_to_start(rest_client: &RestClient) -> Result<(), ClientError> {
    // Depending on the running environment, the server could take a bit of time
    // to start. We try to querying it with a dummy request until it is started.
    for i in 1..=5 {
        info!("...checking if the server is up...");
        if let Err(err) = rest_client.version().await {
            info!(
                "The server is not up yet, retrying in {}s... ({err:?}) ",
                2 * i
            );
            thread::sleep(Duration::from_secs(2 * i));
        } else {
            info!("UP!");
            return Ok(());
        }
    }
    info!("The server is still not up, stop trying");
    client_bail!("Can't start the Findex server to run tests");
}

fn generate_http_config(port: u16, use_https: bool, use_client_cert: bool) -> HttpConfig {
    // This create root dir
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    if use_https {
        if use_client_cert {
            HttpConfig {
                port,
                https_p12_file: Some(root_dir.join(
                    "../../test_data/certificates/client_server/server/kmserver.acme.com.p12",
                )),
                https_p12_password: Some("password".to_owned()),
                authority_cert_file: Some(
                    root_dir.join("../../test_data/certificates/client_server/server/ca.crt"),
                ),
                ..HttpConfig::default()
            }
        } else {
            HttpConfig {
                port,
                https_p12_file: Some(root_dir.join(
                    "../../test_data/certificates/client_server/server/kmserver.acme.com.p12",
                )),
                https_p12_password: Some("password".to_owned()),
                ..HttpConfig::default()
            }
        }
    } else {
        HttpConfig {
            port,
            ..HttpConfig::default()
        }
    }
}

fn generate_server_params(
    db_config: DBConfig,
    port: u16,
    authentication_options: &AuthenticationOptions,
) -> Result<ServerParams, ClientError> {
    // Configure the server
    let clap_config = ClapConfig {
        auth: if authentication_options.use_jwt_token {
            get_auth0_jwt_config()
        } else {
            JwtAuthConfig::default()
        },
        db: db_config,
        http: generate_http_config(
            port,
            authentication_options.use_https,
            authentication_options.use_client_cert,
        ),
        ..ClapConfig::default()
    };

    // TODO: Not available on Findex
    // // Configure API token authentication if requested
    // if authentication_options.use_api_token {
    //     // For testing, we use a fixed API token identifier that can be predictably accessed
    //     clap_config.api_token_id = Some("test_api_token_id".to_string());
    // }

    ServerParams::try_from(clap_config)
        .map_err(|e| ClientError::Default(format!("failed initializing the server config: {e}")))
}

fn set_access_token(server_params: &ServerParams) -> Option<String> {
    server_params
        .identity_provider_configurations
        .is_some()
        .then(|| {
            trace!("Setting access token for JWT: {AUTH0_TOKEN:?}");
            AUTH0_TOKEN.to_owned()
        })
}
fn get_owner_certificate(root_dir: &Path, server_params: &ServerParams) -> Option<String> {
    server_params.authority_cert_file.is_some().then(|| {
        let path = "../../test_data/certificates/client_server/owner/owner.client.acme.com.p12";
        root_dir.join(path).to_str().unwrap().to_owned()
    })
}

fn generate_owner_conf(server_params: &ServerParams) -> Result<RestClientConfig, ClientError> {
    // This create root dir
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let owner_client_conf = RestClientConfig {
        http_config: HttpClientConfig {
            server_url: format!(
                "{}://localhost:{}",
                if matches!(server_params.http_params, HttpParams::Https(_)) {
                    "https"
                } else {
                    "http"
                },
                server_params.port
            ),
            accept_invalid_certs: true,
            access_token: set_access_token(server_params),
            ssl_client_pkcs12_path: get_owner_certificate(&root_dir, server_params),
            ssl_client_pkcs12_password: server_params
                .authority_cert_file
                .is_some()
                .then(|| "password".to_owned()),
            ..Default::default()
        },
    };

    Ok(owner_client_conf)
}

/// Generate a user configuration for user.client@acme.com and return the file
/// path
fn generate_user_conf(
    owner_client_conf: &RestClientConfig,
) -> Result<RestClientConfig, ClientError> {
    // This create root dir
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut user_conf = owner_client_conf.clone();
    user_conf.http_config.ssl_client_pkcs12_path = {
        let p = root_dir
            .join("../../test_data/certificates/client_server/user/user.client.acme.com.p12");
        Some(
            p.to_str()
                .ok_or_else(|| ClientError::Default("Can't convert path to string".to_owned()))?
                .to_owned(),
        )
    };
    user_conf.http_config.ssl_client_pkcs12_password = Some("password".to_owned());

    // return the path
    Ok(user_conf)
}

#[cfg(test)]
mod findex_server {
    use cosmian_findex_client::ClientError;
    use tracing::trace;

    use crate::{
        AuthenticationOptions, start_test_server_with_options, test_server::get_db_config,
    };

    #[tokio::test]
    async fn test_server_auth_matrix() -> Result<(), ClientError> {
        let test_cases = vec![
            (false, false, false, "all_disabled"),
            (true, false, false, "https_no_auth"),
            (true, false, true, "https_cert"),
            (false, true, false, "https_jwt"),
            (true, true, true, "all_enabled"),
        ];
        for (use_https, use_jwt_token, use_client_cert, description) in test_cases {
            trace!("Running test case: {}", description);
            let context = start_test_server_with_options(
                get_db_config(),
                6667,
                AuthenticationOptions {
                    use_https,
                    use_jwt_token,
                    use_client_cert,
                    use_api_token: false,
                    do_not_send_client_certificate: false,
                    do_not_send_api_token: false,
                    do_not_send_jwt_token: false,
                },
            )
            .await?;
            context.stop_server().await?;
        }
        Ok(())
    }
}
