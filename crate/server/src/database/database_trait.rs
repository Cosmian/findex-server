use crate::result::FResult;
use async_trait::async_trait;

#[async_trait(?Send)]
pub(crate) trait Database {
    /// Insert the given Object in the database.
    ///
    /// A new UUID will be created if none is supplier.
    /// This method will fail if a `uid` is supplied
    /// and an object with the same id already exists
    #[allow(dead_code)]
    async fn create(&self) -> FResult<()>;
}
