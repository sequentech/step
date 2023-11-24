// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::KeycloakAdminClient;
use crate::types::keycloak::*;
use anyhow::{anyhow, Result};
use keycloak::types::RoleRepresentation;
use std::convert::From;
use tracing::{event, instrument, Level};

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

impl KeycloakAdminClient {
    #[instrument(skip(self))]
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
}
