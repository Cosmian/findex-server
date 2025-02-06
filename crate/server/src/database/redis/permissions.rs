use super::Redis;
use crate::{
    database::database_traits::PermissionsTrait,
    error::{result::FResult, server::FindexServerError},
};
use async_trait::async_trait;
use cosmian_findex_structs::{Permission, Permissions, WORD_LENGTH};
use redis::{pipe, AsyncCommands};
use tracing::{debug, instrument, trace};
use uuid::Uuid;

// async fn transaction_async<
//     C: ConnectionLike + Clone,
//     K: ToRedisArgs,
//     T: Send,
//     F: Fn(C, Pipeline) -> Fut,
//     Fut: Future<Output = Result<Option<T>, RedisError>> + Send,
// >(
//     mut cnx: C,
//     guards: &[K],
//     transaction: F,
// ) -> Result<T, RedisError> {
//     loop {
//         cmd("WATCH").arg(guards).exec_async(&mut cnx).await?;
//         match transaction(cnx.clone(), pipe().atomic().to_owned()).await? {
//             None => continue,
//             Some(response) => {
//                 return Ok(response);
//             }
//         }
//     }
// }

#[async_trait]
impl PermissionsTrait for Redis<WORD_LENGTH> {
    /// Creates a new index ID and grants admin privileges.
    /// Instead of serializing/deserializing permissions, we use Redis hashes directly.
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn create_index_id(&self, user_id: &str) -> FResult<Uuid> {
        let index_id = Uuid::new_v4();
        let user_redis_key = format!("user:permissions:{}", user_id);
        let index_key = format!("index:permissions:{}", index_id);

        // Use a single atomic transaction to set both mappings
        // TODO(hatem): use the new transaction async function on this
        let _ = pipe()
            .atomic() // encloses the Pipe in Multi/Exec
            .hset(
                &user_redis_key,
                index_id.to_string(),
                u8::from(Permission::Admin).to_string(),
            )
            .hset(&index_key, user_id, u8::from(Permission::Admin))
            .exec_async(&mut self.manager.clone()) // TODO(hatem) change to query_async if working
            .await?;

        debug!("new index with id {index_id} created for user {user_id}");
        println!("new index with id {index_id} created for user {user_id}");
        Ok(index_id)
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn get_permissions(&self, user_id: &str) -> FResult<Permissions> {
        let user_redis_key = format!("user:permissions:{}", user_id);

        let mut pipe = pipe();
        let values: Vec<Vec<String>> = {
            pipe.atomic()
                .hgetall(user_redis_key)
                .query_async(&mut self.manager.clone())
                .await
                .map_err(FindexServerError::from)?
        };

        // // TODO: format
        println!("permissions for user {user_id}: {values:?}");
        Ok(Permissions::default())
    }

    /// More efficient as it doesn't need to read ALL existing permissions first.
    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<Permission> {
        let user_key = format!("user:permissions:{}", user_id);

        let permission: Option<String> = self
            .manager
            .clone()
            .hget(&user_key, index_id.to_string())
            .await
            .map_err(FindexServerError::from)?;

        println!("\npermissions for user {user_id}: {permission:?}\n");
        // Convert the string permission to your numeric type
        let _permission = match permission.as_deref() {
            Some("2") => Permission::Admin,
            Some("1") => Permission::Write,
            Some("0") => Permission::Read,
            _ => {
                return Err(FindexServerError::Unauthorized(format!(
                    "No permission found for index {index_id}"
                )))
            }
        };

        println!("\npermissions for user {user_id}: AFTER PARSE {_permission:?}\n");

        Ok(_permission)
    }

    async fn grant_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> FResult<()> {
        let user_key = format!("user:permissions:{}", user_id);
        let index_key = format!("index:permissions:{}", index_id);
        let perm_number = u8::from(permission);

        let mut pipe = pipe();
        pipe.atomic()
            .hset(&user_key, index_id.to_string(), &perm_number)
            .hset(&index_key, user_id, &perm_number)
            .query_async::<()>(&mut self.manager.clone())
            .await
            .map_err(FindexServerError::from)?;

        trace!("Granted {perm_number} permission to {user_id} for index {index_id}");
        Ok(())
    }

    /// More efficient than before, directly removes the hash field.
    #[instrument(ret, err, skip(self), level = "trace")]
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<()> {
        let user_key = format!("user:permissions:{}", user_id);
        let index_key = format!("index:permissions:{}", index_id);

        let mut pipe = pipe();
        pipe.atomic()
            .hdel(&user_key, index_id.to_string())
            .hdel(&index_key, user_id)
            .query_async::<()>(&mut self.manager.clone())
            .await
            .map_err(FindexServerError::from)?;

        trace!("Revoked permission for {user_id} on index {index_id}");
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::get_unwrap,
    clippy::too_many_lines
)]
mod tests {

