use clap::Parser;
use cosmian_config_utils::ConfigUtils;
use cosmian_findex_client::FindexClientConfig;

use crate::error::{result::CliResult, CliError};

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
    /// Returns an error if there is an issue loading or saving the
    /// configuration file.
    #[allow(clippy::print_stdout)]
    pub fn run(&self, conf: &FindexClientConfig) -> CliResult<()> {
        let mut conf = conf.to_owned();
        conf.http_config.access_token = None;
        let conf_path = conf.conf_path.clone().ok_or_else(|| {
            CliError::Default("Configuration path `conf_path` must be filled".to_owned())
        })?;
        conf.to_toml(&conf_path)?;

        println!(
            "\nThe access token was removed from the Findex CLI configuration file: {:?}",
            conf.conf_path
        );

        Ok(())
    }
}
