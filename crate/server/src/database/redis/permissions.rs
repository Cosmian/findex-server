use super::Redis;
use crate::{
    database::database_traits::PermissionsTrait,
    error::{result::FResult, server::FindexServerError},
};
use async_trait::async_trait;
use cosmian_crypto_core::bytes_ser_de::Serializable;
use cosmian_findex_structs::{Permission, Permissions, WORD_LENGTH};
use redis::{
    aio::{ConnectionLike, ConnectionManager},
    cmd, pipe, AsyncCommands, Pipeline, RedisError, ToRedisArgs,
};
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::match_same_arms,
    clippy::option_if_let_else,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::get_unwrap
)]
mod tests {

    use std::{collections::HashMap, env, sync::Arc};

    use super::*;
    use crate::config::{DBConfig, DatabaseType};

    use rand::{thread_rng, Rng};
    use tokio;
    use uuid::Uuid;

    fn get_redis_url(redis_url_var_env: &str) -> String {
        env::var(redis_url_var_env).unwrap_or_else(|_| "redis://localhost:6379".to_owned())
    }

    fn redis_db_config() -> DBConfig {
        let url = get_redis_url("REDIS_URL");
        trace!("TESTS: using redis on {url}");
        DBConfig {
            database_type: DatabaseType::Redis,
            clear_database: false,
            database_url: url,
        }
    }

    fn get_db_config() -> DBConfig {
        env::var_os("FINDEX_TEST_DB").map_or_else(redis_db_config, |v| {
            match v.to_str().unwrap_or("") {
                "redis" => redis_db_config(),
                _ => redis_db_config(),
            }
        })
    }

    async fn setup_test_db() -> Redis<WORD_LENGTH> {
        let url = get_db_config().database_url;
        Redis::instantiate(url.as_str(), false)
            .await
            .expect("Test failed to instantiate Redis")
    }

    #[tokio::test]
    async fn test_permissions_create_index_id() {
        let db = setup_test_db().await;
        let binding = Uuid::new_v4().to_string(); // we need a long lived string to be used as a &str
        let user_id = binding.as_str();

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
    async fn test_permissions_grant_and_revoke_permissions() {
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
    async fn test_permissions_revoke_permission() {
        let db = setup_test_db().await;
        let (binding1, binding2) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string()); // we need a long lived string to be used as a &str
        let other_user_id = binding1.as_str();
        let test_user_id = binding2.as_str();

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
    async fn test_permissions_nonexistent_user_and_permission() {
        let db = setup_test_db().await;
        let binding = Uuid::new_v4().to_string(); // we need a long lived string to be used as a &str
        let new_random_user = binding.as_str();
        let index_id = Uuid::new_v4();

        // Try to get permissions for nonexistent user
        let result = db.get_permissions(new_random_user).await;
        assert!(result.unwrap().permissions.is_empty());

        // Try to get specific permission
        let result = db.get_permission(new_random_user, &index_id).await;
        result.unwrap_err();

        // Revoke a non existent permission, should not fail
        db.revoke_permission(new_random_user, &Uuid::new_v4())
            .await
            .unwrap();
    }

    fn update_expected_results(
        previous_state: HashMap<Uuid, Permission>,
        operation: usize,
        permission: u8,
        index: Uuid,
    ) -> HashMap<Uuid, Permission> {
        let mut updated_state = previous_state;
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

    /// The testing strategy will be the following:
    ///
    /// Initialization:
    /// - u random users are created where u is a random number between 1 and MAX_USERS (inclusive)
    /// - A fixed set of MAX_INDEXES random UUIDs is generated to represent available indexes
    ///
    /// Assumption:
    /// - Each user starts with no permissions
    /// - The function "grant_permission" is correct, for practical purposes, we will simulate its usage to ensure predictable test outcomes
    ///
    /// For each user:
    /// - MAX_OPS random operations are generated, where each operation is one of:
    ///   0: Create new index (grants Admin permission)
    ///   1: Grant new permission (Read, Write, or Admin)
    ///   2: Revoke permission
    /// - The expected permission state is tracked after each operation
    ///
    /// Concurrent Execution:
    /// - Each user's operations run concurrently in separate tasks
    /// - After each operation:
    ///   - The actual permissions are retrieved from the database
    ///   - The actual state is compared with the expected state
    ///   - Any mismatch fails the test
    #[tokio::test]
    async fn test_permissions_concurrent_grand_revoke_permissions() {
        const MAX_USERS: usize = 100;
        const MAX_OPS: usize = 100;
        let mut rng = thread_rng();

        let users: Vec<String> = (0..rng.gen_range(1..=MAX_USERS))
            .map(|_| Uuid::new_v4().to_string())
            .collect();

        let mut operations: HashMap<&str, Vec<(usize, u8, Uuid)>> = HashMap::new();
        let mut expected_state: HashMap<&str, Vec<HashMap<Uuid, Permission>>> = HashMap::new();
        // Initialize empty vectors for each user
        for user in &users {
            operations.insert(user.as_str(), Vec::new());
            expected_state.insert(user.as_str(), Vec::new());
        }
        for user in users.clone() {
            let user_str = user.as_str();
            for i in 0..MAX_OPS {
                let mut op = if i == 0 {
                    // First operation is always to create a new index
                    (0, 0, Uuid::new_v4())
                } else {
                    (
                        rng.gen_range(0..=2), // 0: create new index, 1: grant new permission, 2: revoke permission
                        rng.gen_range(0..=2), // If the operation is to grant a new permission, the permission is either Read 0, Write 1 or Admin 2
                        Uuid::new_v4(),
                    )
                };
                // Get the previous state
                let previous_state: HashMap<Uuid, Permission> = if i == 0 {
                    HashMap::new()
                } else {
                    expected_state.get(user_str).unwrap()[i - 1].clone()
                };

                if op.0 > 0 && i > 0 {
                    // if operation is not "create", rather use one of the created indexes to stay realistic
                    let available_indexes = previous_state.keys().collect::<Vec<&Uuid>>();
                    if available_indexes.is_empty() {
                        // If there are no available indexes, we can't grant or revoke permissions, change the operation to create a new index
                        op.0 = 0;
                    } else {
                        let chosen_index = rng.gen_range(0..available_indexes.len());
                        op.2 = *available_indexes[chosen_index];
                    }
                }

                // Append the operation to the user's vector
                operations
                    .get_mut(user_str)
                    .expect("User should exist")
                    .push(op);

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
            let ops = operations.get(user.as_str()).unwrap().clone();

            for (op_idx, op) in ops.into_iter().enumerate() {
                let db = Arc::clone(&db); // This is the only clone we need
                let user = user.clone();
                let expected_states = expected_state[user.as_str()].clone();

                handles.push(tokio::spawn(async move {
                    // Execute operation
                    match op.0 {
                        0 => {
                            // Simulate new index creation
                            db.grant_permission(&user, Permission::Admin, &op.2)
                                .await
                                .expect("Failed to grant permission");
                        }
                        1 => {
                            // Grant permission
                            let permission =
                                Permission::try_from(op.1).expect("Invalid permission value");
                            db.grant_permission(&user, permission.clone(), &op.2)
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
                        "Permissions mismatch for user {user} after operation {op_idx}",
                    );
                }));
            }
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
    }
}
