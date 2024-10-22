use crate::error::result::CliResult;
use clap::{Parser, Subcommand};
use cosmian_findex_client::FindexClient;
use index::IndexAction;
use search::SearchAction;

pub mod index;
pub mod search;

/// Index data with Findex
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
// todo(manu): review global struct exposition
pub struct FindexParameters {
    /// The user findex key used to index and search
    #[clap(long, short = 'k')]
    pub key: String,
    /// The Findex label
    #[clap(long, short = 'l')]
    pub label: String,
}

/// Index or Search with Findex
#[derive(Subcommand)]
pub enum FindexCommands {
    Index(IndexAction),
    Search(SearchAction),
}

impl FindexCommands {
    /// Process the Findex commands action.
    ///
    /// # Arguments
    ///
    /// * `findex_rest_client` - The Findex client instance used to communicate
    ///   with the Findex server.
    ///
    /// # Errors
    ///
    /// Returns an error if the version query fails or if there is an issue
    /// writing to the console.
    #[allow(clippy::future_not_send)] // todo(manu): remove this
    pub async fn process(&self, findex_rest_client: &FindexClient) -> CliResult<()> {
        match self {
            Self::Index(action) => action.process(findex_rest_client).await,
            Self::Search(action) => action.process(findex_rest_client).await,
        }
    }
}
