use clap::Parser;
use cosmian_findex_client::{RestClientConfig, reexport::cosmian_http_client::LoginState};

use crate::error::{FindexCliError, result::FindexCliResult};

/// Login to the Identity Provider of the Findex server using the `OAuth2` authorization code flow.
///
/// This command will open a browser window and ask you to login to the Identity
/// Provider. Once you have logged in, the access token will be saved in the
/// cosmian-findex-cli configuration file.
///
/// The configuration file must contain an `oauth2_conf` object with the
/// following fields:
/// - `client_id`: The client ID of your application. This is provided by the
///   Identity Provider.
/// - `client_secret`: The client secret of your application. This is provided
///   by the Identity Provider.
/// - `authorize_url`: The authorization URL of the provider. For example, for Google it is `https://accounts.google.com/o/oauth2/v2/auth`.
/// - `token_url`: The token URL of the provider. For example, for Google it is `https://oauth2.googleapis.com/token`.
/// - `scopes`: The scopes to request. For example, for Google it is `["openid",
///   "email"]`.
///
/// The callback url must be authorized on the Identity Provider with value `http://localhost:17899/token`.
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct LoginAction;

impl LoginAction {
    /// Process the login action.
    ///
    /// # Errors
    /// Fails if the configuration file is missing or if the `oauth2_conf` object
    /// Fails if credentials are invalid. No access token could be retrieved.
    #[expect(clippy::print_stdout)]
    pub async fn run(&self, config: RestClientConfig) -> FindexCliResult<String> {
        let login_config = config.http_config.oauth2_conf.as_ref().ok_or_else(|| {
            FindexCliError::Default(
                "ERROR: Login command requires OAuth2 configuration\n\n\
                 The `login` command needs an Identity Provider (IdP) configuration in your config file.\n\
                 Please add an [http_config.oauth2_conf] section to your configuration file.\n\n\
                 Example configuration:\n\n\
                 [http_config.oauth2_conf]\n\
                 client_id = \"your-client-id\"\n\
                 client_secret = \"your-client-secret\"\n\
                 authorize_url = \"https://your-idp.com/authorize\"\n\
                 token_url = \"https://your-idp.com/token\"\n\
                 scopes = [\"openid\", \"email\"]\n".to_owned()
        )
        })?;

        let state = LoginState::try_from(login_config.clone())?;
        println!("Browse to: {}", state.auth_url);
        let access_token = state.finalize().await?;

        println!("\nSuccess! The access token was saved in the Findex configuration (in memory)");

        Ok(access_token)
    }
}
