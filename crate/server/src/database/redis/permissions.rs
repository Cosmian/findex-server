use super::Redis;
use crate::{
    database::database_traits::PermissionsTrait,
    error::{result::FResult, server::ServerError},
};
use async_trait::async_trait;
use cosmian_findex_structs::{CUSTOM_WORD_LENGTH, Permission, Permissions};
use redis::{AsyncCommands, RedisError, aio::ConnectionManager};
use tracing::{instrument, trace};
use uuid::Uuid;

const PERMISSIONS_PREFIX: &str = "permissions";

async fn hset_redis_permission(
    manager: &ConnectionManager,
    user_id: &str,
    index_id: &Uuid,
    permission: Permission,
) -> Result<(), RedisError> {
    let user_redis_key = format!("{PERMISSIONS_PREFIX}:{user_id}");

    manager
        .clone()
        .hset::<_, _, u8, _>(&user_redis_key, index_id.to_string(), u8::from(permission))
        .await
}

#[async_trait]
impl PermissionsTrait for Redis<CUSTOM_WORD_LENGTH> {
    /// Creates a new index ID and sets admin privileges.
    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn create_index_id(&self, user_id: &str) -> FResult<Uuid> {
        let index_id = Uuid::new_v4();
        hset_redis_permission(&self.manager, user_id, &index_id, Permission::Admin).await?;
        trace!("New index with id {index_id} created for user {user_id}");
        Ok(index_id)
    }

