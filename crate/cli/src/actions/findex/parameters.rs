use clap::Parser;

use cosmian_findex::{Secret, KEY_LENGTH as BYTE_KEY_LENGTH};
use uuid::Uuid;

use crate::{cli_error, error::result::CliResult};

#[derive(Parser, Debug)]
#[clap(verbatim_doc_comment)]
pub struct FindexParameters {
    /// The user findex seed used (to insert, search and delete).
    /// The seed is a 32 bytes hex string.
    #[clap(long, short = 's')]
    pub seed: String,
    /// The index ID
    #[clap(long, short = 'i')]
    pub index_id: Uuid,
}

impl FindexParameters {
    /// Returns the user key decoded from hex.
    /// # Errors
    /// This function will return an error if the key is not a valid hex string.
    pub fn seed(&self) -> CliResult<Secret<BYTE_KEY_LENGTH>> {
        let mut seed: [u8; BYTE_KEY_LENGTH] =
            hex::decode(self.seed.clone())?.try_into().map_err(|_err| {
                cli_error!(format!(
                    "Failed to convert hex key to {} bytes. Provided key : {}, length: {}",
                    BYTE_KEY_LENGTH,
                    self.seed,
                    self.seed.len()
                ))
            })?;
        Ok(Secret::<BYTE_KEY_LENGTH>::from_unprotected_bytes(&mut seed))
    }
}
