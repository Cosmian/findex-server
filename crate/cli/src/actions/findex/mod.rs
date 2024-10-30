use clap::Parser;
use cloudproof_findex::{
    reexport::{
        cosmian_crypto_core::FixedSizeCBytes,
        cosmian_findex::{Label, UserKey},
    },
    Configuration, InstantiatedFindex,
};
use cosmian_findex_client::FindexClient;
use tracing::debug;

use crate::error::result::CliResult;

pub mod add_or_delete;
pub mod search;

#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
// todo(manu): review global struct exposition
pub(crate) struct FindexParameters {
    /// The user findex key used (to add, search, delete and compact).
    /// The key is a 16 bytes hex string.
    #[clap(long, short = 'k')]
    pub key: String,
    /// The Findex label
    #[clap(long, short = 'l')]
    pub label: String,
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
    findex_rest_client: FindexClient,
) -> CliResult<InstantiatedFindex> {
    let config = Configuration::Rest(
        findex_rest_client.client,
        findex_rest_client.server_url.clone(),
        findex_rest_client.server_url,
    );
    let findex = InstantiatedFindex::new(config).await?;
    debug!("Findex instantiated");
    Ok(findex)
}
