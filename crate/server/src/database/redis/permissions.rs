use std::future::Future;

use async_trait::async_trait;
use cosmian_crypto_core::bytes_ser_de::Serializable;
use cosmian_findex_structs::{Permission, Permissions, WORD_LENGTH};
use redis::{
    aio::ConnectionLike, cmd, pipe, AsyncCommands, FromRedisValue, Pipeline, RedisError,
    ToRedisArgs,
};
use tracing::{debug, instrument, trace, warn};
use uuid::Uuid;

use super::Redis;
use crate::{
    database::database_traits::PermissionsTrait,
    error::{result::FResult, server::FindexServerError},
};

// TODO(hatem) : change this to smth with compare and swapb
async fn transaction_async<
    C: ConnectionLike + Clone,
    K: ToRedisArgs,
    T: FromRedisValue,
    F: Fn(C, Pipeline) -> Fut,
    Fut: Future<Output = Result<Option<T>, RedisError>>,
>(
    mut connection: C,
    keys: &[K],
    func: F,
) -> Result<T, RedisError> {
    loop {
        cmd("WATCH").arg(keys).exec_async(&mut connection).await?;

        let mut p = pipe();
        let response = func(connection.clone(), p.atomic().to_owned()).await?;

        match response {
            None => continue,
            Some(response) => {
                cmd("UNWATCH").exec_async(&mut connection).await?;

                return Ok(response);
            }
        }
    }
}

#[async_trait]
impl PermissionsTrait for Redis<WORD_LENGTH> {
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn create_index_id(&self, user_id: &str) -> FResult<Uuid> {
        let redis_key = user_id.as_bytes();

        let con_manager = self.memory.manager.clone();

        let uuid = Uuid::new_v4();

        // run the transaction block.
        let (mut returned_permissions_by_redis,): (Vec<Vec<u8>>,) =
            transaction_async(con_manager, &[redis_key], |mut con, mut pipe| async move {
                // load the old value, so we know what to increment.
                let mut values: Vec<Vec<u8>> = con.get(redis_key).await?;
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
                    .query_async(&mut con)
                    .await
            })
            .await?;

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

        let mut values: Vec<Vec<u8>> = pipe
            .atomic()
            // .pipe
            .get(redis_key)
            .query_async(&mut self.memory.manager.clone())
            .await
            .map_err(FindexServerError::from)?;

        let serialized_value = &values.pop().ok_or_else(|| {
            FindexServerError::Unauthorized(format!("No permission found for {user_id}"))
        })?;

        Permissions::deserialize(serialized_value).map_err(FindexServerError::from)
    }

    // so far the error is here
    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<Permission> {
        let permissions = self.get_permissions(user_id).await?;
        // warn!("WARNING : permissions that were digged out are : {permissions:?}",);
        let permission = permissions.get_permission(index_id).ok_or_else(|| {
            FindexServerError::Unauthorized(format!(
                "No permission for {user_id} on index {index_id}"
            ))
        })?;

        Ok(permission.clone())
    }

    // #[instrument(ret, err, skip(self), level = "trace")]
    #[allow(dependency_on_unit_never_type_fallback)]
    async fn grant_permission(
        &self,
        user_id: &str,
        arg_permission: Permission,
        index_id: &Uuid,
    ) -> FResult<()> {
        let redis_key = user_id.as_bytes().to_vec();
        warn!("WARNING redis_key: {redis_key:?}",);
        warn!("WARNING LA PERMISSION: {arg_permission:?}",);
        let _p = self.get_permissions(user_id).await;

        let permissions = match _p {
            Ok(mut permissions) /* some hashmap */ => {
                warn!("WARNING : permissions that are going to be set: {permissions:?}",);
                permissions.grant_permission(*index_id, arg_permission); // adds the "new" permission to the existing ones
                permissions
            }
            Err(_) => Permissions::new(*index_id, arg_permission),
        };
        warn!("WARNING : permissions after we added the new one: {permissions:?}",);
        // so far this is correct

        // in other terms, the error happens here
        let mut pipe = pipe();
        pipe.atomic()
            .set(redis_key.clone(), permissions.serialize()?.as_slice())
            .query_async(&mut self.memory.manager.clone())
            .await
            .map_err(FindexServerError::from)?;
        // Read back the data to ensure it's correct
        let mut pipe = redis::pipe();
        let values: Vec<Vec<u8>> = pipe
            .atomic()
            .get(redis_key)
            .query_async(&mut self.memory.manager.clone())
            .await
            .map_err(FindexServerError::from)?;

        let serialized_value = &values.clone().pop().ok_or_else(|| {
            FindexServerError::Unauthorized(format!("No permission found for {user_id}"))
        })?;

        let deserialized_permissions =
            Permissions::deserialize(serialized_value).map_err(FindexServerError::from)?;
        warn!(
            "WARNING Deserialized permissions: {:?}",
            deserialized_permissions
        );
        Ok(())
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
