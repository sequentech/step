// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_areas_by_id;
use crate::services::authorized_elections::{authorized_election_id, AuthorizedElectionIds};
use crate::services::database::{get_keycloak_pool, PgConfig};
use crate::services::election::{get_election_event_elections, ElectionHead};
use crate::services::import::import_users::ELECTION_COL_PREFIX;
use crate::services::users::ListUsersFilter;
use crate::services::users::{list_users, list_users_with_vote_info};
use crate::services::voter_secret_attributes::{
    get_secret_attribute_config, VoterSecretAttributeDecryptor,
};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::{
    get_event_realm, get_tenant_realm, MULTIVALUE_USER_ATTRIBUTE_SEPARATOR,
};
use sequent_core::types::keycloak::{User, UserProfileAttribute, AUTHORIZED_ELECTION_IDS_NAME};
use sequent_core::util::aws::get_max_upload_size;
use sequent_core::util::temp_path::generate_temp_file;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, info, instrument, Level};

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

#[instrument(skip(elections))]
fn get_headers(
    elections: &Option<Vec<ElectionHead>>,
    user_attributes: &Vec<UserProfileAttribute>,
) -> Vec<String> {
    let mut user_headers: Vec<String> = vec![
        "id".to_string(),
        "email".to_string(),
        "email_verified".to_string(),
        "enabled".to_string(),
        "first_name".to_string(),
        "last_name".to_string(),
        "username".to_string(),
        "area_name".to_string(),
    ];
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
            // Display names are not unique, and import ignores these columns.
            Some(ref some_elections) => some_elections
                .iter()
                .map(|election| {
                    format!("{ELECTION_COL_PREFIX}{}", authorized_election_id(election))
                })
                .collect::<Vec<String>>(),
            None => vec![],
        },
    ]
    .concat()
}

/// Writes the voter's authorized elections the way import reads them. Values
/// that name no election are kept, so that importing them fails rather than
/// leaving the voter unrestricted.
fn get_authorized_election_ids(
    user: &User,
    authorized_elections: Option<&AuthorizedElectionIds>,
) -> String {
    let mut values: Vec<String> = Vec::new();
    for value in user.get_authorized_election_ids().unwrap_or_default() {
        let value = authorized_elections
            .and_then(|elections| elections.resolve(&value))
            .map(str::to_string)
            .unwrap_or(value);
        if !values.contains(&value) {
            values.push(value);
        }
    }
    values.join(MULTIVALUE_USER_ATTRIBUTE_SEPARATOR)
}

