use std::path::PathBuf;

use clap::{Parser, Subcommand};
use cosmian_findex_client::{RestClient, RestClientConfig};
use cosmian_kms_cli::reexport::cosmian_kms_client::{KmsClient, KmsClientConfig};
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
    pub fn prepare_config(&self) -> CliResult<RestClientConfig> {
        // Load configuration file and override with command line options
        let mut config = RestClientConfig::load(self.conf_path.clone())?;
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
    /// Create new indexes
    Index(InsertOrDeleteAction),
    Search(SearchAction),
    /// Delete indexed keywords
    Delete(InsertOrDeleteAction),

    #[command(subcommand)]
    Permissions(PermissionsAction),

    #[command(subcommand)]
    Datasets(DatasetsAction),

    Login(LoginAction),
    Logout(LogoutAction),

    ServerVersion(ServerVersionAction),
}

impl CoreFindexActions {
    /// Process the command line arguments
    ///
    /// # Arguments
    /// * `findex_client` - The Findex client
    /// * `config` - The Findex client configuration
    ///
    /// # Errors
    /// - If the configuration file is not found or invalid
    pub async fn run(
        &self, // we do not want to consume self because we need post processing for the login/logout commands
        findex_client: &mut RestClient,
        kms_client: KmsClient,
        config: &mut RestClientConfig,
    ) -> CliResult<()> {
        let result = match self {
            // actions that don't edit the configuration
            Self::Datasets(action) => action.run(findex_client).await,
            Self::Permissions(action) => action.run(findex_client).await,
            Self::ServerVersion(action) => action.run(findex_client).await,
            Self::Delete(action) => {
                let deleted_keywords = action.delete(findex_client, kms_client).await?;
                Ok(format!("Deleted keywords: {deleted_keywords}"))
            }
            Self::Index(action) => {
                let inserted_keywords = action.insert(findex_client, kms_client).await?;
                Ok(format!("Inserted keywords: {inserted_keywords}"))
            }
            Self::Search(action) => {
                let search_results = action.run(findex_client, &kms_client).await?;
                Ok(format!("Search results: {search_results}"))
            }

            // actions that edit the configuration
            Self::Login(action) => action.run(config).await,
            Self::Logout(action) => action.run(config),
        };
        match result {
            Ok(output) => {
                println!("{output}");
                Ok(())
            }
            Err(e) => Err(e),
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
    let mut config = cli_opts.prepare_config()?;

    // Instantiate the Findex REST client
    let mut rest_client = RestClient::new(&config)?;
    let kms_client = KmsClient::new(KmsClientConfig::default())?;

    // Process the command
    cli_opts
        .command
        .run(&mut rest_client, kms_client, &mut config)
        .await?;

    // Post-process the login/logout actions: save Findex CLI configuration
    // The reason why it is done here is that the login/logout actions are also call by meta Cosmian CLI using its own Findex client configuration
    // !!! Do not edit this without reciprocal changes in the Cosmian CLI : https://github.com/Cosmian/cli
    match cli_opts.command {
        CoreFindexActions::Login(_) | CoreFindexActions::Logout(_) => {
            config.save(cli_opts.conf_path)?;
        }
        _ => {}
    }
    Ok(())
}
