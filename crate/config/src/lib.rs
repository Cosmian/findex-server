pub use config::{FindexClientConfig, FINDEX_CLI_CONF_ENV};
pub use error::FindexConfigError;

mod config;
mod error;

pub mod reexport {
    pub use cosmian_config_utils;
}
// TODO(review) : should we keep these ?
// keep a SSOT for the encode/decode functions to be used in the findex instance, as WORD_LENGTH depends of the serialization function
pub use cosmian_findex::dummy_decode as decode_fn;
pub use cosmian_findex::dummy_encode as encode_fn;
// Word length is function of the serialization function provided when findex is instantiated
// In the (naïve) case of dummy_encode / dummy_decode as provided in findex benches,
// WORD_LENGTH = 1 + CHUNK_LENGTH = 1 + (8 * BLOCK_LENGTH) = 129 for a BLOCK_LENGTH set to 16.
pub const WORD_LENGTH: usize = 129;
