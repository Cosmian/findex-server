use super::{parameters::FindexParameters, MAX_PERMITS};
use crate::error::{result::CliResult, CliError};
use clap::Parser;
use cosmian_findex::IndexADT;
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::{Keyword, SearchResults};
use std::{collections::HashSet, sync::Arc};
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
    pub async fn run(&self, rest_client: FindexRestClient) -> CliResult<SearchResults> {
        let keywords = Keywords::from(self.keyword.clone()).0;
        if keywords.is_empty() {
            return Err(CliError::Default("No search results found".to_owned()));
        }

        let findex_instance = rest_client.instantiate_findex(
            self.findex_parameters.index_id,
            &self.findex_parameters.seed()?,
        )?;

        let semaphore = Arc::new(Semaphore::new(MAX_PERMITS));

        let mut handles = self
            .keyword
            .iter()
            .map(|k| {
                let semaphore = semaphore.clone();
                let k = Keyword::from(k.as_ref());
                let findex_instance = findex_instance.clone();
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.map_err(|e| {
                        CliError::Default(format!(
                            "Acquire error while trying to ask for permit: {e:?}"
                        ))
                    })?;
                    Ok::<_, CliError>(findex_instance.search(&k).await?)
                })
            })
            .collect::<Vec<_>>();

        if let Some(initial_handle) = handles.pop() {
            let mut acc_results = initial_handle
                .await
                .map_err(|e| CliError::Default(e.to_string()))??;
            for h in handles {
                // The empty set is the fixed point of the intersection.
                if acc_results.is_empty() {
                    break;
                }
                let next_search_result =
                    h.await.map_err(|e| CliError::Default(e.to_string()))??;
                acc_results.retain(|item| next_search_result.contains(item));
            }
            Ok(SearchResults(acc_results))
        } else {
            Ok(SearchResults(HashSet::new()))
        }
    }
}
