use super::parameters::FindexParameters;
use crate::{
    actions::findex::{instantiated_findex::InstantiatedFindex, retrieve_key_from_kms},
    cli_error,
    error::result::CliResult,
};
use clap::Parser;
use cosmian_findex::{MemoryEncryptionLayer, Value};
use cosmian_findex_client::{FindexRestClient, KmsEncryptionLayer, RestClient};
use cosmian_findex_structs::{Keyword, Keywords, CUSTOM_WORD_LENGTH};
use cosmian_kms_cli::reexport::cosmian_kms_client::KmsClient;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::PathBuf,
};
use tracing::trace;

#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct InsertOrDeleteAction {
    #[clap(flatten)]
    pub(crate) findex_parameters: FindexParameters,
    /// The path to the CSV file containing the data to index
    #[clap(long)]
    pub(crate) csv: PathBuf,
}

impl InsertOrDeleteAction {
    /// First, converts a CSV file to a hashmap where the keys are keywords and
    /// the values are sets of indexed values (Data). Then, inserts or deletes
    /// using the Findex instance.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The CSV file cannot be opened.
    /// - There is an error reading the CSV records.
    /// - There is an error converting the CSV records to the expected data
    ///   types.
    /// - The Findex instance cannot be instantiated.
    /// - The Findex instance cannot insert or delete the data.
    /// - The semaphore cannot acquire a permit.
    async fn insert_or_delete(
        &self,
        rest_client: &RestClient,
        kms_client: KmsClient,
        is_insert: bool,
    ) -> CliResult<Keywords> {
        let file = File::open(self.csv.clone())?;

        let bindings = csv::Reader::from_reader(file).byte_records().fold(
            HashMap::new(),
            |mut acc: HashMap<Keyword, HashSet<Value>>, result| {
                if let Ok(record) = result {
                    let indexed_value = Value::from(record.as_slice());
                    // Extract keywords from the record and associate them with the indexed values
                    // Index the lowercase only
                    for keyword in record
                        .iter()
                        .map(|x| Keyword::from(x.to_ascii_lowercase().as_slice()))
                    {
                        acc.entry(keyword)
                            .or_default()
                            .insert(indexed_value.clone());
                    }
                }
                acc
            },
        );

        let memory = FindexRestClient::new(rest_client.clone(), self.findex_parameters.index_id);

        let (operation_name, written_keywords) =
            if let Some(seed_key_id) = &self.findex_parameters.seed_key_id {
                trace!("Using client side encryption");
                let seed = retrieve_key_from_kms(seed_key_id, kms_client.clone()).await?;

                let encryption_layer =
                    MemoryEncryptionLayer::<CUSTOM_WORD_LENGTH, _>::new(&seed, memory);

                let findex = InstantiatedFindex::new(encryption_layer);
                let written_keywords = findex.insert_or_delete(bindings, is_insert).await?;
                let operation_name = if is_insert { "Indexing" } else { "Deleting" };
                (operation_name, written_keywords)
            } else {
                trace!("Using KMS server side encryption");
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
                    kms_client,
                    hmac_key_id,
                    aes_xts_key_id,
                    memory,
                );

                let findex = InstantiatedFindex::new(encryption_layer);
                let written_keywords = findex.insert_or_delete(bindings, is_insert).await?;
                let operation_name = if is_insert { "Indexing" } else { "Deleting" };
                (operation_name, written_keywords)
            };

        trace!("{operation_name} is done. Keywords: {written_keywords}");
        Ok(written_keywords)
    }

    /// Insert new indexes
    ///
    /// # Errors
    /// - If insert new indexes fails
    pub async fn insert(
        &self,
        rest_client: &mut RestClient,
        kms_client: KmsClient,
    ) -> CliResult<Keywords> {
        Self::insert_or_delete(self, rest_client, kms_client, true).await
    }

    /// Deletes indexes
    ///
    /// # Errors
    /// - If deleting indexes fails
    pub async fn delete(
        &self,
        rest_client: &mut RestClient,
        kms_client: KmsClient,
    ) -> CliResult<Keywords> {
        Self::insert_or_delete(self, rest_client, kms_client, false).await
    }
}
