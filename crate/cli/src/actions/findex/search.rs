use crate::error::{result::CliResult, CliError};
use clap::Parser;
use cosmian_findex::IndexADT;
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::{Keywords, SearchResults};

use super::parameters::FindexParameters;

/// Search words.
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
    /// # Errors
    ///
    /// Returns an error if the version query fails or if there is an issue
    /// writing to the console.
    pub async fn run(&self, rest_client: &mut FindexRestClient) -> CliResult<SearchResults> {
        // cloning will be eliminated in the future, cf https://github.com/Cosmian/findex-server/issues/28
        let findex_instance = rest_client.clone().instantiate_findex(
            self.findex_parameters.index_id,
            &self.findex_parameters.seed()?,
        )?;

        // First accumulate all search results in a vector
        let mut all_results = Vec::new();
        for k in Keywords::from(self.keyword.clone()).0 {
            let search_result = findex_instance.search(&k).await?;
            all_results.push(search_result);
        }

        // Then take the intersection of all search results
        let search_results = all_results
            .into_iter()
            .reduce(|acc, results| acc.intersection(&results).cloned().collect())
            .ok_or_else(|| CliError::Default("No search results found".to_owned()))?;

        Ok(SearchResults(search_results))
    }
}
