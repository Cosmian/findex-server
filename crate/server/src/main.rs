use std::path::PathBuf;

use clap::Parser;
use cosmian_findex_server::{
    config::{ClapConfig, ServerParams},
    error::{result::FResult, server::ServerError},
    findex_server::start_findex_server,
    server_bail,
};
use cosmian_logger::log_init;
use dotenvy::dotenv;
use tracing::{debug, info};

const FINDEX_SERVER_CONF: &str = "/etc/cosmian/findex_server.toml";

/// The main entrypoint of the program.
///
/// This function sets up the necessary environment variables and logging
/// options, then parses the command line arguments using [`ClapConfig::parse()`](https://docs.rs/clap/latest/clap/struct.ClapConfig.html#method.parse).
#[tokio::main]
async fn main() -> FResult<()> {
    // Set up environment variables and logging options
    if std::env::var("RUST_BACKTRACE").is_err() {
        unsafe {
            std::env::set_var("RUST_BACKTRACE", "full");
        }
    }
    if std::env::var("RUST_LOG").is_err() {
        unsafe {
            std::env::set_var(
                "RUST_LOG",
                "info,cosmian=info,cosmian_findex_server=info,actix_web=info",
            );
        }
    }

    log_init(None);

    // Load variable from a .env file
    dotenv().ok();

    let conf = if let Ok(conf_path) = std::env::var("COSMIAN_FINDEX_SERVER_CONF") {
        let conf_path = PathBuf::from(conf_path);
        if !conf_path.exists() {
            server_bail!(ServerError::ServerError(format!(
                "Cannot read findex server config at specified path: {conf_path:?} - file does \
                 not exist"
            )));
        }
        conf_path
    } else {
        PathBuf::from(FINDEX_SERVER_CONF)
    };

    let clap_config = if conf.exists() {
        ClapConfig::parse(); // Do that do catch --help or --version even if we use a conf file

        info!(
            "Configuration file {conf:?} found. Command line arguments and env variables are \
             ignored."
        );

        let conf_content = std::fs::read_to_string(&conf).map_err(|e| {
            ServerError::ServerError(format!(
                "Cannot read findex server config at: {conf:?} - {e:?}"
            ))
        })?;
        toml::from_str(&conf_content).map_err(|e| {
            ServerError::ServerError(format!(
                "Cannot parse findex server config at: {conf:?} - {e:?}"
            ))
        })?
    } else {
        ClapConfig::parse()
    };

    // Instantiate a config object using the env variables and the args of the
    // binary
    debug!("Command line config: {clap_config:#?}");

    // Parse the Server Config from the command line arguments
    let server_params = ServerParams::try_from(clap_config)?;

    #[cfg(feature = "insecure")]
    info!("Feature Insecure enabled");

    // Start Findex server
    Box::pin(start_findex_server(server_params, None)).await
}
