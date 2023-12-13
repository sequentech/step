// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::KeycloakAdminClient;
use crate::types::keycloak::*;
use anyhow::{anyhow, Result};
use keycloak::types::{CredentialRepresentation, UserRepresentation};
use serde_json::Value;
use std::collections::HashMap;
use std::convert::From;
use tracing::instrument;
use uuid::Uuid;

impl User {
    pub fn get_mobile_phone(&self) -> Option<String> {
        match self.attributes {
            Some(ref attributes) => {
                let mobile_phone = attributes.get(MOBILE_PHONE_ATTR_NAME)?.clone();
                serde_json::from_value(mobile_phone).ok()?
            },
            None => None,
        }
    }
}

impl From<UserRepresentation> for User {
    fn from(item: UserRepresentation) -> Self {
        User {
            id: item.id.clone(),
            attributes: item.attributes.clone(),
            email: item.email.clone(),
            email_verified: item.email_verified.clone(),
            enabled: item.enabled.clone(),
            first_name: item.first_name.clone(),
            last_name: item.last_name.clone(),
            username: item.username.clone(),
            area: None,
        }
    }
}

impl From<User> for UserRepresentation {
    fn from(item: User) -> Self {
        UserRepresentation {
            access: None,
            attributes: item.attributes.clone(),
            client_consents: None,
            client_roles: None,
            created_timestamp: None,
            credentials: None,
            disableable_credential_types: None,
            email: item.email.clone(),
            email_verified: item.email_verified.clone(),
            enabled: item.enabled.clone(),
            federated_identities: None,
            federation_link: None,
            first_name: item.first_name.clone(),
            groups: None,
            id: item.id.clone(),
            last_name: item.last_name.clone(),
            not_before: None,
            origin: None,
            realm_roles: None,
            required_actions: None,
            self_: None,
            service_account_client_id: None,
            username: item.username.clone(),
        }
    }
}

impl KeycloakAdminClient {
    #[instrument(skip(self))]
    pub async fn list_users(
        self,
        tenant_id: &str,
        election_event_id: &str,
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
            .clone()
            .into_iter()
            .map(|user| user.into())
            .collect();
        Ok((users, count))
    }

    #[instrument(skip(self))]
    pub async fn get_user(self, realm: &str, user_id: &str) -> Result<User> {
        let current_user: UserRepresentation = self
            .client
            .realm_users_with_id_get(realm, user_id)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(current_user.into())
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
        username: Option<String>,
        password: Option<String>,
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

        current_user.username = match username {
            Some(val) => Some(val),
            None => current_user.username,
        };

        current_user.credentials = match password {
            Some(val) => Some(
                [
                    // the new credential
                    vec![CredentialRepresentation {
                        type_: Some("password".to_string()),
                        temporary: Some(true),
                        value: Some(val),
                        ..Default::default()
                    }],
                    // the filtered list, without password
                    current_user
                        .credentials
                        .unwrap_or(vec![])
                        .clone()
                        .into_iter()
                        .filter(|credential| {
                            credential.type_ != Some("password".to_string())
                        })
                        .collect(),
                ]
                .concat(),
            ),
            None => current_user.credentials,
        };

        self.client
            .realm_users_with_id_put(realm, user_id, current_user.clone())
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        Ok(current_user.into())
    }

    #[instrument(skip(self))]
    pub async fn delete_user(self, realm: &str, user_id: &str) -> Result<()> {
        self.client
            .realm_users_with_id_delete(realm, user_id)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn create_user(self, realm: &str, user: &User) -> Result<User> {
        let mut new_user = user.clone();
        let new_user_id =
            new_user.id.clone().unwrap_or(Uuid::new_v4().to_string());
        new_user.id = Some(new_user_id.clone());
        self.client
            .realm_users_post(realm, new_user.clone().into())
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(new_user.clone())
    }
}
