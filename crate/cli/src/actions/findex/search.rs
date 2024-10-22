use super::FindexParameters;
use crate::{actions::console, error::result::CliResult};
use clap::Parser;
use cloudproof_findex::{
    reexport::{
        cosmian_crypto_core::FixedSizeCBytes,
        cosmian_findex::{Keyword, Keywords, Label, UserKey},
    },
    Configuration, InstantiatedFindex,
};
use cosmian_findex_client::FindexClient;
use tracing::trace;

/// Index data with Findex
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct SearchAction {
    #[clap(flatten)]
    pub findex_parameters: FindexParameters,

    /// The word to search. Can be repeated.
    #[clap(long)]
    pub word: Vec<String>,
}

impl SearchAction {
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
    #[allow(clippy::future_not_send)] // todo(manu): remove this
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

        let results = findex
            .search(
                &user_key,
                &label,
                self.word
                    .clone()
                    .into_iter()
                    .map(|word| Keyword::from(word.as_bytes()))
                    .collect::<Keywords>(),
                &|_| async move { Ok(false) },
            )
            .await?;

        console::Stdout::new(&results.to_string()).write()?;
        trace!("Search results: {results}");

        Ok(())
    }
}
