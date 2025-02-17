use super::{
    instantiated_findex::InstantiatedFindex, parameters::FindexParameters, retrieve_key_from_kms,
};
use crate::{cli_error, error::result::CliResult};
use clap::Parser;
use cosmian_findex::MemoryEncryptionLayer;
use cosmian_findex_client::{FindexRestClient, KmsEncryptionLayer, RestClient};
use cosmian_findex_structs::{SearchResults, CUSTOM_WORD_LENGTH};
use cosmian_kms_cli::reexport::cosmian_kms_client::KmsClient;

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
    pub async fn run(
        &self,
        rest_client: &mut RestClient,
        kms_client: &KmsClient,
    ) -> CliResult<SearchResults> {
        let memory = FindexRestClient::new(rest_client.clone(), self.findex_parameters.index_id);

        let search_results = if let Some(seed_key_id) = self.findex_parameters.seed_key_id.clone() {
            let seed = retrieve_key_from_kms(&seed_key_id, kms_client.clone()).await?;

            let encryption_layer =
                MemoryEncryptionLayer::<CUSTOM_WORD_LENGTH, _>::new(&seed, memory);

            let findex = InstantiatedFindex::new(encryption_layer);
            findex.search(&self.keyword).await?
        } else {
            let hmac_key_id = self
                .findex_parameters
                .hmac_key_id
                .clone()
                .ok_or_else(|| cli_error!("The HMAC key ID is required for indexing"))?;
            let aes_xts_key_id = self
                .findex_parameters
                .aes_xts_key_id
                .clone()
                .ok_or_else(|| cli_error!("The AES XTS key ID is required for indexing"))?;

            let encryption_layer = KmsEncryptionLayer::<CUSTOM_WORD_LENGTH, _>::new(
                kms_client.clone(),
                hmac_key_id.clone(),
                aes_xts_key_id.clone(),
                memory,
            );

            let findex = InstantiatedFindex::new(encryption_layer);
            findex.search(&self.keyword).await?
        };

        Ok(search_results)
    }
}
