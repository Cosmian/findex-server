use std::path::PathBuf;

use clap::{Parser, Subcommand};
use cosmian_findex_client::{FindexClientConfig, FindexRestClient};
use cosmian_logger::log_init;
use tracing::info;

use crate::{
    actions::{
        datasets::DatasetsAction,
        findex::{insert_or_delete::InsertOrDeleteAction, search::SearchAction},
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
    Delete(InsertOrDeleteAction),
    /// Insert new keywords.
    Insert(InsertOrDeleteAction),
    Login(LoginAction),
    Logout(LogoutAction),
    #[command(subcommand)]
    Permissions(PermissionsAction),
    Search(SearchAction),
    ServerVersion(ServerVersionAction),
}

impl CoreFindexActions {
    /// Process the command line arguments
    ///
    /// # Arguments
    /// * `findex_client` - The Findex client
    /// * `config` - The Findex client configuration
    /// * `conf_path` - The path to the configuration file
    ///
    /// # Errors
    /// - If the configuration file is not found or invalid
    #[allow(clippy::unit_arg)] // println! does return () but it prints the output of action.run() beforehand, nothing is "lost" and hence this lint will only cause useless boilerplate code
    pub async fn run(
        &self,
        findex_client: FindexRestClient,
        config: FindexClientConfig,
        conf_path: Option<PathBuf>,
    ) -> CliResult<()> {
        let action: &Self = self;
        {
            let result = match action {
                // actions that don't need to return a value and don't edit the configuration
                Self::Permissions(action) => action.run(findex_client).await,
                Self::Datasets(action) => action.run(findex_client).await,
                Self::ServerVersion(action) => action.run(findex_client).await,

                // actions that return a value that needs to be formatted, and don't edit the configuration
                Self::Delete(action) => {
                    let deleted_keywords = action.delete(findex_client).await?;
                    Ok(format!("Deleted keywords: {deleted_keywords}"))
                }
                Self::Insert(action) => {
                    let inserted_keywords = action.insert(findex_client).await?;
                    Ok(format!("Inserted keywords: {inserted_keywords}"))
                }
                Self::Search(action) => {
                    let search_results = action.run(findex_client).await?;
                    Ok(format!("Search results: {search_results}"))
                }

                // actions that edit the configuration, and don't return a value
                Self::Login(action) => action.run(config, conf_path).await,
                Self::Logout(action) => action.run(config, &conf_path),
            };
            match result {
                Ok(output) => Ok(println!("{output}")),
                Err(e) => Err(e),
            }
        }
    }
}

/// Main function for the Findex CLI
/// # Errors
/// - If the configuration file is not found or invalid
/// - If the command line arguments are invalid
pub async fn findex_cli_main() -> CliResult<()> {
    log_init(None);
    let cli_opts = FindexCli::parse();
    let config = cli_opts.prepare_config()?;

    // Process the command
    cli_opts
        .command
        .run(
            FindexRestClient::new(&config)?,
            config,
            cli_opts.conf_path.clone(),
        )
        .await?;

    Ok(())
}
