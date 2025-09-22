#[allow(
    clippy::as_conversions,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::assertions_on_result_states,
    clippy::expect_used,
    clippy::unwrap_in_result,
    clippy::get_unwrap,
    clippy::panic,
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    clippy::items_after_test_module,
    clippy::uninlined_format_args,
    clippy::use_self,
    clippy::unreachable
)] // The below module is only compiled for tests, and those lints are not useful in tests
#[cfg(test)]
pub(crate) mod tests_mod {
    // use async_trait::async_trait;
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };

    use cosmian_crypto_core::{
        CsRng,
        reexport::rand_core::{RngCore, SeedableRng},
    };
    use cosmian_findex_structs::{Permission, Permissions};
    use futures::future;
    use tokio;
    use tracing::trace;
    use uuid::Uuid;

    use crate::database::{database_traits::PermissionsTrait, findex_database::DatabaseResult};

    /// Test if creating an index ID also creates the correct Admin permission
    #[cfg(test)]
    pub(crate) async fn create_index_id<T: PermissionsTrait>(db: T) -> DatabaseResult<()> {
        let user_id = Uuid::new_v4().to_string();

        // Create new index
        let index_id = db.create_index_id(&user_id).await?;

        // Verify permissions were created
        let permissions = db.get_permissions(&user_id).await?;

        assert!(permissions.get_permission(&index_id).is_some());
        assert_eq!(
            permissions.get_permission(&index_id).unwrap(),
            &Permission::Admin
        );

        Ok(())
    }

    /// Test setting and revoking permissions for a user
    #[cfg(test)]
    pub(crate) async fn set_and_revoke_permissions<T: PermissionsTrait>(
        db: T,
    ) -> DatabaseResult<()> {
        let user_id = "test_user_1";
        let index_id = Uuid::new_v4();

        // Set Read permission
        db.set_permission(user_id, Permission::Read, &index_id)
            .await?;

        // Verify permission was set
        let permission = db.get_permission(user_id, &index_id).await?;
        assert_eq!(permission, Permission::Read);

        // Set Read, then update to Admin
        db.set_permission(user_id, Permission::Admin, &index_id)
            .await?;

        let permission = db.get_permission(user_id, &index_id).await?;
        assert_eq!(permission, Permission::Admin);

        // Now, we create a new user and give him Read permission on the same index
        let different_user_id = "test_user_2";
        db.set_permission(different_user_id, Permission::Read, &index_id)
            .await?;

        // Verify that the first user still has Admin permission
        let permission = db.get_permission(user_id, &index_id).await?;
        assert_eq!(permission, Permission::Admin);

        // Revoke permission
        db.revoke_permission(user_id, &index_id).await?;

        // Verify permission was revoked - should return an error
        let result = db.get_permission(user_id, &index_id).await;
        assert!(result.is_err());

        Ok(())
    }

    /// Test revoking permissions for multiple permission types
    #[cfg(test)]
    pub(crate) async fn revoke_permission<T: PermissionsTrait>(db: T) -> DatabaseResult<()> {
        let (other_user_id, test_user_id) =
            (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());

        // Create new index by another user
        let (admin_index_id, write_index_id, read_index_id) = (
            db.create_index_id(&other_user_id).await?,
            db.create_index_id(&other_user_id).await?,
            db.create_index_id(&other_user_id).await?,
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

        Ok(())
    }

    /// Test behavior with non-existent users and permissions
    #[cfg(test)]
    pub(crate) async fn nonexistent_user_and_permission<T: PermissionsTrait>(
        db: T,
    ) -> DatabaseResult<()> {
        let new_random_user = Uuid::new_v4().to_string();
        let index_id = Uuid::new_v4();

        // Try to get permissions for nonexistent user
        let result = db.get_permissions(&new_random_user).await?;
        assert!(result.permissions.is_empty());

        // Try to get specific permission
        let result = db.get_permission(&new_random_user, &index_id).await;
        assert!(result.is_err());

        // Revoke a non existent permission, should not fail
        db.revoke_permission(&new_random_user, &Uuid::new_v4())
            .await?;

        Ok(())
    }

    /// Test concurrent index creation for the same user
    #[cfg(test)]
    pub(crate) async fn concurrent_create_index_id<T: PermissionsTrait + Send + Sync + 'static>(
        db: T,
    ) -> DatabaseResult<()> {
        let db = Arc::new(db);
        let user_id = Uuid::new_v4().to_string();
        let tasks_count = 99;

        // Create multiple concurrent tasks to create index IDs
        let tasks: Vec<_> = (0..tasks_count)
            .map(|_| {
                let dba = Arc::clone(&db);
                let user_id = user_id.clone();
                tokio::spawn(async move { dba.create_index_id(&user_id).await })
            })
            .collect();

        // Wait for all tasks to complete
        let results: Vec<_> = future::join_all(tasks)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to join tasks");

        // Verify that all tasks succeeded
        assert_eq!(results.len(), tasks_count, "Not all tasks completed");

        // Verify that the IDs were actually stored in the db
        let current_permissions = db.get_permissions(&user_id).await?.permissions;

        // Collect the unique IDs and permissions in Hashes
        // Verify that the number of unique IDs is equal to the number of tasks
        let unique_ids: HashSet<_> = current_permissions.keys().collect();
        assert_eq!(unique_ids.len(), tasks_count, "Not all IDs were stored");

        // Verify that all permissions are Admin
        for perm in current_permissions.values() {
            assert_eq!(perm, &Permission::Admin, "Unexpected permission found");
        }

        Ok(())
    }

    /// Test concurrent permission operations across multiple users
    #[cfg(test)]
    pub(crate) async fn concurrent_set_revoke_permissions<
        T: PermissionsTrait + Send + Sync + 'static,
    >(
        db: T,
    ) -> DatabaseResult<()> {
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
                    updated_state.insert(*index_id, *permission);
                }
                Operation::RevokePermission { index_id } => {
                    // revoke permission
                    updated_state.remove(index_id);
                }
            }
            updated_state
        }

        /// Generate a random permission operation for testing
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

        const MAX_USERS: usize = 20; // Reduced from 100
        const MAX_OPS: usize = 20; // Reduced from 100
        let mut rng = CsRng::from_entropy();

        let user_count = (rng.next_u64() % MAX_USERS as u64) + 1; // At least 1 user
        let users: Vec<String> = (0..user_count)
            .map(|_| Uuid::new_v4().to_string())
            .collect();

        let mut operations: HashMap<String, Vec<Operation>> = HashMap::new();
        let mut expected_state: HashMap<String, HashMap<Uuid, Permission>> = HashMap::new();

        // Initialize empty vectors for each user
        for user in &users {
            operations.insert(user.to_owned(), Vec::new());
            expected_state.insert(user.to_owned(), HashMap::new());
        }

        for user in &users {
            // init the first op to be always create index
            let op0 = Operation::CreateIndex {
                index_id: Uuid::new_v4(),
            };
            expected_state.insert(
                user.to_owned(),
                update_expected_results(HashMap::new(), &op0),
            );
            operations
                .get_mut(user)
                .expect("User should exist at this point")
                .push(op0);

            for _ in 1..MAX_OPS {
                let mut op = generate_random_operation(&mut rng);

                // Get the previous state
                let previous_state = expected_state
                    .get(user)
                    .expect("User should exist at this point")
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
                        let chosen_index =
                            usize::try_from(rng.next_u64() % available_indexes.len() as u64)
                                .unwrap_or(0);
                        match &mut op {
                            Operation::SetPermission { index_id, .. }
                            | Operation::RevokePermission { index_id } => {
                                *index_id = *available_indexes[chosen_index];
                            }
                            Operation::CreateIndex { .. } => unreachable!("Invalid operation type"),
                        }
                    }
                }

                expected_state.insert(
                    user.to_owned(),
                    update_expected_results(previous_state, &op),
                );
                // Append the operation to the user's vector to reproduce in the real concurrent scenario
                operations
                    .get_mut(user)
                    .expect("User should exist at this point")
                    .push(op);
            }
        }

        let mut handles = vec![];
        let db_arc = Arc::new(db);
        let operations = Arc::new(operations);

        for user in users.clone() {
            let operations = Arc::clone(&operations);
            let db = Arc::clone(&db_arc);

            handles.push(tokio::spawn(async move {
                let ops = operations.get(&user).unwrap().clone();

                for op in ops {
                    match op {
                        Operation::CreateIndex { index_id } => {
                            // Simulate new index creation
                            db.set_permission(&user, Permission::Admin, &index_id)
                                .await
                                .unwrap();
                        }
                        Operation::SetPermission {
                            permission,
                            index_id,
                        } => {
                            db.set_permission(&user, permission, &index_id)
                                .await
                                .unwrap();
                        }
                        Operation::RevokePermission { index_id } => {
                            // Revoke permission
                            db.revoke_permission(&user, &index_id).await.unwrap();
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
            let current_permissions = db_arc.get_permissions(user).await?;

            let expected_permissions = Permissions {
                permissions: expected_state[user].clone(),
            };

            trace!(
                "User: {}, Current: {:?}, Expected: {:?}",
                user, current_permissions, expected_permissions
            );

            assert_eq!(
                current_permissions, expected_permissions,
                "Final permissions mismatch for user {user}"
            );
        }

        Ok(())
    }

    #[macro_export]
    macro_rules! generate_permission_tests {
    (
        $setup:expr; // Capture the single mandatory setup DBS expression
        $($name:ident),+ // Capture one or more test names separated by commas
        $(,)? // Allow optional trailing comma for the names to avoid syntax errors in case of new formatting rules
    ) => {
        $(
            mod $name {
                use super::*;
                #[tokio::test]
                async fn permissions_test() {
                    debug!("RUNNING TEST: {}", stringify!($name));
                    let db = $setup;
                    $name(db)
                        .await
                        .unwrap_or_else(|e| panic!("Test {} failed: {:?}", stringify!($name), e));
                }
        })+
        };
    }
}

#[cfg(test)]
pub(crate) use tests_mod::*;
