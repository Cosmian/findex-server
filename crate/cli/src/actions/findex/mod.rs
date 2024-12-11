use clap::{
    error::{ContextKind, ContextValue, ErrorKind},
    Parser,
};

use cosmian_findex_client::{
    reexport::{Secret, HEX_KEY_LENGTH},
    FindexRestClient,
};
use tracing::debug;
use uuid::Uuid;

use crate::error::result::CliResult;

pub mod index_or_delete;
pub mod search;

const BYTE_KEY_LENGTH: usize = HEX_KEY_LENGTH / 2;

#[derive(Clone)]
struct KeyLengthValueParser;

impl clap::builder::TypedValueParser for KeyLengthValueParser {
    type Value = Secret<BYTE_KEY_LENGTH>;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let arg_name = arg.map(|a| a.get_id().to_string()).unwrap_or_default();
        let hex_bytes = hex::decode(value.to_str().unwrap()).map_err(|e| {
            let mut err = clap::Error::new(ErrorKind::ValueValidation).with_cmd(cmd);
            err.insert(
                ContextKind::InvalidValue,
                ContextValue::String(format!("{} parsing error.\n{}", arg_name, e.to_string())),
            ); // the .insert method does not return the error, so no chaining
            err
        })?;
        if hex_bytes.len() != HEX_KEY_LENGTH {
            let mut err = clap::Error::new(ErrorKind::ValueValidation).with_cmd(cmd);
            err.insert(
                ContextKind::InvalidValue,
                ContextValue::String(format!(
                    "{} parsing error. Key must be exactly {} hex chars to produce {} bytes",
                    arg_name, HEX_KEY_LENGTH, BYTE_KEY_LENGTH
                )),
            );
            return Err(err);
        }
        Ok(Secret::<BYTE_KEY_LENGTH>::from_unprotected_bytes(
            &mut hex_bytes.try_into().map_err(|_| {
                let mut err = clap::Error::new(ErrorKind::ValueValidation).with_cmd(cmd);
                err.insert(
                    ContextKind::InvalidValue,
                    ContextValue::String(format!(
                        "{} parsing error: Failed to convert bytes to Secret",
                        arg_name
                    )),
                );
                err
            })?,
        ))
    }
}
#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct FindexParameters {
    /// The user findex key used (to add, search, delete and compact).
    /// The key is a 64 bytes hex string.
    #[clap(long, short = 'k', value_parser = KeyLengthValueParser)]
    pub key: Secret<BYTE_KEY_LENGTH>,
    /// The index ID
    #[clap(long, short = 'i')]
    pub index_id: Uuid,
}

impl FindexParameters {
    /// Returns the user key decoded from hex.
    /// # Errors
    /// This function will return an error if the key is not a valid hex string.
    pub fn user_key(&self) -> CliResult<UserKey> {
        Ok(&hex::decode(self.key.clone())?)
    }
}

#[allow(clippy::future_not_send)]
/// Instantiates a Findex client.
/// # Errors
/// This function will return an error if there is an error instantiating the
/// Findex client.
pub async fn instantiate_findex(
    rest_client: &FindexRestClient,
    index_id: &Uuid,
) -> CliResult<InstantiatedFindex> {
    let config = Configuration::Rest(
        rest_client.clone().client.client,
        rest_client.clone().client.server_url,
        rest_client.clone().client.server_url,
        index_id.to_string(),
    );
    let findex = InstantiatedFindex::new(config).await?;
    debug!("Findex instantiated");
    Ok(findex)
}
