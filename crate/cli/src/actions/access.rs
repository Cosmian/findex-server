use clap::Parser;
use cosmian_findex_client::FindexClient;
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
    // Revoke(RevokeAccess),
    // List(ListAccessesGranted),
}

impl AccessAction {
    /// Processes the access action.
    ///
    /// # Arguments
    ///
    /// * `findex_rest_client` - The Findex client used for the action.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem running the action.
    pub async fn process(&self, findex_rest_client: FindexClient) -> CliResult<()> {
        match self {
            Self::Create(action) => action.run(findex_rest_client).await?,
            Self::Grant(action) => action.run(findex_rest_client).await?,
            // Self::Revoke(action) => action.run(findex_rest_client).await?,
            // Self::List(action) => action.run(findex_rest_client).await?,
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct CreateAccess;

impl CreateAccess {
    /// Runs the `GrantAccess` action.
    ///
    /// # Arguments
    ///
    /// * `findex_rest_client` - A reference to the Findex client used to communicate with the KMS server.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution on the KMS server fails.
    ///
    pub async fn run(&self, findex_rest_client: FindexClient) -> CliResult<String> {
        let response = findex_rest_client
            .create_access()
            .await
            .with_context(|| "Can't execute the version query on the findex server")?; //todo(manu): rephrase error message

        trace!("cli: New access successfully created: {}", response.success);
        console::Stdout::new(&response.success).write()?;

        Ok(response.success)
    }
}

/// Grant access.
///
/// This command can only be called by the owner of the object.
///
/// The right is granted for one or multiple supported KMIP operations:
/// `create`, `get`, `encrypt`, `decrypt`, `import`, `revoke`, `locate`, `rekey`, `destroy`.
///
/// Multiple operations must be supplied whitespace separated, such as: 'create get rekey'
#[derive(Parser, Debug)]
pub struct GrantAccess {
    /// The user identifier to allow
    #[clap(long, required = true)]
    pub user: String,

    /// The object unique identifier stored in the KMS
    #[clap(long, required = true)]
    pub index_id: String,

    /// The role to grant (`read`, `writer`, `admin`)
    #[clap(long, required = true)]
    pub role: String,
}

impl GrantAccess {
    /// Runs the `GrantAccess` action.
    ///
    /// # Arguments
    ///
    /// * `findex_rest_client` - A reference to the Findex client used to communicate with the KMS server.
    ///
    /// # Errors
    ///
    /// Returns an error if the query execution on the KMS server fails.
    ///
    pub async fn run(&self, findex_rest_client: FindexClient) -> CliResult<String> {
        let response = findex_rest_client
            .grant_access(&self.user, &self.role, &self.index_id)
            .await
            .with_context(|| "Can't execute the version query on the findex server")?;

        console::Stdout::new(&response.success).write()?;

        Ok(response.success)
    }
}

// /// Revoke another user one or multiple access rights to an object.
// ///
// /// This command can only be called by the owner of the object.
// ///
// /// The right is revoked for one or multiple supported KMIP operations:
// /// `create`, `get`, `encrypt`, `decrypt`, `import`, `revoke`, `locate`, `rekey`, `destroy`
// ///
// /// Multiple operations must be supplied whitespace separated, such as: 'create get rekey'
// #[derive(Parser, Debug)]
// pub struct RevokeAccess {
//     /// The user to revoke access to
//     #[clap(required = true)]
//     user: String,

//     /// The object unique identifier stored in the KMS
//     #[clap(required = true)]
//     index_id: String,
// }

// impl RevokeAccess {
//     /// Runs the `RevokeAccess` action.
//     ///
//     /// # Arguments
//     ///
//     /// * `findex_rest_client` - A reference to the Findex client used to communicate with the KMS server.
//     ///
//     /// # Errors
//     ///
//     /// Returns an error if the query execution on the KMS server fails.
//     ///
//     pub async fn run(&self, findex_rest_client: &FindexClient) -> CliResult<()> {
//         let response = findex_rest_client
//             .revoke_access(&self.user, &self.index_id)
//             .await
//             .with_context(|| "Can't execute the version query on the findex server")?;

//         console::Stdout::new(&response).write()?;

//         Ok(())
//     }
// }

// /// List the access rights granted on an object to other users.
// ///
// /// This command can only be called by the owner of the object.
// /// Returns a list of users and the operations they have been granted access to.
// #[derive(Parser, Debug)]
// pub struct ListAccessesGranted {
//     /// The object unique identifier
//     #[clap(required = true)]
//     object_uid: String,
// }

// impl ListAccessesGranted {
//     /// Runs the `ListAccessesGranted` action.
//     ///
//     /// # Arguments
//     ///
//     /// * `findex_rest_client` - A reference to the Findex client used to communicate with the KMS server.
//     ///
//     /// # Errors
//     ///
//     /// Returns an error if the query execution on the KMS server fails.
//     ///
//     pub async fn run(&self, findex_rest_client: &FindexClient) -> CliResult<()> {
//         let accesses = findex_rest_client
//             .list_access(&self.object_uid)
//             .await
//             .with_context(|| "Can't execute the query on the kms server")?;

//         let stdout = format!(
//             "The access rights granted on object {} are:",
//             &self.object_uid
//         );
//         let mut stdout = console::Stdout::new(&stdout);
//         stdout.set_accesses(accesses);
//         stdout.write()?;

//         Ok(())
//     }
// }
