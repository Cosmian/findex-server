use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::PathBuf,
};

use crate::actions::findex::structs::{Keyword, KeywordToDataSetsMap};
use clap::Parser;

use cosmian_findex::{IndexADT, Value};
use cosmian_findex_client::FindexRestClient;
use tracing::{instrument, trace};

use crate::{actions::console, error::result::CliResult};

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
    /// Reads a CSV file and maps each record to a set of keywords to HashSet<Data>.
    #[instrument(err, skip(self))]
    pub(crate) fn to_indexed_value_keywords_map(&self) -> CliResult<KeywordToDataSetsMap> {
        // Initialize a HashMap to store CSV data in memory
        let mut csv_in_memory: KeywordToDataSetsMap = KeywordToDataSetsMap(HashMap::new());

        // Open the CSV file
        let file = File::open(self.csv.clone())?;
        let mut rdr = csv::Reader::from_reader(file);

        // Iterate over each record in the CSV file
        for result in rdr.byte_records() {
            // Check for errors in reading the record
            let record = result?;

            // Convert the record into a Findex indexed value
            let indexed_value = Value::from(record.as_slice());

            // Extract keywords from the record and associate them with the indexed values
            record.iter().map(Keyword::from).for_each(|k| {
                csv_in_memory
                    .entry(k)
                    .or_insert_with(HashSet::new)
                    .insert(indexed_value.clone());
            });

            // Log the CSV line for traceability
            trace!("CSV line: {record:?}");
        }

        // Return the resulting HashMap
        Ok(csv_in_memory)
    }

    async fn add_or_delete(&self, rest_client: FindexRestClient, is_insert: bool) -> CliResult<()> {
        let bindings = self.to_indexed_value_keywords_map()?;
        let iterable_bindings = bindings.iter().map(|(k, v)| (k.clone(), v.clone()));
        let findex = rest_client
            .instantiate_findex(
                &self.findex_parameters.index_id,
                &self.findex_parameters.user_key()?,
            )
            .unwrap();
        if is_insert {
            findex.insert(iterable_bindings).await
        } else {
            findex.delete(iterable_bindings).await
        }?;
        let written_keywords = bindings.keys().collect::<Vec<_>>();
        let operation_name = if is_insert { "Indexing" } else { "Deleting" };
        trace!("{} done: keywords: {:?}", operation_name, written_keywords);

        console::Stdout::new(&format!(
            "{} done: keywords: {:?}",
            operation_name, written_keywords
        ))
        .write()?;

        Ok(())
    }

    #[allow(clippy::future_not_send)]
    /// Adds the data from the CSV file to the Findex index.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an error instantiating the Findex client.
    /// - There is an error retrieving the user key or label from the Findex
    ///   parameters.
    /// - There is an error converting the CSV file to a hashmap.
    /// - There is an error adding the data to the Findex index.
    /// - There is an error writing the result to the console.
    pub async fn add(&self, rest_client: FindexRestClient) -> CliResult<()> {
        Self::add_or_delete(self, rest_client, true).await
    }

    /// Deletes the data from the CSV file from the Findex index.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an error instantiating the Findex client.
    /// - There is an error retrieving the user key or label from the Findex
    ///   parameters.
    /// - There is an error converting the CSV file to a hashmap.
    /// - There is an error deleting the data from the Findex index.
    /// - There is an error writing the result to the console.
    pub async fn delete(&self, rest_client: FindexRestClient) -> CliResult<()> {
        Self::add_or_delete(self, rest_client, false).await
    }
}
