// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::KeycloakAdminClient;
use crate::types::keycloak::*;
use crate::util::convert_vec::convert_map;
use anyhow::{anyhow, Result};
use keycloak::{
    types::{
        CredentialRepresentation, GroupRepresentation, UPAttribute, UPConfig,
        UserRepresentation,
    },
    KeycloakError,
};
use serde_json::Value;
use std::collections::HashMap;
use std::convert::From;
use tokio_postgres::row::Row;
use tracing::{info, instrument};

use super::PubKeycloakAdmin;

pub const MULTIVALUE_USER_ATTRIBUTE_SEPARATOR: &str = "|";
#[derive(Debug)]
pub struct GroupInfo {
    pub group_id: String,
    pub group_name: String,
}

async fn error_check(
    response: reqwest::Response,
) -> Result<reqwest::Response, KeycloakError> {
    if !response.status().is_success() {
        let status = response.status().into();
        let text = response.text().await?;
        return Err(KeycloakError::HttpFailure {
            status,
            body: serde_json::from_str(&text).ok(),
            text,
        });
    }

    Ok(response)
}

impl User {
    pub fn get_mobile_phone(&self) -> Option<String> {
        Some(
            self.attributes
                .as_ref()?
                .get(MOBILE_PHONE_ATTR_NAME)?
                .get(0)?
                .to_string(),
        )
    }

    pub fn get_attribute_val(&self, attribute_name: &String) -> Option<String> {
        Some(
            self.attributes
                .as_ref()?
                .get(attribute_name)?
                .get(0)?
                .to_string(),
        )
    }

    pub fn get_attribute_multival(
        &self,
        attribute_name: &String,
    ) -> Option<String> {
        Some(
            self.attributes
                .as_ref()?
                .get(attribute_name)?
                .join(MULTIVALUE_USER_ATTRIBUTE_SEPARATOR)
                .to_string(),
        )
    }

    pub fn get_authorized_election_ids(&self) -> Option<Vec<String>> {
        let result = self
            .attributes
            .as_ref()?
            .get(AUTHORIZED_ELECTION_IDS_NAME)
            .cloned();

        info!("get_authorized_election_ids: {:?}", result);
        info!("attributes: {:?}", self.attributes);

        result
    }

    pub fn get_area_id(&self) -> Option<String> {
        Some(
            self.attributes
                .as_ref()?
                .get(AREA_ID_ATTR_NAME)?
                .get(0)?
                .to_string(),
        )
    }

    pub fn get_votes_info_by_election_id(
        &self,
    ) -> Option<HashMap<String, VotesInfo>> {
        self.votes_info.as_ref().and_then(|votes_info_vec| {
            Some(
                votes_info_vec
                    .iter()
                    .map(|votes_info| {
                        (votes_info.election_id.clone(), votes_info.clone())
                    })
                    .collect::<HashMap<String, VotesInfo>>(),
            )
        })
    }
}

impl TryFrom<Row> for User {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        let attributes_value: Value = item.try_get("attributes")?;
        let attributes_map: HashMap<String, Value> =
            serde_json::from_value(attributes_value)?;
        Ok(User {
            id: item.try_get("id")?,
            attributes: Some(convert_map(attributes_map)),
            email: item.try_get("email")?,
            email_verified: item.try_get("email_verified")?,
            enabled: item.try_get("enabled")?,
            first_name: item.try_get("first_name")?,
            last_name: item.try_get("last_name")?,
            username: item.try_get("username")?,
            area: None,
            votes_info: None,
        })
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
            votes_info: None,
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
            application_roles: None,
            social_links: None,
            totp: None,
            user_profile_metadata: None,
        }
    }
}

impl KeycloakAdminClient {
    #[instrument(skip(self), err)]
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
                realm, email, None, None, None, None, search, None, None,
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

    #[instrument(skip(self), err)]
    pub async fn get_user(&self, realm: &str, user_id: &str) -> Result<User> {
        let current_user: UserRepresentation = self
            .client
            .realm_users_with_user_id_get(realm, user_id, None)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(current_user.into())
    }

    #[instrument(skip(self, password), err)]
    pub async fn edit_user(
        self,
        realm: &str,
        user_id: &str,
        enabled: Option<bool>,
        attributes: Option<HashMap<String, Vec<String>>>,
        email: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        username: Option<String>,
        password: Option<String>,
        temporary: Option<bool>,
    ) -> Result<User> {
        let credentials = match password {
            Some(val) => Some(
                [
                    // the new credential
                    vec![CredentialRepresentation {
                        type_: Some("password".to_string()),
                        temporary: match temporary {
                            Some(temportay) => Some(temportay),
                            _ => Some(true),
                        },
                        value: Some(val),
                        ..Default::default()
                    }],
                ]
                .concat(),
            ),
            None => None,
        };

        self.edit_user_with_credentials(
            realm,
            user_id,
            enabled,
            attributes,
            email,
            first_name,
            last_name,
            username,
            credentials,
            temporary,
        )
        .await
    }

