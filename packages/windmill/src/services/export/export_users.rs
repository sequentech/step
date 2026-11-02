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
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::types::keycloak::{User, UserProfileAttribute};
use sequent_core::util::aws::get_max_upload_size;
use sequent_core::util::temp_path::generate_temp_file;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, info, instrument, Level};

/// Prefix of a participation column. The rest of the header is the election's
/// external ID, which an election event keeps unique per election.
pub(crate) const ELECTION_EXTERNAL_ID_HEADER_PREFIX: &str = "election__external_id__";

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

/// The participation column for the election with this external ID: the prefix,
/// then the ID's UTF-8 bytes in lowercase hexadecimal.
///
/// The encoding is reversible and fits the import header pattern
/// (`[a-zA-Z0-9._-]`). Unlike replacing unsafe characters, it keeps IDs that
/// differ only by case, punctuation, spacing or non-ASCII characters distinct,
/// for example `A-B` and `A_B`, or `Board` and `board`.
fn election_header(external_id: &str) -> String {
    format!(
        "{ELECTION_EXTERNAL_ID_HEADER_PREFIX}{}",
        hex::encode(external_id.as_bytes())
    )
}

/// One participation column per election, in the order given.
///
/// The external ID is the election's only identity in the file, so an election
/// without one, or two elections sharing one, stops the export rather than
/// falling back to another identifier. The ID is encoded exactly as stored:
/// never trimmed or case-folded.
fn election_headers(elections: &[ElectionHead]) -> Result<Vec<String>> {
    // Elections are named by display name and id; the list is already limited
    // to the elections this export may show.
    let describe = |election: &ElectionHead| format!("{:?} ({})", election.name, election.id);
    let mut seen: HashMap<&str, &ElectionHead> = HashMap::new();
    elections
        .iter()
        .map(|election| {
            let external_id = election
                .external_id
                .as_deref()
                .filter(|external_id| !external_id.trim().is_empty())
                .ok_or_else(|| {
                    Error::String(format!(
                        "Election {} has no external ID. The voters export names each \
                         election's participation column by its external ID: set one \
                         before exporting.",
                        describe(election)
                    ))
                })?;
            if let Some(other) = seen.insert(external_id, election) {
                return Err(Error::String(format!(
                    "Elections {} and {} share the external ID {external_id:?}. \
                     External IDs must be unique within the election event before \
                     exporting voters.",
                    describe(other),
                    describe(election)
                )));
            }
            Ok(election_header(external_id))
        })
        .collect()
}

/// The temp table column the voter import makes from a header: `.` and `-`
/// become `_`, and case is folded.
fn import_column_key(header: &str) -> String {
    header.replace(['.', '-'], "_").to_lowercase()
}

