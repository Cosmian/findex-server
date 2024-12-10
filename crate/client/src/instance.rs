use cosmian_findex::{Findex, Secret, Value};
use cosmian_findex_server::database::redis::{decode_fn, encode_fn, WORD_LENGTH};
use rand_chacha::{rand_core::SeedableRng, ChaChaRng};
use tracing::trace;

use crate::{FindexClientError, FindexRestClient};

#[allow(clippy::future_not_send)]
pub async fn instantiate_findex(
    rest_client: FindexRestClient,
) -> Result<
    Findex<{ WORD_LENGTH }, Value, std::convert::Infallible, FindexRestClient>,
    FindexClientError,
> {
    // TODO: install crypto core
    let mut rng = ChaChaRng::from_entropy();
    let seed = Secret::random(&mut rng);
    trace!("Instantiating Findex rest client with seed: {:?}", seed); // TODO(review) : should we log the seed?

    let res = Findex::new(seed, rest_client, encode_fn::<WORD_LENGTH, _>, decode_fn);
    Ok(res)
}
