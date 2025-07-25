use clap::Parser;
use cosmian_findex_client::RestClient;
use cosmian_findex_structs::{CUSTOM_WORD_LENGTH, SearchResults};
use cosmian_kms_cli::reexport::cosmian_kms_client::KmsClient;

use super::{findex_instance::FindexInstance, parameters::FindexParameters};
use crate::{cli_error, error::result::FindexCliResult};

/// Search words among encrypted indexes.
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct SearchAction {
    #[clap(flatten)]
    pub findex_parameters: FindexParameters,
    /// The word to search. Can be repeated.
    #[clap(long)]
    pub keyword: Vec<String>,
}

impl SearchAction {
    /// Search indexed keywords.
    ///
    /// # Errors
    ///
    /// Returns an error if the version query fails or if there is an issue
    /// writing to the console.
    pub async fn run(
        &self,
        rest_client: RestClient,
        kms_client: KmsClient,
    ) -> FindexCliResult<SearchResults> {
        // Either seed key is required or both hmac_key_id and aes_xts_key_id are required
        match (
            &self.findex_parameters.seed_key_id,
            &self.findex_parameters.hmac_key_id,
            &self.findex_parameters.aes_xts_key_id,
        ) {
            (Some(_), None, None) | (None, Some(_), Some(_)) => (),
            _ => {
                return Err(cli_error!(
                    "Either seed key ID is required or both HMAC key ID and AES XTS key ID are \
                     required"
                ));
            }
        }

        let findex_instance = FindexInstance::<CUSTOM_WORD_LENGTH>::instantiate_findex(
            rest_client,
            kms_client,
            self.findex_parameters.clone().instantiate_keys()?,
        )
        .await?;

        findex_instance
            .search(&self.keyword, self.findex_parameters.num_threads)
            .await
    }
}
