use std::{collections::HashMap, fs::File, path::PathBuf};

use crate::actions::findex::structs::{Keyword, KeywordToDataSetsMap};
use clap::Parser;

use cosmian_findex::{IndexADT, Value};
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::WORD_LENGTH;
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

    /// Insert or delete indexes
    async fn insert_or_delete(
        &self,
        rest_client: &FindexRestClient,
        is_insert: bool,
    ) -> CliResult<String> {
        let bindings = self.to_indexed_value_keywords_map()?;
        let findex: cosmian_findex::Findex<WORD_LENGTH, Value, String, FindexRestClient> =
        // cloning will be eliminated in the future, cf https://github.com/Cosmian/findex-server/issues/28
            rest_client.clone().instantiate_findex(
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

    /// Insert new indexes
    ///
    /// # Errors
    /// - If insert new indexes fails
    pub async fn insert(&self, rest_client: &mut FindexRestClient) -> CliResult<String> {
        Self::insert_or_delete(self, rest_client, true).await
    }

    /// Deletes indexes
    ///
    /// # Errors
    /// - If deleting indexes fails
    pub async fn delete(&self, rest_client: &mut FindexRestClient) -> CliResult<String> {
        Self::insert_or_delete(self, rest_client, false).await
    }
}
