use clap::Parser;
use cosmian_findex_client::FindexRestClient;

use super::console;
use crate::error::result::{CliResult, CliResultHelper};

/// Print the version of the server
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct ServerVersionAction;

impl ServerVersionAction {
    /// Process the server version action.
    ///
    /// # Errors
    ///
    /// Returns an error if the version query fails or if there is an issue
    /// writing to the console.
    pub async fn process(&self, rest_client: FindexRestClient) -> CliResult<()> {
        let version = rest_client
            .version()
            .await
            .with_context(|| "Can't execute the version query on the findex server")?;

        console::Stdout::new(&version).write()?;

        Ok(())
    }
}
