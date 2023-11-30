// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::KeycloakAdminClient;
use crate::types::keycloak::*;
use anyhow::{anyhow, Result};
use keycloak::types::UserRepresentation;
use std::convert::From;
use tracing::{event, instrument, Level};

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
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<(Vec<User>, i32)> {
        let user_representations: Vec<UserRepresentation> = self
            .client
            .realm_users_get(
                realm.clone(),
                Some(false),
                email.clone(),
                None,
                None,
                None,
                offset.clone(),
                None,
                None,
                None,
                None,
                limit.clone(),
                None,
                search.clone(),
                None,
            )
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        let count: i32 = self
            .client
            .realm_users_count_get(
                realm, email, None, None, None, None, search, None,
            )
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        let users = user_representations
            .into_iter()
            .map(|user| user.into())
            .collect();
        Ok((users, count))
    }
}
