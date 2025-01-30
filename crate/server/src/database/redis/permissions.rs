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
        // TODO: https://github.com/Cosmian/findex-server/issues/34
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
mod tests {
    use std::{collections::HashMap, sync::Arc};

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

        println!("index_id: {:?}", index_id);

        // Verify permissions were created
        let permissions = db
            .get_permissions(user_id)
            .await
            .expect("Failed to get permissions");

        println!("permissions: {:?}", permissions);

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
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revoque_permission() {
        let db = setup_test_db().await;
        let user_id_boss = "user_boss";
        let user_id_subbordinate = "user_subbordinate";

        // Create new index by BOSS
        let (admin_index_id, write_index_id, read_index_id) = (
            db.create_index_id(&user_id_boss)
                .await
                .expect("Failed to create index"),
            db.create_index_id(&user_id_boss)
                .await
                .expect("Failed to create index"),
            db.create_index_id(&user_id_boss)
                .await
                .expect("Failed to create index"),
        );
        let permission_kinds = [Permission::Admin, Permission::Write, Permission::Read];
        for (index_id, permission_kind) in vec![admin_index_id, write_index_id, read_index_id]
            .into_iter()
            .zip(permission_kinds.into_iter())
        {
            // Grant permission
            db.grant_permission(user_id_subbordinate, permission_kind.clone(), &index_id)
                .await
                .expect("Failed to grant permission {permission_kind}");

            // Verify permission was granted
            let permission = db
                .get_permission(user_id_subbordinate, &index_id)
                .await
                .expect("Failed to get permission {permission_kind}");
            assert_eq!(permission, permission_kind);

            // Revoke permission
            db.revoke_permission(user_id_subbordinate, &index_id)
                .await
                .expect("Failed to revoke permission {permission_kind}");

            // Verify permission was revoked
            let result = db.get_permission(user_id_subbordinate, &index_id).await;
            assert!(result.is_err());
        }

        // Now, we create two indexes for the subbordinate, we revoke the permission for one of them and we check that the other one is still there
        let (index_id1, index_id2) = (
            db.create_index_id(&user_id_subbordinate)
                .await
                .expect("Failed to create index"),
            db.create_index_id(&user_id_subbordinate)
                .await
                .expect("Failed to create index"),
        );

        // revoke permission for index_id1
        db.revoke_permission(user_id_subbordinate, &index_id1)
            .await
            .expect("Failed to revoke permission");

        // Verify permission of index_id2 is still there
        let permission = db
            .get_permission(user_id_subbordinate, &index_id2)
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
        assert!(result.is_err());

        // Try to get specific permission
        let result = db.get_permission(user_id, &index_id).await;
        assert!(result.is_err());

        // Revoke a non existent permission, should not fail
        db.revoke_permission("someone", &Uuid::new_v4())
            .await
            .unwrap();
    }
    fn update_expected_results(
        previous_state: HashMap<Uuid, Permission>,
        operation: usize,
        permission: u8,
        index: Uuid,
    ) -> HashMap<Uuid, Permission> {
        let mut updated_state = previous_state.clone();
        match operation {
            0 => {
                // create new index
                updated_state.insert(index, Permission::Admin);
            }
            1 => {
                // grant new permission
                // won't panic because permission is either 0, 1 or 2
                updated_state.insert(index, Permission::try_from(permission).unwrap());
            }
            2 => {
                // revoke permission
                updated_state.remove(&index);
            }
            _ => panic!("Invalid operation"),
        }
        updated_state
    }

    use rand::{thread_rng, Rng};
    /// The testing strategy will be the following :
    ///
    /// Ininitialization:
    /// - u random users are created where u is a random number between 1 and 100 (inclusive)
    /// - (test will be documented later on)
    #[tokio::test]
    async fn test_concurrent_grand_revoque_permissions() {
        const MAX_USERS: usize = 1;
        const MAX_INDEXES: usize = 100;
        const MAX_OPS: usize = 20;
        let mut rng = thread_rng();

        let users: Vec<String> = (0..rng.gen_range(1..=MAX_USERS))
            .map(|i| format!("user_{}", i))
            .collect();

        let indexes: Vec<Uuid> = (0..=MAX_INDEXES).map(|_| Uuid::new_v4()).collect();

        let mut operations: HashMap<&str, Vec<(usize, u8, Uuid)>> = HashMap::new();
        let mut expected_state: HashMap<&str, Vec<HashMap<Uuid, Permission>>> = HashMap::new();
        // Initialize empty vectors for each user
        for user in users.iter() {
            operations.insert(user.as_str(), Vec::new());
            expected_state.insert(user.as_str(), Vec::new());
        }
        for user in users.clone() {
            for i in 0..MAX_OPS {
                let user_str = user.as_str();
                let op = (
                    rng.gen_range(0..=2), // 0: create new index, 1: grant new permission, 2: revoke permission
                    rng.gen_range(0..=2), // If the operation is to grant a new permission, the permission is either Read 0, Write 1 or Admin 2
                    indexes[rng.gen_range(0..indexes.len())], // Get UUID directly from indexes
                );

                // Append the operation to the user's vector
                operations
                    .get_mut(user_str)
                    .expect("User should exist")
                    .push(op);

                // Get the previous state
                let previous_state: HashMap<Uuid, Permission> = if i == 0 {
                    HashMap::new()
                } else {
                    expected_state.get(user_str).unwrap()[i - 1].clone()
                };

                // Update the expected state
                let updated_state = update_expected_results(previous_state, op.0, op.1, op.2);

                // Append the updated state to the user's vector
                expected_state
                    .get_mut(user_str)
                    .expect("User should exist")
                    .push(updated_state);
            }
        }

        let mut handles = vec![];
        let dba = setup_test_db().await;
        let be = Arc::new(dba);

        for user in &users {
            let db = Arc::clone(&be);
            let user = user.clone();
            let ops = operations[user.as_str()].clone();
            let expected_states = expected_state[user.as_str()].clone();
            // let db = Arc::clone(&db);

            handles.push(tokio::spawn(async move {
                for (op_idx, op) in ops.iter().enumerate() {
                    // Execute operation
                    match op.0 {
                        0 => {
                            // Create index operation
                            // simulate the create index by granting an admin permission on a new index froom the list
                            // let permission =
                            //     Permission::try_from(op.1).expect("Invalid permission value");
                            db.grant_permission(&user, Permission::Admin, &op.2)
                                .await
                                .expect("Failed to grant permission");
                            // db.create_index_id(&user)
                            //     .await
                            //     .expect("Failed to create index");
                        }
                        1 => {
                            // Grant permission
                            let permission =
                                Permission::try_from(op.1).expect("Invalid permission value");
                            db.grant_permission(&user, permission, &op.2)
                                .await
                                .expect("Failed to grant permission");
                        }
                        2 => {
                            // Revoke permission
                            db.revoke_permission(&user, &op.2)
                                .await
                                .expect("Failed to revoke permission");
                        }
                        _ => panic!("Invalid operation type"),
                    }

                    // Validate permissions after operation
                    let current_permissions = db
                        .get_permissions(&user)
                        .await
                        .expect("Failed to get permissions");

                    // Convert expected state to Permissions struct
                    let expected_permissions = Permissions {
                        permissions: expected_states[op_idx].clone(),
                    };

                    assert_eq!(
                        current_permissions, expected_permissions,
                        "Permissions mismatch for user {} after operation {}",
                        user, op_idx
                    );
                }
            }));
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap(); //.expect("Task failed");
        }

        println!("All tasks completed successfully");
    }
}
