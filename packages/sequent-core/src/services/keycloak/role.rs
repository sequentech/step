// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::KeycloakAdminClient;
use crate::types::keycloak::*;
use anyhow::{anyhow, Result};
use keycloak::types::GroupRepresentation;
use std::convert::From;
use tracing::instrument;

impl From<GroupRepresentation> for Role {
    fn from(item: GroupRepresentation) -> Self {
        Role {
            id: item.id.clone(),
            name: item.name.clone(),
            permissions: item.realm_roles.clone(),
            access: item.access.clone(),
            attributes: item.attributes.clone(),
            client_roles: item.client_roles.clone(),
        }
    }
}

impl From<Role> for GroupRepresentation {
    fn from(item: Role) -> Self {
        GroupRepresentation {
            access: item.access.clone(),
            attributes: item.attributes.clone(),
            client_roles: item.client_roles.clone(),
            id: item.id.clone(),
            name: item.name.clone(),
            path: None,
            realm_roles: item.permissions.clone(),
            sub_groups: None,
            parent_id: None,
            sub_group_count: None,
        }
    }
}

impl KeycloakAdminClient {
    #[instrument(skip(self), err)]
    pub async fn list_roles(
        self,
        realm: &str,
        search: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<(Vec<Role>, usize)> {
        let group_representations: Vec<GroupRepresentation> = self
            .client
            .realm_groups_get(
                realm.clone(),
                Some(false),
                None,
                None,
                None,
                None,
                search.clone(),
                None,
            )
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        let count = group_representations.len();
        let start = offset.unwrap_or(0);
        let end = match limit {
            Some(num) => usize::min(count, start + num),
            None => count,
        };
        let slized_group_representations = &group_representations[start..end];
        let roles = slized_group_representations
            .into_iter()
            .map(|role| role.clone().into())
            .collect();
        Ok((roles, count))
    }

    #[instrument(skip(self), err)]
    pub async fn list_user_roles(
        self,
        realm: &str,
        user_id: &str,
    ) -> Result<Vec<Role>> {
        let groups: Vec<GroupRepresentation> = self
            .client
            .realm_users_with_user_id_groups_get(
                realm,
                user_id,
                Some(false),
                None,
                None,
                None,
            )
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        let roles = groups.into_iter().map(|group| group.into()).collect();
        Ok(roles)
    }

    #[instrument(skip(self), err)]
    pub async fn set_user_role(
        self: &KeycloakAdminClient,
        realm: &str,
        user_id: &str,
        role_id: &str,
    ) -> Result<()> {
        self.client
            .realm_users_with_user_id_groups_with_group_id_put(
                realm, user_id, role_id,
            )
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(())
    }

    #[instrument(skip(self), err)]
    pub async fn delete_user_role(
        self,
        realm: &str,
        user_id: &str,
        role_id: &str,
    ) -> Result<()> {
        self.client
            .realm_users_with_user_id_groups_with_group_id_delete(
                realm, user_id, role_id,
            )
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(())
    }

    #[instrument(skip(self), err)]
    pub async fn delete_role(self, realm: &str, role_id: &str) -> Result<()> {
        self.client
            .realm_groups_with_group_id_delete(realm, role_id)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(())
    }

    #[instrument(skip(self), err)]
    pub async fn create_role(self, realm: &str, role: &Role) -> Result<Role> {
        self.client
            .realm_groups_post(realm, role.clone().into())
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(role.clone())
    }

    #[instrument(skip(self), err)]
    pub async fn get_role_by_name(
        self,
        realm: &str,
        role: &Role,
    ) -> Result<Role> {
        let (roles, count) = self.list_roles(realm, None, None, None).await?;
        let role_by_named = roles.iter().find(|r| role.name == r.name);
        let new_role = match role_by_named {
            Some(new_rolee) => new_rolee,
            None => role,
        };
        Ok(new_role.clone())
    }
}
