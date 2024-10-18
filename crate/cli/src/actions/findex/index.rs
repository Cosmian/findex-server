use std::collections::{HashMap, HashSet};

use clap::Parser;
use cloudproof_findex::{
    db_interfaces::DbInterfaceError,
    reexport::{
        cosmian_crypto_core::FixedSizeCBytes,
        cosmian_findex::{Data, IndexedValue, IndexedValueToKeywordsMap, Keyword, Label, UserKey},
    },
    Configuration, InstantiatedFindex,
};
use cosmian_findex_client::FindexClient;
use serde::{Deserialize, Serialize};
use tracing::trace;

use super::FindexParameters;
use crate::{
    actions::console,
    error::result::{CliResult, CliResultHelper},
};

// to be deleted - start
// todo(manu): replace this by adding a CLI argument to provide the dataset file

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub(crate) firstName: String,
    pub(crate) lastName: String,
    pub(crate) phone: String,
    pub(crate) email: String,
    pub(crate) country: String,
    pub(crate) region: String,
    pub(crate) employeeNumber: String,
    pub(crate) security: String,
}

impl User {
    #[must_use]
    pub fn values(&self) -> Vec<String> {
        vec![
            self.firstName.clone(),
            self.lastName.clone(),
            self.phone.clone(),
            self.email.clone(),
            self.country.clone(),
            self.region.clone(),
            self.employeeNumber.clone(),
            self.security.clone(),
        ]
    }
}

/// Get the users from the dataset
/// # Errors
/// It returns an error if the dataset cannot be read or if the dataset cannot
/// be deserialized into a list of users
pub fn get_users() -> Result<Vec<User>, DbInterfaceError> {
    trace!("Current working directory: {:?}", std::env::current_dir()?);
    let dataset = std::fs::read_to_string("../../crate/client/datasets/users.json")?;
    serde_json::from_str::<Vec<User>>(&dataset)
        .map_err(|e| DbInterfaceError::Serialization(e.to_string()))
}
// to be deleted - end

/// Index data with Findex
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct IndexAction {
    #[clap(flatten)]
    pub findex_parameters: FindexParameters,
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

        // to be deleted - start
        let users = get_users()?;
        #[allow(clippy::cast_possible_wrap, clippy::as_conversions)]
        let additions = users
            .iter()
            .enumerate()
            .map(|(idx, user)| {
                (
                    IndexedValue::Data(Data::from((idx as i64).to_be_bytes().as_slice())),
                    user.values()
                        .iter()
                        .map(|word| Keyword::from(word.as_bytes()))
                        .collect::<HashSet<_>>(),
                )
            })
            .collect::<Vec<(IndexedValue<Keyword, Data>, HashSet<Keyword>)>>();
        let additions: HashMap<IndexedValue<Keyword, Data>, HashSet<Keyword>> =
            additions.iter().cloned().collect();
        // to be deleted - end

        findex
            .add(
                &user_key,
                &label,
                IndexedValueToKeywordsMap::from(additions),
            )
            .await?;

        let version = findex_rest_client
            .version()
            .await
            .with_context(|| "Can't execute the version query on the findex server")?;

        console::Stdout::new(&version).write()?;

        Ok(())
    }
}
