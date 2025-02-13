use std::sync::Arc;

use crate::error::{result::CliResult, CliError};
use clap::Parser;
use cosmian_findex::IndexADT;
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::{Keywords, SearchResults};
use tokio::sync::Semaphore;

use super::{parameters::FindexParameters, MAX_SEMAPHORES};

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

        let semaphore = Arc::new(Semaphore::new(MAX_SEMAPHORES));

        // First accumulate all search results in a vector
        let mut all_results = Vec::new();

        let handles = Keywords::from(self.keyword.clone())
            .0
            .into_iter()
            .map(|k| {
                let semaphore = semaphore.clone();
                let findex_instance = findex_instance.clone();
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await;
                    findex_instance.search(&k).await
                })
            })
            .collect::<Vec<_>>();

        for h in handles {
            all_results.push(h.await.map_err(|e| CliError::Default(e.to_string()))??);
        }

        // Then take the intersection of all search results
        let search_results = all_results
            .into_iter()
            .reduce(|acc, results| acc.intersection(&results).cloned().collect())
            .ok_or_else(|| CliError::Default("No search results found".to_owned()))?;

        Ok(SearchResults(search_results))
    }
}
