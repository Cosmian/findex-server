use clap::Parser;
use cosmian_rest_client::RestClient;
use tracing::trace;

use crate::{
    actions::console,
    error::result::{CliResult, CliResultHelper},
};

/// Manage the users' access rights to the indexes
#[derive(Parser, Debug)]
pub enum AccessAction {
    Create(CreateAccess),
    Grant(GrantAccess),
    Revoke(RevokeAccess),
}

impl AccessAction {
    /// Processes the access action.
    ///
    /// # Arguments
    ///
    /// * `rest_client` - The Findex client used for the action.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem running the action.
    pub async fn process(&self, rest_client: RestClient) -> CliResult<()> {
        match self {
            Self::Create(action) => action.run(rest_client).await?,
            Self::Grant(action) => action.run(rest_client).await?,
            Self::Revoke(action) => action.run(rest_client).await?,
        };

        Ok(())
    }
}

/// Create a new access right.
#[derive(Parser, Debug)]
pub struct CreateAccess;

impl CreateAccess {
    /// Create a new Index with a default `admin` role.
    ///
    /// Generates an unique index ID which is returned to the owner.
    /// This ID will be shared between several users that will be able to:
    ///   * index new keywords with their own datasets
    ///   * or search keywords in the index
    ///
    /// # Arguments
    ///
    /// * `rest_client` - A reference to the Findex client used to
    ///   communicate with the Findex server.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution on the Findex server fails.
    pub async fn run(&self, rest_client: RestClient) -> CliResult<String> {
        let response = rest_client
            .create_access()
            .await
            .with_context(|| "Can't execute the create access query on the findex server")?;

        trace!("cli: New access successfully created: {}", response.success);
        console::Stdout::new(&response.success).write()?;

        Ok(response.success)
    }
}

/// Grant access.
///
/// This command can only be called by the owner of the index. It allows to
/// grant:
/// * `reader` access: the user can only read the index
/// * `writer` access: the user can read and write the index
/// * `admin` access: the user can read, write and grant access to the index
#[derive(Parser, Debug)]
pub struct GrantAccess {
    /// The user identifier to allow
    #[clap(long, required = true)]
    pub user: String,

    /// The index ID
    #[clap(long, required = true)]
    pub index_id: String,

    /// The role to grant (`reader`, `writer`, `admin`)
    #[clap(long, required = true)]
    pub role: String,
}

impl GrantAccess {
    /// Runs the `GrantAccess` action.
    ///
    /// # Arguments
    ///
    /// * `rest_client` - A reference to the Findex client used to
    ///   communicate with the Findex server.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution on the Findex server fails.
    pub async fn run(&self, rest_client: RestClient) -> CliResult<String> {
        let response = rest_client
            .grant_access(&self.user, &self.role, &self.index_id)
            .await
            .with_context(|| "Can't execute the grant access query on the findex server")?;

        console::Stdout::new(&response.success).write()?;

        Ok(response.success)
    }
}

/// Revoke user access.
///
/// This command can only be called by the owner of the index.
#[derive(Parser, Debug)]
pub struct RevokeAccess {
    /// The user identifier to revoke
    #[clap(long, required = true)]
    pub user: String,

    /// The index id
    #[clap(long, required = true)]
    pub index_id: String,
}

impl RevokeAccess {
    /// Runs the `RevokeAccess` action.
    ///
    /// # Arguments
    ///
    /// * `rest_client` - A reference to the Findex client used to
    ///   communicate with the Findex server.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution on the Findex server fails.
    pub async fn run(&self, rest_client: RestClient) -> CliResult<String> {
        let response = rest_client
            .revoke_access(&self.user, &self.index_id)
            .await
            .with_context(|| "Can't execute the revoke access query on the findex server")?;

        console::Stdout::new(&response.success).write()?;

        Ok(response.success)
    }
}
