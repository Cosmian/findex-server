use crate::error::result::CliResult;
use clap::Parser;
use cosmian_findex::{IndexADT, Value};
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::Keywords;
use std::collections::HashSet;

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
    /// # Errors
    ///
    /// Returns an error if the version query fails or if there is an issue
    /// writing to the console.
    pub async fn run(&self, rest_client: &mut FindexRestClient) -> CliResult<HashSet<Value>> {
        // cloning will be eliminated in the future, cf https://github.com/Cosmian/findex-server/issues/28
        let findex_instance = rest_client.clone().instantiate_findex(
            &self.findex_parameters.index_id,
            &self.findex_parameters.seed()?,
        )?;
        let mut search_results: HashSet<Value> = HashSet::new();
        for k in Keywords::from(self.keyword.clone()).0 {
            let search_result = findex_instance.search(&k).await?;
            search_results.extend(search_result);
        }
        Ok(search_results)
    }
}
