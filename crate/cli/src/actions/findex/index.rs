use super::FindexParameters;
use crate::{actions::console, error::result::CliResult};
use clap::Parser;
use cloudproof_findex::{
    reexport::{
        cosmian_crypto_core::FixedSizeCBytes,
        cosmian_findex::{Data, IndexedValue, IndexedValueToKeywordsMap, Keyword, Label, UserKey},
    },
    Configuration, InstantiatedFindex,
};
use cosmian_findex_client::FindexClient;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::PathBuf,
};
use tracing::trace;

/// Index data with Findex
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct IndexAction {
    #[clap(flatten)]
    pub findex_parameters: FindexParameters,

    /// The path to the CSV file containing the data to index
    #[clap(long)]
    pub csv: PathBuf,
}

impl IndexAction {
    /// Process the server version action.
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
    pub async fn process(&self, findex_rest_client: &FindexClient) -> CliResult<()> {
        let config = Configuration::Rest(
            findex_rest_client.client.clone(),
            findex_rest_client.server_url.clone(),
            findex_rest_client.server_url.clone(),
        );
        let findex = InstantiatedFindex::new(config).await?;

        let key = hex::decode(self.findex_parameters.key.clone())?;
        let user_key = UserKey::try_from_slice(&key)?;
        let label = Label::from(self.findex_parameters.label.as_str());

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

        let keywords = findex
            .add(
                &user_key,
                &label,
                IndexedValueToKeywordsMap::from(additions),
            )
            .await?;
        trace!("indexing done: keywords: {keywords}");

        console::Stdout::new(&format!("indexing done: keywords: {keywords}")).write()?;

        Ok(())
    }
}
