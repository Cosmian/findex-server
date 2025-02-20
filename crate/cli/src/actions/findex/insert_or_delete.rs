use super::parameters::FindexParameters;
use crate::{
    actions::findex::MAX_PERMITS,
    error::{result::CliResult, CliError},
};
use clap::Parser;
use cosmian_findex::{Findex, IndexADT, Value};
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::{Keyword, WORD_LENGTH};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::Semaphore;
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
        self,
        rest_client: FindexRestClient,
        is_insert: bool,
    ) -> CliResult<String> {
        let file = File::open(self.csv)?;

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

        let findex: Arc<Findex<200, Value, String, FindexRestClient>> =
            Arc::<Findex<WORD_LENGTH, Value, String, FindexRestClient>>::new(
                rest_client.instantiate_findex(
                    self.findex_parameters.index_id,
                    &self.findex_parameters.seed()?,
                )?,
            );

        let semaphore = Arc::new(Semaphore::new(MAX_PERMITS));
        let written_keywords = format!("{:?}", bindings.keys().collect::<Vec<_>>());

        let handles = bindings
            .into_iter()
            .map(|(kw, vs)| {
                let findex = findex.clone();
                let semaphore = semaphore.clone();
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.map_err(|e| {
                        CliError::Default(format!("failed to acquire permit with error: {e:?}"))
                    })?;
                    if is_insert {
                        findex.insert(kw, vs).await?;
                    } else {
                        findex.delete(kw, vs).await?;
                    }
                    Ok::<_, CliError>(())
                })
            })
            .collect::<Vec<_>>();

        for h in handles {
            h.await.map_err(|e| CliError::Default(e.to_string()))??;
        }

        Ok(written_keywords)
    }

    /// Insert new indexes
    ///
    /// # Errors
    /// - If insert new indexes fails
    pub async fn insert(self, rest_client: FindexRestClient) -> CliResult<String> {
        Self::insert_or_delete(self, rest_client, true)
            .await
            .map(|fmt| {
                trace!("Insert done: keywords: {fmt}");
                format!("Inserted keywords: {fmt}")
            })
    }

    /// Deletes indexes
    ///
    /// # Errors
    /// - If deleting indexes fails
    pub async fn delete(self, rest_client: FindexRestClient) -> CliResult<String> {
        Self::insert_or_delete(self, rest_client, false)
            .await
            .map(|fmt| {
                trace!("Delete done: keywords: {fmt}");
                format!("Deleted keywords: {fmt}")
            })
    }
}
