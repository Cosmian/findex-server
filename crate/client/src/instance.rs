use cosmian_findex::{Findex, Secret, Value};
use cosmian_findex_server::database::redis::{decode_fn, encode_fn, WORD_LENGTH};
use rand_chacha::{rand_core::SeedableRng, ChaChaRng};
use tracing::trace;

use crate::{FindexClientError, FindexRestClient};
use uuid::Uuid;

#[allow(clippy::future_not_send)]
pub async fn instantiate_findex(
    rest_client: FindexRestClient,
    index_id: &Uuid,
) -> Result<
    Findex<{ WORD_LENGTH }, Value, std::convert::Infallible, FindexRestClient>, // TODO: is this the correct error type ?
    FindexClientError,
> {
    trace!(
        "This function will return an error if there is an error instantiating the Findex client."
    );
    // let config = Configuration::Rest(rest_client.client.client, "dummy_value_1".to_owned());
    // crypto CSRNG
    let mut rng = ChaChaRng::from_entropy();
    let seed = Secret::random(&mut rng);

    let res = Findex::new(seed, rest_client, encode_fn::<WORD_LENGTH, _>, decode_fn);
    Ok(res)
}
