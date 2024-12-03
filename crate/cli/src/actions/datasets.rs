use base64::{engine::general_purpose, Engine};
use clap::Parser;
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::{EncryptedEntries, Uuids};
use std::collections::HashMap;
use std::error::Error;
use uuid::Uuid;

use crate::{
    actions::console,
    error::result::{CliResult, CliResultHelper},
};

/// Manage encrypted datasets
#[derive(Parser, Debug)]
pub enum DatasetsAction {
    Add(AddEntries),
    Delete(DeleteEntries),
    Get(GetEntries),
}

impl DatasetsAction {
    /// Processes the Datasets action.
    ///
    /// # Arguments
    ///
    /// * `rest_client` - The Findex client used for the action.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem running the action.
    pub async fn process(&self, rest_client: FindexRestClient) -> CliResult<()> {
        match self {
            Self::Add(action) => action.run(rest_client).await?,
            Self::Delete(action) => action.run(rest_client).await?,
            Self::Get(action) => action.run(rest_client).await?,
        };

        Ok(())
    }
}

/// Add datasets entries.
#[derive(Parser, Debug)]
pub struct AddEntries {
    /// The index ID
    #[clap(long, required = true)]
    pub index_id: Uuid,

    /// The entries to add under the format `KEY=VALUE` where:
    /// - `KEY` is a UUID
    /// - `VALUE` is a base64 encoded string
    ///
    /// Can be repeated multiple times
    #[arg(short = 'D', value_parser = parse_key_val::<Uuid, String>)]
    pub entries: Vec<(Uuid, String)>,
}

/// Parse a single key-value pair
fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

impl AddEntries {
    /// Runs the `AddEntries` action.
    ///
    /// # Arguments
    /// * `rest_client` - A reference to the Findex client used to communicate
    ///     with the Findex server.
    ///
    /// # Errors
    /// Returns an error if the query execution on the Findex server fails.
    /// Returns an error if the base64 decoding fails.
    /// Returns an error if the UUID parsing fails.
    pub async fn run(&self, rest_client: FindexRestClient) -> CliResult<String> {
        let mut encrypted_entries = HashMap::new();
        for (key, value) in &self.entries {
            encrypted_entries.insert(*key, general_purpose::STANDARD.decode(value)?);
        }

        let response = rest_client
            .add_entries(&self.index_id, &EncryptedEntries::from(encrypted_entries))
            .await
            .with_context(|| "Can't execute the add entries query on the findex server")?;

        console::Stdout::new(&format!("{response}")).write()?;

        Ok(response.to_string())
    }
}

/// Delete datasets entries using corresponding entries UUID.
#[derive(Parser, Debug)]
pub struct DeleteEntries {
    /// The index ID
    #[clap(long, required = true)]
    pub index_id: Uuid,

    /// The entries UUIDs to delete
    #[clap(long, required = true)]
    pub uuids: Vec<Uuid>,
}

impl DeleteEntries {
    /// Runs the `DeleteEntries` action.
    ///
    /// # Arguments
    ///
    /// * `rest_client` - A reference to the Findex client used to communicate
    ///   with the Findex server.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution on the Findex server fails.
    pub async fn run(&self, rest_client: FindexRestClient) -> CliResult<String> {
        let response = rest_client
            .delete_entries(&self.index_id, &Uuids::from(self.uuids.clone()))
            .await
            .with_context(|| "Can't execute the delete entries query on the findex server")?;

        console::Stdout::new(&response.success).write()?;

        Ok(response.success)
    }
}

/// Get datasets entries using corresponding entries UUID.
/// Returns the entries.
#[derive(Parser, Debug)]
pub struct GetEntries {
    /// The index id
    #[clap(long, required = true)]
    pub index_id: Uuid,

    /// The entries uuids
    #[clap(long, required = true)]
    pub uuids: Vec<Uuid>,
}

impl GetEntries {
    /// Runs the `GetEntries` action.
    ///
    /// # Arguments
    ///
    /// * `rest_client` - A reference to the Findex client used to communicate
    ///     with the Findex server.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution on the Findex server fails.
    /// Returns an error if the UUID parsing fails.
    pub async fn run(&self, rest_client: FindexRestClient) -> CliResult<String> {
        let encrypted_entries = rest_client
            .get_entries(&self.index_id, &Uuids::from(self.uuids.clone()))
            .await
            .with_context(|| "Can't execute the get entries query on the findex server")?;

        console::Stdout::new(&format!("{encrypted_entries}")).write()?;

        Ok(encrypted_entries.to_string())
    }
}
