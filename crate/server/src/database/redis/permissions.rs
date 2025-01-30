use super::Redis;
use crate::{
    database::database_traits::PermissionsTrait,
    error::{result::FResult, server::FindexServerError},
};
use async_trait::async_trait;
use cosmian_crypto_core::bytes_ser_de::Serializable;
use cosmian_findex_structs::{Permission, Permissions, WORD_LENGTH};
use redis::{
    aio::ConnectionLike, cmd, pipe, AsyncCommands, FromRedisValue, Pipeline, RedisError,
    ToRedisArgs,
};
use std::future::Future;
use tracing::{debug, instrument, trace};
use uuid::Uuid;

async fn transaction_async<
    C: ConnectionLike + Send + Clone,
    K: ToRedisArgs + Send + Sync,
    T: FromRedisValue + Send,
    F: Fn(C, Pipeline) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Option<T>, RedisError>> + Send,
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

        let con_manager = self.manager.clone();

        let uuid = Uuid::new_v4();

        // run the transaction block.
        let (mut returned_permissions_by_redis,): (Vec<Vec<u8>>,) = {
            let _guard = self.lock.lock().await;
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
            .await?
        };

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
            .get(redis_key)
            .query_async(&mut self.manager.clone())
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
        let () = pipe
            .atomic()
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {

    use std::sync::Arc;

    use super::*;
    use crate::database::redis::Redis;

    use tokio;
    use uuid::Uuid;

    async fn setup_test_db() -> Redis<WORD_LENGTH> {
        Redis::instantiate("redis://localhost:6379", true)
            .await
            .expect("Test failed to instantiate Redis")
    }

    #[tokio::test]
    async fn test_create_index_id() {
        let db = setup_test_db().await;
        let user_id = "test_user";

        // Create new index
        let index_id = db
            .create_index_id(user_id)
            .await
            .expect("Failed to create index");

        // Verify permissions were created
        let permissions = db
            .get_permissions(user_id)
            .await
            .expect("Failed to get permissions");

        assert!(permissions.get_permission(&index_id).is_some());
        assert_eq!(
            permissions.get_permission(&index_id).unwrap(),
            &Permission::Admin
        );
    }

    #[tokio::test]
    async fn test_grant_and_revoke_permissions() {
        let db = setup_test_db().await;
        let user_id = "test_user_2";
        let index_id = Uuid::new_v4();

        // Grant Read permission
        db.grant_permission(user_id, Permission::Read, &index_id)
            .await
            .expect("Failed to grant permission");

        // Verify permission was granted
        let permission = db
            .get_permission(user_id, &index_id)
            .await
            .expect("Failed to get permission");
        assert_eq!(permission, Permission::Read);

        // Grant Read, then update to Admin
        db.grant_permission(user_id, Permission::Admin, &index_id)
            .await
            .unwrap();

        let permission = db.get_permission(user_id, &index_id).await.unwrap();
        assert_eq!(permission, Permission::Admin);

        // Revoke permission
        db.revoke_permission(user_id, &index_id)
            .await
            .expect("Failed to revoke permission");

        // Verify permission was revoked
        let result = db.get_permission(user_id, &index_id).await;
        result.unwrap_err();
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used, clippy::assertions_on_result_states)]
    async fn test_revoque_permission() {
        let db = setup_test_db().await;
        let other_user_id = "another_user";
        let test_user_id = "main_user";

        // Create new index by another user
        let (admin_index_id, write_index_id, read_index_id) = (
            db.create_index_id(other_user_id)
                .await
                .expect("Failed to create index"),
            db.create_index_id(other_user_id)
                .await
                .expect("Failed to create index"),
            db.create_index_id(other_user_id)
                .await
                .expect("Failed to create index"),
        );
        let permission_kinds = [Permission::Admin, Permission::Write, Permission::Read];
        for (index_id, permission_kind) in vec![admin_index_id, write_index_id, read_index_id]
            .into_iter()
            .zip(permission_kinds.into_iter())
        {
            // Grant permission
            db.grant_permission(test_user_id, permission_kind.clone(), &index_id)
                .await
                .expect("Failed to grant permission {permission_kind}");

            // Verify permission was granted
            let permission = db
                .get_permission(test_user_id, &index_id)
                .await
                .expect("Failed to get permission {permission_kind}");
            assert_eq!(permission, permission_kind);

            // Revoke permission
            db.revoke_permission(test_user_id, &index_id)
                .await
                .expect("Failed to revoke permission {permission_kind}");

            // Verify permission was revoked
            let result = db.get_permission(test_user_id, &index_id).await;
            result.unwrap_err();
        }

        // Now, we create two indexes for the test_user, we revoke the permission for one of them and we check that the other one is still there
        let (index_id1, index_id2) = (
            db.create_index_id(test_user_id)
                .await
                .expect("Failed to create index"),
            db.create_index_id(test_user_id)
                .await
                .expect("Failed to create index"),
        );

        // revoke permission for index_id1
        db.revoke_permission(test_user_id, &index_id1)
            .await
            .expect("Failed to revoke permission");

        // Verify permission of index_id2 is still there
        let permission = db
            .get_permission(test_user_id, &index_id2)
            .await
            .expect("Failed to get permission");
        assert_eq!(permission, Permission::Admin);
    }

    #[tokio::test]
    async fn test_nonexistent_user_and_permission() {
        let db = setup_test_db().await;
        let user_id = "nonexistent_user";
        let index_id = Uuid::new_v4();

        // Try to get permissions for nonexistent user
        let result = db.get_permissions(user_id).await;
        result.unwrap_err();

        // Try to get specific permission
        let result = db.get_permission(user_id, &index_id).await;
        result.unwrap_err();

        // Revoke a non existent permission, should not fail
        db.revoke_permission("someone", &Uuid::new_v4())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_concurrent_permission_operations() {
        /*
           Only one user
        */
        let db_init = setup_test_db().await;
        let db_arc = Arc::new(db_init);
        let user_id = "concurrent_user";
        let num_tasks = 10;
        let mut handles = vec![];

        for _ in 0..num_tasks {
            let db = Arc::clone(&db_arc);
            let user_id_clone = user_id.to_owned();
            let handle = tokio::spawn(async move {
                let index_id = db
                    .create_index_id(&user_id_clone)
                    .await
                    .expect("Failed to create index");

                db.grant_permission(&user_id_clone, Permission::Read, &index_id)
                    .await
                    .expect("Failed to grant permission");

                let permission = db
                    .get_permission(&user_id_clone, &index_id)
                    .await
                    .expect("Failed to get permission");

                assert_eq!(permission, Permission::Read);

                db.revoke_permission(&user_id_clone, &index_id)
                    .await
                    .expect("Failed to revoke permission");

                let result = db.get_permission(&user_id_clone, &index_id).await;

                result.unwrap_err();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("Task panicked");
        }
    }
}
