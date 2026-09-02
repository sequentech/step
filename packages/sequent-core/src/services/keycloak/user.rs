// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::KeycloakAdminClient;
use crate::types::keycloak::*;
use crate::util::convert_vec::convert_map;
use anyhow::{anyhow, Context, Result};
use keycloak::{
    types::{
        CredentialRepresentation, GroupRepresentation, UPAttribute, UPConfig,
        UPGroup, UserRepresentation,
    },
    KeycloakError,
};
use serde::Deserialize;
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

/// A user profile constraint that Keycloak refused a write against.
///
/// Keycloak reports these as a 400 whose body names the offending attribute,
/// an i18n key for the constraint, and the constraint's own arguments, e.g.
/// `{"field": "roll", "errorMessage": "error-invalid-length", "params": ["roll", 1, 2]}`.
/// The `keycloak` crate parses that body into `KeycloakHttpError`, which keeps
/// only `errorMessage` and drops both the field and the arguments, so the raw
/// body is parsed here instead.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct UserProfileValidationError {
    pub field: Option<String>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
    #[serde(default)]
    pub params: Option<Vec<Value>>,
}

/// Keycloak reports several rejected attributes as a list and a single one as a
/// bare object, so both are accepted.
#[derive(Deserialize)]
#[serde(untagged)]
enum UserProfileValidationBody {
    Many {
        errors: Vec<UserProfileValidationError>,
    },
    One(UserProfileValidationError),
}

impl UserProfileValidationError {
    /// Keycloak reports every error in this shape, and the untagged parse below
    /// accepts any object, so only an entry that names the attribute it refused
    /// is one of these. Without a name it is some other rejection — a password
    /// against the realm policy, for instance — and belongs to whatever handles
    /// that instead.
    fn is_meaningful(&self) -> bool {
        self.field.is_some()
    }
}

/// Extract the user profile constraints Keycloak rejected a write against, so a
/// caller can tell the operator which field was refused and why rather than
/// only that the write failed. Returns an empty vector for any other error.
pub fn get_user_profile_validation_errors(
    error: &anyhow::Error,
) -> Vec<UserProfileValidationError> {
    error
        .chain()
        .find_map(|source| {
            let keycloak_error = source.downcast_ref::<KeycloakError>()?;
            let KeycloakError::HttpFailure {
                status: 400, text, ..
            } = keycloak_error
            else {
                return None;
            };

            let parsed = match serde_json::from_str(text).ok()? {
                UserProfileValidationBody::Many { errors } => errors,
                UserProfileValidationBody::One(error) => vec![error],
            };
            let meaningful: Vec<UserProfileValidationError> = parsed
                .into_iter()
                .filter(UserProfileValidationError::is_meaningful)
                .collect();

            (!meaningful.is_empty()).then_some(meaningful)
        })
        .unwrap_or_default()
}