    use std::{
        collections::{HashMap, HashSet},
        env,
        sync::Arc,
    };

    use super::*;
    use crate::config::{DBConfig, DatabaseType};

    use rand::{rng, Rng};
    // use futures::future::join_all;
    use rand::{thread_rng, Rng};
    // use redis::aio::ConnectionManager;
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

    async fn setup_test_db() -> Redis<WORD_LENGTH> {
        let url = redis_db_config().database_url;
        Redis::instantiate(url.as_str(), false)
            .await
            .expect("Test failed to instantiate Redis")
    }

    // #[tokio::test]
    // async fn test_transaction() {
    //     const N_ITER: usize = 10;
    //     const N_WORKERS: usize = 10;

    //     let db = setup_test_db().await;
    //     let manager = db.manager.clone();
    //     let user_id = Uuid::new_v4();

    //     // Each concurrent worker grant this user admin rights for N_ITER new
    //     // indexes.
    //     let worker = || async move {
    //         let mut added_index_ids = Vec::new();
    //         for _ in 0..N_ITER {
    //             // The following code is mostly taken from create_index_id: it
    //             // adds the admin rights for a new index_id to the user.
    //             let index_id = Uuid::new_v4();
    //             let tx = |mut con: ConnectionManager, mut pipe: Pipeline| async move {
    //                 let permissions = if let Some(permissions) = con
    //                     .get::<_, Vec<Vec<u8>>>(user_id.as_bytes())
    //                     .await?
    //                     .first()
    //                 {
    //                     let mut permissions =
    //                         Permissions::deserialize(permissions).map_err(|e| {
    //                             RedisError::from((
    //                                 redis::ErrorKind::TypeError,
    //                                 "Failed to deserialize permissions",
    //                                 format!("{e}"),
    //                             ))
    //                         })?;
    //                     permissions.grant_permission(index_id, Permission::Admin);
    //                     permissions
    //                 } else {
    //                     Permissions::new(index_id, Permission::Admin)
    //                 };

    //                 let permissions_bytes = permissions.serialize().map_err(|e| {
    //                     RedisError::from((
    //                         redis::ErrorKind::TypeError,
    //                         "Failed to serialize permissions",
    //                         format!("{e}"),
    //                     ))
    //                 })?;

    //                 pipe.set(user_id.as_bytes(), permissions_bytes.as_slice())
    //                     .query_async::<()>(&mut con)
    //                     .await?;

    //                 Ok(Some(permissions))
    //             };

    //             transaction_async(manager.clone(), &[user_id.as_bytes()], tx)
    //                 .await
    //                 .unwrap();
    //             added_index_ids.push(index_id);
    //         }

    //         // Returns the list of all IDs granted for verification.
    //         added_index_ids
    //     };

    //     let handles = (0..N_WORKERS)
    //         .map(|_| tokio::spawn(worker.clone()()))
    //         .collect::<Vec<_>>();

    //     let added_ids = join_all(handles)
    //         .await
    //         .into_iter()
    //         .collect::<Result<Vec<_>, _>>()
    //         .unwrap();

    //     let expected_permissions = Permissions {
    //         permissions: added_ids
    //             .into_iter()
    //             .flatten()
    //             .map(|id| (id, Permission::Admin))
    //             .collect::<HashMap<_, _>>(),
    //     };

    //     let permissions = db
    //         .manager
    //         .clone()
    //         .get::<_, Vec<Vec<u8>>>(user_id.as_bytes())
    //         .await
    //         .unwrap()
    //         .first()
    //         .map(|permissions| {
    //             Permissions::deserialize(permissions)
    //                 .map_err(|e| {
    //                     RedisError::from((
    //                         redis::ErrorKind::TypeError,
    //                         "Failed to deserialize permissions",
    //                         format!("{e}"),
    //                     ))
    //                 })
    //                 .unwrap()
    //         })
    //         .unwrap_or_default();

    //     assert_eq!(permissions, expected_permissions);
    // }

