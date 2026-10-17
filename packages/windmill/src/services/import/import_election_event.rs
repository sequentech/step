// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::application::insert_applications;
use crate::postgres::election_event::{get_election_event_by_id_if_exist, update_bulletin_board};
use crate::postgres::reports::insert_reports;
use crate::postgres::reports::Report;
use crate::postgres::trustee::get_all_trustees;
use crate::services::import::import_publications::import_ballot_publications;
use crate::services::import::import_scheduled_events::import_scheduled_events;
use crate::services::import::import_tally::process_tally_file;
use crate::services::protocol_manager::get_event_board;
use crate::services::reports::template_renderer::EReportEncryption;
use crate::services::reports_vault::get_report_key_pair;
use crate::services::tasks_execution::update_fail;
use crate::tasks::insert_election_event::CreateElectionEventInput;
use crate::types::documents::ETallyDocuments;
use ::keycloak::types::{
    ComponentExportRepresentation, GroupRepresentation, IdentityProviderMapperRepresentation,
    ProtocolMapperRepresentation, RealmRepresentation,
};
use anyhow::{anyhow, Context, Result};
use chrono::format;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::future::try_join_all;
use once_cell::sync::Lazy;
use sequent_core::ballot::AllowTallyStatus;
use sequent_core::ballot::ElectionEventStatistics;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::ElectionStatistics;
use sequent_core::ballot::ElectionStatus;
use sequent_core::ballot::PeriodDates;
use sequent_core::ballot::VotingPeriodDates;
use sequent_core::ballot::VotingStatus;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::{get_client_credentials, KeycloakAdminClient};
use sequent_core::services::replace_uuids::replace_uuids;
use sequent_core::types::hasura::core::Application;
use sequent_core::types::hasura::core::AreaContest;
use sequent_core::types::hasura::core::Document;
use sequent_core::types::hasura::core::KeysCeremony;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::mime::{get_mime_types, matches_mime};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Cursor;
use std::io::Seek;
use std::io::{self, Read, Write};
use std::path::Path;
use std::str::FromStr;
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;
use zip::read::ZipArchive;

use super::import_users::import_users_file;
use crate::postgres;
use crate::postgres::area::insert_areas;
use crate::postgres::area_contest::insert_area_contests;
use crate::postgres::candidate::insert_candidates;
use crate::postgres::contest::insert_contest;
use crate::postgres::election::insert_elections;
use crate::postgres::election_event::insert_election_event;
use crate::postgres::keys_ceremony;
use crate::postgres::scheduled_event::insert_scheduled_event;
use crate::services::consolidation::aes_256_cbc_encrypt::decrypt_file_aes_256_cbc;
use crate::services::documents;
use crate::services::documents::upload_and_return_document;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_board::BoardSerializable;
use crate::services::electoral_log::ElectoralLog;
use crate::services::import::import_bulletin_boards::*;
use crate::services::jwks::upsert_realm_jwks;
use crate::services::protocol_manager::get_election_board;
use crate::services::protocol_manager::get_protocol_manager_secret_path;
use crate::services::protocol_manager::{
    create_protocol_manager_keys, get_b3_pgsql_client, get_board_client,
};
use crate::tasks::import_election_event::ImportElectionEventBody;
use crate::types::documents::EDocuments;
use regex::Regex;
use sequent_core::types::hasura::core::{Area, Candidate, Contest, Election, ElectionEvent};
use sequent_core::types::scheduled_event::*;
use sequent_core::util::temp_path::{generate_temp_file, get_file_size};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportElectionEventSchema {
    pub tenant_id: Uuid,
    pub keycloak_event_realm: Option<RealmRepresentation>,
    pub election_event: ElectionEvent,
    pub elections: Vec<Election>,
    pub contests: Vec<Contest>,
    pub candidates: Vec<Candidate>,
    pub areas: Vec<Area>,
    pub area_contests: Vec<AreaContest>,
    pub scheduled_events: Option<Vec<ScheduledEvent>>,
    pub reports: Vec<Report>,
    pub keys_ceremonies: Option<Vec<KeysCeremony>>,
    pub applications: Option<Vec<Application>>,
}

#[instrument(err)]
pub async fn upsert_b3_and_elog(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &Vec<String>,
    dont_auto_generate_keys: bool, // avoid creating protocol manager keys
) -> Result<Value> {
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);
    // FIXME must also create the electoral log board here
    let mut immudb_client = get_board_client().await?;
    immudb_client.upsert_electoral_log_db(&board_name).await?;

    let mut board_client = get_b3_pgsql_client().await?;

    // Create board and protocol manager keys for election event (assert)
    let existing: Option<b3::client::pgsql::B3IndexRow> =
        board_client.get_board(board_name.as_str()).await?;
    // insert into the index of boards
    board_client.create_index_ine().await?;
    // create board table
    board_client.create_board_ine(board_name.as_str()).await?;

    if existing.is_none() && !dont_auto_generate_keys {
        event!(
            Level::INFO,
            "creating protocol manager keys for Election event {}",
            election_event_id
        );
        create_protocol_manager_keys(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &board_name,
        )
        .await?;
    }

    // board was created, checking it is now present
    let board = board_client
        .get_board(board_name.as_str())
        .await?
        .ok_or(anyhow!(
            "Unexpected error: could not retrieve created board '{}'",
            &board_name
        ))?;

    for election_id in election_ids.clone() {
        // Create board and protocol manager keys for election (insert, not asssert)
        let board_name = get_election_board(tenant_id, &election_id, &slug);

        let existing: Option<b3::client::pgsql::B3IndexRow> =
            board_client.get_board(board_name.as_str()).await?;

        // assert board table
        board_client.create_board_ine(board_name.as_str()).await?;
        // create board table

        if existing.is_none() && !dont_auto_generate_keys {
            event!(
                Level::INFO,
                "creating protocol manager keys for election {}",
                election_id
            );
            create_protocol_manager_keys(
                hasura_transaction,
                tenant_id,
                election_event_id,
                &board_name,
            )
            .await?;
        }
        // board was created, checking it is now present
        board_client
            .get_board(board_name.as_str())
            .await?
            .ok_or(anyhow!(
                "Unexpected error: could not retrieve created board '{}'",
                &board_name
            ))?;
    }

    let board_serializable: BoardSerializable = board.into();

    let board_value = serde_json::to_value(board_serializable.clone())?;
    Ok(board_value)
}

#[instrument(err)]
pub fn read_default_election_event_realm() -> Result<RealmRepresentation> {
    let realm_config_path = env::var("KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH")
        .with_context(|| "KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH must be set")?;
    let realm_config = fs::read_to_string(&realm_config_path)
        .with_context(|| "Should have been able to read the configuration file in KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH={realm_config_path}")?;

    deserialize_str(&realm_config)
        .map_err(|err| anyhow!("Error parsing KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH into RealmRepresentation: {err}"))
}

/// Claim namespace that Keycloak protocol mappers use to emit Hasura session
/// variables (roles and tenant). A mapper writing under this namespace decides
/// which roles and tenant a token is accepted with, so only the platform
/// template may define these on an imported event realm.
const HASURA_CLAIM_NAMESPACE: &str = "https://hasura.io/jwt/claims";
/// Protocol-mapper config key holding the claim a mapper writes.
const PROTOCOL_MAPPER_CLAIM_NAME_KEY: &str = "claim.name";
/// Prefix of the per-realm composite role Keycloak generates for every realm.
const DEFAULT_REALM_ROLE_PREFIX: &str = "default-roles-";
/// How an uploaded election-event realm's token-claim authority (realm roles,
/// Hasura claim mappers and identity-provider mappers) is reconciled against
/// the platform template before the realm is applied to Keycloak. Modeled as
/// an enum so further reconciliation modes can be added without touching call
/// sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportedRealmReconciliation {
    /// Keep only the token-claim authority the platform template defines and
    /// log every element removed from the uploaded realm. Platform exports
    /// contain only template elements, so they are applied unchanged.
    #[default]
    AlignWithTemplate,
    /// Reject the import when the uploaded realm carries token-claim authority
    /// the platform template does not define, instead of removing it.
    RejectForeignClaims,
}

