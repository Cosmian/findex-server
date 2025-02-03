use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::PathBuf,
};

use clap::Parser;
use cosmian_findex::{Findex, IndexADT, Value};
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::{Keyword, KeywordToDataSetsMap, Keywords, WORD_LENGTH};
use tracing::{instrument, trace};

use crate::error::result::CliResult;

use super::parameters::FindexParameters;

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
    pub(crate) fn to_keywords_indexed_value_map(&self) -> CliResult<KeywordToDataSetsMap> {
        let file = File::open(self.csv.clone())?;

        let csv_in_memory = csv::Reader::from_reader(file).byte_records().fold(
            HashMap::new(),
            |mut acc: HashMap<Keyword, HashSet<Value>>, result| {
                if let Ok(record) = result {
                    let indexed_value = Value::from(record.as_slice());
                    // Extract keywords from the record and associate them with the indexed values
                    for keyword in record.iter().map(Keyword::from) {
                        acc.entry(keyword)
                            .or_default()
                            .insert(indexed_value.clone());
                    }
                }
                acc
            },
        );
        trace!("CSV lines are OK");
        Ok(KeywordToDataSetsMap(csv_in_memory))
    }

    /// Insert or delete indexes
    async fn insert_or_delete(
        &self,
        rest_client: &FindexRestClient,
        is_insert: bool,
    ) -> CliResult<Keywords> {
        let keywords_indexed_value = self.to_keywords_indexed_value_map()?;
        let findex: Findex<WORD_LENGTH, Value, String, FindexRestClient> =
        // cloning will be eliminated in the future, cf https://github.com/Cosmian/findex-server/issues/28
            rest_client.clone().instantiate_findex(
                self.findex_parameters.index_id,
                &self.findex_parameters.seed()?,
            )?;
        for (key, value) in keywords_indexed_value.iter() {
            if is_insert {
                trace!("Attempt to insert ...");
                findex.insert(key, value.clone()).await
            } else {
                findex.delete(key, value.clone()).await
            }?;
        }
        let written_keywords = keywords_indexed_value.keys().collect::<Vec<_>>();
        let operation_name = if is_insert { "Indexing" } else { "Deleting" };
        let written_keywords = Keywords::from(written_keywords);

        trace!("{} done: keywords: {}", operation_name, written_keywords);
        Ok(written_keywords)
    }

    /// Insert new indexes
    ///
    /// # Errors
    /// - If insert new indexes fails
    pub async fn insert(&self, rest_client: &mut FindexRestClient) -> CliResult<Keywords> {
        Self::insert_or_delete(self, rest_client, true).await
    }

    /// Deletes indexes
    ///
    /// # Errors
    /// - If deleting indexes fails
    pub async fn delete(&self, rest_client: &mut FindexRestClient) -> CliResult<Keywords> {
        Self::insert_or_delete(self, rest_client, false).await
    }
}
