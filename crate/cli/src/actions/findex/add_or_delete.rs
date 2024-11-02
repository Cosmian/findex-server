use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::PathBuf,
};

use clap::Parser;
use cloudproof_findex::reexport::cosmian_findex::{
    Data, IndexedValue, IndexedValueToKeywordsMap, Keyword,
};
use cosmian_findex_client::FindexClient;
use tracing::trace;

use super::FindexParameters;
use crate::{
    actions::{console, findex::instantiate_findex},
    error::result::CliResult,
};

#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct AddOrDeleteAction {
    #[clap(flatten)]
    pub(crate) findex_parameters: FindexParameters,

    /// The path to the CSV file containing the data to index
    #[clap(long)]
    pub(crate) csv: PathBuf,
}

impl AddOrDeleteAction {
    /// Converts a CSV file to a hashmap where the keys are indexed values and the values are sets of keywords.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The CSV file cannot be opened.
    /// - There is an error reading the CSV records.
    /// - There is an error converting the CSV records to the expected data types.
    pub(crate) fn csv_to_hashmap(&self) -> CliResult<IndexedValueToKeywordsMap> {
        // read the database
        let mut csv_in_memory = Vec::new();
        let file = File::open(self.csv.clone())?;
        let mut rdr = csv::Reader::from_reader(file);
        for result in rdr.byte_records() {
            // The iterator yields Result<StringRecord, Error>, so we check the
            // error here.
            let record = result?;
            let indexed_value: IndexedValue<Keyword, Data> =
                IndexedValue::Data(Data::from(record.as_slice()));
            trace!("bytes conversion: {:?}", record.as_slice());
            let keywords = record.iter().map(Keyword::from).collect::<HashSet<_>>();
            csv_in_memory.push((indexed_value, keywords));
            trace!("CSV line: {record:?}");
        }
        let result: HashMap<IndexedValue<Keyword, Data>, HashSet<Keyword>> =
            csv_in_memory.iter().cloned().collect();
        trace!("csv_to_hashmap: result: {result:?}");
        Ok(IndexedValueToKeywordsMap::from(result))
    }

    #[allow(clippy::future_not_send)]
    /// Adds the data from the CSV file to the Findex index.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an error instantiating the Findex client.
    /// - There is an error retrieving the user key or label from the Findex parameters.
    /// - There is an error converting the CSV file to a hashmap.
    /// - There is an error adding the data to the Findex index.
    /// - There is an error writing the result to the console.
    pub async fn add(&self, findex_rest_client: FindexClient) -> CliResult<()> {
        let keywords = instantiate_findex(findex_rest_client, &self.findex_parameters.index_id)
            .await?
            .add(
                &self.findex_parameters.user_key()?,
                &self.findex_parameters.label(),
                self.csv_to_hashmap()?,
            )
            .await?;
        trace!("indexing done: keywords: {keywords}");

        console::Stdout::new(&format!("indexing done: keywords: {keywords}")).write()?;

        Ok(())
    }

    #[allow(clippy::future_not_send)]
    /// Deletes the data from the CSV file from the Findex index.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - There is an error instantiating the Findex client.
    /// - There is an error retrieving the user key or label from the Findex parameters.
    /// - There is an error converting the CSV file to a hashmap.
    /// - There is an error deleting the data from the Findex index.    pub async fn delete(&self, `findex_rest_client`: `FindexClient`) -> `CliResult`<()> {
    /// - There is an error writing the result to the console.
    pub async fn delete(&self, findex_rest_client: FindexClient) -> CliResult<()> {
        let keywords = instantiate_findex(findex_rest_client, &self.findex_parameters.index_id)
            .await?
            .delete(
                &self.findex_parameters.user_key()?,
                &self.findex_parameters.label(),
                self.csv_to_hashmap()?,
            )
            .await?;
        trace!("deleting keywords done: {keywords}");

        console::Stdout::new(&format!("deleting keywords done: {keywords}")).write()?;

        Ok(())
    }
}
