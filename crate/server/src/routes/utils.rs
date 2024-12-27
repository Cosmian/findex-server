use uuid::Uuid;

use crate::error::result::FResult;

pub(crate) fn get_index_id(index_id: &str) -> FResult<Uuid> {
    let res = Uuid::parse_str(index_id);
    Ok(res?)
}