/// True when a protocol mapper writes a Hasura session variable or runs a
/// script, i.e. it can set the roles or tenant a token carries.
fn is_claim_authority_mapper(mapper: &ProtocolMapperRepresentation) -> bool {
    if mapper
        .protocol_mapper
        .as_deref()
        .is_some_and(|kind| kind.contains("script"))
    {
        return true;
    }
    mapper
        .config
        .as_ref()
        .and_then(|config| config.get(PROTOCOL_MAPPER_CLAIM_NAME_KEY))
        // Keycloak stores the namespace with the dot escaped
        // (`https://hasura\.io/jwt/claims`); compare without backslashes.
        .is_some_and(|claim_name| {
            claim_name
                .replace('\\', "")
                .starts_with(HASURA_CLAIM_NAMESPACE)
        })
}

/// A protocol mapper compared by its effect rather than its per-realm id.
fn claim_mapper_key(
    mapper: &ProtocolMapperRepresentation,
) -> (String, String, BTreeMap<String, String>) {
    (
        mapper.name.clone().unwrap_or_default(),
        mapper.protocol_mapper.clone().unwrap_or_default(),
        mapper
            .config
            .clone()
            .map(|config| config.into_iter().collect())
            .unwrap_or_default(),
    )
}

/// A realm role is allowed on an imported realm when the platform template
/// defines it, or it is the realm's own default composite.
fn realm_role_allowed(name: &str, allowed: &HashSet<String>) -> bool {
    allowed.contains(name)
}

/// Client-role assignments of a user or group, keyed by client id.
type ClientRoleAssignments = HashMap<String, Vec<String>>;

/// Keep only the client-role assignments the template gives the same subject.
fn retain_template_client_roles(
    assigned: &mut Option<ClientRoleAssignments>,
    allowed: Option<&ClientRoleAssignments>,
    subject: &str,
    discarded: &mut Vec<String>,
) {
    let Some(assigned_roles) = assigned.as_mut() else {
        return;
    };
    for (client_id, roles) in assigned_roles.iter_mut() {
        let allowed_roles = allowed.and_then(|allowed| allowed.get(client_id));
        roles.retain(|role| {
            let keep = allowed_roles.is_some_and(|allowed| allowed.contains(role));
            if !keep {
                discarded.push(format!(
                    "client role '{role}' of client '{client_id}' assigned to {subject}"
                ));
            }
            keep
        });
    }
    assigned_roles.retain(|_, roles| !roles.is_empty());
}

/// Client-role assignments of the template's groups, keyed by group name.
fn collect_group_client_roles(
    groups: &[GroupRepresentation],
    collected: &mut HashMap<String, ClientRoleAssignments>,
) {
    for group in groups {
        if let (Some(name), Some(client_roles)) = (group.name.clone(), group.client_roles.clone()) {
            collected.insert(name, client_roles);
        }
        if let Some(sub_groups) = group.sub_groups.as_ref() {
            collect_group_client_roles(sub_groups, collected);
        }
    }
}

/// Drop every realm-role and client-role reference a group (and its
/// subgroups) makes that the template does not define.
fn strip_group_roles(
    group: &mut GroupRepresentation,
    allowed: &HashSet<String>,
    template_client_roles: &HashMap<String, ClientRoleAssignments>,
    discarded: &mut Vec<String>,
) {
    let group_name = group
        .name
        .clone()
        .unwrap_or_else(|| "<unknown>".to_string());
    if let Some(roles) = group.realm_roles.as_mut() {
        roles.retain(|name| {
            let keep = realm_role_allowed(name, allowed);
            if !keep {
                discarded.push(format!(
                    "realm role '{name}' assigned to group '{group_name}'"
                ));
            }
            keep
        });
    }
    retain_template_client_roles(
        &mut group.client_roles,
        template_client_roles.get(&group_name),
        &format!("group '{group_name}'"),
        discarded,
    );
    if let Some(sub_groups) = group.sub_groups.as_mut() {
        for sub_group in sub_groups.iter_mut() {
            strip_group_roles(sub_group, allowed, template_client_roles, discarded);
        }
    }
}

/// Reconcile the claim-authority protocol mappers of one client or client
/// scope: keep those the template defines for it, restore the ones missing and
/// drop every other.
fn reconcile_claim_mappers(
    existing: Vec<ProtocolMapperRepresentation>,
    template_mappers: Option<&Vec<ProtocolMapperRepresentation>>,
    owner: &str,
    discarded: &mut Vec<String>,
) -> Vec<ProtocolMapperRepresentation> {
    let (claim_mappers, mut kept): (Vec<_>, Vec<_>) = existing
        .into_iter()
        .partition(|mapper| is_claim_authority_mapper(mapper));
    let template_mappers = template_mappers.map(Vec::as_slice).unwrap_or_default();
    let template_keys: HashSet<_> = template_mappers.iter().map(claim_mapper_key).collect();
    let mut present_keys: HashSet<_> = HashSet::new();
    for mapper in claim_mappers {
        let key = claim_mapper_key(&mapper);
        if template_keys.contains(&key) {
            present_keys.insert(key);
            kept.push(mapper);
        } else {
            discarded.push(format!(
                "claim mapper '{}' on {owner}",
                mapper.name.as_deref().unwrap_or("<unnamed>")
            ));
        }
    }
    for template_mapper in template_mappers {
        if !present_keys.contains(&claim_mapper_key(template_mapper)) {
            let mut restored = template_mapper.clone();
            restored.id = None;
            kept.push(restored);
        }
    }
    kept
}

/// Key of an identity-provider mapper compared by its effect, not its id.
fn idp_mapper_key(
    mapper: &IdentityProviderMapperRepresentation,
) -> (String, String, String, BTreeMap<String, String>) {
    (
        mapper.identity_provider_alias.clone().unwrap_or_default(),
        mapper.identity_provider_mapper.clone().unwrap_or_default(),
        mapper.name.clone().unwrap_or_default(),
        mapper
            .config
            .clone()
            .map(|config| config.into_iter().collect())
            .unwrap_or_default(),
    )
}

