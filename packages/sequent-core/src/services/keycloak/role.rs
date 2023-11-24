// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::KeycloakAdminClient;
use crate::types::keycloak::*;
use anyhow::{anyhow, Result};
use keycloak::types::GroupRepresentation;
use std::convert::From;
use tracing::{event, instrument, Level};

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

impl KeycloakAdminClient {
    #[instrument(skip(self))]
    pub async fn list_roles(
        self,
        realm: &str,
        search: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<(Vec<Role>, usize)> {
        let group_representations: Vec<GroupRepresentation> = self
            .client
            .realm_groups_get(realm.clone(), Some(false), None, None, None, None, search.clone())
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
}
