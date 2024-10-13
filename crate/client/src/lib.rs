#![allow(clippy::upper_case_acronyms)]
//required to detect generic type in Serializer
#![feature(min_specialization)]

pub use config::{ClientConf, GmailApiConf, FINDEX_CLI_CONF_ENV};
pub use error::ClientError;
pub use file_utils::{
    read_bytes_from_file, read_bytes_from_files_to_bulk, read_from_json_file,
    write_bulk_decrypted_data, write_bulk_encrypted_data, write_bytes_to_file,
    write_json_object_to_file, write_single_decrypted_data, write_single_encrypted_data,
};
pub use findex_rest_client::FindexClient;
pub use result::{ClientResultHelper, RestClientResult};

mod certificate_verifier;
mod config;
mod error;
mod file_utils;
mod findex_rest_client;
mod result;
