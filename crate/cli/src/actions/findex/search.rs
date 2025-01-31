use crate::error::{result::CliResult, CliError};
use clap::Parser;
use cosmian_findex::IndexADT;
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::Keywords;
use cosmian_findex_structs::SearchResults;
use std::sync::Arc;

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
    /// Returns the intersection of the values bound to the searched keywords.
    ///
    /// # Errors
    ///
    /// Returns an error if the version query fails or if there is an issue
    /// writing to the console.
    pub async fn run(&self, rest_client: &mut FindexRestClient) -> CliResult<SearchResults> {
        // cloning will be eliminated in the future, cf https://github.com/Cosmian/findex-server/issues/28
        let findex_instance = Arc::new(rest_client.clone().instantiate_findex(
            self.findex_parameters.index_id,
            &self.findex_parameters.seed()?,
        )?);

        // Execute all queries in parallel.
        let all_results = {
            let mut handles = Vec::with_capacity(self.keyword.len());
            for k in Keywords::from(self.keyword.clone()).0 {
                let findex = findex_instance.clone();
                handles.push(tokio::spawn(async move { findex.search(&k).await }));
            }

            let mut res = Vec::with_capacity(handles.len());
            for h in handles {
                res.push(h.await.map_err(|e| CliError::Default(e.to_string()))??);
            }
            res
        };

        // Compute the intersection of the results.
        #[allow(clippy::pedantic)] // contradicting lints
        let search_results = all_results
            .first()
            .map(|result| {
                all_results
                    .get(1..)
                    .map(|res| {
                        res.iter().fold(result.clone(), |mut acc, results| {
                            acc.retain(|v| results.contains(v));
                            acc
                        })
                    })
                    .unwrap_or_else(|| result.clone())
            })
            .unwrap_or_default();

        Ok(SearchResults(search_results))
    }
}
