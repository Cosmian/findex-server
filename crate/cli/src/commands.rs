use std::path::PathBuf;

use clap::{Parser, Subcommand};
use cosmian_config_utils::ConfigUtils;
use cosmian_findex_client::{reexport::cosmian_findex_config::FindexClientConfig, FindexClient};
use cosmian_logger::log_init;
use tracing::info;

use crate::{
    actions::{
        findex::{add_or_delete::AddOrDeleteAction, search::SearchAction},
        login::LoginAction,
        logout::LogoutAction,
        permissions::PermissionsAction,
        version::ServerVersionAction,
    },
    error::result::CliResult,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: CoreFindexActions,

    /// Configuration file location
    ///
    /// This is an alternative to the env variable `FINDEX_CLI_CONF`.
    /// Takes precedence over `FINDEX_CLI_CONF` env variable.
    #[arg(short, long)]
    conf: Option<PathBuf>,

    /// The URL of the Findex
    #[arg(long, action)]
    pub(crate) url: Option<String>,

    /// Allow to connect using a self-signed cert or untrusted cert chain
    ///
    /// `accept_invalid_certs` is useful if the CLI needs to connect to an HTTPS
    /// Findex server running an invalid or insecure SSL certificate
    #[arg(long)]
    pub(crate) accept_invalid_certs: Option<bool>,
}

#[derive(Subcommand)]
pub enum CoreFindexActions {
    /// Index new keywords.
    Add(AddOrDeleteAction),
    /// Delete indexed keywords
    Delete(AddOrDeleteAction),
    Search(SearchAction),
    ServerVersion(ServerVersionAction),
    Login(LoginAction),
    Logout(LogoutAction),
    #[command(subcommand)]
    Permissions(PermissionsAction),
}

impl CoreFindexActions {
    /// Process the command line arguments
    /// # Errors
    /// - If the configuration file is not found or invalid
    #[allow(clippy::future_not_send)]
    pub async fn run(&self, findex_client: FindexClient) -> CliResult<()> {
        match self {
            Self::Login(action) => action.process(&findex_client.conf).await,
            Self::Logout(action) => action.process(&findex_client.conf),
            Self::Add(action) => action.add(findex_client).await,
            Self::Delete(action) => action.delete(findex_client).await,
            Self::Search(action) => action.process(findex_client).await,
            Self::ServerVersion(action) => action.process(findex_client).await,
            Self::Permissions(action) => action.process(findex_client).await,
        }
    }
}

/// Main function for the Findex CLI
/// # Errors
/// - If the configuration file is not found or invalid
/// - If the command line arguments are invalid
#[allow(clippy::future_not_send)]
pub async fn findex_cli_main() -> CliResult<()> {
    log_init(None);
    let opts = Cli::parse();

    // Load configuration file and override with command line options
    let conf_path = FindexClientConfig::location(opts.conf)?;
    let mut conf = FindexClientConfig::load(&conf_path)?;
    if opts.url.is_some() {
        info!("Override URL from configuration file with: {:?}", opts.url);
        conf.http_config.server_url = opts.url.unwrap_or_default();
    }
    if opts.accept_invalid_certs.is_some() {
        info!(
            "Override accept_invalid_certs from configuration file with: {:?}",
            opts.accept_invalid_certs
        );
        conf.http_config.accept_invalid_certs = opts.accept_invalid_certs.unwrap_or_default();
    }

    // Instantiate the Findex REST client
    let rest_client = FindexClient::new(conf)?;

    // Process the command
    opts.command.run(rest_client).await?;

    Ok(())
}
