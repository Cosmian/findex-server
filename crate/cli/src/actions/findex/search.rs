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
        let keywords = Keywords::from(self.keyword.clone()).0;
        if keywords.is_empty() {
            return Err(CliError::Default("No search results found".to_owned()));
        }

        // cloning will be eliminated in the future, cf https://github.com/Cosmian/findex-server/issues/28
        let findex_instance = rest_client.clone().instantiate_findex(
            self.findex_parameters.index_id,
            &self.findex_parameters.seed()?,
        )?;

        let semaphores = Arc::new(Semaphore::new(MAX_SEMAPHORES));

        let mut handles = keywords
            .into_iter()
            .map(|k| {
                let semaphores = semaphores.clone();
                let findex_instance = findex_instance.clone();
                tokio::spawn(async move {
                    let _permit = semaphores.acquire().await;
                    findex_instance.search(&k).await
                })
            })
            .collect::<Vec<_>>();

        // for overall effeciency, we perform the first search outside the loop, then we intersect any further results with it
        let mut acc_results = handles
            .remove(0)
            .await
            .map_err(|e| CliError::Default(e.to_string()))??;

        for h in handles {
            // if we have no more results, we can break early because and intersection with an empty set will certainly be empty
            if acc_results.is_empty() {
                break;
            }
            let _a = h.await.map_err(|e| CliError::Default(e.to_string()))??;
            acc_results.retain(|item| _a.contains(item));
        }

        Ok(SearchResults(acc_results))
    }
}
