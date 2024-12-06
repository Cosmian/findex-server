use clap::Parser;
use cosmian_findex_client::FindexRestClient;
use cosmian_findex_structs::Permission;
use uuid::Uuid;

use crate::{
    actions::console,
    error::result::{CliResult, CliResultHelper},
};

/// Manage the users permissions to the indexes
#[derive(Parser, Debug)]
pub enum PermissionsAction {
    Create(CreateIndex),
    List(ListPermissions),
    Grant(GrantPermission),
    Revoke(RevokePermission),
}

impl PermissionsAction {
    /// Processes the permissions action.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem running the action.
    pub async fn run(&self, rest_client: FindexRestClient) -> CliResult<()> {
        match self {
            Self::Create(action) => action.run(rest_client).await?,
            Self::List(action) => action.run(rest_client).await?,
            Self::Grant(action) => action.run(rest_client).await?,
            Self::Revoke(action) => action.run(rest_client).await?,
        };

        Ok(())
    }
}

/// Create a new index. It results on an `admin` permission on a new index.
///
/// Users can have 1 permission on multiple indexes
#[derive(Parser, Debug)]
pub struct CreateIndex;

impl CreateIndex {
    /// Create a new Index with a default `admin` permission.
    ///
    /// Generates an unique index ID which is returned to the owner.
    /// This ID will be shared between several users that will be able to:
    ///   * index new keywords with their own datasets
    ///   * or search keywords in the index
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution on the Findex server fails.
    pub async fn run(&self, rest_client: FindexRestClient) -> CliResult<String> {
        let response = rest_client
            .create_index_id()
            .await
            .with_context(|| "Can't execute the create index id query on the findex server")?;
        // should replace the user configuration file
        console::Stdout::new(&response.success).write()?;

        Ok(response.success)
    }
}

/// List user's permission. Returns a list of indexes with their permissions.
#[derive(Parser, Debug)]
pub struct ListPermissions {
    /// The user identifier to allow
    #[clap(long, required = true)]
    pub user: String,
}

impl ListPermissions {
    /// Runs the `ListPermissions` action.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution on the Findex server fails.
    pub async fn run(&self, rest_client: FindexRestClient) -> CliResult<String> {
        let response = rest_client
            .list_permission(&self.user)
            .await
            .with_context(|| "Can't execute the list permission query on the findex server")?;

        console::Stdout::new(&format!("{response}")).write()?;

        Ok(response.to_string())
    }
}

/// Grant permission on a index.
///
/// This command can only be called by the owner of the index. It allows to
/// grant:
/// * `read` permission: the user can only read the index
/// * `write` permission: the user can read and write the index
/// * `admin` permission: the user can read, write and grant permission to the
///   index
#[derive(Parser, Debug)]
pub struct GrantPermission {
    /// The user identifier to allow
    #[clap(long, required = true)]
    pub user: String,

    /// The index ID
    #[clap(long, required = true)]
    pub index_id: Uuid,

    #[clap(long, required = true)]
    pub permission: Permission,
}

impl GrantPermission {
    /// Runs the `GrantPermission` action.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution on the Findex server fails.
    pub async fn run(&self, rest_client: FindexRestClient) -> CliResult<String> {
        let response = rest_client
            .grant_permission(&self.user, &self.permission, &self.index_id)
            .await
            .with_context(|| "Can't execute the grant permission query on the findex server")?;

        console::Stdout::new(&response.success).write()?;

        Ok(response.success)
    }
}

/// Revoke user permission.
///
/// This command can only be called by the owner of the index.
#[derive(Parser, Debug)]
pub struct RevokePermission {
    /// The user identifier to revoke
    #[clap(long, required = true)]
    pub user: String,

    /// The index id
    #[clap(long, required = true)]
    pub index_id: Uuid,
}

impl RevokePermission {
    /// Runs the `RevokePermission` action.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution on the Findex server fails.
    pub async fn run(&self, rest_client: FindexRestClient) -> CliResult<String> {
        let response = rest_client
            .revoke_permission(&self.user, &self.index_id)
            .await
            .with_context(|| "Can't execute the revoke permission query on the findex server")?;

        console::Stdout::new(&response.success).write()?;

        Ok(response.success)
    }
}
