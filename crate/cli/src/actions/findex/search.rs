use std::collections::HashSet;

use crate::{actions::findex::structs::Keywords, error::result::CliResult};
use clap::Parser;
use cosmian_findex::{IndexADT, Value};
use cosmian_findex_client::FindexRestClient;
use tracing::trace;

use super::parameters::FindexParameters;

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
    /// Search indexed keywords.
    ///
    /// # Arguments
    ///
    /// * `rest_client` - The Findex server client instance used to communicate
    ///   with the Findex server server.
    ///
    /// # Errors
    ///
    /// Returns an error if the version query fails or if there is an issue
    /// writing to the console.
    // #[allow(clippy::future_not_send)] // todo(manu): to remove this, changes must be done on `findex` repository
    pub async fn run(&self, rest_client: &mut FindexRestClient) -> CliResult<String> {
        // tod(hatem) : optimise post findex pr
        let findex_instance = rest_client.instantiate_findex(
            &self.findex_parameters.index_id,
            &self.findex_parameters.user_key()?,
        )?;
        let mut search_results: Vec<(_, HashSet<Value>)> = Vec::new();
        for k in &Keywords::from(self.keyword.clone()).0 {
            let search_result = findex_instance.search(k).await?;
            search_results.push((k.clone(), search_result));
        }

        let formatted_string = search_results
            .iter()
            .map(|(key, value)| format!("{key}: {value:?}"))
            .collect::<Vec<_>>()
            .join("\n");
        trace!("Search results: {formatted_string}");

        Ok(formatted_string)
    }
}