#[instrument(
    skip(elections, authorized_elections, areas_by_id, user, user_attributes),
    level = "trace"
)]
fn get_user_record(
    elections: &Option<Vec<ElectionHead>>,
    authorized_elections: Option<&AuthorizedElectionIds>,
    areas_by_id: &Option<HashMap<String, String>>,
    user: &User,
    user_attributes: &Vec<UserProfileAttribute>,
) -> Vec<String> {
    let votes_info_map_opt = user.get_votes_info_by_election_id();

    let mut user_info: Vec<String> = vec![
        user.id.clone().unwrap_or_default(),
        user.email.clone().unwrap_or_default(),
        format!("{}", user.email_verified.unwrap_or_default()),
        format!("{}", user.enabled.unwrap_or_default()),
        user.first_name.clone().unwrap_or_default(),
        user.last_name.clone().unwrap_or_default(),
        user.username.clone().unwrap_or_default(),
        match user.get_area_id() {
            Some(ref area_id) => areas_by_id
                .as_ref()
                .unwrap_or(&HashMap::new())
                .get(area_id)
                .unwrap_or(area_id)
                .to_string(),
            None => "-".to_string(),
        },
    ];
    for attr in user_attributes {
        match &attr.name {
            Some(name) => {
                if !USER_FIELDS.contains(&name.as_str()) {
                    if name == AUTHORIZED_ELECTION_IDS_NAME {
                        user_info.push(get_authorized_election_ids(user, authorized_elections))
                    } else if let Some(true) = &attr.multivalued {
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
    // A decrypted export needs a valid configuration; an ordinary export only
    // needs to know which columns to leave out.
    let configured_secret_names = match &body {
        ExportBody::Users {
            tenant_id,
            election_event_id: Some(election_event_id),
            ..
        } => {
            let config = get_secret_attribute_config(tenant_id, election_event_id)
                .await
                .with_context(|| "Error reading the secret-attribute configuration")?;
            if include_secret_attributes {
                config.validated_names()?
            } else {
                config.redacted_names().clone()
            }
        }
        _ => HashSet::new(),
    };
    let secret_decryptor = if include_secret_attributes {
        Some(
            VoterSecretAttributeDecryptor::new()
                .await
                .with_context(|| "Error obtaining the voter secret-attribute master key")?,
        )
    } else {
        None
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
    let headers = get_headers(&elections, &attributes);
    let authorized_elections = elections.as_deref().map(AuthorizedElectionIds::new);

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
                authorized_elections.as_ref(),
                &areas_by_id,
                &user,
                &attributes,
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
    use crate::services::import::import_users::{
        imported_fields, is_election_column, resolve_authorized_election_ids, HEADER_RE,
    };

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

    const ELECTION_A: &str = "6f1c2d3e-4a5b-4c6d-8e7f-0a1b2c3d4e5f";
    const ELECTION_B: &str = "7a2b3c4d-5e6f-4a7b-9c8d-1e2f3a4b5c6d";
    const ELECTION_C: &str = "8b3c4d5e-6f7a-4b8c-ad9e-2f3a4b5c6d7e";
    const ELECTION_D: &str = "9c4d5e6f-7a8b-4c9d-be0f-3a4b5c6d7e8f";

    fn election(id: &str, external_id: Option<&str>) -> ElectionHead {
        ElectionHead {
            id: id.to_string(),
            // Elections named only in other languages all fall back to the
            // same placeholder in the event's default language.
            name: "-".to_string(),
            alias: None,
            external_id: external_id.map(str::to_string),
        }
    }

    fn elections() -> Vec<ElectionHead> {
        vec![
            election(ELECTION_A, Some("GTELEC31+GCIBER30-1-01")),
            election(ELECTION_B, Some("GIAMBI30-3-31")),
            election(ELECTION_C, None),
            election(ELECTION_D, Some("")),
        ]
    }

    #[test]
    fn election_columns_are_named_after_external_ids_or_ids() {
        let headers = get_headers(&Some(elections()), &vec![]);
        let election_headers = headers
            .iter()
            .filter(|header| header.starts_with("election__"))
            .map(String::as_str)
            .collect::<Vec<_>>();

        assert_eq!(
            election_headers,
            vec![
                "election__GTELEC31+GCIBER30-1-01".to_string(),
                "election__GIAMBI30-3-31".to_string(),
                format!("election__{ELECTION_C}"),
                format!("election__{ELECTION_D}"),
            ]
        );
    }

    fn voter(username: &str, authorized_election_ids: &[&str]) -> User {
        let mut attributes = HashMap::from([("area-id".to_string(), vec!["area-1".to_string()])]);
        if !authorized_election_ids.is_empty() {
            attributes.insert(
                AUTHORIZED_ELECTION_IDS_NAME.to_string(),
                authorized_election_ids
                    .iter()
                    .map(|value| value.to_string())
                    .collect(),
            );
        }
        User {
            id: Some(format!("{username}-id")),
            username: Some(username.to_string()),
            enabled: Some(true),
            attributes: Some(attributes),
            ..Default::default()
        }
    }

    fn exported_authorized_election_ids(user: &User) -> String {
        let elections = elections();
        let attributes = vec![attribute(AUTHORIZED_ELECTION_IDS_NAME)];
        let headers = get_headers(&Some(elections.clone()), &attributes);
        let record = get_user_record(
            &Some(elections.clone()),
            Some(&AuthorizedElectionIds::new(&elections)),
            &None,
            user,
            &attributes,
        );
        let index = headers
            .iter()
            .position(|header| header == AUTHORIZED_ELECTION_IDS_NAME)
            .expect("the column is exported");
        record[index].clone()
    }

    /// Voters imported before the fix kept the election IDs they were given.
    /// The profile attribute is not flagged multivalued here, as in older
    /// realms, and every value must still be written.
    #[test]
    fn authorized_elections_are_exported_by_external_id_or_id_without_one() {
        assert_eq!(
            exported_authorized_election_ids(&voter(
                "legacy",
                &[ELECTION_A, ELECTION_C, ELECTION_D]
            )),
            format!("GTELEC31+GCIBER30-1-01|{ELECTION_C}|{ELECTION_D}")
        );
        assert_eq!(
            exported_authorized_election_ids(&voter("current", &["GIAMBI30-3-31"])),
            "GIAMBI30-3-31"
        );
        assert_eq!(
            exported_authorized_election_ids(&voter("both", &[ELECTION_B, "GIAMBI30-3-31"])),
            "GIAMBI30-3-31"
        );
        assert_eq!(
            exported_authorized_election_ids(&voter("unrestricted", &[])),
            ""
        );
    }

    /// A value that names no election cannot be authorized by anything, and
    /// dropping it could leave the voter unrestricted.
    #[test]
    fn authorized_elections_naming_no_election_are_exported_unchanged() {
        assert_eq!(
            exported_authorized_election_ids(&voter("stale", &["GONE-1"])),
            "GONE-1"
        );
    }

    /// Reads an exported file the way the voters import does, returning each
    /// row's imported fields by column, with `authorized-election-ids` as it
    /// would be stored.
    fn import(csv: &[u8], elections: &AuthorizedElectionIds) -> Vec<HashMap<String, String>> {
        let mut reader = csv::Reader::from_reader(csv);
        let all_headers = reader.headers().expect("headers").clone();
        let imported_columns = all_headers
            .iter()
            .map(|header| !is_election_column(header))
            .collect::<Vec<bool>>();
        let headers = imported_fields(&all_headers, &imported_columns);
        assert!(
            headers.iter().all(|header| HEADER_RE.is_match(header)),
            "{headers:?} must pass the header check"
        );
        let mut seen = HashSet::new();
        assert!(
            headers.iter().all(|header| seen.insert(header.to_string())),
            "{headers:?} must not repeat"
        );
        reader
            .records()
            .enumerate()
            .map(|(index, record)| {
                let record = imported_fields(&record.expect("record"), &imported_columns);
                headers
                    .iter()
                    .zip(record.iter())
                    .map(|(header, field)| {
                        let field = if header == AUTHORIZED_ELECTION_IDS_NAME {
                            resolve_authorized_election_ids(field, index + 2, elections)
                                .expect("every value names an election")
                        } else {
                            field.to_string()
                        };
                        (header.to_string(), field)
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn exported_voters_import_with_their_elections() {
        let elections = elections();
        let attributes = vec![
            attribute("custom_attribute"),
            attribute(AUTHORIZED_ELECTION_IDS_NAME),
        ];
        let authorized_elections = AuthorizedElectionIds::new(&elections);
        let voters = vec![
            voter("legacy", &[ELECTION_A, ELECTION_C]),
            voter("current", &["GIAMBI30-3-31", "GTELEC31+GCIBER30-1-01"]),
            voter("unrestricted", &[]),
        ];

        let mut writer = csv::Writer::from_writer(vec![]);
        writer
            .write_record(get_headers(&Some(elections.clone()), &attributes))
            .expect("headers");
        for user in &voters {
            writer
                .write_record(get_user_record(
                    &Some(elections.clone()),
                    Some(&authorized_elections),
                    &None,
                    user,
                    &attributes,
                ))
                .expect("record");
        }
        let csv = writer.into_inner().expect("csv");

        let imported = import(&csv, &authorized_elections);
        let stored = imported
            .iter()
            .map(|row| {
                (
                    row["username"].as_str(),
                    row[AUTHORIZED_ELECTION_IDS_NAME].clone(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            stored,
            vec![
                ("legacy", format!("GTELEC31+GCIBER30-1-01|{ELECTION_C}")),
                (
                    "current",
                    "GIAMBI30-3-31|GTELEC31+GCIBER30-1-01".to_string()
                ),
                ("unrestricted", String::new()),
            ]
        );
    }

    /// A voters file in an election event exported before the fix: repeated
    /// election columns, and elections named by the exporting event's IDs,
    /// which the event import replaces.
    #[test]
    fn voters_exported_before_the_fix_import_into_a_new_event() {
        let exported_a = "1d2e3f4a-5b6c-4d7e-8f9a-0b1c2d3e4f5a";
        let exported_c = "2e3f4a5b-6c7d-4e8f-9a0b-1c2d3e4f5a6b";
        let csv = format!(
            "id,email,email_verified,enabled,first_name,last_name,username,area_name,\
             {AUTHORIZED_ELECTION_IDS_NAME},election__-,election__-\n\
             1,,true,true,,,legacy,EHU,{exported_a}|{exported_c},,\n\
             2,,true,true,,,current,EHU,GIAMBI30-3-31,2025-05-01T10:00:00Z,\n\
             3,,true,true,,,unrestricted,EHU,,,\n"
        );
        let replaced_ids = HashMap::from([
            (exported_a.to_string(), ELECTION_A.to_string()),
            (exported_c.to_string(), ELECTION_C.to_string()),
        ]);
        let authorized_elections =
            AuthorizedElectionIds::new(&elections()).with_replaced_ids(&replaced_ids);

        let stored = import(csv.as_bytes(), &authorized_elections)
            .into_iter()
            .map(|row| row[AUTHORIZED_ELECTION_IDS_NAME].clone())
            .collect::<Vec<_>>();
        assert_eq!(
            stored,
            vec![
                format!("GTELEC31+GCIBER30-1-01|{ELECTION_C}"),
                "GIAMBI30-3-31".to_string(),
                String::new(),
            ]
        );
    }

    #[test]
    fn area_name_profile_attribute_does_not_duplicate_or_shift_export_columns() {
        let attributes = vec![attribute("area_name"), attribute("custom_attribute")];
        let headers = get_headers(&None, &attributes);
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
        let record = get_user_record(&None, None, &areas_by_id, &user, &attributes);

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
        let configured_secret_names = HashSet::from(["private-reference".to_string()]);
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
        let headers = get_headers(&None, &attributes);
        let user = User {
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
        let record = get_user_record(&None, None, &None, &user, &attributes);

        assert!(!headers.contains(&"private-reference".to_string()));
        assert!(headers.contains(&"public-reference".to_string()));
        assert_eq!(headers.len(), record.len());
        assert!(!record.iter().any(|value| value.starts_with("seqenc:v1:")));
        assert!(record.contains(&"public-value".to_string()));
    }

    #[test]
    fn opted_in_csv_preserves_secret_values_and_multi_value_import_format() {
        let mut secret = attribute("login-code");
        secret.multivalued = Some(true);
        let attributes = vec![secret];
        let user = User {
            attributes: Some(HashMap::from([(
                "login-code".to_string(),
                vec!["first-secret".to_string(), "second-secret".to_string()],
            )])),
            ..Default::default()
        };
        let headers = get_headers(&None, &attributes);
        let record = get_user_record(&None, None, &None, &user, &attributes);
        let index = headers
            .iter()
            .position(|name| name == "login-code")
            .unwrap();
        assert_eq!(
            record[index],
            user.get_attribute_multival(&"login-code".to_string())
                .unwrap()
        );
        assert!(record[index].contains("first-secret"));
        assert!(record[index].contains("second-secret"));
        assert!(!record[index].contains("seqenc:"));
    }
}
