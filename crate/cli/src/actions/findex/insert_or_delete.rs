use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::PathBuf,
    sync::Arc,
};

use clap::Parser;
use cosmian_findex::{Findex, IndexADT, Value};
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::{Keyword, KeywordToDataSetsMap, WORD_LENGTH};
use tracing::{instrument, trace};

use crate::error::{result::CliResult, CliError};

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
    pub(crate) fn to_indexed_value_keywords_map(&self) -> CliResult<KeywordToDataSetsMap> {
        let file = File::open(self.csv.clone())?;

        let csv_in_memory = csv::Reader::from_reader(file).byte_records().fold(
            HashMap::new(),
            |mut acc: HashMap<Keyword, HashSet<Value>>, result| {
                if let Ok(record) = result {
                    let indexed_value = Value::from(record.as_slice());
                    // Extract keywords from the record and associate them with the indexed values
                    let keywords = record.iter().map(Keyword::from).collect::<HashSet<_>>();
                    for keyword in keywords {
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

    async fn insert_or_delete(
        &self,
        rest_client: &FindexRestClient,
        is_insert: bool,
    ) -> CliResult<String> {
        // cloning will be eliminated in the future, cf https://github.com/Cosmian/findex-server/issues/28
        let findex = Arc::<Findex<WORD_LENGTH, Value, String, FindexRestClient>>::new(
            rest_client.clone().instantiate_findex(
                self.findex_parameters.index_id,
                &self.findex_parameters.seed()?,
            )?,
        );

        let bindings = self.to_indexed_value_keywords_map()?;

        // Write messages before consuming values to avoid cloning.
        let written_keywords = format!(
            "Indexing done: keywords: {:?}",
            bindings.keys().collect::<Vec<_>>()
        );
        let operation_name = if is_insert { "Indexing" } else { "Deleting" };
        let output = format!("Indexing done: keywords: {written_keywords:?}");

        let handles = bindings
            .0
            .into_iter()
            .map(|(kw, vs)| {
                let findex = findex.clone();
                if is_insert {
                    tokio::spawn(async move { findex.insert(kw, vs).await })
                } else {
                    tokio::spawn(async move { findex.delete(kw, vs).await })
                }
            })
            .collect::<Vec<_>>();

        for h in handles {
            h.await.map_err(|e| CliError::Default(e.to_string()))??;
        }

        trace!("{} done: keywords: {:?}", operation_name, written_keywords);

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
