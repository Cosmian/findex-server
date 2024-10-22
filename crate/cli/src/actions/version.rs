use super::console;
use crate::error::result::{CliResult, CliResultHelper};
use clap::Parser;
use cosmian_findex_client::FindexClient;

/// Print the version of the server
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct ServerVersionAction;

impl ServerVersionAction {
    /// Process the server version action.
    ///
    /// # Arguments
    ///
    /// * `findex_rest_client` - The Findex server client instance used to
    ///   communicate with the Findex server server.
    ///
    /// # Errors
    ///
    /// Returns an error if the version query fails or if there is an issue
    /// writing to the console.
    pub async fn process(&self, findex_rest_client: &FindexClient) -> CliResult<()> {
        let version = findex_rest_client
            .version()
            .await
            .with_context(|| "Can't execute the version query on the findex server")?;

        console::Stdout::new(&version).write()?;

        Ok(())
    }
}
