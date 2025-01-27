use super::parameters::FindexParameters;
use crate::{actions::findex::structs::Keywords, error::result::CliResult};
use clap::Parser;
use cosmian_findex::{IndexADT, Value};
use cosmian_findex_client::FindexRestClient;
use std::collections::HashSet;
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
    pub async fn run(&self, rest_client: &mut FindexRestClient) -> CliResult<String> {
        // cloning will be eliminated in the future, cf https://github.com/Cosmian/findex-server/issues/28
        let findex_instance = rest_client.clone().instantiate_findex(
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
