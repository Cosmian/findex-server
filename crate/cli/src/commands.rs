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
    #[command(subcommand)]
    Datasets(DatasetsAction),
    /// Delete indexed keywords
    Delete(InsertOrDeleteAction),
    /// Insert new keywords
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
    /// # Errors
    /// - If the configuration file is not found or invalid
    #[allow(clippy::unit_arg)] // println! does return () but it prints the output of action.run() beforehand, nothing is "lost" and hence this lint will only cause useless boilerplate code
    pub async fn run(&self, rest_client: &mut RestClient, kms_client: KmsClient) -> CliResult<()> {
        let action = self;
        {
            let result = match action {
                Self::Datasets(action) => action.run(rest_client).await,
                Self::Delete(action) => {
                    let deleted_keywords = action.delete(rest_client, kms_client).await?;
                    Ok(format!("Deleted keywords: {deleted_keywords}"))
                }
                Self::Insert(action) => {
                    let inserted_keywords = action.insert(rest_client, kms_client).await?;
                    Ok(format!("Inserted keywords: {inserted_keywords}"))
                }
                Self::Permissions(action) => action.run(rest_client).await,
                Self::Login(action) => action.run(&mut rest_client.config).await,
                Self::Logout(action) => action.run(&mut rest_client.config),
                Self::Search(action) => {
                    let search_results = action.run(rest_client, &kms_client).await?;
                    Ok(format!("Search results: {search_results}"))
                }
                Self::ServerVersion(action) => action.run(rest_client).await,
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

    // Instantiate the Findex REST client
    let mut rest_client = RestClient::new(config.clone())?;
    let kms_client = KmsClient::new(KmsClientConfig::default())?;

    // Process the command
    cli_opts.command.run(&mut rest_client, kms_client).await?;

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