/// Reconcile an uploaded election-event realm against the platform template so
/// that only the template's realm roles, Hasura claim mappers and
/// identity-provider mappers survive. This prevents an imported realm from
/// granting roles (such as `service-account` or `admin-user`) or minting
/// Hasura claims that the platform never intended an event realm to carry.
#[instrument(skip(imported, template))]
fn sanitize_imported_event_realm(
    imported: &RealmRepresentation,
    template: &RealmRepresentation,
    reconciliation: ImportedRealmReconciliation,
) -> Result<RealmRepresentation> {
    let mut realm = imported.clone();
    let mut discarded: Vec<String> = Vec::new();

    let default_role_name = |realm: &RealmRepresentation| -> Option<String> {
        realm
            .default_role
            .as_ref()
            .and_then(|role| role.name.clone())
            .filter(|name| name.starts_with(DEFAULT_REALM_ROLE_PREFIX))
    };
    let template_default_role = template.default_role.clone();
    let imported_default_role_name = default_role_name(imported);

    let mut allowed_realm_roles: HashSet<String> = template
        .roles
        .as_ref()
        .and_then(|roles| roles.realm.as_ref())
        .map(|realm_roles| {
            realm_roles
                .iter()
                .filter_map(|role| role.name.clone())
                .collect()
        })
        .unwrap_or_default();
    allowed_realm_roles.extend(imported_default_role_name.clone());
    allowed_realm_roles.extend(default_role_name(template));

    // Role definitions come from the template, so an allowed role cannot carry
    // composites the template does not give it.
    let mut template_composites: HashMap<String, Option<_>> = template
        .roles
        .as_ref()
        .and_then(|roles| roles.realm.as_ref())
        .map(|realm_roles| {
            realm_roles
                .iter()
                .filter_map(|role| Some((role.name.clone()?, role.composites.clone())))
                .collect()
        })
        .unwrap_or_default();
    if let Some(name) = imported_default_role_name {
        template_composites.insert(name, template_default_role.and_then(|role| role.composites));
    }

    // 1. Realm roles: keep only the template's roles and the default composite.
    if let Some(roles) = realm.roles.as_mut() {
        if let Some(realm_roles) = roles.realm.as_mut() {
            realm_roles.retain(|role| {
                let name = role.name.as_deref().unwrap_or_default();
                let keep = realm_role_allowed(name, &allowed_realm_roles);
                if !keep {
                    discarded.push(format!("realm role '{name}'"));
                }
                keep
            });
            for role in realm_roles.iter_mut() {
                let name = role.name.clone().unwrap_or_default();
                role.composites = template_composites.get(&name).cloned().flatten();
            }
        }
    }

    // 2. Default role composites.
    if let Some(default_role) = realm.default_role.as_mut() {
        let name = default_role.name.clone().unwrap_or_default();
        default_role.composites = template_composites.get(&name).cloned().flatten();
    }

    let template_user_client_roles: HashMap<String, ClientRoleAssignments> = template
        .users
        .iter()
        .flatten()
        .filter_map(|user| Some((user.username.clone()?, user.client_roles.clone()?)))
        .collect();
    let mut template_group_client_roles = HashMap::new();
    collect_group_client_roles(
        template.groups.as_deref().unwrap_or_default(),
        &mut template_group_client_roles,
    );

    // 3. Role assignments on users.
    if let Some(users) = realm.users.as_mut() {
        for user in users.iter_mut() {
            let username = user
                .username
                .clone()
                .unwrap_or_else(|| "<unknown>".to_string());
            retain_template_client_roles(
                &mut user.client_roles,
                template_user_client_roles.get(&username),
                &format!("user '{username}'"),
                &mut discarded,
            );
            if let Some(user_roles) = user.realm_roles.as_mut() {
                user_roles.retain(|name| {
                    let keep = realm_role_allowed(name, &allowed_realm_roles);
                    if !keep {
                        discarded
                            .push(format!("realm role '{name}' assigned to user '{username}'"));
                    }
                    keep
                });
            }
        }
    }

    // 4. Role assignments on groups.
    if let Some(groups) = realm.groups.as_mut() {
        for group in groups.iter_mut() {
            strip_group_roles(
                group,
                &allowed_realm_roles,
                &template_group_client_roles,
                &mut discarded,
            );
        }
    }

    // 5. Scope mappings that reference dropped roles.
    if let Some(scope_mappings) = realm.scope_mappings.as_mut() {
        for mapping in scope_mappings.iter_mut() {
            if let Some(roles) = mapping.roles.as_mut() {
                roles.retain(|name| realm_role_allowed(name, &allowed_realm_roles));
            }
        }
    }

    // 6. Claim mappers on clients and client scopes: each keeps only the
    //    template's Hasura mappers for the client or scope of that name.
    let template_client_claim_mappers: HashMap<String, Vec<ProtocolMapperRepresentation>> =
        template
            .clients
            .iter()
            .flatten()
            .filter_map(|client| {
                Some((client.client_id.clone()?, client.protocol_mappers.clone()?))
            })
            .collect();
    let template_scope_claim_mappers: HashMap<String, Vec<ProtocolMapperRepresentation>> = template
        .client_scopes
        .iter()
        .flatten()
        .filter_map(|scope| Some((scope.name.clone()?, scope.protocol_mappers.clone()?)))
        .collect();

    if let Some(clients) = realm.clients.as_mut() {
        for client in clients.iter_mut() {
            let client_id = client.client_id.clone().unwrap_or_default();
            let existing = client.protocol_mappers.take().unwrap_or_default();
            client.protocol_mappers = Some(reconcile_claim_mappers(
                existing,
                template_client_claim_mappers.get(&client_id),
                &format!("client '{client_id}'"),
                &mut discarded,
            ));
        }
    }
    if let Some(client_scopes) = realm.client_scopes.as_mut() {
        for scope in client_scopes.iter_mut() {
            let scope_name = scope.name.clone().unwrap_or_default();
            let existing = scope.protocol_mappers.take().unwrap_or_default();
            scope.protocol_mappers = Some(reconcile_claim_mappers(
                existing,
                template_scope_claim_mappers.get(&scope_name),
                &format!("client scope '{scope_name}'"),
                &mut discarded,
            ));
        }
    }

    // 7. Identity-provider mappers: only the ones the template defines survive,
    //    as any of them can grant roles or set attributes the claims carry.
    let template_idp_mapper_keys: HashSet<_> = template
        .identity_provider_mappers
        .iter()
        .flatten()
        .map(idp_mapper_key)
        .collect();
    if let Some(idp_mappers) = realm.identity_provider_mappers.as_mut() {
        idp_mappers.retain(|mapper| {
            let keep = template_idp_mapper_keys.contains(&idp_mapper_key(mapper));
            if !keep {
                discarded.push(format!(
                    "identity-provider mapper '{}'",
                    mapper.name.as_deref().unwrap_or("<unnamed>")
                ));
            }
            keep
        });
    }

    if !discarded.is_empty() {
        match reconciliation {
            ImportedRealmReconciliation::RejectForeignClaims => {
                return Err(anyhow!(
                    "Imported election-event realm carries token-claim \
                     elements not defined by the platform template: {}",
                    discarded.join(", ")
                ));
            }
            ImportedRealmReconciliation::AlignWithTemplate => {
                tracing::warn!(
                    "Removed {} token-claim element(s) from the imported \
                     election-event realm not defined by the platform \
                     template: {}",
                    discarded.len(),
                    discarded.join(", ")
                );
            }
        }
    }

    Ok(realm)
}

#[instrument(skip(realm))]
pub fn remove_keycloak_realm_secrets(realm: &RealmRepresentation) -> Result<RealmRepresentation> {
    // set a specific client secret for a specific client id by env config
    let client_id = env::var("KEYCLOAK_CLIENT_ID").with_context(|| "missing KEYCLOAK_CLIENT_ID")?;
    let client_secret =
        env::var("KEYCLOAK_CLIENT_SECRET").with_context(|| "missing KEYCLOAK_CLIENT_SECRET")?;
    // we remove secrets and certs so that keycloak regenerates them
    // remove client secrets
    let mut realm_copy = realm.clone();
    realm_copy.clients = realm_copy.clients.map(|clients| {
        clients
            .iter()
            .map(|client| {
                let mut client_copy = client.clone();
                if client.client_id == Some(client_id.clone()) {
                    client_copy.secret = Some(client_secret.clone());
                } else {
                    client_copy.secret = None;
                }
                client_copy
            })
            .collect()
    });
    // remove certificates, only leaving their algorithm/priority
    let valid_keys: Vec<String> = vec!["priority".to_string(), "algorithm".to_string()];
    if let Some(components) = realm_copy.components.clone() {
        let mut newcomponents = components.clone();
        let key: &'static str = "org.keycloak.keys.KeyProvider";
        if let Some(val) = components.get(key) {
            let newval: Vec<ComponentExportRepresentation> = val
                .iter()
                .map(|el| {
                    let mut elnew = el.clone();
                    if let Some(config) = elnew.config.clone() {
                        let mut newconfig = config.clone();
                        for k in config.keys() {
                            if !valid_keys.contains(&k) {
                                info!("Removing key {} from {}", k, key);
                                newconfig.remove(k);
                            }
                        }
                        elnew.config = Some(newconfig);
                    }
                    elnew
                })
                .collect();
            newcomponents.insert(key.to_string(), newval.clone());
        }
        realm_copy.components = Some(newcomponents);
    }
    Ok(realm_copy)
}

