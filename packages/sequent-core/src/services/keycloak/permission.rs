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
