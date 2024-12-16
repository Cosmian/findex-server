use clap::Parser;
use cloudproof_findex::reexport::cosmian_findex::{Keyword, Keywords};
use cosmian_findex_client::FindexRestClient;
use tracing::trace;

use super::FindexParameters;
use crate::{actions::findex::instantiate_findex, error::result::CliResult};

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
    #[allow(clippy::future_not_send)] // TODO(manu): to remove this, changes must be done on `findex` repository
    pub async fn run(&self, rest_client: &FindexRestClient) -> CliResult<()> {
        let findex = instantiate_findex(rest_client, &self.findex_parameters.index_id).await?;
        let results = findex
            .search(
                &self.findex_parameters.user_key()?,
                &self.findex_parameters.label(),
                self.keyword
                    .clone()
                    .into_iter()
                    .map(|word| Keyword::from(word.as_bytes()))
                    .collect::<Keywords>(),
                &|_| async move { Ok(false) },
            )
            .await?;

        println!("{results}");
        trace!("Search results: {results}");

        Ok(())
    }
}