#[instrument(err, skip(keycloak_event_realm))]
pub async fn upsert_keycloak_realm(
    tenant_id: &str,
    election_event_id: &str,
    keycloak_event_realm: Option<RealmRepresentation>,
) -> Result<()> {
    let mut realm = if let Some(uploaded) = keycloak_event_realm.clone() {
        let template = read_default_election_event_realm()?;
        sanitize_imported_event_realm(&uploaded, &template, ImportedRealmReconciliation::default())?
    } else {
        let realm = read_default_election_event_realm()?;
        realm
    };
    realm = remove_keycloak_realm_secrets(&realm)?;
    let realm_config = serde_json::to_string(&realm)?;
    let client = KeycloakAdminClient::new().await?;
    let realm_name = get_event_realm(tenant_id, election_event_id);
    client
        .upsert_realm(
            realm_name.as_str(),
            &realm_config,
            tenant_id,
            keycloak_event_realm.is_none(),
            None,
            Some(election_event_id.to_string()),
        )
        .await?;
    upsert_realm_jwks(realm_name.as_str()).await?;
    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_election_event_db(
    hasura_transaction: &Transaction<'_>,
    object: &CreateElectionEventInput,
) -> Result<()> {
    let election_event_id = object
        .id
        .clone()
        .ok_or(anyhow!("Empty election event id"))?;
    let tenant_id = object.tenant_id.clone();
    // fetch election_event
    let found_election_event = get_election_event_by_id_if_exist(
        hasura_transaction,
        &tenant_id.clone(),
        &election_event_id.clone(),
    )
    .await?;

    if found_election_event.is_some() {
        event!(
            Level::INFO,
            "Election event {} for tenant {} already exists",
            election_event_id,
            tenant_id
        );
        return Ok(());
    }

    let new_election_input = ElectionEvent {
        id: election_event_id.clone(),
        tenant_id: object.tenant_id.clone(),
        name: object.name.clone(),
        description: object.description.clone(),
        public_key: object.public_key.clone(),
        status: object.status.clone(),
        created_at: None,
        updated_at: None,
        labels: object.labels.clone(),
        annotations: object.annotations.clone(),
        presentation: object.presentation.clone(),
        bulletin_board_reference: object.bulletin_board_reference.clone(),
        is_archived: object.is_archived.unwrap_or(false),
        voting_channels: object.voting_channels.clone(),
        user_boards: object.user_boards.clone(),
        encryption_protocol: object
            .encryption_protocol
            .clone()
            .unwrap_or("RSA256".to_string()),
        is_audit: object.is_audit.clone(),
        audit_election_event_id: object.audit_election_event_id.clone(),
        alias: object.alias.clone(),
        statistics: Some(json!({
            "num_emails_sent": 0,
            "num_sms_sent": 0
        })),
    };

    insert_election_event(&hasura_transaction, &new_election_input).await?;
    Ok(())
}

#[instrument(err, skip(data_str, original_data))]
pub fn replace_ids(
    data_str: &str,
    original_data: &ImportElectionEventSchema,
    id_opt: Option<String>,
    tenant_id: String,
) -> Result<(ImportElectionEventSchema, HashMap<String, String>)> {
    let mut keep: Vec<String> = vec![];
    keep.push(original_data.tenant_id.clone().to_string());
    if id_opt.is_some() {
        keep.push(original_data.election_event.id.clone());
    }
    // find other ids to maintain
    if let Some(realm) = original_data.keycloak_event_realm.clone() {
        if let Some(authenticator_configs) = realm.authenticator_config.clone() {
            for authenticator_config in authenticator_configs {
                let Some(config) = authenticator_config.config.clone() else {
                    continue;
                };
                for (_key, value) in config {
                    if Uuid::parse_str(&value).is_ok() {
                        keep.push(value.clone());
                    }
                }
            }
        }
    }

    let (mut new_data, replacement_map) = replace_uuids(data_str, keep);

    if let Some(id) = id_opt {
        new_data = new_data.replace(&original_data.election_event.id, &id);
    }
    if original_data.tenant_id.to_string() != tenant_id {
        new_data = new_data.replace(&original_data.tenant_id.to_string(), &tenant_id);
    }

    let data: ImportElectionEventSchema = deserialize_str(&new_data)?;
    Ok((data, replacement_map))
}

#[instrument(err, skip_all)]
pub async fn get_document(
    hasura_transaction: &Transaction<'_>,
    object: ImportElectionEventBody,
    election_event_id: Option<String>,
) -> Result<(NamedTempFile, Document, String)> {
    let document = postgres::document::get_document(
        hasura_transaction,
        &object.tenant_id,
        None,
        &object.document_id,
    )
    .await?
    .ok_or(anyhow!(
        "Error trying to get document id {}: not found",
        &object.document_id
    ))?;

    let mut temp_file = documents::get_document_as_temp_file(&object.tenant_id, &document)
        .await
        .map_err(|err| anyhow!("Error trying to get document as temporary file {err}"))?;

    let document_type = document
        .clone()
        .media_type
        .unwrap_or("application/ezip".to_string());

    temp_file = decrypt_document(object.password.clone(), temp_file)
        .await
        .map_err(|err| anyhow!("error decrypting document {:?}", document.id))?;

    Ok((temp_file, document, document_type))
}

#[instrument(err, skip_all)]
pub async fn decrypt_document(
    password: Option<String>,
    mut temp_file_path: NamedTempFile,
) -> Result<NamedTempFile> {
    let password = password.unwrap_or_else(|| "".to_string());
    let is_encrypted = !password.is_empty();

    if is_encrypted {
        let decrypted_path = env::temp_dir().join("election-event.zip");

        decrypt_file_aes_256_cbc(
            &temp_file_path.path().to_string_lossy().to_string(),
            &decrypted_path.as_path().to_string_lossy().to_string(),
            &password,
        )
        .map_err(|err| anyhow!("Error generating decrypted file"))?;

        // Create a new NamedTempFile for the decrypted content
        let mut temp_file = NamedTempFile::new()?;
        let content = fs::read(decrypted_path)?;
        temp_file.write_all(&content)?;

        return Ok(temp_file);
    }

    Ok(temp_file_path)
}

#[instrument(err, skip_all)]
pub async fn get_election_event_schema(
    data_str: &str,
    id: Option<String>,
    tenant_id: String,
) -> Result<(ImportElectionEventSchema, HashMap<String, String>)> {
    let original_data: ImportElectionEventSchema = deserialize_str(data_str)?;
    replace_ids(data_str, &original_data, id, tenant_id.clone())
}

#[instrument(err, skip_all)]
pub async fn process_election_event_file(
    hasura_transaction: &Transaction<'_>,
    document_type: &String,
    file_election_event_schema: &str,
    object: ImportElectionEventBody,
    election_event_id: String,
    tenant_id: String,
    is_importing_keys: bool,
) -> Result<(ImportElectionEventSchema, HashMap<String, String>)> {
    let (mut data, replacement_map) = get_election_event_schema(
        file_election_event_schema,
        Some(election_event_id.clone()),
        tenant_id.clone(),
    )
    .await
    .with_context(|| format!("Error getting document for election event ID {election_event_id} and tenant ID {tenant_id}"))?;

    let election_ids: Vec<String> = data
        .elections
        .clone()
        .into_iter()
        .map(|election| election.id.clone())
        .collect();

    data.election_event.public_key = None;
    data.election_event.statistics = Some(
        serde_json::to_value(ElectionEventStatistics::default())
            .with_context(|| "Error serializing election event statistics")?,
    );

    data.election_event.status = Some(
        serde_json::to_value(ElectionEventStatus::default())
            .with_context(|| "Error serializing election event status")?,
    );

    // Process elections
    data.elections = data
        .elections
        .into_iter()
        .map(|election| -> Result<Election> {
            let mut clone = election.clone();
            clone.statistics = Some(
                serde_json::to_value(ElectionStatistics::default())
                    .with_context(|| "Error serializing election statistics")?,
            );

            let mut status: ElectionStatus = clone
                .status
                .clone()
                .map(|value| deserialize_value::<ElectionStatus>(value))
                .transpose()
                .unwrap_or_default()
                .unwrap_or_default();

            status.voting_status = VotingStatus::default();
            status.kiosk_voting_status = VotingStatus::default();
            status.voting_period_dates = PeriodDates::default();
            status.kiosk_voting_period_dates = PeriodDates::default();

            clone.status = Some(
                serde_json::to_value(status)
                    .with_context(|| "Error serializing election status")?,
            );
            clone.initialization_report_generated = Some(false);

            Ok(clone)
        })
        .collect::<Result<Vec<Election>>>()
        .with_context(|| "Error processing elections")?;

    upsert_keycloak_realm(
        tenant_id.as_str(),
        &election_event_id,
        data.keycloak_event_realm.clone(),
    )
    .await
    .with_context(|| format!("Error upserting Keycloak realm for tenant ID {tenant_id} and election event ID {election_event_id}"))?;

    insert_election_event(hasura_transaction, &data.election_event)
        .await
        .with_context(|| "Error inserting election event")?;

    manage_dates(&data, hasura_transaction)
        .await
        .with_context(|| "Error managing dates")?;

    // Upsert immutable board
    let board = upsert_b3_and_elog(hasura_transaction, tenant_id.as_str(), &election_event_id, &election_ids, is_importing_keys)
        .await
        .with_context(|| format!("Error upserting b3 board for tenant ID {tenant_id} and election event ID {election_event_id}"))?;

    update_bulletin_board(
        hasura_transaction,
        tenant_id.as_str(),
        election_event_id.as_str(),
        &board,
    )
    .await
    .with_context(|| {
        format!(
            "Error updating bulletin board reference for tenant ID {} and election event ID {}",
            tenant_id, election_event_id
        )
    })?;

    if let Some(keys_ceremonies) = data.keys_ceremonies.clone() {
        let trustees = get_all_trustees(&hasura_transaction, &tenant_id).await?;

        let trustee_map: HashMap<String, String> = trustees
            .into_iter()
            .map(|trustee| (trustee.name.clone().unwrap_or_default(), trustee.id.clone()))
            .collect();

        try_join_all(
            keys_ceremonies
                .into_iter()
                .map(|keys_ceremony| {
                    let trustee_ids = keys_ceremony
                        .trustee_ids
                        .into_iter()
                        .map(|trustee_id| trustee_map.get(&trustee_id).cloned().unwrap_or_default())
                        .collect();

                    keys_ceremony::insert_keys_ceremony(
                        hasura_transaction,
                        keys_ceremony.id,
                        keys_ceremony.tenant_id,
                        keys_ceremony.election_event_id,
                        trustee_ids,
                        /* threshold */ keys_ceremony.threshold as i32,
                        /* status */ keys_ceremony.status,
                        /* execution_status */ keys_ceremony.execution_status,
                        keys_ceremony.name,
                        keys_ceremony.settings,
                        keys_ceremony.is_default.clone().unwrap_or_default(),
                        keys_ceremony.permission_label.unwrap_or_default(),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .await?;
    }

    insert_elections(hasura_transaction, &data)
        .await
        .with_context(|| "Error inserting election")?;

    insert_contest(hasura_transaction, &data)
        .await
        .with_context(|| "Error inserting contest")?;

    insert_candidates(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &data.candidates,
    )
    .await
    .with_context(|| "Error inserting candidates")?;

    insert_areas(hasura_transaction, &data.areas)
        .await
        .with_context(|| "Error inserting areas")?;

    insert_area_contests(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &data.area_contests,
    )
    .await
    .with_context(|| "Error inserting area contests")?;

    if let Some(applications) = data.applications.clone() {
        insert_applications(hasura_transaction, &applications)
            .await
            .with_context(|| "Error inserting applications")?;
    }

    Ok((data, replacement_map))
}

#[instrument(err, skip(hasura_transaction, temp_file))]
async fn process_voters_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    file_name: &String,
    election_event_id: Option<String>,
    tenant_id: String,
    is_admin: bool,
) -> Result<()> {
    let separator = if file_name.ends_with(".tsv") {
        b'\t'
    } else {
        b','
    };

    import_users_file(
        hasura_transaction,
        temp_file,
        separator,
        election_event_id,
        tenant_id,
        is_admin,
    )
    .await
    .map_err(|err| anyhow!("Error importing users file: {err}"))?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn process_reports_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: String,
    election_event_id: Option<String>,
    replacement_map: &HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let election_event_id =
        election_event_id.ok_or_else(|| anyhow!("Missing election event ID"))?;

    let mut reports = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;

        let report = Report {
            id: Uuid::new_v4().to_string(),
            election_event_id: election_event_id.clone(),
            tenant_id: tenant_id.clone(),
            election_id: match record.get(1) {
                None => None,
                Some(election_id) if election_id.is_empty() => None,
                Some(election_id) => Some(
                    replacement_map
                        .get(election_id)
                        .ok_or_else(|| {
                            anyhow!("Can't find election_id={election_id:?} in replacement map")
                        })?
                        .clone(),
                ),
            },
            report_type: record
                .get(2)
                .ok_or_else(|| anyhow!("Missing Report Type"))?
                .to_string(),
            template_alias: record
                .get(3)
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty()),
            cron_config: match record.get(4) {
                None => None,
                Some(cron_config_str) if cron_config_str.is_empty() => None,
                Some(cron_config_str) => deserialize_str(&cron_config_str).map_err(|err| {
                    anyhow!("Error parsing cron_config: {err:?}\nThe string: {cron_config_str}")
                })?,
            },
            encryption_policy: EReportEncryption::from_str(
                record
                    .get(5)
                    .ok_or_else(|| anyhow!("Missing encryption policy"))?,
            )
            .map_err(|err| anyhow!("Error parsing encryption_policy: {err:?}"))?,
            created_at: Utc::now(),
            permission_label: record.get(7).and_then(|permission_labels| {
                if permission_labels.is_empty() {
                    None
                } else {
                    Some(
                        permission_labels
                            .split("|")
                            .map(|label| label.to_string())
                            .collect(),
                    )
                }
            }),
        };

        if let Some(password) = record
            .get(6)
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
        {
            let cloned_report = report.clone();
            get_report_key_pair(
                hasura_transaction,
                cloned_report.tenant_id,
                cloned_report.election_event_id,
                Some(cloned_report.id),
                password,
            )
            .await
            .with_context(|| "Error creating secret for encrypted report")?;
        }

        reports.push(report);
    }

    insert_reports(
        hasura_transaction,
        tenant_id.as_str(),
        election_event_id.as_str(),
        &reports,
    )
    .await
    .with_context(|| "Error inserting reports into the database")?;

    Ok(())
}

#[instrument(err, skip(temp_file))]
async fn process_activity_logs_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    election_event_id: &str,
    tenant_id: &str,
) -> Result<()> {
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);

    let electoral_log = ElectoralLog::new(
        hasura_transaction,
        &tenant_id,
        Some(&election_event_id),
        board_name.as_str(),
    )
    .await?;
    electoral_log.import_from_csv(temp_file).await?;

    Ok(())
}

