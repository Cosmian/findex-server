use crate::error::result::CliResult;
use clap::Parser;
use cosmian_findex_client::FindexClientConfig;
use std::path::PathBuf;
use tracing::info;

/// Logout from the Identity Provider.
///
/// The access token will be removed from the findex configuration file.
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct LogoutAction;

impl LogoutAction {
    /// Process the logout action.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue saving the configuration file.
    pub fn run(
        &self,
        config: &mut FindexClientConfig,
        conf_path: &Option<PathBuf>,
    ) -> CliResult<String> {
        config.http_config.access_token = None;
        info!(
            "Deleting access token from the configuration file {:?} ...",
            conf_path
        );
        Ok("\nThe access token was removed from the Findex CLI configuration".to_owned())
    }
}
