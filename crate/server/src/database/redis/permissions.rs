use async_trait::async_trait;
use cloudproof_findex::reexport::cosmian_crypto_core::bytes_ser_de::Serializable;
use cosmian_findex_structs::{Permission, Permissions};
use redis::{pipe, transaction, Commands, RedisError};
use tracing::{debug, instrument, trace};
use uuid::Uuid;

use super::Redis;
use crate::{
    database::database_traits::PermissionsTrait,
    error::{result::FResult, server::FindexServerError},
};

#[async_trait]
impl PermissionsTrait for Redis {
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn create_index_id(&self, user_id: &str) -> FResult<Uuid> {
        let redis_key = user_id.as_bytes();

        let mut con = self.client.get_connection()?;

        let uuid = Uuid::new_v4();

        // run the transaction block.
        let (mut returned_permissions_by_redis,): (Vec<Vec<u8>>,) =
            transaction(&mut con, &[redis_key], |con, pipe| {
                // load the old value, so we know what to increment.
                let mut values: Vec<Vec<u8>> = con.get(redis_key)?;
                trace!("values: {values:?}");

                let permissions = if values.is_empty() {
                    // if there is no value, we create a new one
                    Permissions::new(uuid, Permission::Admin)
                } else {
                    // Deserialize permissions.
                    // We expect only one value here but the redis.get() signature is a Vec<Vec<u8>> so we need to get only the first element.
                    let serialized_value = &values.pop().ok_or_else(|| {
                        RedisError::from((redis::ErrorKind::TypeError, "No permission found"))
                    })?;
                    let mut permissions =
                        Permissions::deserialize(serialized_value).map_err(|_e| {
                            RedisError::from((redis::ErrorKind::TypeError, "Failed to deserialize"))
                        })?;

                    permissions.grant_permission(uuid, Permission::Admin);
                    permissions
                };
                let permissions_bytes = permissions.serialize().map_err(|_e| {
                    RedisError::from((redis::ErrorKind::TypeError, "Failed to serialize"))
                })?;

                // increment
                pipe.set(redis_key, permissions_bytes.as_slice())
                    .ignore()
                    .get(redis_key)
                    .query(con)
            })?;

        // Deserialize permissions
        let serialized_value = returned_permissions_by_redis.pop().ok_or_else(|| {
            FindexServerError::Unauthorized(format!(
                "No permission found written for user {user_id}"
            ))
        })?;
        let returned_permissions = Permissions::deserialize(&serialized_value)?;

        debug!("new permissions for user {user_id}: {returned_permissions:?}",);

        Ok(uuid)
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn get_permissions(&self, user_id: &str) -> FResult<Permissions> {
        let redis_key = user_id.as_bytes();

        let mut pipe = pipe();
        pipe.get(redis_key);

        let mut values: Vec<Vec<u8>> = pipe
            .atomic()
            .query_async(&mut self.mgr.clone())
            .await
            .map_err(FindexServerError::from)?;

        let serialized_value = &values.pop().ok_or_else(|| {
            FindexServerError::Unauthorized(format!("No permission found for {user_id}"))
        })?;

        Permissions::deserialize(serialized_value).map_err(FindexServerError::from)
    }

    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<Permission> {
        let permissions = self.get_permissions(user_id).await?;
        let permission = permissions.get_permission(index_id).ok_or_else(|| {
            FindexServerError::Unauthorized(format!(
                "No permission for {user_id} on index {index_id}"
            ))
        })?;

        Ok(permission.clone())
    }

    #[instrument(ret, err, skip(self), level = "trace")]
    async fn grant_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> FResult<()> {
        let redis_key = user_id.as_bytes().to_vec();
        let permissions = match self.get_permissions(user_id).await {
            Ok(mut permissions) => {
                permissions.grant_permission(*index_id, permission);
                permissions
            }
            Err(_) => Permissions::new(*index_id, permission),
        };

        let mut pipe = pipe();
        pipe.set::<_, _>(redis_key, permissions.serialize()?.as_slice());
        pipe.atomic()
            .query_async(&mut self.mgr.clone())
            .await
            .map_err(FindexServerError::from)
    }

    #[instrument(ret, err, skip(self), level = "trace")]
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<()> {
        let key = user_id.as_bytes();
        match self.get_permissions(user_id).await {
            Ok(mut permissions) => {
                permissions.revoke_permission(index_id);

                let mut pipe = pipe();
                pipe.set::<_, _>(key, permissions.serialize()?.as_slice());

                pipe.atomic()
                    .query_async::<()>(&mut self.mgr.clone())
                    .await
                    .map_err(FindexServerError::from)?;
            }
            Err(_) => {
                trace!("Nothing to revoke since no permission found for index {index_id}");
            }
        };

        Ok(())
    }
}
