// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::KeycloakAdminClient;
use crate::types::keycloak::*;
use anyhow::{anyhow, Result};
use keycloak::types::RoleRepresentation;
use rocket::futures::future::join_all;
use std::convert::From;
use tracing::instrument;

impl From<RoleRepresentation> for Permission {
    fn from(item: RoleRepresentation) -> Self {
        Permission {
            id: item.id.clone(),
            attributes: item.attributes.clone(),
            container_id: item.container_id.clone(),
            description: item.description.clone(),
            name: item.name.clone(),
        }
    }
}

impl From<Permission> for RoleRepresentation {
    fn from(item: Permission) -> Self {
        RoleRepresentation {
            attributes: item.attributes.clone(),
            client_role: None,
            composite: None,
            composites: None,
            container_id: item.container_id.clone(),
            description: item.description.clone(),
            id: item.id.clone(),
            name: item.name.clone(),
            scope_param_required: None,
        }
    }
}

impl KeycloakAdminClient {
    #[instrument(skip(self), err)]
    pub async fn list_permissions(
        self,
        realm: &str,
        search: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<(Vec<Permission>, usize)> {
        let role_representations: Vec<RoleRepresentation> = self
            .client
            .realm_roles_get(realm.clone(), None, None, None, search.clone())
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        let count = role_representations.len();
        let start = offset.unwrap_or(0);
        let end = match limit {
            Some(num) => usize::min(count, start + num),
            None => count,
        };
        let slized_role_representations = &role_representations[start..end];
        let permissions = slized_role_representations
            .into_iter()
            .map(|role| role.clone().into())
            .collect();
        Ok((permissions, count))
    }

    #[instrument(skip(self), err)]
    pub async fn set_role_permission(
        self,
        realm: &str,
        role_id: &str,
        permission_name: &str,
    ) -> Result<()> {
        let role_representation = self
            .client
            .realm_roles_with_role_name_get(realm, permission_name)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        self.client
            .realm_groups_with_group_id_role_mappings_realm_post(
                realm,
                role_id,
                vec![role_representation],
            )
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(())
    }

    #[instrument(skip(self), err)]
    pub async fn set_role_permissions(
        self,
        realm: &str,
        role_id: &str,
        permissions_name: &Vec<String>,
    ) -> Result<()> {
        let permission_roles: Vec<_> = permissions_name
            .into_iter()
            .map(|permission_name| {
                self.client
                    .realm_roles_with_role_name_get(realm, permission_name)
            })
            .collect();

        // Await all futures to complete
        let results = join_all(permission_roles).await;

        // Collect results into a Vec, handling any errors
        let successful_results: Vec<_> = results
            .into_iter()
            .filter_map(|result| match result {
                Ok(value) => Some(value),
                Err(e) => {
                    eprintln!("Error processing item: {:?}", e);
                    None
                }
            })
            .collect();
        self.client
            .realm_groups_with_group_id_role_mappings_realm_post(
                realm,
                role_id,
                successful_results,
            )
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(())
    }

    #[instrument(skip(self), err)]
    pub async fn delete_role_permission(
        self,
        realm: &str,
        role_id: &str,
        permission_name: &str,
    ) -> Result<()> {
        let role_representation = self
            .client
            .realm_roles_with_role_name_get(realm, permission_name)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        self.client
            .realm_groups_with_group_id_role_mappings_realm_delete(
                realm,
                role_id,
                vec![role_representation],
            )
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(())
    }

    #[instrument(skip(self), err)]
    pub async fn delete_permission(
        self,
        realm: &str,
        permission_name: &str,
    ) -> Result<()> {
        self.client
            .realm_roles_with_role_name_delete(realm, permission_name)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(())
    }

    pub async fn create_permission(
        self,
        realm: &str,
        permission: &Permission,
    ) -> Result<Permission> {
        self.client
            .realm_roles_post(realm, permission.clone().into())
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        Ok(permission.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::keycloak::test_support::FakeKeycloak;

    const REALM: &str = "tenant-test";
    const GROUP_ID: &str = "group-id";
    const RESERVED_NAMES: [&str; 5] = [
        "admin",
        "service-account",
        "datafix-account",
        "super-admin-user",
        "cli-account-admin",
    ];
    const ORDINARY_NAME: &str = "election-event-read";
    const ROLE_PATH: &str = "/admin/realms/tenant-test/roles";
    const MAPPING_PATH: &str =
        "/admin/realms/tenant-test/groups/group-id/role-mappings/realm";

    fn permission(name: &str) -> Permission {
        Permission {
            id: None,
            attributes: None,
            container_id: None,
            description: None,
            name: Some(name.to_string()),
        }
    }

    fn assigned_to_a_group(requests: &[String]) -> bool {
        requests
            .iter()
            .any(|request| request == &format!("POST {MAPPING_PATH}"))
    }

    #[rocket::async_test]
    async fn reserved_permission_names_are_not_created() {
        for name in RESERVED_NAMES
            .into_iter()
            .chain(["Service-Account", " admin "])
        {
            let keycloak = FakeKeycloak::start(&[]);
            let result = keycloak
                .client()
                .create_permission(REALM, &permission(name))
                .await;

            assert!(result.is_err(), "{name:?} was created");
            assert!(
                keycloak.requests().is_empty(),
                "{name:?} reached Keycloak"
            );
        }
    }

    #[rocket::async_test]
    async fn ordinary_permission_is_created() {
        let keycloak = FakeKeycloak::start(&[]);

        keycloak
            .client()
            .create_permission(REALM, &permission(ORDINARY_NAME))
            .await
            .unwrap();

        assert_eq!(keycloak.requests(), [format!("POST {ROLE_PATH}")]);
    }

    #[rocket::async_test]
    async fn reserved_permission_names_are_not_attached_to_a_group() {
        for name in RESERVED_NAMES {
            let keycloak = FakeKeycloak::start(&[]);
            let result = keycloak
                .client()
                .set_role_permission(REALM, GROUP_ID, name)
                .await;

            assert!(result.is_err(), "{name:?} was attached");
            assert!(
                keycloak.requests().is_empty(),
                "{name:?} reached Keycloak"
            );
        }
    }

    #[rocket::async_test]
    async fn a_reserved_role_reached_by_another_spelling_is_not_attached() {
        for requested in ["%73ervice-account", "other/../service-account"] {
            let keycloak =
                FakeKeycloak::start(&[(requested, "service-account")]);
            let result = keycloak
                .client()
                .set_role_permission(REALM, GROUP_ID, requested)
                .await;

            assert!(result.is_err(), "{requested:?} was attached");
            assert!(!assigned_to_a_group(&keycloak.requests()));
        }
    }

    #[rocket::async_test]
    async fn ordinary_permission_is_attached_to_a_group() {
        let keycloak = FakeKeycloak::start(&[]);

        keycloak
            .client()
            .set_role_permission(REALM, GROUP_ID, ORDINARY_NAME)
            .await
            .unwrap();

        assert_eq!(
            keycloak.requests(),
            [
                format!("GET {ROLE_PATH}/{ORDINARY_NAME}"),
                format!("POST {MAPPING_PATH}")
            ]
        );
    }

    #[rocket::async_test]
    async fn reserved_permission_names_are_not_attached_in_bulk() {
        for name in RESERVED_NAMES {
            let keycloak = FakeKeycloak::start(&[]);
            let names = vec![ORDINARY_NAME.to_string(), name.to_string()];
            let result = keycloak
                .client()
                .set_role_permissions(REALM, GROUP_ID, &names)
                .await;

            assert!(result.is_err(), "{name:?} was attached");
            assert!(!assigned_to_a_group(&keycloak.requests()));
        }
    }

    #[rocket::async_test]
    async fn a_reserved_role_reached_by_another_spelling_is_not_attached_in_bulk(
    ) {
        let keycloak =
            FakeKeycloak::start(&[("%73ervice-account", "service-account")]);
        let names =
            vec![ORDINARY_NAME.to_string(), "%73ervice-account".to_string()];
        let result = keycloak
            .client()
            .set_role_permissions(REALM, GROUP_ID, &names)
            .await;

        assert!(result.is_err());
        assert!(!assigned_to_a_group(&keycloak.requests()));
    }

    #[rocket::async_test]
    async fn ordinary_permissions_are_attached_in_bulk() {
        let keycloak = FakeKeycloak::start(&[]);
        let names = vec![ORDINARY_NAME.to_string(), "tally-read".to_string()];

        keycloak
            .client()
            .set_role_permissions(REALM, GROUP_ID, &names)
            .await
            .unwrap();

        assert!(assigned_to_a_group(&keycloak.requests()));
    }
}