    #[tokio::test]
    async fn test_permissions_create_index_id() {
        let db = setup_test_db().await;
        let user_id = Uuid::new_v4().to_string();

        // Create new index
        let index_id = db
            .create_index_id(&user_id)
            .await
            .expect("Failed to create index");

        // Verify permissions were created
        let permission = db
            .get_permission(user_id, &index_id)
            .await
            .expect("Failed to get permissions");

        // assert!(permissions.get_permission(&index_id).is_some());
        assert_eq!(permission, Permission::Admin);
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
        let (other_user_id, test_user_id) =
            (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());

        // Create new index by another user
        let (admin_index_id, write_index_id, read_index_id) = (
            db.create_index_id(&other_user_id)
                .await
                .expect("Failed to create index"),
            db.create_index_id(&other_user_id)
                .await
                .expect("Failed to create index"),
            db.create_index_id(&other_user_id)
                .await
                .expect("Failed to create index"),
        );
        for (index_id, permission_kind) in [
            (admin_index_id, Permission::Admin),
            (write_index_id, Permission::Write),
            (read_index_id, Permission::Read),
        ] {
            // Grant permission
            db.grant_permission(&test_user_id, permission_kind.clone(), &index_id)
                .await
                .expect("Failed to grant permission {permission_kind}");

            // Verify permission was granted
            let permission = db
                .get_permission(&test_user_id, &index_id)
                .await
                .expect("Failed to get permission {permission_kind}");
            assert_eq!(permission, permission_kind);

            // Revoke permission
            db.revoke_permission(&test_user_id, &index_id)
                .await
                .expect("Failed to revoke permission {permission_kind}");

            // Verify permission was revoked
            let result = db.get_permission(&test_user_id, &index_id).await;
            result.unwrap_err();
        }

        // Now, we create two indexes for the test_user, we revoke the permission for one of them and we check that the other one is still there
        let (index_id1, index_id2) = (
            db.create_index_id(&test_user_id)
                .await
                .expect("Failed to create index"),
            db.create_index_id(&test_user_id)
                .await
                .expect("Failed to create index"),
        );

        // revoke permission for index_id1
        db.revoke_permission(&test_user_id, &index_id1)
            .await
            .expect("Failed to revoke permission");

        // Verify permission of index_id2 is still there
        let permission = db
            .get_permission(&test_user_id, &index_id2)
            .await
            .expect("Failed to get permission");
        assert_eq!(permission, Permission::Admin);
    }

    #[tokio::test]
    async fn test_permissions_nonexistent_user_and_permission() {
        let db = setup_test_db().await;
        let new_random_user = Uuid::new_v4().to_string();
        let index_id = Uuid::new_v4();

        // Try to get permissions for nonexistent user
        let result = db.get_permissions(&new_random_user).await;
        assert!(result.unwrap().permissions.is_empty());

        // Try to get specific permission
        let result = db.get_permission(&new_random_user, &index_id).await;
        result.unwrap_err();

        // Revoke a non existent permission, should not fail
        db.revoke_permission(&new_random_user, &Uuid::new_v4())
            .await
            .unwrap();
    }

    fn update_expected_results(
        previous_state: HashMap<Uuid, Permission>,
        operation: &Operation,
    ) -> HashMap<Uuid, Permission> {
        let mut updated_state = previous_state;
        match operation {
            Operation::CreateIndex { index_id } => {
                // create new index
                updated_state.insert(*index_id, Permission::Admin);
            }
            Operation::GrantPermission {
                permission,
                index_id,
            } => {
                // grant new permission
                // won't panic because permission is either 0, 1 or 2
                updated_state.insert(*index_id, permission.clone());
            }
            Operation::RevokePermission { index_id } => {
                // revoke permission
                updated_state.remove(index_id);
            }
        }
        updated_state
    }

    /// In the next test, we will simulate concurrent operations
    /// An operation can be one of the following:
    #[derive(Debug, Clone, Eq, PartialEq)]
    enum Operation {
        CreateIndex {
            index_id: Uuid,
        },
        GrantPermission {
            permission: Permission,
            index_id: Uuid,
        },
        RevokePermission {
            index_id: Uuid,
        },
    }

    fn generate_random_operation(rng: &mut impl Rng) -> Operation {
        match rng.random_range(0..3) {
            0 => Operation::CreateIndex {
                index_id: Uuid::new_v4(),
            },
            1 => Operation::GrantPermission {
                permission: Permission::try_from(rng.random_range(0..=2)).unwrap(),
                index_id: Uuid::new_v4(),
            },
            2 => Operation::RevokePermission {
                index_id: Uuid::new_v4(),
            },
            _ => panic!("Invalid operation"),
        }
    }

