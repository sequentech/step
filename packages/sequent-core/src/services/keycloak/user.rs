// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::KeycloakAdminClient;
use crate::types::keycloak::*;
use anyhow::{anyhow, Result};
use keycloak::types::UserRepresentation;
use serde_json::Value;
use std::collections::HashMap;
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

    #[instrument(skip(self))]
    pub async fn edit_user(
        self,
        realm: &str,
        user_id: &str,
        enabled: Option<bool>,
        attributes: Option<HashMap<String, Value>>,
        email: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        groups: Option<Vec<String>>,
        username: Option<String>,
    ) -> Result<User> {
        let mut current_user: UserRepresentation = self
            .client
            .realm_users_with_id_get(realm, user_id)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        current_user.enabled = match enabled {
            Some(val) => Some(val),
            None => current_user.enabled,
        };

        current_user.attributes = match attributes {
            Some(val) => {
                let mut new_attributes =
                    current_user.attributes.unwrap_or(HashMap::new());
                for (key, value) in val.iter() {
                    new_attributes.insert(key.clone(), value.clone());
                }
                Some(new_attributes)
            }
            None => current_user.attributes,
        };

        current_user.email = match email {
            Some(val) => Some(val),
            None => current_user.email,
        };

        current_user.first_name = match first_name {
            Some(val) => Some(val),
            None => current_user.first_name,
        };

        current_user.last_name = match last_name {
            Some(val) => Some(val),
            None => current_user.last_name,
        };

        current_user.groups = match groups {
            Some(val) => Some(val),
            None => current_user.groups,
        };

        current_user.username = match username {
            Some(val) => Some(val),
            None => current_user.username,
        };

        self.client
            .realm_users_with_id_put(realm, user_id, current_user.clone())
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        Ok(current_user.into())
    }
}