#[instrument(skip(elections))]
pub(crate) fn get_headers(
    elections: &Option<Vec<ElectionHead>>,
    user_attributes: &Vec<UserProfileAttribute>,
) -> Result<Vec<String>> {
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
        Some(some_elections) => election_headers(some_elections)?,
        None => vec![],
    };
    // A voter attribute read back as a participation column would be dropped
    // on import, or rejected as a duplicate.
    if let Some(attribute) = user_headers
        .iter()
        .find(|header| election_headers.contains(&import_column_key(header)))
    {
        return Err(Error::String(format!(
            "The voter attribute `{attribute}` has the same name as an election \
             participation column. Rename the attribute before exporting voters."
        )));
    }
    Ok(vec![user_headers, election_headers].concat())
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
    let headers = get_headers(&elections, &attributes)?;

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
        let headers = get_headers(&None, &attributes).unwrap();
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
        let headers = get_headers(&None, &attributes).unwrap();
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
        let headers = get_headers(&None, &attributes).unwrap();
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

    fn election(id: &str, external_id: Option<&str>) -> ElectionHead {
        ElectionHead {
            id: id.to_string(),
            name: "General Election".to_string(),
            alias: None,
            external_id: external_id.map(str::to_string),
        }
    }

    fn election_columns(elections: Vec<ElectionHead>) -> Result<Vec<String>> {
        let user_columns = get_headers(&None, &vec![])?.len();
        Ok(get_headers(&Some(elections), &vec![])?[user_columns..].to_vec())
    }

    fn decode(header: &str) -> String {
        let encoded = header
            .strip_prefix(ELECTION_EXTERNAL_ID_HEADER_PREFIX)
            .expect("participation prefix");
        String::from_utf8(hex::decode(encoded).expect("hex")).expect("utf-8")
    }

    #[test]
    fn elections_with_the_same_name_are_told_apart_by_external_id() {
        // Regression for meta#14667: two elections named "General Election"
        // both became `election__General_Election`, and import refused the file.
        assert_eq!(
            election_columns(vec![
                election("1", Some("GE-2026")),
                election("2", Some("GE-2027")),
            ])
            .unwrap(),
            [
                "election__external_id__47452d32303236",
                "election__external_id__47452d32303237",
            ]
        );
    }

    #[test]
    fn raw_external_ids_round_trip_through_the_header() {
        let raw = [
            "Élection «Nord» 選挙",
            "with \"quotes\", a comma",
            "line\nbreak",
            "  padded  ",
            "3f6c1d2e-8a4b-4c1e-9f00-1b2c3d4e5f60",
        ];
        let headers = election_columns(
            raw.iter()
                .enumerate()
                .map(|(at, id)| election(&at.to_string(), Some(id)))
                .collect(),
        )
        .unwrap();
        assert_eq!(
            headers[4],
            "election__external_id__\
             33663663316432652d386134622d346331652d396630302d316232633364346535663630"
        );
        for (header, id) in headers.iter().zip(raw) {
            assert_eq!(decode(header), id);
            assert!(header
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
        }
    }

    #[test]
    fn ids_differing_only_by_case_or_punctuation_stay_distinct() {
        let raw = [
            "A-B", "A_B", "A.B", "A B", "a-b", "Board", "board", "X", " X",
        ];
        let headers = election_columns(
            raw.iter()
                .enumerate()
                .map(|(at, id)| election(&at.to_string(), Some(id)))
                .collect(),
        )
        .unwrap();
        assert_eq!(headers[0], "election__external_id__412d42");
        assert_eq!(headers[1], "election__external_id__415f42");
        assert_eq!(headers[5], "election__external_id__426f617264");
        assert_eq!(headers[6], "election__external_id__626f617264");
        assert_eq!(headers[8], "election__external_id__2058");
        let import_keys = headers
            .iter()
            .map(|header| import_column_key(header))
            .collect::<HashSet<_>>();
        assert_eq!(import_keys.len(), raw.len());
    }

    #[test]
    fn missing_or_blank_external_ids_stop_the_export() {
        for external_id in [None, Some(""), Some("   "), Some("\t\n")] {
            let error = election_columns(vec![
                election("first", Some("GE-2026")),
                election("second", external_id),
            ])
            .expect_err("an election without an external ID must be refused");
            let message = format!("{error:?}");
            assert!(message.contains("second"), "{message}");
            assert!(message.contains("no external ID"), "{message}");
        }
    }

    #[test]
    fn a_repeated_external_id_stops_the_export() {
        let error = election_columns(vec![
            election("first", Some("GE-2026")),
            election("second", Some("GE-2027")),
            election("third", Some("GE-2026")),
        ])
        .expect_err("a repeated external ID must be refused");
        let message = format!("{error:?}");
        assert!(
            message.contains("first") && message.contains("third"),
            "{message}"
        );
        assert!(message.contains("GE-2026"), "{message}");
    }

    #[test]
    fn a_voter_attribute_named_like_a_participation_column_stops_the_export() {
        for name in [
            "election__external_id__6e",
            "ELECTION__EXTERNAL_ID__6E",
            "election--external.id__6e",
        ] {
            let error = get_headers(
                &Some(vec![election("north", Some("n"))]),
                &vec![attribute(name)],
            )
            .expect_err("a colliding attribute must be refused");
            assert!(format!("{error:?}").contains(name));
        }
        assert!(get_headers(
            &Some(vec![election("north", Some("n"))]),
            &vec![attribute("election__external_id__6f")],
        )
        .is_ok());
    }

    #[test]
    fn participation_values_follow_their_own_election_column() {
        let elections = Some(vec![
            election("first", Some("GE-2026")),
            election("second", Some("GE-2027")),
        ]);
        let user = User {
            votes_info: Some(vec![sequent_core::types::keycloak::VotesInfo {
                election_id: "second".into(),
                num_votes: 1,
                last_voted_at: String::new(),
            }]),
            ..Default::default()
        };
        let headers = get_headers(&elections, &vec![]).unwrap();
        let record = get_user_record(&elections, &None, &user, &vec![]);
        assert_eq!(headers.len(), record.len());
        let value = |external_id: &str| {
            let at = headers
                .iter()
                .position(|header| *header == election_header(external_id))
                .unwrap();
            record[at].clone()
        };
        assert_eq!(value("GE-2026"), "");
        assert_eq!(value("GE-2027"), "true");
    }

    #[test]
    fn exports_without_elections_keep_their_columns() {
        let attributes = vec![attribute("custom_attribute")];
        let expected = [
            "id",
            "email",
            "email_verified",
            "enabled",
            "first_name",
            "last_name",
            "username",
            "area_name",
            "custom_attribute",
        ];
        // Tenant-wide exports have no elections at all.
        assert_eq!(get_headers(&None, &attributes).unwrap(), expected);
        // An event export with no authorized election adds no columns and
        // validates nothing.
        assert_eq!(get_headers(&Some(vec![]), &attributes).unwrap(), expected);
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
            // Only the authorized election needs an external ID: the others are
            // dropped before the export validates or names anything.
            external_id: (id == "north").then(|| "NORTH-1".into()),
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
    let headers = get_headers(&elections, &vec![]).unwrap();
    let record = get_user_record(&elections, &None, &user, &vec![]);
    assert_eq!(headers.last().unwrap(), &election_header("NORTH-1"));
    assert_eq!(record.last().unwrap(), "");
    assert_eq!(
        1,
        headers
            .iter()
            .filter(|value| value.starts_with("election__"))
            .count()
    );
    let empty = Some(scoped_export_elections(elections.unwrap(), &[]));
    assert!(!get_headers(&empty, &vec![])
        .unwrap()
        .iter()
        .any(|value| value.starts_with("election__")));
}
