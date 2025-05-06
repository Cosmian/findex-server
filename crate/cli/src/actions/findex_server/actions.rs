use clap::Subcommand;
use cosmian_findex_client::{RestClient, RestClientConfig};
use cosmian_kms_cli::reexport::cosmian_kms_client::KmsClient;

use super::{
    datasets::DatasetsAction,
    encrypt_and_index::EncryptAndIndexAction,
    findex::{insert_or_delete::InsertOrDeleteAction, search::SearchAction},
    login::LoginAction,
    permissions::PermissionsAction,
    search_and_decrypt::SearchAndDecryptAction,
    version::ServerVersionAction,
};
use crate::error::result::FindexCliResult;

#[derive(Subcommand)]
pub enum FindexActions {
    /// Create new indexes
    Index(InsertOrDeleteAction),
    EncryptAndIndex(EncryptAndIndexAction),
    Search(SearchAction),
    SearchAndDecrypt(SearchAndDecryptAction),

    /// Delete indexed keywords
    Delete(InsertOrDeleteAction),

    #[command(subcommand)]
    Permissions(PermissionsAction),

    #[command(subcommand)]
    Datasets(DatasetsAction),

    Login(LoginAction),
    /// Logout from the Identity Provider.
    ///
    /// The access token will be removed from the findex configuration file.
    Logout,

    ServerVersion(ServerVersionAction),
}

impl FindexActions {
    /// Actions that can be performed on the Findex server such as:
    /// - indexing, searching with or without datasets-encryption (indexes are always encrypted),
    /// - permissions management,
    /// - datasets management,
    /// - login and logout,
    ///
    /// # Errors
    /// Returns an error if the action fails
    #[expect(clippy::print_stdout)]
    pub async fn run(
        &self,
        findex_client: RestClient,
        kms_client: KmsClient,
    ) -> FindexCliResult<RestClientConfig> {
        let mut new_config = findex_client.config.clone();

        match self {
            // actions that don't edit the configuration
            Self::Datasets(action) => {
                println!("{}", action.run(findex_client).await?);
            }
            Self::Permissions(action) => {
                println!("{}", action.run(findex_client).await?);
            }
            Self::ServerVersion(action) => {
                println!("{}", action.run(findex_client).await?);
            }
            Self::Delete(action) => {
                println!("{}", action.delete(findex_client, kms_client).await?);
            }
            Self::Index(action) => {
                println!("{}", action.insert(findex_client, kms_client).await?);
            }
            Self::Search(action) => {
                println!("{}", action.run(findex_client, kms_client).await?);
            }
            Self::EncryptAndIndex(action) => {
                println!("{}", action.run(findex_client, kms_client).await?);
            }
            Self::SearchAndDecrypt(action) => {
                let res = action.run(findex_client, &kms_client).await?;
                println!("Decrypted records: {res:?}");
            }

            // actions that edit the configuration
            Self::Login(action) => {
                let access_token = action.run(findex_client.config).await?;
                new_config.http_config.access_token = Some(access_token);
            }
            Self::Logout => {
                new_config.http_config.access_token = None;
            }
        }

        Ok(new_config)
    }
}
