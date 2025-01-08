use async_trait::async_trait;
use cosmian_crypto_core::bytes_ser_de::Serializable;
use cosmian_findex_structs::{Permission, Permissions, WORD_LENGTH};
use redis::{RedisError, cmd, pipe};
use tracing::{debug, instrument, trace};
use uuid::Uuid;

use super::Redis;
use crate::{
    database::database_traits::PermissionsTrait,
    error::{result::FResult, server::FindexServerError},
};

// TODO(hatem) : change this to smth with compare and swapb

#[async_trait]
impl PermissionsTrait for Redis<WORD_LENGTH> {
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn create_index_id(&self, user_id: &str) -> FResult<Uuid> {
        let redis_key = user_id.as_bytes();

        let mut con_manager: redis::aio::ConnectionManager = self.memory.manager.clone();

        let uuid = Uuid::new_v4();

        // simulate the transaction block using async code
        loop {
            // first, WATCH the redis_key for changes
            let _: () = cmd("WATCH")
                .arg(redis_key)
                .query_async(&mut con_manager)
                .await?;
            // then, read the value of the redis_key
            let mut values: Vec<Vec<u8>> = redis::cmd("GET")
                .arg(redis_key)
                .query_async(&mut con_manager)
                .await?;
            trace!("values: {values:?}");

            // apply what we need to apply
            let permissions = if values.is_empty() {
                // if there is no value, we create a new one
                Permissions::new(uuid, Permission::Admin)
            } else {
                // Deserialize permissions.
                // We expect only one value here but the redis.get() signature is a Vec<Vec<u8>> so we need to get only the first element.
                let serialized_value = &values.pop().ok_or_else(|| {
                    RedisError::from((redis::ErrorKind::TypeError, "No permission found"))
                })?;
                let mut permissions = Permissions::deserialize(serialized_value).map_err(|_e| {
                    RedisError::from((redis::ErrorKind::TypeError, "Failed to deserialize"))
                })?;

                permissions.grant_permission(uuid, Permission::Admin);
                permissions
            };
            let permissions_bytes = permissions.serialize().map_err(|_e| {
                RedisError::from((redis::ErrorKind::TypeError, "Failed to serialize"))
            })?;

            // now, prepare the pipe that will increment
            let mut pipe = redis::pipe();
            pipe.atomic()
                .set(redis_key, permissions_bytes.as_slice())
                .ignore()
                .get(redis_key);

            // final step : execute the pipe and restart in case of WATCH failure
            let returned_permissions: Result<(Vec<Vec<u8>>,), _> = redis::cmd("GET")
                .arg(redis_key)
                .query_async(&mut con_manager)
                .await;
            match returned_permissions {
                Ok((returned_permissions_by_redis,)) => {
                    // Deserialize permissions
                    let serialized_value =
                        returned_permissions_by_redis.clone().pop().ok_or_else(|| {
                            // todo(review): can we avoid the clone here?
                            FindexServerError::Unauthorized(format!(
                                "No permission found written for user {user_id}"
                            ))
                        })?;
                    let returned_permissions = Permissions::deserialize(&serialized_value)?;

                    debug!("new permissions for user {user_id}: {returned_permissions:?}",);

                    break Ok(uuid);
                }
                Err(e) => {
                    if e.to_string().contains("WATCH") {
                        // Key was modified, retry the transaction
                        continue;
                    }
                    // Some other error occurred
                    break Err(e.into());
                }
            }
        }
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn get_permissions(&self, user_id: &str) -> FResult<Permissions> {
        let redis_key = user_id.as_bytes();

        let mut pipe = pipe();
        pipe.get(redis_key);

        let mut values: Vec<Vec<u8>> = pipe
            .atomic()
            .query_async(&mut self.memory.manager.clone())
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
            .query_async(&mut self.memory.manager.clone())
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
                    .query_async::<()>(&mut self.memory.manager.clone())
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
