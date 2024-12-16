use clap::Parser;
use cosmian_findex_client::FindexClientConfig;

use crate::error::result::CliResult;

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
    pub fn run(&self, conf: &mut FindexClientConfig) -> CliResult<()> {
        conf.http_config.access_token = None;
        println!("\nThe access token was removed from the Findex CLI configuration",);

        Ok(())
    }
}