/// Return whether an anyhow error chain contains an HTTP 400 returned by
/// Keycloak. Password-only user edits use this to preserve policy violations as
/// a structured client error instead of flattening them into a generic 500.
pub fn is_keycloak_bad_request(error: &anyhow::Error) -> bool {
    error.chain().any(|source| {
        source
            .downcast_ref::<KeycloakError>()
            .is_some_and(|keycloak_error| {
                matches!(
                    keycloak_error,
                    KeycloakError::HttpFailure { status: 400, .. }
                )
            })
    })
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
            .map_err(|err| {
                let message =
                    format!("Failed to edit user in keycloak: {err:?}");
                anyhow::Error::new(err).context(message)
            })?;

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
        new_user_keycloak.groups = groups.clone();
        self.client
            .realm_users_post(realm, new_user_keycloak.clone())
            .await
            .map_err(|err| {
                // Keep the KeycloakError as the source so callers can downcast
                // it and react to the HTTP status Keycloak returned.
                let message =
                    format!("Failed to create user in keycloak: {:?}", err);
                anyhow::Error::new(err).context(message)
            })?;
        let found_users = self
            .client
            .realm_users_get(
                realm,
                Some(false),
                None,
                None,
                None,
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
        Ok(self.get_user_profile_configuration(realm).await?.attributes)
    }

    #[instrument(skip(self), err)]
    pub async fn get_user_profile_configuration(
        self: &KeycloakAdminClient,
        realm: &str,
    ) -> Result<UserProfileConfiguration> {
        let response: UPConfig = self
            .client
            .realm_users_profile_get(&realm)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(Self::get_formatted_user_profile_configuration(response))
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
            Some(FIRST_NAME) => Some(FIRST_NAME_ATTRIBUTE.to_string()),
            Some(LAST_NAME) => Some(LAST_NAME_ATTRIBUTE.to_string()),
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

    pub fn get_formatted_groups(
        groups: &Vec<UPGroup>,
    ) -> Vec<UserProfileAttributeGroup> {
        groups
            .iter()
            .map(|group| UserProfileAttributeGroup {
                annotations: group.annotations.clone(),
                display_description: group.display_description.clone(),
                display_header: group.display_header.clone(),
                name: group.name.clone(),
            })
            .collect()
    }

    pub fn get_formatted_user_profile_configuration(
        configuration: UPConfig,
    ) -> UserProfileConfiguration {
        let attributes: Vec<UPAttribute> =
            configuration.attributes.map(Into::into).unwrap_or_default();
        let groups: Vec<UPGroup> =
            configuration.groups.map(Into::into).unwrap_or_default();

        UserProfileConfiguration {
            attributes: Self::get_formatted_attributes(&attributes),
            groups: Self::get_formatted_groups(&groups),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        get_user_profile_validation_errors, is_keycloak_bad_request,
        KeycloakAdminClient,
    };
    use anyhow::Context;
    use keycloak::{
        types::{UPAttribute, UPAttributePermissions, UPConfig, UPGroup},
        KeycloakError,
    };

    fn editable_attribute(name: &str, group: Option<&str>) -> UPAttribute {
        UPAttribute {
            name: Some(name.to_string()),
            group: group.map(str::to_string),
            permissions: Some(UPAttributePermissions {
                edit: Some(vec!["admin".to_string()].into()),
                view: None,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn formats_profile_attributes_and_groups_without_reordering() {
        let configuration = UPConfig {
            attributes: Some(
                vec![
                    editable_attribute("first", Some("identity")),
                    editable_attribute("tenant-id", Some("internal")),
                    editable_attribute("second", Some("contact")),
                ]
                .into(),
            ),
            groups: Some(
                vec![
                    UPGroup {
                        name: Some("identity".to_string()),
                        display_header: Some("Identity".to_string()),
                        ..Default::default()
                    },
                    UPGroup {
                        name: Some("contact".to_string()),
                        display_header: Some("Contact".to_string()),
                        ..Default::default()
                    },
                ]
                .into(),
            ),
            ..Default::default()
        };

        let formatted =
            KeycloakAdminClient::get_formatted_user_profile_configuration(
                configuration,
            );

        assert_eq!(
            vec![Some("first".to_string()), Some("second".to_string())],
            formatted
                .attributes
                .iter()
                .map(|attribute| attribute.name.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            vec![Some("identity".to_string()), Some("contact".to_string())],
            formatted
                .groups
                .iter()
                .map(|group| group.name.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn formats_missing_profile_groups_as_an_empty_collection() {
        let formatted =
            KeycloakAdminClient::get_formatted_user_profile_configuration(
                UPConfig {
                    attributes: Some(
                        vec![editable_attribute("first", None)].into(),
                    ),
                    groups: None,
                    ..Default::default()
                },
            );

        assert_eq!(1, formatted.attributes.len());
        assert!(formatted.groups.is_empty());
    }

    fn bad_request(text: &str) -> anyhow::Error {
        anyhow::Error::new(KeycloakError::HttpFailure {
            status: 400,
            body: serde_json::from_str(text).ok(),
            text: text.to_string(),
        })
        .context("Failed to create user in keycloak")
    }

    #[test]
    fn reads_the_attribute_and_bounds_keycloak_refused() {
        let errors = get_user_profile_validation_errors(&bad_request(
            r#"{"field":"roll","errorMessage":"error-invalid-length","params":["roll",1,2]}"#,
        ));

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field.as_deref(), Some("roll"));
        assert_eq!(
            errors[0].error_message.as_deref(),
            Some("error-invalid-length")
        );
        assert_eq!(errors[0].params.as_ref().map(Vec::len), Some(3));
    }

    #[test]
    fn reads_an_attribute_whose_arguments_keycloak_left_null() {
        let errors = get_user_profile_validation_errors(&bad_request(
            r#"{"field":"roll","errorMessage":"error-invalid-length","params":null}"#,
        ));

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field.as_deref(), Some("roll"));
    }

    #[test]
    fn reads_every_attribute_when_keycloak_refuses_several() {
        let errors = get_user_profile_validation_errors(&bad_request(
            r#"{"errors":[{"field":"roll","errorMessage":"error-invalid-length","params":["roll",1,2]},{"field":"ward","errorMessage":"error-user-attribute-required","params":["ward"]}]}"#,
        ));

        assert_eq!(errors.len(), 2);
        assert_eq!(errors[1].field.as_deref(), Some("ward"));
    }

    #[test]
    fn says_nothing_about_a_rejection_that_carries_no_attribute_name() {
        // Keycloak's generic error shape: a rejected password reads like this,
        // and is not a refused attribute.
        assert!(get_user_profile_validation_errors(&bad_request(
            r#"{"errorMessage":"invalidPasswordMinLengthMessage","params":["8"]}"#
        ))
        .is_empty());
    }

    #[test]
    fn says_nothing_about_a_rejection_that_names_no_attribute() {
        assert!(get_user_profile_validation_errors(&bad_request(
            "Password policy violation"
        ))
        .is_empty());
        assert!(get_user_profile_validation_errors(&bad_request(
            r#"{"error":"invalid_grant"}"#
        ))
        .is_empty());
        assert!(get_user_profile_validation_errors(&anyhow::anyhow!(
            "connection refused"
        ))
        .is_empty());
    }

    #[test]
    fn ignores_a_rejection_that_is_not_a_bad_request() {
        let error = anyhow::Error::new(KeycloakError::HttpFailure {
            status: 500,
            body: None,
            text: r#"{"field":"roll","errorMessage":"error-invalid-length"}"#
                .to_string(),
        });

        assert!(get_user_profile_validation_errors(&error).is_empty());
    }

    #[test]
    fn detects_a_keycloak_bad_request_through_anyhow_context() {
        let error = anyhow::Error::new(KeycloakError::HttpFailure {
            status: 400,
            body: None,
            text: "Password policy violation".to_string(),
        })
        .context("Failed to edit user");

        assert!(is_keycloak_bad_request(&error));
    }

    #[test]
    fn does_not_classify_other_keycloak_failures_as_bad_requests() {
        let error = anyhow::Error::new(KeycloakError::HttpFailure {
            status: 500,
            body: None,
            text: "Keycloak unavailable".to_string(),
        });

        assert!(!is_keycloak_bad_request(&error));
    }
}
