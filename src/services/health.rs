// use crate::findex_backend::SqliteFindexBackend;
use log::info;

pub fn health() -> bool {
    info!("Health check !");
    true
}
