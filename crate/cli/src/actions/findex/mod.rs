use clap::Parser;
use cloudproof_findex::{
    reexport::{
        cosmian_crypto_core::FixedSizeCBytes,
        cosmian_findex::{Label, UserKey},
    },
    Configuration, InstantiatedFindex,
};
use cosmian_findex_client::FindexRestClient;
use tracing::debug;
use uuid::Uuid;

use crate::error::result::CliResult;

pub mod index_or_delete;
pub mod search;

#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct FindexParameters {
    /// The user findex key used (to add, search, delete and compact).
    /// The key is a 16 bytes hex string.
    #[clap(long, short = 'k')]
    pub key: String,
    /// The Findex label
    #[clap(long, short = 'l')]
    pub label: String,
    /// The index ID
    #[clap(long, short = 'i')]
    pub index_id: Uuid,
}

impl FindexParameters {
    /// Returns the user key decoded from hex.
    /// # Errors
    /// This function will return an error if the key is not a valid hex string.
    pub fn user_key(&self) -> CliResult<UserKey> {
        Ok(UserKey::try_from_slice(&hex::decode(self.key.clone())?)?)
    }

    /// Returns the label.
    pub fn label(&self) -> Label {
        Label::from(self.label.as_str())
    }
}

#[allow(clippy::future_not_send)]
/// Instantiates a Findex client.
/// # Errors
/// This function will return an error if there is an error instantiating the
/// Findex client.
pub async fn instantiate_findex(
    rest_client: FindexRestClient,
    index_id: &Uuid,
) -> CliResult<InstantiatedFindex> {
    let config = Configuration::Rest(
        rest_client.client.client,
        rest_client.client.server_url.clone(),
        rest_client.client.server_url,
        index_id.to_string(),
    );
    let findex = InstantiatedFindex::new(config).await?;
    debug!("Findex instantiated");
    Ok(findex)
}
