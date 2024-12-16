use std::{
    env,
    path::{Path, PathBuf},
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

use actix_server::ServerHandle;
use cosmian_findex_client::{
    findex_client_bail, findex_client_error,
    reexport::{cosmian_config_utils::ConfigUtils, cosmian_http_client::HttpClientConfig},
    FindexClientConfig, FindexClientError, FindexRestClient,
};
use cosmian_findex_server::{
    config::{
        ClapConfig, DBConfig, DatabaseType, HttpConfig, HttpParams, JwtAuthConfig, ServerParams,
    },
    findex_server::start_findex_server,
};
use tokio::sync::OnceCell;
use tracing::{info, trace};

use crate::test_jwt::{get_auth0_jwt_config, AUTH0_TOKEN};

/// In order to run most tests in parallel,
/// we use that to avoid to try to start N Findex servers (one per test)
/// with a default configuration.
/// Otherwise, we get: "Address already in use (os error 98)"
/// for N-1 tests.
pub(crate) static ONCE: OnceCell<TestsContext> = OnceCell::const_new();
pub(crate) static ONCE_SERVER_WITH_AUTH: OnceCell<TestsContext> = OnceCell::const_new();

fn redis_db_config() -> DBConfig {
    let url = if let Ok(var_env) = env::var("REDIS_HOST") {
        format!("redis://{var_env}:6379")
    } else {
        "redis://localhost:6379".to_owned()
    };
    trace!("TESTS: using redis on {url}");
    DBConfig {
        database_type: DatabaseType::Redis,
        clear_database: false,
        database_url: url,
    }
}

fn get_db_config() -> DBConfig {
    env::var_os("FINDEX_TEST_DB").map_or_else(redis_db_config, |v| match v.to_str().unwrap_or("") {
        "redis" => redis_db_config(),
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
                },
            )
        })
        .await
        .unwrap()
}

pub struct TestsContext {
    pub owner_client_conf_path: String,
    pub user_client_conf_path: String,
    pub owner_client_conf: FindexClientConfig,
    pub server_handle: ServerHandle,
    pub thread_handle: JoinHandle<Result<(), FindexClientError>>,
}

impl TestsContext {
    pub async fn stop_server(self) -> Result<(), FindexClientError> {
        self.server_handle.stop(false).await;
        self.thread_handle
            .join()
            .map_err(|_e| findex_client_error!("failed joining the stop thread"))?
    }
}

pub struct AuthenticationOptions {
    pub use_jwt_token: bool,
    pub use_https: bool,
    pub use_client_cert: bool,
}

/// Start a Findex server in a thread with the given options
pub async fn start_test_server_with_options(
    db_config: DBConfig,
    port: u16,
    authentication_options: AuthenticationOptions,
) -> Result<TestsContext, FindexClientError> {
    cosmian_logger::log_init(None);
    let server_params = generate_server_params(db_config.clone(), port, &authentication_options)?;

    // Create a (object owner) conf
    let (owner_client_conf_path, owner_client_conf) = generate_owner_conf(&server_params)?;
    let findex_client = FindexRestClient::new(owner_client_conf.clone())?;

    info!(
        "Starting Findex test server at URL: {} with server params {:?}",
        owner_client_conf.http_config.server_url, &server_params
    );

    let (server_handle, thread_handle) = start_test_findex_server(server_params);

    // wait for the server to be up
    wait_for_server_to_start(&findex_client)
        .await
        .expect("server timeout");

    // generate a user conf
    let user_client_conf_path =
        generate_user_conf(port, &owner_client_conf).expect("Can't generate user conf");

    Ok(TestsContext {
        owner_client_conf_path,
        user_client_conf_path,
        owner_client_conf,
        server_handle,
        thread_handle,
    })
}

/// Start a test Findex server with the given config in a separate thread
fn start_test_findex_server(
    server_params: ServerParams,
) -> (ServerHandle, JoinHandle<Result<(), FindexClientError>>) {
    let (tx, rx) = mpsc::channel::<ServerHandle>();

    let thread_handle = thread::spawn(move || {
        // allow others `spawn` to happen within the Findex server future
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(start_findex_server(server_params, Some(tx)))
            .map_err(|e| FindexClientError::UnexpectedError(e.to_string()))
    });
    trace!("Waiting for test Findex server to start...");
    let server_handle = rx
        .recv_timeout(Duration::from_secs(25))
        .expect("Can't get test Findex server handle after 25 seconds");
    trace!("... got handle ...");
    (server_handle, thread_handle)
}

/// Wait for the server to start by reading the version
async fn wait_for_server_to_start(
    findex_client: &FindexRestClient,
) -> Result<(), FindexClientError> {
    // Depending on the running environment, the server could take a bit of time to
    // start We try to query it with a dummy request until be sure it is
    // started.
    let mut retry = true;
    let mut timeout = 5;
    let mut waiting = 1;
    while retry {
        info!("...checking if the server is up...");
        let result = findex_client.version().await;
        if result.is_err() {
            timeout -= 1;
            retry = timeout >= 0;
            if retry {
                info!("The server is not up yet, retrying in {waiting}s... ({result:?}) ",);
                thread::sleep(Duration::from_secs(waiting));
                waiting *= 2;
            } else {
                info!("The server is still not up, stop trying");
                findex_client_bail!("Can't start the Findex server to run tests");
            }
        } else {
            info!("UP!");
            retry = false;
        }
    }
    Ok(())
}