async fn extract_document_uuid(filename: &str) -> Result<Option<&str>> {
    // Regex to match the UUID after "document_"
    let re = Regex::new(
        r"document_([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})",
    )
    .ok()
    .ok_or_else(|| anyhow!("Invalid regex"))?;

    let uuid = re
        .captures(filename)
        .and_then(|caps| caps.get(1).map(|m| m.as_str()));
    Ok(uuid)
}

async fn extract_document_name(filename: &str) -> Result<Option<&str>> {
    let re = Regex::new(
        r"document_[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}_(.+)"
    )
    .ok()
    .ok_or_else(|| anyhow!("Invalid regex"))?;

    let name = re
        .captures(filename)
        .and_then(|caps| caps.get(1).map(|m| m.as_str()));
    Ok(name)
}

static UUID_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b").unwrap()
});

pub fn replace_ids_in_filename(
    file_name: &str,
    replacement_map: &HashMap<String, String>,
) -> String {
    UUID_RE
        .replace_all(file_name, |caps: &regex::Captures| {
            let id = caps.get(0).unwrap().as_str();
            replacement_map
                .get(id)
                .map(String::as_str)
                .unwrap_or(id)
                .to_owned()
        })
        .into_owned()
}

#[instrument(err, skip(hasura_transaction, temp_file_path, replacement_map))]
pub async fn process_s3_file(
    hasura_transaction: &Transaction<'_>,
    temp_file_path: &NamedTempFile,
    file_name: &str,
    election_event_id: Option<String>,
    tenant_id: String,
    replacement_map: HashMap<String, String>,
    is_public: bool,
) -> Result<()> {
    let file_path_string = temp_file_path.path().to_string_lossy().to_string();

    let file_size = get_file_size(file_path_string.as_str())
        .with_context(|| format!("Error obtaining file size for {}", file_path_string))?;

    let file_suffix = Path::new(&file_path_string)
        .extension()
        .ok_or(anyhow!("Empty extension"))?
        .to_str()
        .ok_or(anyhow!("Empty file suffix"))?;
    let document_type = get_mime_types(file_suffix)[0];

    let document_uuid = extract_document_uuid(file_name)
        .await
        .map_err(|e| anyhow!("Error extracting document UUID from filename: {e}"))?
        .ok_or_else(|| anyhow!("Error extracting document UUID as str"))?;

    let new_document_id = replacement_map
        .get(document_uuid)
        .ok_or_else(|| anyhow!("Error finding document UUID in replacement map"))?;

    let file_name = extract_document_name(file_name)
        .await
        .map_err(|e| anyhow!("Error extracting document name from filename: {e}"))?
        .ok_or_else(|| anyhow!("Error getting document name as str"))?;

    let new_file_name = replace_ids_in_filename(&file_name, &replacement_map);
    // Upload the file and return the document
    let _document = upload_and_return_document(
        hasura_transaction,
        &file_path_string.clone(),
        file_size,
        &document_type,
        &tenant_id,
        election_event_id,
        &new_file_name,
        Some(new_document_id.to_string()),
        is_public.clone(),
    )
    .await?;

    Ok(())
}

