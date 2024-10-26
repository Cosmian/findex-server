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

/// Findex: Index data.
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct AddAction {
    #[clap(flatten)]
    pub(crate) findex_parameters: FindexParameters,

    /// The path to the CSV file containing the data to index
    #[clap(long)]
    pub(crate) csv: PathBuf,
}

impl AddAction {
    /// Add keywords to be indexed with `Findex`.
    ///
    /// # Arguments
    ///
    /// * `findex_rest_client` - The Findex server client instance used to
    ///   communicate with the Findex server server.
    ///
    /// # Errors
    ///
    /// Returns an error if the version query fails or if there is an issue
    /// writing to the console.
    #[allow(clippy::future_not_send)]
    pub async fn process(&self, findex_rest_client: FindexClient) -> CliResult<()> {
        // read the database
        let mut csv_additions = Vec::new();
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
            csv_additions.push((indexed_value, keywords));
            trace!("CSV line: {record:?}");
        }
        let additions: HashMap<IndexedValue<Keyword, Data>, HashSet<Keyword>> =
            csv_additions.iter().cloned().collect();
        trace!("additions: {additions:?}");

        let findex = instantiate_findex(findex_rest_client).await?;
        let keywords = findex
            .add(
                &self.findex_parameters.user_key()?,
                &self.findex_parameters.label(),
                IndexedValueToKeywordsMap::from(additions),
            )
            .await?;
        trace!("indexing done: keywords: {keywords}");

        console::Stdout::new(&format!("indexing done: keywords: {keywords}")).write()?;

        Ok(())
    }
}
