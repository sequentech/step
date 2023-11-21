// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::KeycloakAdminClient;
use crate::types::keycloak::*;
use anyhow::Result;
use keycloak::types::UserRepresentation;
use std::convert::From;
use tracing::instrument;

impl From<UserRepresentation> for User {
    fn from(item: UserRepresentation) -> Self {
        User {
            id: item.id.clone(),
            attributes: item.attributes.clone(),
            email: item.email.clone(),
            email_verified: item.email_verified.clone(),
            enabled: item.enabled.clone(),
            first_name: item.first_name.clone(),
            groups: item.groups.clone(),
            last_name: item.last_name.clone(),
            username: item.username.clone(),
        }
    }
}

impl KeycloakAdminClient {
    #[instrument(skip(self))]
    pub async fn list_users(
        self,
        realm: &str,
        search: Option<String>,
        email: Option<String>,
        max: Option<i32>,
    ) -> Result<Vec<User>> {
        let users: Vec<UserRepresentation> = self
            .client
            .realm_users_get(
                realm, None, email, None, None, None, None, None, None, None,
                None, max, None, search, None,
            )
            .await?;
        Ok(users.into_iter().map(|user| user.into()).collect())
    }
}
