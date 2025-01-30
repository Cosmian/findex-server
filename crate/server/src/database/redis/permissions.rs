use super::Redis;
use crate::{
    database::database_traits::PermissionsTrait,
    error::{result::FResult, server::FindexServerError},
};
use async_trait::async_trait;
use cosmian_crypto_core::bytes_ser_de::Serializable;
use cosmian_findex_structs::{Permission, Permissions, WORD_LENGTH};
use redis::{aio::ConnectionLike, cmd, pipe, AsyncCommands, Pipeline, RedisError, ToRedisArgs};
use std::future::Future;
use tracing::{debug, instrument, trace};
use uuid::Uuid;

async fn transaction_async<
    C: ConnectionLike + Clone,
    K: ToRedisArgs,
    T: Send,
    F: Fn(C, Pipeline) -> Fut,
    Fut: Future<Output = Result<Option<T>, RedisError>> + Send,
>(
    mut cnx: C,
    guards: &[K],
    transaction: F,
) -> Result<T, RedisError> {
    loop {
        cmd("WATCH").arg(guards).exec_async(&mut cnx).await?;
        match transaction(cnx.clone(), pipe().atomic().to_owned()).await? {
            None => continue,
            Some(response) => {
                return Ok(response);
            }
        }
    }
}

#[async_trait]
impl PermissionsTrait for Redis<WORD_LENGTH> {
    /// Creates a new index ID and grant the given user ID admin privileges to
    /// it. Returns this index ID.
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn create_index_id(&self, user_id: &str) -> FResult<Uuid> {
        let user_rights = user_id.as_bytes();
        let index_id = Uuid::new_v4();

        let tx = |mut con: ConnectionManager, mut pipe: Pipeline| async move {
            let old_permissions: Vec<Vec<u8>> = con.get(user_rights).await?;

            trace!("old permissions for user {user_id}: {old_permissions:?}");

            let permissions = if let Some(permissions) = old_permissions.first() {
                // We expect only one value here but the redis.get() signature
                // is a Vec<Vec<u8>> so we need to get only the first element.
                let mut permissions = Permissions::deserialize(permissions).map_err(|e| {
                    RedisError::from((
                        redis::ErrorKind::TypeError,
                        "Failed to deserialize permissions",
                        format!("{e}"),
                    ))
                })?;
                permissions.grant_permission(index_id, Permission::Admin);
                permissions
            } else {
                Permissions::new(index_id, Permission::Admin)
            };

            let permissions_bytes = permissions.serialize().map_err(|e| {
                RedisError::from((
                    redis::ErrorKind::TypeError,
                    "Failed to serialize permissions",
                    format!("{e}"),
                ))
            })?;

            pipe.set(user_rights, permissions_bytes.as_slice())
                .query_async::<()>(&mut con)
                .await?;

            Ok(Some(permissions))
        };

        let permissions = {
            let _guard = self.lock.lock().await;
            transaction_async(self.manager.clone(), &[user_rights], tx).await?
        };

        debug!("new permissions for user {user_id}: {permissions:?}");

        Ok(index_id)
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn get_permissions(&self, user_id: &str) -> FResult<Permissions> {
        let redis_key = user_id.as_bytes();

        let mut pipe = pipe();
        let mut values: Vec<Vec<u8>> = {
            pipe.atomic()
                .get(redis_key)
                .query_async(&mut self.manager.clone())
                .await
                .map_err(FindexServerError::from)?
        };

        let serialized_value = &values.pop().ok_or_else(|| {
            FindexServerError::Unauthorized(format!("No permission found for {user_id}"))
        })?;

        if serialized_value.is_empty() {
            Ok(Permissions::default())
        } else {
            Permissions::deserialize(serialized_value).map_err(FindexServerError::from)
        }
    }

    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<Permission> {
        let _guard = self.lock.lock().await;
        let permissions = self.get_permissions(user_id).await?;
        let permission = permissions.get_permission(index_id).ok_or_else(|| {
            FindexServerError::Unauthorized(format!(
                "No permission for {user_id} on index {index_id}"
            ))
        })?;

        Ok(permission.clone())
    }

    #[allow(dependency_on_unit_never_type_fallback)]
    async fn grant_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> FResult<()> {
        let redis_key = user_id.as_bytes().to_vec();

        let _guard = self.lock.lock().await;
        let _p = self.get_permissions(user_id).await;

        let permissions = match _p {
            Ok(mut permissions) => {
                debug!("permissions that are going to be set: {permissions:?}",);
                permissions.grant_permission(*index_id, permission); // adds the "new" permission to the existing ones
                permissions
            }
            Err(_) => Permissions::new(*index_id, permission),
        };

        let mut pipe = pipe();
        pipe.atomic()
            .set(&redis_key, permissions.serialize()?.as_slice())
            .query_async(&mut self.manager.clone())
            .await
            .map_err(FindexServerError::from)?;

        let mut pipe = redis::pipe();
        let mut values: Vec<Vec<u8>> = pipe
            .atomic()
            .get(redis_key)
            .query_async(&mut self.manager.clone())
            .await
            .map_err(FindexServerError::from)?;

        let _ = &values.pop().ok_or_else(|| {
            FindexServerError::Unauthorized(format!("No permission found for {user_id}"))
        })?;

        Ok(())
    }

    #[instrument(ret, err, skip(self), level = "trace")]
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<()> {
        let key = user_id.as_bytes();
        let _guard = self.lock.lock().await;
        match self.get_permissions(user_id).await {
            Ok(mut permissions) => {
                permissions.revoke_permission(index_id);

                let mut pipe = pipe();
                pipe.set::<_, _>(key, permissions.serialize()?.as_slice());

                pipe.atomic()
                    .query_async::<()>(&mut self.manager.clone())
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