    #[tokio::test]
    async fn test_permissions_concurrent_create_index_id() {
        let db = Arc::new(setup_test_db().await);
        let user_id = Uuid::new_v4().to_string();
        let tasks_count = 20;

        // Create multiple concurrent tasks to create index IDs
        let tasks: Vec<_> = (0..tasks_count)
            .map(|_| {
                let dba = Arc::clone(&db);
                let user_id = user_id.clone();
                tokio::spawn(async move { dba.create_index_id(&user_id).await })
            })
            .collect();

        // Wait for all tasks to complete
        let results: Vec<_> = futures::future::join_all(tasks)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to join tasks");

        // Verify that all tasks succeeded
        assert_eq!(results.len(), tasks_count, "Not all tasks completed");

        // Verify that the IDs were actually stored in the db
        let current_permissions = db.get_permissions(&user_id).await.unwrap().permissions;

        // Collect the unique IDs and permissions in Hashes
        // Verify that the number of unique IDs is equal to the number of tasks
        let unique_ids: HashSet<_> = current_permissions.keys().collect();
        assert_eq!(unique_ids.len(), tasks_count, "Not all IDs were stored");

        // Verify that all permissions are Admin
        for perm in current_permissions.values() {
            assert_eq!(perm, &Permission::Admin, "Unexpected permission found");
        }
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
        let mut rng = rng();

        let users: Vec<String> = (0..rng.random_range(1..=MAX_USERS))
            .map(|_| Uuid::new_v4().to_string())
            .collect();

        let mut operations: HashMap<&str, Vec<Operation>> = HashMap::new();
        let mut expected_state: HashMap<&str, Vec<HashMap<Uuid, Permission>>> = HashMap::new();
        // Initialize empty vectors for each user
        for user in &users {
            operations.insert(user, Vec::new());
            expected_state.insert(user, Vec::new());
        }
        for user in &users {
            // init the first op to be always create index
            let op0 = Operation::CreateIndex {
                index_id: Uuid::new_v4(),
            };
            expected_state
                .get_mut(user.as_str())
                .expect("User should exist")
                .push(update_expected_results(HashMap::new(), &op0));
            operations
                .get_mut(user.as_str())
                .expect("User should exist")
                .push(op0);

            for i in 1..MAX_OPS {
                let mut op = generate_random_operation(&mut rng);

                // Get the previous state
                let previous_state: HashMap<Uuid, Permission> =
                    expected_state.get(user.as_str()).unwrap()[i - 1].clone();

                if !matches!(op, Operation::CreateIndex { .. }) {
                    // if operation is not "create", rather use one of the created indexes to stay realistic
                    let available_indexes = previous_state.keys().collect::<Vec<&Uuid>>();
                    if available_indexes.is_empty() {
                        // If there are no available indexes, we can't grant or revoke permissions, change the operation to create a new index
                        op = Operation::CreateIndex {
                            index_id: Uuid::new_v4(),
                        };
                    } else {
                        let chosen_index = rng.random_range(0..available_indexes.len());
                        match &mut op {
                            Operation::GrantPermission { index_id, .. }
                            | Operation::RevokePermission { index_id } => {
                                *index_id = *available_indexes[chosen_index];
                            }
                            Operation::CreateIndex { .. } => panic!("Invalid operation type"),
                        }
                    }
                }

                // Append the updated state to the user's vector
                expected_state
                    .get_mut(user.as_str())
                    .expect("User should exist")
                    .push(update_expected_results(previous_state, &op));

                // Append the operation to the user's vector to reproduce in the real concurrent scenario
                operations
                    .get_mut(user.as_str())
                    .expect("User should exist")
                    .push(op);
            }
        }

        let mut handles = vec![];
        let db_arc = Arc::new(setup_test_db().await);

        for user in &users {
            let db = Arc::clone(&db_arc);
            let ops = operations.get(user.as_str()).unwrap().clone();

            for (op_idx, op) in ops.into_iter().enumerate() {
                let db = Arc::clone(&db); // This is the only clone we need
                let user = user.clone();
                let expected_states = expected_state[user.as_str()].clone();

                handles.push(tokio::spawn(async move {
                    // Execute operation
                    match op {
                        Operation::CreateIndex { index_id } => {
                            // Simulate new index creation
                            db.grant_permission(&user, Permission::Admin, &index_id)
                                .await
                                .expect("Failed to grant permission");
                        }
                        Operation::GrantPermission {
                            permission,
                            index_id,
                        } => {
                            db.grant_permission(&user, permission.clone(), &index_id)
                                .await
                                .expect("Failed to grant permission");
                        }
                        Operation::RevokePermission { index_id } => {
                            // Revoke permission
                            db.revoke_permission(&user, &index_id)
                                .await
                                .expect("Failed to revoke permission");
                        }
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