fn generate_http_config(port: u16, use_https: bool, use_client_cert: bool) -> HttpConfig {
    // This create root dir
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    if use_https {
        if use_client_cert {
            HttpConfig {
                port,
                https_p12_file: Some(
                    root_dir.join("../../test_data/certificates/server/findex.server.acme.com.p12"),
                ),
                https_p12_password: Some("password".to_owned()),
                authority_cert_file: Some(
                    root_dir.join("../../test_data/certificates/server/ca.crt"),
                ),
                ..HttpConfig::default()
            }
        } else {
            HttpConfig {
                port,
                https_p12_file: Some(
                    root_dir.join("../../test_data/certificates/server/findex.server.acme.com.p12"),
                ),
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
) -> Result<ServerParams, FindexClientError> {
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
    ServerParams::try_from(clap_config).map_err(|e| {
        FindexClientError::Default(format!("failed initializing the server config: {e}"))
    })
}

fn set_access_token(server_params: &ServerParams) -> Option<String> {
    if server_params.identity_provider_configurations.is_some() {
        trace!("Setting access token for JWT: {AUTH0_TOKEN:?}");
        Some(AUTH0_TOKEN.to_string())
    } else {
        None
    }
}

fn generate_owner_conf(
    server_params: &ServerParams,
) -> Result<(String, FindexClientConfig), FindexClientError> {
    // This create root dir
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Create a conf
    let owner_client_conf_path = format!("/tmp/owner_findex_{}.toml", server_params.port);

    let owner_client_conf = FindexClientConfig {
        http_config: HttpClientConfig {
            server_url: if matches!(server_params.http_params, HttpParams::Https(_)) {
                format!("https://0.0.0.0:{}", server_params.port)
            } else {
                format!("http://0.0.0.0:{}", server_params.port)
            },
            accept_invalid_certs: true,
            access_token: set_access_token(server_params),
            ssl_client_pkcs12_path: if server_params.authority_cert_file.is_some() {
                #[cfg(not(target_os = "macos"))]
                let p =
                    root_dir.join("../../test_data/certificates/owner/owner.client.acme.com.p12");
                #[cfg(target_os = "macos")]
                let p = root_dir.join(
                    "../../test_data/certificates/owner/owner.client.acme.com.old.format.p12",
                );
                Some(
                    p.to_str()
                        .ok_or_else(|| {
                            FindexClientError::Default("Can't convert path to string".to_owned())
                        })?
                        .to_string(),
                )
            } else {
                None
            },
            ssl_client_pkcs12_password: if server_params.authority_cert_file.is_some() {
                Some("password".to_owned())
            } else {
                None
            },
            ..Default::default()
        },
    };
    // write the conf to a file
    FindexClientConfig::to_toml(
        &owner_client_conf,
        &Path::new(&owner_client_conf_path).to_path_buf(),
    )?;

    Ok((owner_client_conf_path, owner_client_conf))
}

/// Generate a user configuration for user.client@acme.com and return the file
/// path
fn generate_user_conf(
    port: u16,
    owner_client_conf: &FindexClientConfig,
) -> Result<String, FindexClientError> {
    // This create root dir
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut user_conf = owner_client_conf.clone();
    user_conf.http_config.ssl_client_pkcs12_path = {
        #[cfg(not(target_os = "macos"))]
        let p = root_dir.join("../../test_data/certificates/user/user.client.acme.com.p12");
        #[cfg(target_os = "macos")]
        let p =
            root_dir.join("../../test_data/certificates/user/user.client.acme.com.old.format.p12");
        Some(
            p.to_str()
                .ok_or_else(|| {
                    FindexClientError::Default("Can't convert path to string".to_owned())
                })?
                .to_string(),
        )
    };
    user_conf.http_config.ssl_client_pkcs12_password = Some("password".to_owned());

    // write the user conf
    let user_conf_path = format!("/tmp/user_findex_{port}.toml");
    FindexClientConfig::to_toml(&user_conf, &Path::new(&user_conf_path).to_path_buf())?;

    // return the path
    Ok(user_conf_path)
}

#[cfg(test)]
mod test {
    use cosmian_findex_client::FindexClientError;
    use tracing::trace;

    use crate::{
        start_test_server_with_options, test_server::redis_db_config, AuthenticationOptions,
    };

    #[tokio::test]
    async fn test_server_auth_matrix() -> Result<(), FindexClientError> {
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
                redis_db_config(),
                6667,
                AuthenticationOptions {
                    use_https,
                    use_jwt_token,
                    use_client_cert,
                },
            )
            .await?;
            context.stop_server().await?;
        }
        Ok(())
    }
}