    #[instrument(skip(self, credentials), err)]
    pub async fn edit_user_with_credentials(
        self,
        realm: &str,
        user_id: &str,
        enabled: Option<bool>,
        attributes: Option<HashMap<String, Vec<String>>>,
        email: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        username: Option<String>,
        credentials: Option<Vec<CredentialRepresentation>>,
        temporary: Option<bool>,
    ) -> Result<User> {
        info!("Editing user in keycloak ?: {:?}", attributes);
        let mut current_user: UserRepresentation = self
            .client
            .realm_users_with_user_id_get(realm, user_id, None)
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

        current_user.credentials = match credentials {
            Some(val) => Some(
                [
                    // the new credential
                    val,
                    // the filtered list, without password
                    current_user.credentials.unwrap_or(vec![]).clone(),
                ]
                .concat(),
            ),
            None => current_user.credentials,
        };

        self.client
            .realm_users_with_user_id_put(realm, user_id, current_user.clone())
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        Ok(current_user.into())
    }

    #[instrument(skip(self), err)]
    pub async fn delete_user(&self, realm: &str, user_id: &str) -> Result<()> {
        self.client
            .realm_users_with_user_id_delete(realm, user_id)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(())
    }

    #[instrument(skip(self), err)]
    pub async fn create_user(
        self: &KeycloakAdminClient,
        realm: &str,
        user: &User,
        attributes: Option<HashMap<String, Vec<String>>>,
        groups: Option<Vec<String>>,
    ) -> Result<User> {
        let mut new_user_keycloak: UserRepresentation = user.clone().into();
        new_user_keycloak.attributes = attributes.clone();
        info!("Creating user in keycloak ?: {:?}", new_user_keycloak);
        new_user_keycloak.groups = groups.clone();
        self.client
            .realm_users_post(realm, new_user_keycloak.clone())
            .await
            .map_err(|err| {
                anyhow!("Failed to create user in keycloak: {:?}", err)
            })?;
        let found_users = self
            .client
            .realm_users_get(
                realm,
                Some(false),
                None,
                None,
                Some(true),
                Some(true),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                user.username.clone(),
            )
            .await
            .map_err(|err| {
                anyhow!("Failed to find user in keycloak: {:?}", err)
            })?;

        match found_users.first() {
            Some(found_user) => Ok(found_user.clone().into()),
            None => Ok(user.clone()),
        }
    }

    #[instrument(skip(self), err)]
    pub async fn get_user_profile_attributes(
        self: &KeycloakAdminClient,
        realm: &str,
    ) -> Result<Vec<UserProfileAttribute>> {
        let response: UPConfig = self
            .client
            .realm_users_profile_get(&realm)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        match response.attributes {
            Some(attributes) => {
                Ok(Self::get_formatted_attributes(&attributes.clone().into()))
            }
            None => Ok(vec![]),
        }
    }

    #[instrument(skip(self), err)]
    pub async fn get_user_groups(
        self: &KeycloakAdminClient,
        realm: &str,
        user_id: &str,
    ) -> Result<Vec<GroupInfo>> {
        let response: Vec<GroupRepresentation> = self
            .client
            .realm_users_with_user_id_groups_get(
                &realm, user_id, None, None, None, None,
            )
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        // Map to custom struct
        let groups: Vec<GroupInfo> = response
            .into_iter()
            .map(|group| GroupInfo {
                group_id: group
                    .id
                    .clone()
                    .unwrap_or_else(|| "Unknown Group ID".to_string()), // Default if None
                // Handle Option<String> for groupname safely
                group_name: group
                    .name
                    .clone()
                    .unwrap_or_else(|| "Unknown Group".to_string()), // Default to "Unknown Group" if None
            })
            .collect();
        Ok(groups)
    }

    pub fn get_attribute_name(name: &Option<String>) -> Option<String> {
        match name.as_deref() {
            Some(FIRST_NAME) => Some("first_name".to_string()),
            Some(LAST_NAME) => Some("last_name".to_string()),
            Some(other) => Some(other.to_string()),
            None => None,
        }
    }

    pub fn get_formatted_attributes(
        attributes_res: &Vec<UPAttribute>,
    ) -> Vec<UserProfileAttribute> {
        let formatted_attributes: Vec<UserProfileAttribute> = attributes_res
            .iter()
            .filter(|attr| match (&attr.permissions, &attr.name) {
                (Some(permissions), Some(name)) => {
                    let has_permission =
                        permissions.edit.as_ref().map_or(true, |edit| {
                            edit.contains(&PERMISSION_TO_EDIT.to_string())
                        });

                    let is_not_tenant_id =
                        !name.contains(&TENANT_ID_ATTR_NAME.to_string());

                    let is_not_area_id =
                        !name.contains(&AREA_ID_ATTR_NAME.to_string());

                    has_permission && is_not_tenant_id && is_not_area_id
                }
                _ => false,
            })
            .map(|attr| UserProfileAttribute {
                annotations: attr.annotations.clone(),
                display_name: attr.display_name.clone(),
                group: attr.group.clone(),
                multivalued: attr.multivalued,
                name: Self::get_attribute_name(&attr.name),
                required: match attr.required.clone() {
                    Some(required) => Some(UPAttributeRequired {
                        roles: required.roles,
                        scopes: required.scopes,
                    }),
                    None => None,
                },
                validations: attr.validations.clone(),
                permissions: match attr.permissions.clone() {
                    Some(permissions) => Some(UPAttributePermissions {
                        edit: permissions.edit,
                        view: permissions.view,
                    }),
                    None => None,
                },
                selector: match attr.selector.clone() {
                    Some(selector) => Some(UPAttributeSelector {
                        scopes: selector.scopes,
                    }),
                    None => None,
                },
            })
            .collect();
        formatted_attributes
    }
}