// return zip entries, and the original string of the json schema
#[instrument(err, skip(temp_file_path))]
pub async fn get_zip_entries(
    temp_file_path: NamedTempFile,
    document_type: &str,
) -> Result<(Vec<(String, Vec<u8>)>, String)> {
    let (mut zip_entries, election_event_schema) =
        if document_type == "application/ezip" || matches_mime("zip", document_type) {
            tokio::task::spawn_blocking(move || -> Result<(Vec<(String, Vec<u8>)>, String)> {
                let file = File::open(&temp_file_path)?;
                let mut zip = ZipArchive::new(file)?;
                let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

                let mut election_event_schema: Option<String> = None;
                for i in 0..zip.len() {
                    let mut file = zip.by_index(i)?;
                    let file_name = file.name().to_string();
                    if file_name.contains(EDocuments::ELECTION_EVENT.to_file_name())
                        && file_name.ends_with(".json")
                    {
                        // Regular JSON document processing
                        let mut file_str = String::new();
                        file.read_to_string(&mut file_str)?;
                        election_event_schema = Some(file_str);
                    } else {
                        let mut file_contents = Vec::new();
                        file.read_to_end(&mut file_contents)?;
                        entries.push((file_name, file_contents));
                    }
                }
                if let Some(schema_str) = election_event_schema {
                    Ok((entries, schema_str))
                } else {
                    Err(anyhow!("No JSON file found in ZIP"))
                }
            })
            .await??
        } else {
            // Regular JSON document processing
            let mut file = File::open(temp_file_path)?;
            let mut data_str = String::new();
            file.read_to_string(&mut data_str)?;
            (vec![], data_str)
        };

    // Sort the ZIP entries by importance:
    // 1. Protocol Manager keys are imported first (rank 0)
    // 2. Regular files come next (rank 1)
    // 3. Inside the TALLY directory:
    //    - TALLY_SESSION and RESULTS_EVENT files are imported just before others (rank 2)
    //    - All other TALLY files come last (rank 3)
    zip_entries.sort_by_key(|(file_name, _)| {
        let rank = if file_name.contains(EDocuments::PROTOCOL_MANAGER_KEYS.to_file_name()) {
            0
        } else if file_name.contains(EDocuments::TALLY.to_file_name()) {
            if file_name.contains(ETallyDocuments::TALLY_SESSION.to_file_name())
                || file_name.contains(ETallyDocuments::RESULTS_EVENT.to_file_name())
            {
                2
            } else {
                3
            }
        } else {
            1
        };

        (rank, file_name.clone()) // rank first, then alphabetically within rank
    });

    Ok((zip_entries, election_event_schema))
}

