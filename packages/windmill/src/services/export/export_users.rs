// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_areas_by_id;
use crate::services::database::{get_keycloak_pool, PgConfig};
use crate::services::election::{get_election_event_elections, ElectionHead};
use crate::services::users::ListUsersFilter;
use crate::services::users::{list_users, list_users_with_vote_info};
use crate::services::voter_secret_attributes::{
    secret_attribute_names, VoterSecretAttributeDecryptor,
};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use deadpool_postgres::Transaction;
use regex::Regex;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::types::keycloak::{User, UserProfileAttribute};
use sequent_core::util::aws::get_max_upload_size;
use sequent_core::util::temp_path::generate_temp_file;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, info, instrument, Level};

static SAFE_CHARS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^a-zA-Z0-9._-]").expect("Failed to build safe chars regex"));

pub const USER_FIELDS: [&str; 9] = [
    "id",
    "email",
    "first_name",
    "last_name",
    "username",
    "enabled",
    "email_verified",
    "area-id",
    // The event export exposes `area-id` under this import-friendly alias.
    // A Keycloak attribute with the same name must not create a second header.
    "area_name",
];

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ExportUsersBody {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub election_id: Option<String>,
    #[serde(default)]
    pub include_secret_attributes: bool,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ExportTenantUsersBody {
    pub tenant_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ExportBody {
    Users {
        tenant_id: String,
        election_event_id: Option<String>,
        election_id: Option<String>,
        #[serde(default)]
        include_secret_attributes: bool,
    },
    TenantUsers {
        tenant_id: String,
    },
}

#[instrument(level = "trace")]
fn sanitize_name(name: &str) -> String {
    // Replace all characters not matching the regex with an underscore '_'
    SAFE_CHARS_RE.replace_all(name, "_").to_string()
}

#[instrument(skip(elections))]
fn get_headers(
    elections: &Option<Vec<ElectionHead>>,
    user_attributes: &Vec<UserProfileAttribute>,
    excluded_user_fields: &HashSet<String>,
) -> Vec<String> {
    let mut user_headers: Vec<String> = [
        "id",
        "email",
        "email_verified",
        "enabled",
        "first_name",
        "last_name",
        "username",
        "area_name",
    ]
    .into_iter()
    .filter(|name| !excluded_user_fields.contains(*name))
    .map(str::to_string)
    .collect();
    for attr in user_attributes {
        match (&attr.name) {
            (Some(name)) => {
                if (!USER_FIELDS.contains(&name.as_str())) {
                    user_headers.push(name.clone())
                }
            }
            _ => (),
        }
    }
    vec![
        user_headers,
        match elections {
            Some(ref some_elections) => some_elections
                .iter()
                .map(|election| match election.alias {
                    Some(ref election_alias) => {
                        format!("election__{}", sanitize_name(&election_alias))
                    }
                    None => format!("election__{}", sanitize_name(&election.name)),
                })
                .collect::<Vec<String>>(),
            None => vec![],
        },
    ]
    .concat()
}

#[instrument(skip(elections, areas_by_id, user, user_attributes), level = "trace")]
fn get_user_record(
    elections: &Option<Vec<ElectionHead>>,
    areas_by_id: &Option<HashMap<String, String>>,
    user: &User,
    user_attributes: &Vec<UserProfileAttribute>,
    excluded_user_fields: &HashSet<String>,
) -> Vec<String> {
    let votes_info_map_opt = user.get_votes_info_by_election_id();

    let fixed_user_values = [
        ("id", user.id.clone().unwrap_or_default()),
        ("email", user.email.clone().unwrap_or_default()),
        (
            "email_verified",
            format!("{}", user.email_verified.unwrap_or_default()),
        ),
        ("enabled", format!("{}", user.enabled.unwrap_or_default())),
        ("first_name", user.first_name.clone().unwrap_or_default()),
        ("last_name", user.last_name.clone().unwrap_or_default()),
        ("username", user.username.clone().unwrap_or_default()),
        (
            "area_name",
            match user.get_area_id() {
                Some(ref area_id) => areas_by_id
                    .as_ref()
                    .unwrap_or(&HashMap::new())
                    .get(area_id)
                    .unwrap_or(area_id)
                    .to_string(),
                None => "-".to_string(),
            },
        ),
    ];
    let mut user_info: Vec<String> = fixed_user_values
        .into_iter()
        .filter(|(name, _)| !excluded_user_fields.contains(*name))
        .map(|(_, value)| value)
        .collect();
    for attr in user_attributes {
        match &attr.name {
            Some(name) => {
                if !USER_FIELDS.contains(&name.as_str()) {
                    if let Some(true) = &attr.multivalued {
                        user_info.push(user.get_attribute_multival(name).unwrap_or_default())
                    } else {
                        user_info.push(user.get_attribute_val(name).unwrap_or_default())
                    }
                }
            }
            _ => (),
        }
    }
    return vec![
        user_info,
        match elections {
            Some(ref some_elections) => some_elections
                .iter()
                .map(|election: &ElectionHead| match votes_info_map_opt {
                    Some(ref votes_info_map) => match votes_info_map.get(&election.id) {
                        Some(ref votes_info) => votes_info.last_voted_at.clone(),
                        None => Default::default(),
                    },
                    None => Default::default(),
                })
                .collect::<Vec<String>>(),
            None => vec![],
        },
    ]
    .concat();
}

#[instrument(err, skip(hasura_transaction))]
pub async fn export_users_file(
    hasura_transaction: &Transaction<'_>,
    body: ExportBody,
) -> Result<TempPath> {
    let realm = match &body {
        ExportBody::Users {
            tenant_id,
            election_event_id,
            ..
        } => get_event_realm(tenant_id, election_event_id.as_deref().unwrap_or("")),
        ExportBody::TenantUsers { tenant_id } => get_tenant_realm(tenant_id),
    };

    let mut keycloak_db_client = get_keycloak_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring Keycloak DB pool")?;

    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .with_context(|| "Error starting Keycloak transaction")?;

    // Retrieve elections and areas, only if exporting users for a specific event
    let (elections, areas_by_id) = match &body {
        ExportBody::Users {
            tenant_id,
            election_event_id,
            ..
        } => {
            let elections = get_election_event_elections(
                &hasura_transaction,
                tenant_id,
                election_event_id.as_deref().unwrap_or(""),
            )
            .await
            .with_context(|| "Error retrieving elections for the event")?;

            let areas_by_id = get_areas_by_id(
                &hasura_transaction,
                tenant_id,
                election_event_id.as_deref().unwrap_or(""),
            )
            .await
            .with_context(|| "Error retrieving areas for the event")?;

            (Some(elections), Some(areas_by_id))
        }
        ExportBody::TenantUsers { .. } => (None, None),
    };

    // Initialize the Keycloak client and CSV writer
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| anyhow!("Error obtaining Keycloak admin client: {e:?}"))?;
    let profile_attributes = client
        .get_user_profile_attributes(&realm)
        .await
        .map_err(|e| anyhow!("Error obtaining Keycloak User Profile Attributes: {e:?}"))?;
    let configured_secret_names = secret_attribute_names(&profile_attributes)?;
    let secret_export_scope = match &body {
        ExportBody::Users {
            tenant_id,
            election_event_id: Some(election_event_id),
            include_secret_attributes: true,
            ..
        } => Some((tenant_id.as_str(), election_event_id.as_str())),
        _ => None,
    };
    let include_secret_attributes = secret_export_scope.is_some();
    let secret_decryptor = if include_secret_attributes {
        Some(
            VoterSecretAttributeDecryptor::new()
                .await
                .with_context(|| "Error obtaining the voter secret-attribute master key")?,
        )
    } else {
        None
    };
    let excluded_user_fields = if include_secret_attributes {
        HashSet::new()
    } else {
        configured_secret_names
            .iter()
            .filter(|name| USER_FIELDS.contains(&name.as_str()))
            .cloned()
            .collect()
    };
    let attributes = profile_attributes
        .into_iter()
        .filter(|attribute| {
            include_secret_attributes
                || attribute
                    .name
                    .as_ref()
                    .is_none_or(|name| !configured_secret_names.contains(name))
        })
        .collect::<Vec<_>>();
    let headers = get_headers(&elections, &attributes, &excluded_user_fields);

    // Pagination loop to export users in batches
    let batch_size = PgConfig::from_env()?.default_sql_batch_size;
    let mut offset: i32 = 0;
    let mut total_count: Option<i32> = None;

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file("export-users-", ".csv")
            .with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&headers)?;

    loop {
        let filter = ListUsersFilter {
            tenant_id: match &body {
                ExportBody::Users { tenant_id, .. } => tenant_id.to_string(),
                ExportBody::TenantUsers { tenant_id } => tenant_id.to_string(),
            },
            election_event_id: match &body {
                ExportBody::Users {
                    election_event_id, ..
                } => election_event_id.clone(),
                ExportBody::TenantUsers { .. } => None,
            },
            election_id: match &body {
                ExportBody::Users { election_id, .. } => election_id.clone(),
                ExportBody::TenantUsers { .. } => None,
            },
            area_id: None,
            realm: realm.clone(),
            search: None,
            first_name: None,
            last_name: None,
            username: None,
            email: None,
            limit: Some(batch_size),
            offset: Some(offset),
            user_ids: None,
            attributes: None,
            enabled: None,
            email_verified: None,
            sort: None,
            has_voted: None,
            authorized_to_election_alias: None,
        };

        let (users, count) = match &body {
            ExportBody::Users {
                election_event_id, ..
            } if election_event_id.is_some() => list_users_with_vote_info(
                &hasura_transaction,
                &keycloak_transaction,
                filter.clone(),
            )
            .await
            .with_context(|| "Error retrieving users with vote info")?,
            _ => list_users(&hasura_transaction, &keycloak_transaction, filter.clone())
                .await
                .with_context(|| "Error listing users")?,
        };

        if total_count.is_none() {
            total_count = Some(count);
        }

        offset += users.len() as i32;

        // Write each user record to the CSV file
        for mut user in users.clone() {
            if let (Some(decryptor), Some((tenant_id, election_event_id))) =
                (&secret_decryptor, secret_export_scope)
            {
                decryptor
                    .decrypt_user_attributes(
                        &mut user,
                        tenant_id,
                        election_event_id,
                        &configured_secret_names,
                    )
                    .with_context(|| {
                        format!(
                            "Error decrypting secret attributes for voter {}",
                            user.id.as_deref().unwrap_or("unknown")
                        )
                    })?;
            }
            let record = get_user_record(
                &elections,
                &areas_by_id,
                &user,
                &attributes,
                &excluded_user_fields,
            );
            writer
                .write_record(&record)
                .with_context(|| "Error writing record")?;
        }

        if users.is_empty() || offset >= total_count.unwrap_or_default() {
            break;
        }
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    let size = temp_path.metadata()?.len();
    if size > get_max_upload_size()? as u64 {
        return Err(anyhow!("File too large: {} > {}", size, get_max_upload_size()?).into());
    }

    Ok(temp_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attribute(name: &str) -> UserProfileAttribute {
        UserProfileAttribute {
            annotations: None,
            display_name: None,
            group: None,
            multivalued: None,
            name: Some(name.to_string()),
            required: None,
            validations: None,
            permissions: None,
            selector: None,
        }
    }

    #[test]
    fn area_name_profile_attribute_does_not_duplicate_or_shift_export_columns() {
        let attributes = vec![attribute("area_name"), attribute("custom_attribute")];
        let excluded_user_fields = HashSet::new();
        let headers = get_headers(&None, &attributes, &excluded_user_fields);
        let user = User {
            id: Some("id".to_string()),
            email: Some("email@example.com".to_string()),
            email_verified: Some(true),
            enabled: Some(true),
            first_name: Some("First".to_string()),
            last_name: Some("Last".to_string()),
            username: Some("username".to_string()),
            attributes: Some(HashMap::from([
                ("area-id".to_string(), vec!["area-1".to_string()]),
                (
                    "custom_attribute".to_string(),
                    vec!["custom-value".to_string()],
                ),
            ])),
            ..Default::default()
        };
        let areas_by_id = Some(HashMap::from([(
            "area-1".to_string(),
            "Area One".to_string(),
        )]));
        let record = get_user_record(
            &None,
            &areas_by_id,
            &user,
            &attributes,
            &excluded_user_fields,
        );

        assert_eq!(
            1,
            headers
                .iter()
                .filter(|header| *header == "area_name")
                .count()
        );
        assert_eq!(headers.len(), record.len());
        assert_eq!(
            Some(&"username".to_string()),
            record.get(
                headers
                    .iter()
                    .position(|header| header == "username")
                    .unwrap()
            )
        );
        assert_eq!(
            Some(&"Area One".to_string()),
            record.get(
                headers
                    .iter()
                    .position(|header| header == "area_name")
                    .unwrap()
            )
        );
        assert_eq!(
            Some(&"custom-value".to_string()),
            record.get(
                headers
                    .iter()
                    .position(|header| header == "custom_attribute")
                    .unwrap()
            )
        );
    }

    #[test]
    fn unchecked_export_omits_secret_columns_and_ciphertext() {
        let configured_secret_names =
            HashSet::from(["last_name".to_string(), "private-reference".to_string()]);
        let attributes = vec![
            attribute("private-reference"),
            attribute("public-reference"),
        ]
        .into_iter()
        .filter(|attribute| {
            attribute
                .name
                .as_ref()
                .is_none_or(|name| !configured_secret_names.contains(name))
        })
        .collect::<Vec<_>>();
        let excluded_user_fields = configured_secret_names
            .iter()
            .filter(|name| USER_FIELDS.contains(&name.as_str()))
            .cloned()
            .collect::<HashSet<_>>();
        let headers = get_headers(&None, &attributes, &excluded_user_fields);
        let user = User {
            last_name: Some("seqenc:v1:last-name-ciphertext".to_string()),
            attributes: Some(HashMap::from([
                (
                    "private-reference".to_string(),
                    vec!["seqenc:v1:private-reference-ciphertext".to_string()],
                ),
                (
                    "public-reference".to_string(),
                    vec!["public-value".to_string()],
                ),
            ])),
            ..Default::default()
        };
        let record = get_user_record(&None, &None, &user, &attributes, &excluded_user_fields);

        assert!(!headers.contains(&"last_name".to_string()));
        assert!(!headers.contains(&"private-reference".to_string()));
        assert!(headers.contains(&"public-reference".to_string()));
        assert_eq!(headers.len(), record.len());
        assert!(!record.iter().any(|value| value.starts_with("seqenc:v1:")));
        assert!(record.contains(&"public-value".to_string()));
    }
}
