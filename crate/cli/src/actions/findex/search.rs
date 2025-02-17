use super::{parameters::FindexParameters, MAX_PERMITS};
use crate::error::{result::CliResult, CliError};
use clap::Parser;
use cosmian_findex::IndexADT;
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::{Keywords, SearchResults};
use std::sync::Arc;
use tokio::sync::Semaphore;

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

        // cloning will be eliminated in the future, cf https://github.com/Cosmian/findex-server/issues/28
        let findex_instance = rest_client.clone().instantiate_findex(
            self.findex_parameters.index_id,
            &self.findex_parameters.seed()?,
        )?;

        let semaphore = Arc::new(Semaphore::new(MAX_PERMITS));

        let mut handles = keywords
            .into_iter()
            .map(|k| {
                let semaphore = semaphore.clone();
                let findex_instance = findex_instance.clone();
                tokio::spawn(async move {
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|e| cosmian_findex::Error::Conversion(e.to_string()))?;
                    findex_instance.search(&k).await
                })
            })
            .collect::<Vec<_>>();

        let mut acc_results = handles
            .pop()
            .ok_or_else(|| CliError::Default("No search handles available".to_owned()))?
            .await
            .map_err(|e| CliError::Default(e.to_string()))??;

        for h in handles {
            // The empty set is the fixed point of the intersection.
            if acc_results.is_empty() {
                break;
            }
            let next_search_result = h.await.map_err(|e| CliError::Default(e.to_string()))??;
            acc_results.retain(|item| next_search_result.contains(item));
        }

        Ok(SearchResults(acc_results))
    }
}
