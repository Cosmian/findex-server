use uuid::Uuid;

use crate::error::{result::FResult, server::FindexServerError};

pub(crate) fn get_index_id(index_id: &str) -> FResult<Uuid> {
    Uuid::parse_str(index_id).map_err(|e| {
        FindexServerError::Deserialization(format!("Invalid index_id: {index_id}. Error: {e}"))
    })
}
