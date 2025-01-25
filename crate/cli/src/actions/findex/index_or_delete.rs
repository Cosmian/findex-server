use std::{collections::HashMap, fs::File, path::PathBuf};

use crate::actions::findex::structs::{Keyword, KeywordToDataSetsMap};
use clap::Parser;

use cosmian_findex::{IndexADT, Value, WORD_LENGTH};
use cosmian_findex_client::FindexRestClient;
use tracing::{instrument, trace};

use crate::error::result::CliResult;

use super::parameters::FindexParameters;
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct IndexOrDeleteAction {
    #[clap(flatten)]
    pub(crate) findex_parameters: FindexParameters,

    /// The path to the CSV file containing the data to index
    #[clap(long)]
    pub(crate) csv: PathBuf,
}

impl IndexOrDeleteAction {
    /// Converts a CSV file to a hashmap where the keys are keywords and
    /// the values are sets of indexed values (Data).
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The CSV file cannot be opened.
    /// - There is an error reading the CSV records.
    /// - There is an error converting the CSV records to the expected data
    ///   types.
    #[instrument(err, skip(self))]
    pub(crate) fn to_indexed_value_keywords_map(&self) -> CliResult<KeywordToDataSetsMap> {
        let mut csv_in_memory = KeywordToDataSetsMap(HashMap::new());
        let file = File::open(self.csv.clone())?;
        let mut rdr = csv::Reader::from_reader(file);

        for result in rdr.byte_records() {
            let record = result?;
            let indexed_value = Value::from(record.as_slice());

            // Extract keywords from the record and associate them with the indexed values
            record.iter().map(Keyword::from).for_each(|k| {
                csv_in_memory
                    .entry(k)
                    .or_default()
                    .insert(indexed_value.clone());
            });
        }
        trace!("CSV lines are OK");
        Ok(csv_in_memory)
    }

    /// Processes the add or delete operation using the provided REST client
    ///
    /// # Arguments
    /// * `rest_client` - The Findex REST client to use for operations
    /// * `is_insert` - Boolean flag indicating whether to insert (true) or delete (false)
    ///
    /// # Errors
    /// * Returns `CliError` if:
    ///   - Failed to create indexed value keywords map
    ///   - Failed to instantiate Findex
    ///   - Failed to insert or delete records
    ///
    /// # Returns
    /// * `CliResult<()>` - Ok if operation succeeds, Error otherwise
    async fn add_or_delete(
        &self,
        rest_client: &FindexRestClient,
        is_insert: bool,
    ) -> CliResult<String> {
        let bindings = self.to_indexed_value_keywords_map()?;
        let findex: cosmian_findex::Findex<WORD_LENGTH, Value, String, FindexRestClient> =
            rest_client.instantiate_findex(
                &self.findex_parameters.index_id,
                &self.findex_parameters.user_key()?,
            )?;
        for (key, value) in bindings.iter() {
            if is_insert {
                trace!("Attempt to insert ...");
                findex.insert(key, value.clone()).await
            } else {
                findex.delete(key, value.clone()).await
            }?;
        }
        let written_keywords = bindings.keys().collect::<Vec<_>>();
        let operation_name = if is_insert { "Indexing" } else { "Deleting" };
        trace!("{} done: keywords: {:?}", operation_name, written_keywords);

        let output = format!("indexing done: keywords: {written_keywords:?}",);

        Ok(output)
    }

    #[allow(clippy::future_not_send)]
    /// Adds the data from the CSV file to the Findex index.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an error instantiating the Findex client.
    /// - (deprecated) There is an error retrieving the user key or label from the Findex
    ///   parameters.
    /// - There is an error converting the CSV file to a hashmap.
    /// - There is an error adding the data to the Findex index.
    /// - There is an error writing the result to the console.
    pub async fn add(&self, rest_client: &mut FindexRestClient) -> CliResult<String> {
        Self::add_or_delete(self, rest_client, true).await
    }

    /// Deletes the data from the CSV file from the Findex index.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an error instantiating the Findex client.
    /// - (deprecated) There is an error retrieving the user key or label from the Findex
    ///   parameters.
    /// - There is an error converting the CSV file to a hashmap.
    /// - There is an error deleting the data from the Findex index.
    /// - There is an error writing the result to the console.
    pub async fn delete(&self, rest_client: &mut FindexRestClient) -> CliResult<String> {
        Self::add_or_delete(self, rest_client, false).await
    }
}
