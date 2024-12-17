use super::FindexParameters;
use crate::{
    actions::{console, findex::structs::Keywords},
    error::result::CliResult,
};
use clap::Parser;
use cosmian_findex::IndexADT;
use cosmian_findex_client::FindexRestClient;
use tracing::trace;

/// Findex: Search keywords.
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct SearchAction {
    #[clap(flatten)]
    pub(crate) findex_parameters: FindexParameters,

    /// The word to search. Can be repeated.
    #[clap(long)]
    pub(crate) keyword: Vec<String>,
}

impl SearchAction {
    // / Search indexed keywords.
    // /
    // / # Arguments
    // /
    // / * `rest_client` - The Findex server client instance used to communicate
    // /   with the Findex server server.
    // /
    // / # Errors
    // /
    // /// Returns an error if the version query fails or if there is an issue
    //zzz/ writing to the console.
    // #[allow(clippy::future_not_send)] // todo(manu): to remove this, changes must be done on `findex` repository
    pub async fn process(&self, rest_client: FindexRestClient) -> CliResult<()> {
        let results = rest_client
            .instantiate_findex(
                &self.findex_parameters.index_id,
                &self.findex_parameters.key,
            )
            .unwrap()
            .search(Keywords::from(self.keyword.clone()).0.iter().cloned()) // TODO(review): is this sub-optimal ? can it be improved some way ?
            .await?;
        let formatted_string = results
            .iter()
            .map(|(key, value)| format!("{}: {:?}", key, value))
            .collect::<Vec<_>>()
            .join("\n");
        console::Stdout::new(&formatted_string).write()?;
        trace!("Search results: {formatted_string}");

        Ok(())
    }
}