    /// Sets a permission to a user for a specific index.
    async fn set_permission(
        &self,
        user_id: &str,
        permission: Permission,
        index_id: &Uuid,
    ) -> FResult<()> {
        hset_redis_permission(&self.manager, user_id, index_id, permission).await?;
        trace!("Set {permission:?} permission to {user_id} for index {index_id}");
        Ok(())
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn get_permissions(&self, user_id: &str) -> FResult<Permissions> {
        let user_redis_key = format!("{PERMISSIONS_PREFIX}:{user_id}");

        let permissions: Permissions = self
            .manager
            .clone()
            .hgetall::<_, Vec<(String, u8)>>(user_redis_key)
            .await
            .map_err(ServerError::from)?
            .into_iter()
            .map(|(index_str, perm)| {
                Ok((
                    Uuid::parse_str(&index_str).map_err(|e| {
                        ServerError::DatabaseError(format!("Invalid index ID. {e}"))
                    })?,
                    Permission::try_from(perm).map_err(|e| {
                        ServerError::DatabaseError(format!("Invalid permission. {e}"))
                    })?,
                ))
            })
            .collect::<FResult<_>>()?;

        trace!("permissions for user {user_id}: {permissions:?}");
        Ok(permissions)
    }

    #[instrument(ret(Display), err, skip(self), level = "trace")]
    async fn get_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<Permission> {
        let user_key = format!("{PERMISSIONS_PREFIX}:{user_id}");

        let permission = self
            .manager
            .clone()
            .hget::<_, _, Option<u8>>(&user_key, index_id.to_string())
            .await
            .map_err(ServerError::from)?
            .ok_or_else(|| {
                ServerError::DatabaseError(format!("No permission found for index {index_id}"))
            })
            .and_then(|p| {
                Permission::try_from(p)
                    .map_err(|e| ServerError::DatabaseError(format!("Invalid permission. {e}")))
            })?;

        trace!("Permissions for user {user_id}: {permission:?}");
        Ok(permission)
    }

    #[instrument(ret, err, skip(self), level = "trace")]
    async fn revoke_permission(&self, user_id: &str, index_id: &Uuid) -> FResult<()> {
        let user_key = format!("{PERMISSIONS_PREFIX}:{user_id}");

        // never type fallbacks will be deprecated in future Rust releases, hence this explicit typing
        let _: () = self
            .manager
            .clone()
            .hdel(&user_key, index_id.to_string())
            .await
            .map_err(ServerError::from)?;

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

    use super::*;
    use crate::config::{DBConfig, DatabaseType};
    use cosmian_crypto_core::{
        CsRng,
        reexport::rand_core::{RngCore, SeedableRng},
    };
    use std::{
        collections::{HashMap, HashSet},
        env,
        sync::Arc,
    };
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

    async fn setup_test_db() -> Redis<CUSTOM_WORD_LENGTH> {
        let url = redis_db_config().database_url;
        Redis::instantiate(url.as_str(), false)
            .await
            .expect("Test failed to instantiate Redis")
    }

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
        let permissions = db
            .get_permissions(&user_id)
            .await
            .expect("Failed to get permissions");

        assert!(permissions.get_permission(&index_id).is_some());
        assert_eq!(
            permissions.get_permission(&index_id).unwrap(),
            &Permission::Admin
        );
    }

    #[tokio::test]
    async fn test_permissions_set_and_revoke_permissions() {
        let db = setup_test_db().await;

        let user_id = "test_user_1";
        let index_id = Uuid::new_v4();

        // Set Read permission
        db.set_permission(user_id, Permission::Read, &index_id)
            .await
            .expect("Failed to set permission");

        // Verify permission was set
        let permission = db
            .get_permission(user_id, &index_id)
            .await
            .expect("Failed to get permission");
        assert_eq!(permission, Permission::Read);

        // Set Read, then update to Admin
        db.set_permission(user_id, Permission::Admin, &index_id)
            .await
            .unwrap();

        let permission = db.get_permission(user_id, &index_id).await.unwrap();
        assert_eq!(permission, Permission::Admin);

        // Now, we create a new use and give him Read permission on the same index
        let different_user_id = "test_user_2";
        db.set_permission(different_user_id, Permission::Read, &index_id)
            .await
            .unwrap();

        // Verify that the first user still has Admin permission
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
            // Set permission
            db.set_permission(&test_user_id, permission_kind, &index_id)
                .await
                .unwrap_or_else(|_| panic!("Failed to get permission {permission_kind}"));

            // Verify permission was set
            let permission = db
                .get_permission(&test_user_id, &index_id)
                .await
                .unwrap_or_else(|_| panic!("Failed to get permission {permission_kind}"));
            assert_eq!(permission, permission_kind);

            // Revoke permission
            db.revoke_permission(&test_user_id, &index_id)
                .await
                .unwrap_or_else(|_| panic!("Failed to get permission {permission_kind}"));

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
            Operation::SetPermission {
                permission,
                index_id,
            } => {
                // set new permission
                // won't panic because permission is either 0, 1 or 2
                updated_state.insert(*index_id, *permission);
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
    #[derive(Clone, Eq, PartialEq)]
    enum Operation {
        CreateIndex {
            index_id: Uuid,
        },
        SetPermission {
            permission: Permission,
            index_id: Uuid,
        },
        RevokePermission {
            index_id: Uuid,
        },
    }

    #[allow(clippy::as_conversions)] //  an u32 in the [0,2] range will always convert to u8
    fn generate_random_operation(rng: &mut impl RngCore) -> Operation {
        match rng.next_u32() % 3 {
            0 => Operation::CreateIndex {
                index_id: Uuid::new_v4(),
            },
            1 => Operation::SetPermission {
                permission: Permission::try_from((rng.next_u32() % 3) as u8).unwrap(),
                index_id: Uuid::new_v4(),
            },
            2 => Operation::RevokePermission {
                index_id: Uuid::new_v4(),
            },
            _ => panic!("Invalid operation"),
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
    /// - The function "set_permission" is correct, for practical purposes, we will simulate its usage to ensure predictable test outcomes
    ///
    /// For each user:
    /// - MAX_OPS random operations are generated, where each operation is one of:
    ///   0: Create new index (sets Admin permission)
    ///   1: Set new permission (Read, Write, or Admin)
    ///   2: Revoke permission
    /// - The expected permission state is tracked after each operation
    ///
    /// Concurrent Execution:
    /// - Each user's operations run concurrently in separate tasks
    /// - After each operation:
    ///   - The actual permissions are retrieved from the database
    ///   - The actual state is compared with the expected state
    ///   - Any mismatch fails the test
    #[allow(clippy::as_conversions)] // won't panic
    #[tokio::test]
    async fn test_permissions_concurrent_set_revoke_permissions() {
        const MAX_USERS: usize = 100;
        const MAX_OPS: usize = 100;
        let mut rng = CsRng::from_entropy();

        let users: Vec<String> = (0..=(rng.next_u64() % MAX_USERS as u64))
            .map(|_| Uuid::new_v4().to_string())
            .collect();

        let mut operations: HashMap<String, Vec<Operation>> = HashMap::new();
        let mut expected_state: HashMap<&str, HashMap<Uuid, Permission>> = HashMap::new();
        // Initialize empty vectors for each user
        for user in &users {
            operations.insert(user.to_owned(), Vec::new()); // a long lived value is needed here
            expected_state.insert(user, HashMap::new());
        }
        for user in &users {
            // init the first op to be always create index
            let op0 = Operation::CreateIndex {
                index_id: Uuid::new_v4(),
            };
            expected_state.insert(user.as_str(), update_expected_results(HashMap::new(), &op0));
            operations
                .get_mut(user.as_str())
                .expect("User should exist at this point ?")
                .push(op0);

            for _ in 1..MAX_OPS {
                let mut op = generate_random_operation(&mut rng);

                // Get the previous state
                let previous_state: HashMap<Uuid, Permission> = expected_state
                    .get(user.as_str())
                    .expect("User should exist at this point ?")
                    .clone();

                if !matches!(op, Operation::CreateIndex { .. }) {
                    // if operation is not "create", rather use one of the created indexes to stay realistic
                    let available_indexes = previous_state.keys().collect::<Vec<&Uuid>>();
                    if available_indexes.is_empty() {
                        // If there are no available indexes, we can't set or revoke permissions, change the operation to create a new index
                        op = Operation::CreateIndex {
                            index_id: Uuid::new_v4(),
                        };
                    } else {
                        let chosen_index = usize::try_from(rng.next_u64() % usize::MAX as u64)
                            .expect("Failed to convert index")
                            % available_indexes.len();
                        match &mut op {
                            Operation::SetPermission { index_id, .. }
                            | Operation::RevokePermission { index_id } => {
                                *index_id = *available_indexes[chosen_index];
                            }
                            Operation::CreateIndex { .. } => panic!("Invalid operation type"),
                        }
                    }
                }

                expected_state.insert(user.as_str(), update_expected_results(previous_state, &op));
                // Append the operation to the user's vector to reproduce in the real concurrent scenario
                operations
                    .get_mut(user.as_str())
                    .expect("User should exist at this point ?")
                    .push(op);
            }
        }

        let mut handles = vec![];
        let db_arc = Arc::new(setup_test_db().await);
        let operations = Arc::new(operations);

        for user in users.clone() {
            let operations = Arc::clone(&operations);
            let db = Arc::clone(&db_arc);

            handles.push(tokio::spawn(async move {
                let ops = operations.get(user.as_str()).unwrap().clone();

                for op in ops {
                    let db = Arc::clone(&db);
                    let user = user.clone();

                    match op {
                        Operation::CreateIndex { index_id } => {
                            // Simulate new index creation
                            db.set_permission(&user, Permission::Admin, &index_id)
                                .await
                                .expect("Failed to set permission");
                        }
                        Operation::SetPermission {
                            permission,
                            index_id,
                        } => {
                            db.set_permission(&user, permission, &index_id)
                                .await
                                .expect("Failed to set permission");
                        }
                        Operation::RevokePermission { index_id } => {
                            // Revoke permission
                            db.revoke_permission(&user, &index_id)
                                .await
                                .expect("Failed to revoke permission");
                        }
                    }
                }
            }));
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        for user in &users {
            let current_permissions = db_arc
                .get_permissions(user)
                .await
                .expect("Failed to get permissions");

            let expected_permissions = Permissions {
                permissions: expected_state[user.as_str()].clone(),
            };

            assert_eq!(
                current_permissions, expected_permissions,
                "Final permissions mismatch for user {user}"
            );
        }
    }
}
