use clap::Parser;
use cloudproof_findex::{
    reexport::{
        cosmian_crypto_core::FixedSizeCBytes,
        cosmian_findex::{Label, UserKey},
    },
    Configuration, InstantiatedFindex,
};
use cosmian_rest_client::RestClient;
use tracing::debug;

use crate::error::result::CliResult;

pub mod add_or_delete;
pub mod search;

#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub(crate) struct FindexParameters {
    /// The user findex key used (to add, search, delete and compact).
    /// The key is a 16 bytes hex string.
    #[clap(long, short = 'k')]
    pub key: String,
    /// The Findex label
    #[clap(long, short = 'l')]
    pub label: String,
    /// The index ID
    #[clap(long, short = 'i')]
    pub index_id: String,
}

impl FindexParameters {
    pub(crate) fn user_key(&self) -> CliResult<UserKey> {
        Ok(UserKey::try_from_slice(&hex::decode(self.key.clone())?)?)
    }

    pub(crate) fn label(&self) -> Label {
        Label::from(self.label.as_str())
    }
}

#[allow(clippy::future_not_send)]
pub(crate) async fn instantiate_findex(
    rest_client: RestClient,
    index_id: &str,
) -> CliResult<InstantiatedFindex> {
    let config = Configuration::Rest(
        rest_client.client,
        rest_client.server_url.clone(),
        rest_client.server_url,
        index_id.to_owned(),
    );
    let findex = InstantiatedFindex::new(config).await?;
    debug!("Findex instantiated");
    Ok(findex)
}
