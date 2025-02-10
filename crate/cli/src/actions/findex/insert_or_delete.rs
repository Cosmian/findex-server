use super::parameters::FindexParameters;
use crate::{
    actions::findex::MAX_PERMITS,
    error::{result::CliResult, CliError},
};
use clap::Parser;
use cosmian_findex::{Findex, IndexADT, Value};
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::{Keyword, KeywordToDataSetsMap, Keywords, WORD_LENGTH};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::Semaphore;
use tracing::{instrument, trace};

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
    #[instrument(err)]
    pub(crate) fn from_csv(p: PathBuf) -> CliResult<KeywordToDataSetsMap> {
        let file = File::open(p)?;

        let bindings = csv::Reader::from_reader(file).byte_records().fold(
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

    async fn insert_or_delete(
        &self,
        rest_client: FindexRestClient,
        is_insert: bool,
    ) -> CliResult<Keywords> {
        let bindings = Self::from_csv(self.csv.clone())?;

        let findex: Arc<Findex<200, Value, String, FindexRestClient>> =
            Arc::<Findex<WORD_LENGTH, Value, String, FindexRestClient>>::new(
                // cheap reference clone
                rest_client.instantiate_findex(
                    self.findex_parameters.index_id,
                    &self.findex_parameters.seed()?,
                )?,
            );
        let semaphore = Arc::new(Semaphore::new(MAX_PERMITS));
        let written_keywords = bindings.keys().collect::<Vec<_>>();

        let handles = bindings
            .clone()
            .0
            .into_iter()
            .map(|(kw, vs)| {
                let findex = findex.clone();
                let semaphore = semaphore.clone();
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await;
                    if is_insert {
                        findex.insert(kw, vs).await
                    } else {
                        findex.delete(kw, vs).await
                    }
                })
            })
            .collect::<Vec<_>>();

        for h in handles {
            h.await.map_err(|e| CliError::Default(e.to_string()))??;
        }

        let operation_name = if is_insert { "Indexing" } else { "Deleting" };

        trace!("{} done: keywords: {:?}", operation_name, &written_keywords);

        Ok(written_keywords.into())
    }

    /// Insert new indexes
    ///
    /// # Errors
    /// - If insert new indexes fails
    pub async fn insert(&self, rest_client: FindexRestClient) -> CliResult<Keywords> {
        Self::insert_or_delete(self, rest_client, true).await
    }

    /// Deletes indexes
    ///
    /// # Errors
    /// - If deleting indexes fails
    pub async fn delete(&self, rest_client: FindexRestClient) -> CliResult<Keywords> {
        Self::insert_or_delete(self, rest_client, false).await
    }
}
