// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_areas_by_id;
use crate::services::database::{get_keycloak_pool, PgConfig};
use crate::services::election::{get_election_event_elections, ElectionHead};
use crate::services::users::ListUsersFilter;
use crate::services::users::{list_users, list_users_with_vote_info};
use crate::services::voter_secret_attributes::{
    get_secret_attribute_config, VoterSecretAttributeDecryptor,
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

fn scoped_export_elections(elections: Vec<ElectionHead>, ids: &[String]) -> Vec<ElectionHead> {
    elections
        .into_iter()
        .filter(|election| ids.contains(&election.id))
        .collect()
}

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

/// Prefix of the informational participation columns; the import skips them.
const ELECTION_HEADER_PREFIX: &str = "election__";

/// PostgreSQL keeps the first 63 bytes of an identifier (`NAMEDATALEN - 1`).
const MAX_IMPORT_COLUMN_BYTES: usize = 63;

/// Shortens `value` to at most `max_bytes`, on a character boundary.
fn truncate_to_bytes(value: &str, max_bytes: usize) -> &str {
    let mut end = value.len().min(max_bytes);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

/// The temp table column a header becomes when the CSV is imported: `.` and
/// `-` turn into `_`, and the unquoted identifier is case-folded and truncated.
fn import_column_key(header: &str) -> String {
    let key = header.replace(['.', '-'], "_").to_lowercase();
    truncate_to_bytes(&key, MAX_IMPORT_COLUMN_BYTES).to_string()
}

/// One `election__<label>` header per election, distinct from each other and
/// from `user_headers` as import columns. A label keeps its usual header
/// unless an earlier column already maps to it; the others get the first free
/// `__<n>` suffix (from 2), shortening the label so the suffix fits in 63
/// bytes.
fn election_headers(elections: &[ElectionHead], user_headers: &[String]) -> Vec<String> {
    let mut taken: HashSet<String> = user_headers
        .iter()
        .map(|header| import_column_key(header))
        .collect();
    let labels: Vec<String> = elections
        .iter()
        .map(|election| {
            let label = election.alias.as_deref().unwrap_or(&election.name);
            format!("{ELECTION_HEADER_PREFIX}{}", sanitize_name(label))
        })
        .collect();
    // First pass: every label whose column is still free keeps its header,
    // so a later label such as `X__2` is not displaced by a repeated `X`.
    let mut headers: Vec<Option<String>> = Vec::with_capacity(labels.len());
    for label in &labels {
        let header = taken
            .insert(import_column_key(label))
            .then(|| label.clone());
        headers.push(header);
    }
    for (header, label) in headers.iter_mut().zip(&labels) {
        if header.is_some() {
            continue;
        }
        // Each attempt either succeeds or skips a taken key, so this ends
        // within `taken.len() + 1` attempts.
        let mut number: usize = 2;
        loop {
            let suffix = format!("__{number}");
            let candidate = format!(
                "{}{suffix}",
                truncate_to_bytes(label, MAX_IMPORT_COLUMN_BYTES - suffix.len())
            );
            if taken.insert(import_column_key(&candidate)) {
                *header = Some(candidate);
                break;
            }
            number += 1;
        }
    }
    headers.into_iter().flatten().collect()
}

/// The export's header row: user columns, then one participation column per
/// election, in the order `get_user_record` writes the values.
#[instrument(skip(elections))]
pub(crate) fn get_headers(
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
    let election_headers = match elections {
        Some(some_elections) => election_headers(some_elections, &user_headers),
        None => vec![],
    };
    [user_headers, election_headers].concat()
}

#[instrument(skip(elections, areas_by_id, user, user_attributes), level = "trace")]
fn get_user_record(
    elections: &Option<Vec<ElectionHead>>,
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
                        Some(ref votes_info) if votes_info.num_votes > 0 => "true".to_string(),
                        Some(_) => String::new(),
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
    permitted_election_ids: Option<&[String]>,
) -> Result<TempPath> {
    if matches!(
        &body,
        ExportBody::Users {
            election_event_id: Some(_),
            ..
        }
    ) && permitted_election_ids.is_none()
    {
        return Err(Error::String(
            crate::services::participation_authorization::REAUTHORIZE_PARTICIPATION.into(),
        ));
    }
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
            let elections =
                scoped_export_elections(elections, permitted_election_ids.unwrap_or_default());

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
            permitted_election_ids: permitted_election_ids.map(<[String]>::to_vec),
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
            let record = get_user_record(&elections, &areas_by_id, &user, &attributes);
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
        let record = get_user_record(&None, &areas_by_id, &user, &attributes);

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

    fn election(id: &str, name: &str, alias: Option<&str>) -> ElectionHead {
        ElectionHead {
            id: id.to_string(),
            name: name.to_string(),
            alias: alias.map(str::to_string),
            external_id: None,
        }
    }

    /// The participation columns that follow the user columns.
    fn election_headers_for(
        elections: Vec<ElectionHead>,
        user_attributes: &Vec<UserProfileAttribute>,
    ) -> Vec<String> {
        let headers = get_headers(&Some(elections), user_attributes);
        let user_columns = get_headers(&None, user_attributes).len();
        headers[user_columns..].to_vec()
    }

    #[test]
    fn unique_election_labels_keep_their_existing_headers() {
        let headers = election_headers_for(
            vec![
                election("1", "North", None),
                election("2", "Ignored name", Some("South Region")),
            ],
            &vec![],
        );
        assert_eq!(headers, ["election__North", "election__South_Region"]);
    }

    /// The reported import failure: two elections sharing a label exported
    /// `election__General_Election` twice and the importer refused the file.
    #[test]
    fn repeated_election_labels_get_numbered_headers() {
        let headers = election_headers_for(
            vec![
                election("1", "General Election", None),
                election("2", "General Election", None),
                election("3", "North", None),
                election("4", "General Election", None),
            ],
            &vec![],
        );
        assert_eq!(
            headers,
            [
                "election__General_Election",
                "election__General_Election__2",
                "election__North",
                "election__General_Election__3",
            ]
        );
    }

    #[test]
    fn alias_colliding_with_another_elections_name_is_numbered() {
        let headers = election_headers_for(
            vec![
                election("1", "Regional", Some("General Election")),
                election("2", "General Election", None),
            ],
            &vec![],
        );
        assert_eq!(
            headers,
            [
                "election__General_Election",
                "election__General_Election__2"
            ]
        );
    }

    /// The importer maps `.` and `-` to `_`, and the export sanitizer maps any
    /// other punctuation to `_`, so these four labels are one import column.
    #[test]
    fn labels_that_collide_after_sanitizing_are_numbered() {
        let headers = election_headers_for(
            vec![
                election("1", "General Election", None),
                election("2", "General-Election", None),
                election("3", "General.Election", None),
                election("4", "General/Election", None),
            ],
            &vec![],
        );
        assert_eq!(
            headers,
            [
                "election__General_Election",
                "election__General-Election__2",
                "election__General.Election__3",
                "election__General_Election__4",
            ]
        );
    }

    /// Unquoted SQL identifiers fold to lowercase on import.
    #[test]
    fn labels_differing_only_by_case_are_numbered() {
        let headers = election_headers_for(
            vec![
                election("1", "Board", None),
                election("2", "board", None),
                election("3", "BOARD", None),
            ],
            &vec![],
        );
        assert_eq!(
            headers,
            [
                "election__Board",
                "election__board__2",
                "election__BOARD__3"
            ]
        );
    }

    /// A label that already looks like a generated header keeps its name; the
    /// repeated label skips to the next free number instead.
    #[test]
    fn natural_suffix_like_label_keeps_its_name() {
        let headers = election_headers_for(
            vec![
                election("1", "General Election", None),
                election("2", "General Election", None),
                election("3", "General Election__2", None),
            ],
            &vec![],
        );
        assert_eq!(
            headers,
            [
                "election__General_Election",
                "election__General_Election__3",
                "election__General_Election__2",
            ]
        );
    }

    /// PostgreSQL keeps the first 63 bytes of an identifier, so long labels
    /// sharing those bytes are one column. A unique long label keeps its full
    /// header; a numbered one is shortened so its number survives truncation.
    #[test]
    fn long_labels_sharing_the_first_63_bytes_are_numbered_within_63_bytes() {
        let shared = "A".repeat(53);
        let headers = election_headers_for(
            vec![
                election("1", &format!("{shared} one"), None),
                election("2", &format!("{shared} two"), None),
                election("3", &format!("{shared} one"), None),
            ],
            &vec![],
        );
        assert_eq!(
            headers,
            [
                format!("election__{shared}_one"),
                format!("election__{}__2", "A".repeat(50)),
                format!("election__{}__3", "A".repeat(50)),
            ]
        );
        assert!(headers[1..].iter().all(|header| header.len() == 63));
    }

    /// A realm attribute already exported under an `election__` name is a user
    /// column: it keeps its name and the election header is numbered instead.
    #[test]
    fn election_header_does_not_reuse_a_user_attribute_column() {
        let attributes = vec![attribute("election__north")];
        let headers = get_headers(&Some(vec![election("1", "North", None)]), &attributes);
        assert_eq!(
            headers[headers.len() - 2..],
            ["election__north", "election__North__2"]
        );
    }

    /// Each participation value stays under its own election's header.
    #[test]
    fn numbered_election_columns_stay_aligned_with_participation_values() {
        let elections = Some(vec![
            election("first", "General Election", None),
            election("second", "General Election", None),
        ]);
        let user = User {
            votes_info: Some(vec![sequent_core::types::keycloak::VotesInfo {
                election_id: "second".into(),
                num_votes: 1,
                last_voted_at: String::new(),
            }]),
            ..Default::default()
        };
        let headers = get_headers(&elections, &vec![]);
        let record = get_user_record(&elections, &None, &user, &vec![]);
        assert_eq!(headers.len(), record.len());
        assert_eq!(
            headers[headers.len() - 2..],
            [
                "election__General_Election",
                "election__General_Election__2"
            ]
        );
        assert_eq!(record[record.len() - 2..], ["", "true"]);
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
        let record = get_user_record(&None, &None, &user, &attributes);

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
        let record = get_user_record(&None, &None, &user, &attributes);
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
#[test]
fn privacy_csv_has_only_authorized_elections_and_no_hidden_participation() {
    let all = ["north", "south", "new-after-authorization"]
        .into_iter()
        .map(|id| ElectionHead {
            id: id.into(),
            name: id.into(),
            alias: None,
            external_id: None,
        })
        .collect();
    let elections = Some(scoped_export_elections(all, &["north".into()]));
    let user = User {
        username: Some("voter".into()),
        votes_info: Some(vec![sequent_core::types::keycloak::VotesInfo {
            election_id: "south".into(),
            num_votes: 1,
            last_voted_at: String::new(),
        }]),
        ..Default::default()
    };
    let headers = get_headers(&elections, &vec![]);
    let record = get_user_record(&elections, &None, &user, &vec![]);
    assert_eq!(headers.last().unwrap(), "election__north");
    assert_eq!(record.last().unwrap(), "");
    assert!(!headers
        .iter()
        .any(|value| value.contains("south") || value.contains("new-after")));
    let empty = Some(scoped_export_elections(elections.unwrap(), &[]));
    assert!(!get_headers(&empty, &vec![])
        .iter()
        .any(|value| value.starts_with("election__")));
}