#[instrument(err, skip_all)]
pub async fn process_document(
    hasura_transaction: &Transaction<'_>,
    object: ImportElectionEventBody,
    election_event_id: String,
    tenant_id: String,
) -> Result<()> {
    let (temp_file_path, document, document_type) = get_document(
        hasura_transaction,
        object.clone(),
        Some(election_event_id.clone()),
    )
    .await
    .map_err(|err| anyhow!("Failed to get document: {err}"))?;

    let (zip_entries, file_election_event_schema) =
        get_zip_entries(temp_file_path, &document_type).await?;

    let is_importing_keys = zip_entries.iter().any(|(file_name, _)| {
        file_name.contains(&format!(
            "{}",
            EDocuments::PROTOCOL_MANAGER_KEYS.to_file_name()
        ))
    });

    let election_event_id_clone = election_event_id.clone();

    let tally_session_file = zip_entries
        .iter()
        .find(|(name, _)| name.contains(ETallyDocuments::TALLY_SESSION.to_file_name()));
    let results_event_file = zip_entries
        .iter()
        .find(|(name, _)| name.contains(ETallyDocuments::RESULTS_EVENT.to_file_name()));

    let mut tally_files_content: Option<String> = None;
    if let (Some(tally_session_file), Some(results_event_file)) =
        (tally_session_file, results_event_file)
    {
        let tally_session_file_content = String::from_utf8(tally_session_file.1.clone())?;
        let results_event_file_content = String::from_utf8(results_event_file.1.clone())?;
        tally_files_content = Some(format!(
            "\n{}\n{}",
            tally_session_file_content, results_event_file_content
        ));
    }
    let file_election_event_schema = match tally_files_content {
        Some(tally_files_content) => {
            format!("{}\n{}", file_election_event_schema, tally_files_content)
        }
        None => file_election_event_schema,
    };

    let (election_event_schema, replacement_map) = process_election_event_file(
        hasura_transaction,
        &document_type,
        &file_election_event_schema,
        object,
        election_event_id.clone(),
        tenant_id.clone(),
        is_importing_keys,
    )
    .await
    .map_err(|err| anyhow!("Error processing election event file: {err}"))?;

    // Zip file processing
    if document_type == "application/ezip" || matches_mime("zip", &document_type) {
        for (file_name, mut file_contents) in zip_entries {
            info!("Importing file: {:?}", file_name);

            let mut cursor = Cursor::new(&mut file_contents[..]);

            if file_name.contains(&format!("{}", EDocuments::ACTIVITY_LOGS.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create activity logs temporary file")?;

                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy contents of activity logs to temporary file")?;
                temp_file.as_file_mut().rewind()?;
                process_activity_logs_file(
                    hasura_transaction,
                    &temp_file,
                    &election_event_id,
                    &tenant_id,
                )
                .await
                .context("Failed to import activity logs")?;
            }

            if file_name.contains(&format!("{}", EDocuments::VOTERS.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create activity logs temporary file")?;
                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy contents of activity logs to temporary file")?;
                temp_file.as_file_mut().rewind()?;

                process_voters_file(
                    &hasura_transaction,
                    &temp_file,
                    &file_name,
                    Some(election_event_schema.election_event.id.clone()),
                    election_event_schema.tenant_id.to_string(),
                    false,
                )
                .await
                .context("Failed to import voters")?;
            }

            if file_name.contains(&format!("{}", EDocuments::REPORTS.to_file_name())) {
                let mut temp_file =
                    NamedTempFile::new().context("Failed to create reports temporary file")?;
                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy contents of reports to temporary file")?;
                temp_file.as_file_mut().rewind()?;

                // Process the reports file
                process_reports_file(
                    &hasura_transaction,
                    &temp_file,
                    election_event_schema.tenant_id.to_string(),
                    Some(election_event_schema.election_event.id.clone()),
                    &replacement_map,
                )
                .await
                .context("Failed to import reports")?;
            }

            if file_name.contains(&format!("{}/", EDocuments::S3_FILES.to_file_name())) {
                let folder_path: Vec<_> = file_name.split("/").collect();
                // Skips the OS created files
                if folder_path[1] == EDocuments::VOTERS.to_file_name() {
                    continue;
                }

                // Write the file contents to a new file within this directory
                let mut temp_file =
                    generate_temp_file(&folder_path[1], &folder_path[folder_path.len() - 1])
                        .context("Error generating temp file")?;

                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy S3 contents to temporary file")?;
                temp_file.as_file_mut().rewind()?;

                process_s3_file(
                    &hasura_transaction,
                    &temp_file,
                    &file_name,
                    Some(election_event_schema.election_event.id.clone()),
                    election_event_schema.tenant_id.to_string(),
                    replacement_map.clone(),
                    false,
                )
                .await
                .context("Failed to import S3 files")?;
            }
            if file_name.contains(&format!("{}/", EDocuments::IMAGES.to_file_name())) {
                let folder_path: Vec<_> = file_name.split("/").collect();

                // Write the file contents to a new file within this directory
                let mut temp_file =
                    generate_temp_file(&folder_path[1], &folder_path[folder_path.len() - 1])
                        .context("Error generating temp file")?;

                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy S3 contents to temporary file")?;
                temp_file.as_file_mut().rewind()?;

                process_s3_file(
                    &hasura_transaction,
                    &temp_file,
                    &file_name,
                    None,
                    election_event_schema.tenant_id.to_string(),
                    replacement_map.clone(),
                    true,
                )
                .await
                .context("Failed to import S3 files")?;
            }

            if file_name.contains(&format!("{}", EDocuments::BULLETIN_BOARDS.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create bulletin boards temporary file")?;

                io::copy(&mut cursor, &mut temp_file)
                    .context("Failed to copy contents of bulletin boards file to temporary file")?;
                temp_file.as_file_mut().rewind()?;
                import_bulletin_boards(
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    temp_file,
                    replacement_map.clone(),
                )
                .await
                .context("Failed to import bulletin boards")?;
            }

            if file_name.contains(&format!("{}", EDocuments::SCHEDULED_EVENTS.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create scheduled events temporary file")?;

                io::copy(&mut cursor, &mut temp_file).context(
                    "Failed to copy contents of scheduled events file to temporary file",
                )?;
                temp_file.as_file_mut().rewind()?;

                import_scheduled_events(
                    hasura_transaction,
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    temp_file,
                    replacement_map.clone(),
                )
                .await
                .with_context(|| "Error managing dates")?;
            }

            if file_name.contains(&format!("{}", EDocuments::PUBLICATIONS.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create ballot publications temporary file")?;

                io::copy(&mut cursor, &mut temp_file).context(
                    "Failed to copy contents of ballot publications file to temporary file",
                )?;
                temp_file.as_file_mut().rewind()?;

                import_ballot_publications(
                    hasura_transaction,
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    temp_file,
                    replacement_map.clone(),
                )
                .await
                .with_context(|| "Error importing publications")?;
            }

            if file_name.contains(&format!(
                "{}",
                EDocuments::PROTOCOL_MANAGER_KEYS.to_file_name()
            )) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create protocol manager keys temporary file")?;

                io::copy(&mut cursor, &mut temp_file).context(
                    "Failed to copy contents of protocol manager keys file to temporary file",
                )?;
                temp_file.as_file_mut().rewind()?;
                import_protocol_manager_keys(
                    hasura_transaction,
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    temp_file,
                    replacement_map.clone(),
                )
                .await
                .context("Failed to import protocol manager keys")?;
            }

            if file_name.contains(&format!("{}/", EDocuments::TALLY.to_file_name())) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create ballot publications temporary file")?;

                io::copy(&mut cursor, &mut temp_file).context(
                    "Failed to copy contents of ballot publications file to temporary file",
                )?;
                temp_file.as_file_mut().rewind()?;
                let tally_file_name = file_name
                    .split("/")
                    .last()
                    .ok_or(anyhow!("Unexpected, tally without filename"))?
                    .split(".")
                    .next()
                    .ok_or(anyhow!("Unexpected tally without extension"))?;

                process_tally_file(
                    hasura_transaction,
                    &temp_file,
                    tally_file_name.to_string(),
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    replacement_map.clone(),
                )
                .await
                .context("Failed to import tally_file")?;
            }
        }
    };

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn manage_dates(
    data: &ImportElectionEventSchema,
    hasura_transaction: &Transaction<'_>,
) -> Result<()> {
    let Some(scheduled_events) = data.scheduled_events.clone() else {
        return Ok(());
    };

    //Manage election event
    let election_event_dates = generate_voting_period_dates(
        scheduled_events.clone(),
        data.tenant_id.to_string().as_str(),
        &data.election_event.id,
        None,
    )?;
    if let Some(start_date) = election_event_dates.start_date {
        maybe_create_scheduled_event(
            hasura_transaction,
            data.tenant_id.to_string().as_str(),
            &data.election_event.id,
            EventProcessors::START_VOTING_PERIOD,
            start_date,
            None,
        )
        .await?;
    }
    if let Some(end_date) = election_event_dates.end_date {
        maybe_create_scheduled_event(
            hasura_transaction,
            data.tenant_id.to_string().as_str(),
            &data.election_event.id,
            EventProcessors::END_VOTING_PERIOD,
            end_date,
            None,
        )
        .await?;
    }
    //Manage elections
    let elections = &data.elections;
    for election in elections {
        let dates = generate_voting_period_dates(
            scheduled_events.clone(),
            data.tenant_id.to_string().as_str(),
            &data.election_event.id,
            Some(&election.id),
        )?;
        if let Some(start_date) = dates.start_date {
            maybe_create_scheduled_event(
                hasura_transaction,
                data.tenant_id.to_string().as_str(),
                &data.election_event.id,
                EventProcessors::START_VOTING_PERIOD,
                start_date,
                Some(&election.id),
            )
            .await?;
        }
        if let Some(end_date) = dates.end_date {
            maybe_create_scheduled_event(
                hasura_transaction,
                data.tenant_id.to_string().as_str(),
                &data.election_event.id,
                EventProcessors::END_VOTING_PERIOD,
                end_date,
                Some(&election.id),
            )
            .await?;
        }
    }
    Ok(())
}

#[instrument(err, skip_all)]
pub async fn maybe_create_scheduled_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    event_processor: EventProcessors,
    start_date: String,
    election_id: Option<&str>,
) -> Result<()> {
    let start_task_id =
        generate_manage_date_task_name(tenant_id, election_event_id, election_id, &event_processor);
    let payload = ManageElectionDatePayload {
        election_id: match election_id {
            Some(id) => Some(id.to_string()),
            None => None,
        },
    };
    let cron_config = CronConfig {
        cron: None,
        scheduled_date: Some(start_date.to_string()),
    };
    insert_scheduled_event(
        hasura_transaction,
        tenant_id,
        election_event_id,
        event_processor,
        &start_task_id,
        cron_config,
        serde_json::to_value(payload)?,
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod imported_realm_sanitization_tests {
    use super::*;
    use ::keycloak::types::{
        ClientRepresentation, ClientScopeRepresentation, Composites, RoleRepresentation,
        UserRepresentation,
    };

    const EVENT_REALM_TEMPLATE: &str = include_str!(
        "../../../../../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559.json"
    );

    fn template_realm() -> RealmRepresentation {
        serde_json::from_str(EVENT_REALM_TEMPLATE).expect("event realm template parses")
    }

    fn realm_role_names(realm: &RealmRepresentation) -> HashSet<String> {
        realm
            .roles
            .as_ref()
            .and_then(|roles| roles.realm.as_ref())
            .map(|realm_roles| {
                realm_roles
                    .iter()
                    .filter_map(|role| role.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn hardcoded_hasura_role_mapper(name: &str, role: &str) -> ProtocolMapperRepresentation {
        let config = HashMap::from([
            (
                PROTOCOL_MAPPER_CLAIM_NAME_KEY.to_string(),
                format!("https://hasura\\.io/jwt/claims.{name}"),
            ),
            ("claim.value".to_string(), role.to_string()),
            ("jsonType.label".to_string(), "String".to_string()),
        ]);
        ProtocolMapperRepresentation {
            name: Some(name.to_string()),
            protocol: Some("openid-connect".to_string()),
            protocol_mapper: Some("oidc-hardcoded-claim-mapper".to_string()),
            config: Some(config),
            ..Default::default()
        }
    }

    /// Build a realm that adds Hasura admin roles, a user holding them and a
    /// client minting admin claims on top of the platform template.
    fn realm_with_injected_claims() -> RealmRepresentation {
        let mut realm = template_realm();

        if let Some(roles) = realm.roles.as_mut() {
            let realm_roles = roles.realm.get_or_insert_with(Vec::new);
            for name in ["service-account", "admin-user"] {
                realm_roles.push(RoleRepresentation {
                    name: Some(name.to_string()),
                    ..Default::default()
                });
            }
        }

        realm
            .users
            .get_or_insert_with(Vec::new)
            .push(UserRepresentation {
                username: Some("imported-user".to_string()),
                enabled: Some(true),
                realm_roles: Some(vec![
                    "service-account".to_string(),
                    "admin-user".to_string(),
                ]),
                ..Default::default()
            });

        realm
            .clients
            .get_or_insert_with(Vec::new)
            .push(ClientRepresentation {
                client_id: Some("rogue-client".to_string()),
                public_client: Some(true),
                direct_access_grants_enabled: Some(true),
                protocol: Some("openid-connect".to_string()),
                protocol_mappers: Some(vec![hardcoded_hasura_role_mapper(
                    "x-hasura-default-role",
                    "service-account",
                )]),
                ..Default::default()
            });

        realm
    }

    fn client<'a>(realm: &'a RealmRepresentation, client_id: &str) -> &'a ClientRepresentation {
        realm
            .clients
            .as_ref()
            .expect("realm has clients")
            .iter()
            .find(|client| client.client_id.as_deref() == Some(client_id))
            .expect("client present")
    }

    fn client_claim_mapper_count(realm: &RealmRepresentation, client_id: &str) -> usize {
        client(realm, client_id)
            .protocol_mappers
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|mapper| is_claim_authority_mapper(mapper))
            .count()
    }

    #[test]
    fn aligning_drops_injected_admin_roles_and_claim_mappers() {
        let template = template_realm();
        let sanitized = sanitize_imported_event_realm(
            &realm_with_injected_claims(),
            &template,
            ImportedRealmReconciliation::AlignWithTemplate,
        )
        .expect("sanitization succeeds");

        let roles = realm_role_names(&sanitized);
        assert!(!roles.contains("service-account"));
        assert!(!roles.contains("admin-user"));

        let imported_user = sanitized
            .users
            .as_ref()
            .unwrap()
            .iter()
            .find(|user| user.username.as_deref() == Some("imported-user"))
            .expect("imported user kept");
        let imported_roles = imported_user.realm_roles.clone().unwrap_or_default();
        assert!(!imported_roles.contains(&"service-account".to_string()));
        assert!(!imported_roles.contains(&"admin-user".to_string()));

        assert_eq!(client_claim_mapper_count(&sanitized, "rogue-client"), 0);
    }

    #[test]
    fn aligning_keeps_platform_template_claim_mappers() {
        let template = template_realm();
        let sanitized = sanitize_imported_event_realm(
            &template,
            &template,
            ImportedRealmReconciliation::AlignWithTemplate,
        )
        .expect("sanitization succeeds");

        assert_eq!(realm_role_names(&sanitized), realm_role_names(&template));
        assert_eq!(
            client_claim_mapper_count(&sanitized, "voting-portal"),
            client_claim_mapper_count(&template, "voting-portal"),
        );
    }

    #[test]
    fn rejecting_mode_leaves_a_legitimate_template_untouched() {
        let template = template_realm();
        assert!(sanitize_imported_event_realm(
            &template,
            &template,
            ImportedRealmReconciliation::RejectForeignClaims,
        )
        .is_ok());
    }

    #[test]
    fn rejecting_mode_refuses_injected_claims() {
        let template = template_realm();
        assert!(sanitize_imported_event_realm(
            &realm_with_injected_claims(),
            &template,
            ImportedRealmReconciliation::RejectForeignClaims,
        )
        .is_err());
    }

    const REALM_ADMIN_CLIENT: &str = "realm-management";
    const REALM_ADMIN_ROLE: &str = "realm-admin";

    fn sanitize(imported: &RealmRepresentation) -> RealmRepresentation {
        sanitize_imported_event_realm(
            imported,
            &template_realm(),
            ImportedRealmReconciliation::AlignWithTemplate,
        )
        .expect("sanitization succeeds")
    }

    fn realm_admin_assignment() -> ClientRoleAssignments {
        HashMap::from([(
            REALM_ADMIN_CLIENT.to_string(),
            vec![REALM_ADMIN_ROLE.to_string()],
        )])
    }

    #[test]
    fn aligning_drops_client_roles_assigned_to_users_and_groups() {
        let mut realm = template_realm();
        realm
            .users
            .get_or_insert_with(Vec::new)
            .push(UserRepresentation {
                username: Some("imported-user".to_string()),
                client_roles: Some(realm_admin_assignment()),
                ..Default::default()
            });
        realm
            .groups
            .get_or_insert_with(Vec::new)
            .push(GroupRepresentation {
                name: Some("imported-group".to_string()),
                client_roles: Some(realm_admin_assignment()),
                sub_groups: Some(vec![GroupRepresentation {
                    name: Some("imported-subgroup".to_string()),
                    client_roles: Some(realm_admin_assignment()),
                    ..Default::default()
                }]),
                ..Default::default()
            });

        let sanitized = sanitize(&realm);

        let user = sanitized
            .users
            .iter()
            .flatten()
            .find(|user| user.username.as_deref() == Some("imported-user"))
            .expect("imported user kept");
        assert!(user.client_roles.clone().unwrap_or_default().is_empty());
        let group = sanitized
            .groups
            .iter()
            .flatten()
            .find(|group| group.name.as_deref() == Some("imported-group"))
            .expect("imported group kept");
        assert!(group.client_roles.clone().unwrap_or_default().is_empty());
        let subgroup = &group.sub_groups.as_ref().expect("subgroup kept")[0];
        assert!(subgroup.client_roles.clone().unwrap_or_default().is_empty());
    }

    #[test]
    fn aligning_resets_the_composites_of_allowed_roles() {
        let mut realm = template_realm();
        let allowed_role = realm_role_names(&realm)
            .into_iter()
            .find(|name| !name.starts_with(DEFAULT_REALM_ROLE_PREFIX))
            .expect("template has a realm role");
        for role in realm
            .roles
            .as_mut()
            .and_then(|roles| roles.realm.as_mut())
            .expect("template has realm roles")
        {
            if role.name.as_deref() == Some(allowed_role.as_str()) {
                role.composites = Some(Composites {
                    client: Some(HashMap::from([(
                        REALM_ADMIN_CLIENT.to_string(),
                        vec![REALM_ADMIN_ROLE.to_string()],
                    )])),
                    ..Default::default()
                });
            }
        }

        let sanitized = sanitize(&realm);

        let template = template_realm();
        let composites_of = |realm: &RealmRepresentation| {
            realm
                .roles
                .iter()
                .flat_map(|roles| roles.realm.iter().flatten())
                .find(|role| role.name.as_deref() == Some(allowed_role.as_str()))
                .and_then(|role| role.composites.clone())
        };
        assert_eq!(composites_of(&sanitized), composites_of(&template));
    }

    #[test]
    fn aligning_only_allows_the_realms_own_default_role() {
        let mut realm = template_realm();
        let fake_default_role = format!("{DEFAULT_REALM_ROLE_PREFIX}attacker");
        realm
            .roles
            .get_or_insert_with(Default::default)
            .realm
            .get_or_insert_with(Vec::new)
            .push(RoleRepresentation {
                name: Some(fake_default_role.clone()),
                ..Default::default()
            });
        realm
            .users
            .get_or_insert_with(Vec::new)
            .push(UserRepresentation {
                username: Some("imported-user".to_string()),
                realm_roles: Some(vec![fake_default_role.clone()]),
                ..Default::default()
            });

        let sanitized = sanitize(&realm);

        assert!(!realm_role_names(&sanitized).contains(&fake_default_role));
        let user = sanitized
            .users
            .iter()
            .flatten()
            .find(|user| user.username.as_deref() == Some("imported-user"))
            .expect("imported user kept");
        assert!(!user
            .realm_roles
            .clone()
            .unwrap_or_default()
            .contains(&fake_default_role));
    }

    #[test]
    fn aligning_drops_identity_provider_mappers_the_template_lacks() {
        let mut realm = template_realm();
        realm
            .identity_provider_mappers
            .get_or_insert_with(Vec::new)
            .push(IdentityProviderMapperRepresentation {
                name: Some("grant-role".to_string()),
                identity_provider_alias: Some("any-idp".to_string()),
                identity_provider_mapper: Some("oidc-role-idp-mapper".to_string()),
                config: Some(HashMap::from([(
                    "role".to_string(),
                    "admin-user".to_string(),
                )])),
                ..Default::default()
            });

        let sanitized = sanitize(&realm);

        assert_eq!(
            sanitized.identity_provider_mappers,
            template_realm().identity_provider_mappers
        );
    }

    #[test]
    fn aligning_drops_claim_mappers_on_unknown_client_scopes() {
        let mut realm = template_realm();
        realm
            .client_scopes
            .get_or_insert_with(Vec::new)
            .push(ClientScopeRepresentation {
                name: Some("rogue-scope".to_string()),
                protocol_mappers: Some(vec![hardcoded_hasura_role_mapper(
                    "x-hasura-default-role",
                    "service-account",
                )]),
                ..Default::default()
            });

        let sanitized = sanitize(&realm);

        let scope = sanitized
            .client_scopes
            .iter()
            .flatten()
            .find(|scope| scope.name.as_deref() == Some("rogue-scope"))
            .expect("scope kept");
        assert!(scope
            .protocol_mappers
            .as_deref()
            .unwrap_or_default()
            .is_empty());
    }
}
