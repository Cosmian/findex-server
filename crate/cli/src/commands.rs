use std::path::PathBuf;

use clap::{Parser, Subcommand};
use cosmian_findex_client::{FindexClientConfig, FindexRestClient};
use cosmian_logger::log_init;
use tracing::info;

use crate::{
    actions::{
        datasets::DatasetsAction,
        findex::{index_or_delete::IndexOrDeleteAction, search::SearchAction},
        login::LoginAction,
        logout::LogoutAction,
        permissions::PermissionsAction,
        version::ServerVersionAction,
    },
    error::result::CliResult,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct FindexCli {
    #[command(subcommand)]
    command: CoreFindexActions,

    /// Configuration file location
    ///
    /// This is an alternative to the env variable `FINDEX_CLI_CONF`.
    /// Takes precedence over `FINDEX_CLI_CONF` env variable.
    #[arg(short, long)]
    conf_path: Option<PathBuf>,

    /// The URL of the Findex
    #[arg(long, action)]
    pub(crate) url: Option<String>,

    /// Allow to connect using a self-signed cert or untrusted cert chain
    ///
    /// `accept_invalid_certs` is useful if the CLI needs to connect to an HTTPS
    /// Findex server running an invalid or insecure SSL certificate
    #[arg(long)]
    pub(crate) accept_invalid_certs: bool,
}

impl FindexCli {
    /// Prepare the configuration
    /// # Errors
    /// - If the configuration file is not found or invalid
    /// - If the command line arguments are invalid
    pub fn prepare_config(&self) -> CliResult<FindexClientConfig> {
        // Load configuration file and override with command line options
        let mut config = FindexClientConfig::load(self.conf_path.clone())?;
        if let Some(url) = self.url.clone() {
            info!("Override URL from configuration file with: {url}");
            config.http_config.server_url = url;
        }
        if self.accept_invalid_certs {
            info!(
                "Override accept_invalid_certs from configuration file with: {:?}",
                self.accept_invalid_certs
            );
            config.http_config.accept_invalid_certs = true;
        }
        Ok(config)
    }
}

#[derive(Subcommand)]
pub enum CoreFindexActions {
    #[command(subcommand)]
    Datasets(DatasetsAction),
    /// Delete indexed keywords
    Delete(IndexOrDeleteAction),
    /// Index new keywords.
    Index(IndexOrDeleteAction),
    Login(LoginAction),
    Logout(LogoutAction),
    #[command(subcommand)]
    Permissions(PermissionsAction),
    Search(SearchAction),
    ServerVersion(ServerVersionAction),
}

impl CoreFindexActions {
    /// Process the command line arguments
    /// # Errors
    /// - If the configuration file is not found or invalid
    #[allow(clippy::future_not_send, clippy::unit_arg)] // println! does return () but it prints the output of action.run() beforehand, nothing is "lost" and hence this lint will only cause useless boilerplate code
    pub async fn run(&self, findex_client: &mut FindexRestClient) -> CliResult<()> {
        match self {
            Self::Datasets(action) => Ok(println!("{}", action.run(findex_client).await?)),
            Self::Delete(action) => Ok(println!("{}", action.delete(findex_client).await?)),
            Self::Index(action) => Ok(println!("{}", action.add(findex_client).await?)),
            Self::Permissions(action) => Ok(println!("{}", action.run(findex_client).await?)),
            Self::Login(action) => action.run(&mut findex_client.config).await, // Login is the only action that needs an intermediary URL output, thus we leave printing to stdout handled internally
            Self::Logout(action) => Ok(println!("{}", action.run(&mut findex_client.config)?)),
            Self::Search(action) => Ok(println!("{}", action.run(findex_client).await?)),
            Self::ServerVersion(action) => Ok(println!("{}", action.run(findex_client).await?)),
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
    let cli_opts = FindexCli::parse();
    let config = cli_opts.prepare_config()?;

    // Instantiate the Findex REST client
    let mut rest_client = FindexRestClient::new(config.clone())?;

    // Process the command
    cli_opts.command.run(&mut rest_client).await?;

    // Post-process the login/logout actions: save Findex CLI configuration
    // The reason why it is done here is that the login/logout actions are also call by meta Cosmian CLI using its own Findex client configuration
    match cli_opts.command {
        CoreFindexActions::Login(_) | CoreFindexActions::Logout(_) => {
            config.save(cli_opts.conf_path.clone())?;
        }
        _ => {}
    }

    Ok(())
}
