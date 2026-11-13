// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::application::insert_applications;
use crate::postgres::election_event::{
    election_event_id_exists, get_election_event_by_id_if_exist, lock_election_event_id,
    update_bulletin_board,
};
use crate::postgres::reports::insert_reports;
use crate::postgres::reports::Report;
use crate::postgres::trustee::get_all_trustees;
use crate::services::ceremonies::auditable_ballots::AUDITABLE_BALLOTS_FILE;
use crate::services::electoral_log::ElectoralLogAdminContext;
use crate::services::import::import_publications::{
    import_ballot_publications, import_election_event_config_file,
};
use crate::services::import::import_scheduled_events::import_scheduled_events;
use crate::services::import::import_tally::process_tally_file;
use crate::services::keycloak::read_realm_config_from_s3;
use crate::services::protocol_manager::get_event_board;
use crate::services::reports::template_renderer::EReportEncryption;
use crate::services::reports_vault::get_report_key_pair;
use crate::services::tasks_execution::update_fail;
use crate::tasks::insert_election_event::CreateElectionEventInput;
use crate::types::documents::ETallyDocuments;
use ::keycloak::types::{
    ComponentExportRepresentation, Composites, GroupRepresentation,
    IdentityProviderMapperRepresentation, ProtocolMapperRepresentation, RealmRepresentation,
    RoleRepresentation,
};
use anyhow::{anyhow, Context, Result};
use chrono::format;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::future::try_join_all;
use keycloak::types::RealmEventsConfigRepresentation;
use once_cell::sync::Lazy;
use sequent_core::ballot::ElectionEventStatistics;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::ElectionStatistics;
use sequent_core::ballot::ElectionStatus;
use sequent_core::ballot::VotingPeriodDates;
use sequent_core::ballot::{AllowTallyStatus, LanguageDetectionPolicy};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::connection;
use sequent_core::services::keycloak::{
    generate_client_secret, get_client_credentials, get_event_realm, replace_realm_ids,
    KeycloakAdminClient,
};
use sequent_core::services::replace_uuids::replace_uuids;
use sequent_core::types::hasura::core::Application;
use sequent_core::types::hasura::core::AreaContest;
use sequent_core::types::hasura::core::Document;
use sequent_core::types::hasura::core::KeysCeremony;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::locale::iso_639_2t_to_bcp47;
use sequent_core::util::mime::{get_mime_types, matches_mime};
use sequent_core::util::version::{
    check_version_compatibility, DEV_APP_VERSION, ENV_VAR_APP_VERSION, HISTORICAL_DEFAULT_VERSION,
    VERSION_KEY,
};
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

const KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY: &str =
    "KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY";

use super::import_users::{import_users_file, ImportUsersPrivileges};
use crate::postgres;
use crate::postgres::area::insert_areas;
use crate::postgres::area_contest::insert_area_contests;
use crate::postgres::candidate::insert_candidates;
use crate::postgres::certificate_authority::{
    insert_certificate_authority, CertificateAuthorityRecord,
};
use crate::postgres::contest::insert_contest;
use crate::postgres::election::insert_elections;
use crate::postgres::election_event::insert_election_event;
use crate::postgres::keys_ceremony;
use crate::postgres::scheduled_event::insert_scheduled_event;
use crate::services::certificate_authority::{parse_certificate_pem, split_pem_bundle};
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
use sequent_core::types::keycloak::{
    CERTIFICATES_IDP_ALIAS, DEFAULT_IVR_SERVICE_CLIENT_ID, IVR_VOTING_CLIENT_ID,
};
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
    #[serde(default = "default_version")]
    pub version: String,
}

/// Set the default version of an imported election event to be compatible with version 9, which is the first version to include this feature.
fn default_version() -> String {
    HISTORICAL_DEFAULT_VERSION.to_string()
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
pub async fn read_default_election_event_realm() -> Result<RealmRepresentation> {
    read_realm_config_from_s3(KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY).await
}

/// Claim namespace that Keycloak protocol mappers use to emit Hasura session
/// variables (roles and tenant). A mapper writing under this namespace decides
/// which roles and tenant a token is accepted with, so only the platform
/// template may define these on an imported event realm.
const HASURA_CLAIM_NAMESPACE: &str = "https://hasura.io/jwt/claims";
/// Protocol-mapper config key holding the claim a mapper writes.
const PROTOCOL_MAPPER_CLAIM_NAME_KEY: &str = "claim.name";
/// Protocol mappers that add or rename a token's roles without naming a claim.
const ROLE_CHANGING_MAPPERS: [&str; 2] = ["oidc-hardcoded-role-mapper", "oidc-role-name-mapper"];
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

/// True when a protocol mapper writes a Hasura session variable, runs a script
/// or changes the token's roles, i.e. it can set the roles or tenant a token
/// carries.
fn is_claim_authority_mapper(mapper: &ProtocolMapperRepresentation) -> bool {
    if mapper
        .protocol_mapper
        .as_deref()
        .is_some_and(|kind| kind.contains("script") || ROLE_CHANGING_MAPPERS.contains(&kind))
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

/// Records the members of `composites` that `allowed` doesn't have.
fn record_foreign_composites(
    composites: Option<&Composites>,
    allowed: Option<&Composites>,
    role: &str,
    discarded: &mut Vec<String>,
) {
    let Some(composites) = composites else {
        return;
    };
    let allowed_realm_roles = allowed.and_then(|allowed| allowed.realm.as_ref());
    for name in composites.realm.iter().flatten() {
        if !allowed_realm_roles.is_some_and(|roles| roles.contains(name)) {
            discarded.push(format!("realm role '{name}' composed into role '{role}'"));
        }
    }
    let allowed_client_roles = allowed.and_then(|allowed| allowed.client.as_ref());
    for (client_id, names) in composites.client.iter().flatten() {
        let allowed_names = allowed_client_roles.and_then(|clients| clients.get(client_id));
        for name in names {
            if !allowed_names.is_some_and(|roles| roles.contains(name)) {
                discarded.push(format!(
                    "client role '{name}' of client '{client_id}' composed into role '{role}'"
                ));
            }
        }
    }
}

/// The realm and client role names a realm defines.
struct DefinedRoles {
    realm: HashSet<String>,
    client: HashMap<String, HashSet<String>>,
}

impl DefinedRoles {
    fn of(realm: &RealmRepresentation) -> Self {
        let names = |roles: &Vec<RoleRepresentation>| -> HashSet<String> {
            roles.iter().filter_map(|role| role.name.clone()).collect()
        };
        let roles = realm.roles.as_ref();
        DefinedRoles {
            realm: roles
                .and_then(|roles| roles.realm.as_ref())
                .map(names)
                .unwrap_or_default(),
            client: roles
                .and_then(|roles| roles.client.as_ref())
                .map(|clients| {
                    clients
                        .iter()
                        .map(|(client_id, roles)| (client_id.clone(), names(roles)))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// Keeps the members of `composites` that the realm defines. Keycloak
    /// refuses a realm whose role composes a role it doesn't define, which a
    /// template from a newer Keycloak would otherwise cause: an export from
    /// before Keycloak 26.7 has no organization roles for `realm-admin`.
    fn retain_defined(&self, composites: Option<Composites>) -> Option<Composites> {
        composites.map(|mut composites| {
            if let Some(names) = composites.realm.as_mut() {
                names.retain(|name| self.realm.contains(name));
            }
            if let Some(clients) = composites.client.as_mut() {
                clients.retain(|client_id, names| {
                    let defined = self.client.get(client_id);
                    names.retain(|name| defined.is_some_and(|defined| defined.contains(name)));
                    !names.is_empty()
                });
            }
            composites
        })
    }
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
    for template_mapper in template_mappers
        .iter()
        .filter(|mapper| is_claim_authority_mapper(mapper))
    {
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

/// Keycloak component holding a realm's declarative user profile.
const USER_PROFILE_PROVIDER_COMPONENT: &str = "org.keycloak.userprofile.UserProfileProvider";
/// Config key under which that component stores the profile as one JSON document.
const USER_PROFILE_CONFIG_KEY: &str = "kc.user.profile.config";
/// User-profile setting that lets users edit attributes the profile doesn't declare.
const UNMANAGED_ATTRIBUTE_POLICY_KEY: &str = "unmanagedAttributePolicy";

/// A realm's user-profile component, if it has one.
fn user_profile_component(realm: &RealmRepresentation) -> Option<&ComponentExportRepresentation> {
    realm
        .components
        .as_ref()?
        .get(USER_PROFILE_PROVIDER_COMPONENT)?
        .first()
}

/// The user profile a component stores, parsed. `None` when it is missing or
/// isn't a profile (no `attributes` list). Keycloak keeps it as a single value.
fn user_profile_config(component: &ComponentExportRepresentation) -> Option<serde_json::Value> {
    let raw = component
        .config
        .as_ref()?
        .get(USER_PROFILE_CONFIG_KEY)?
        .first()?;
    let profile: serde_json::Value = serde_json::from_str(raw).ok()?;
    profile.get("attributes")?.as_array()?;
    Some(profile)
}

/// Reconcile the uploaded realm's declarative user profile with the template's.
/// Token claims come from user attributes (`tenant-id`, `area-id`,
/// `authorized-election-ids`), so a profile that lets users edit them lets a
/// voter choose the tenant or elections of their own tokens. Every attribute
/// the template declares therefore gets the template's permissions, a template
/// attribute missing from the upload is restored, and the template's
/// unmanaged-attribute policy applies, since enabling it would make an
/// undeclared attribute editable. Attributes only the upload declares stay:
/// the claim mappers are already reduced to the template's, which read
/// template attributes only. A missing or unreadable profile is replaced with
/// the template's. A template without a readable profile fails the import,
/// since it is the baseline the uploaded permissions are checked against.
fn reconcile_user_profile(
    realm: &mut RealmRepresentation,
    template: &RealmRepresentation,
    discarded: &mut Vec<String>,
) -> Result<()> {
    let Some((template_component, template_profile)) = user_profile_component(template)
        .and_then(|component| Some((component, user_profile_config(component)?)))
    else {
        return Err(anyhow!(
            "The realm template has no readable user profile, so the imported realm's \
             user-profile permissions can't be checked"
        ));
    };

    let Some(mut profile) = user_profile_component(realm).and_then(user_profile_config) else {
        let mut component = template_component.clone();
        component.id = None;
        realm
            .components
            .get_or_insert_with(HashMap::new)
            .insert(USER_PROFILE_PROVIDER_COMPONENT.to_string(), vec![component]);
        discarded
            .push("user profile (missing or unreadable, replaced with the template's)".to_string());
        return Ok(());
    };

    let template_attributes: Vec<serde_json::Value> = template_profile["attributes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut changed = false;
    if let Some(attributes) = profile
        .get_mut("attributes")
        .and_then(serde_json::Value::as_array_mut)
    {
        for template_attribute in &template_attributes {
            let Some(name) = template_attribute
                .get("name")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            let template_permissions = template_attribute
                .get("permissions")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            match attributes.iter_mut().find(|attribute| {
                attribute.get("name").and_then(serde_json::Value::as_str) == Some(name)
            }) {
                Some(attribute) => {
                    if attribute
                        .get("permissions")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null)
                        != template_permissions
                    {
                        attribute["permissions"] = template_permissions;
                        discarded.push(format!("user-profile permissions of attribute '{name}'"));
                        changed = true;
                    }
                }
                None => {
                    attributes.push(template_attribute.clone());
                    discarded.push(format!(
                        "user-profile attribute '{name}' missing (restored)"
                    ));
                    changed = true;
                }
            }
        }
    }

    let template_policy = template_profile
        .get(UNMANAGED_ATTRIBUTE_POLICY_KEY)
        .cloned();
    if profile.get(UNMANAGED_ATTRIBUTE_POLICY_KEY).cloned() != template_policy {
        if let Some(object) = profile.as_object_mut() {
            match template_policy {
                Some(policy) => object.insert(UNMANAGED_ATTRIBUTE_POLICY_KEY.to_string(), policy),
                None => object.remove(UNMANAGED_ATTRIBUTE_POLICY_KEY),
            };
        }
        discarded.push("user-profile unmanaged-attribute policy".to_string());
        changed = true;
    }

    if changed {
        if let Some(component) = realm
            .components
            .as_mut()
            .and_then(|components| components.get_mut(USER_PROFILE_PROVIDER_COMPONENT))
            .and_then(|components| components.first_mut())
        {
            component.config.get_or_insert_with(HashMap::new).insert(
                USER_PROFILE_CONFIG_KEY.to_string(),
                vec![profile.to_string()],
            );
        }
    }
    Ok(())
}

/// Reconcile an uploaded election-event realm against the platform template so
/// that only the template's realm roles, Hasura claim mappers and
/// identity-provider mappers survive, and its user profile doesn't let users
/// edit the attributes token claims come from. This prevents an imported realm
/// from granting roles (such as `service-account` or `admin-user`) or minting
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
    // The template's top-level default role only names it: its composites are
    // on the full definition among the realm roles.
    if let Some(name) = imported_default_role_name {
        let composites = default_role_name(template)
            .and_then(|template_name| template_composites.get(&template_name).cloned())
            .flatten()
            .or_else(|| template_default_role.and_then(|role| role.composites));
        template_composites.insert(name, composites);
    }

    let defined_roles = DefinedRoles::of(&realm);

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
                let composites = template_composites.get(&name).cloned().flatten();
                record_foreign_composites(
                    role.composites.as_ref(),
                    composites.as_ref(),
                    &name,
                    &mut discarded,
                );
                role.composites = defined_roles.retain_defined(composites);
            }
        }
    }

    // 2. Default role composites.
    if let Some(default_role) = realm.default_role.as_mut() {
        let name = default_role.name.clone().unwrap_or_default();
        let composites = template_composites.get(&name).cloned().flatten();
        record_foreign_composites(
            default_role.composites.as_ref(),
            composites.as_ref(),
            &name,
            &mut discarded,
        );
        default_role.composites = defined_roles.retain_defined(composites);
    }

    // 3. Client role composites: an allowed role could otherwise inherit
    //    what the assignment checks below refuse to grant directly.
    let template_client_composites: HashMap<(&str, &str), &Option<Composites>> = template
        .roles
        .iter()
        .flat_map(|roles| roles.client.iter().flatten())
        .flat_map(|(client_id, roles)| {
            roles.iter().filter_map(move |role| {
                Some((
                    (client_id.as_str(), role.name.as_deref()?),
                    &role.composites,
                ))
            })
        })
        .collect();
    if let Some(client_roles) = realm.roles.as_mut().and_then(|roles| roles.client.as_mut()) {
        for (client_id, roles) in client_roles.iter_mut() {
            for role in roles.iter_mut() {
                let name = role.name.clone().unwrap_or_default();
                let composites = template_client_composites
                    .get(&(client_id.as_str(), name.as_str()))
                    .and_then(|composites| (*composites).clone());
                record_foreign_composites(
                    role.composites.as_ref(),
                    composites.as_ref(),
                    &format!("{client_id}/{name}"),
                    &mut discarded,
                );
                role.composites = defined_roles.retain_defined(composites);
            }
        }
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

    // 4. Role assignments on users.
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

    // 5. Role assignments on groups.
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

    // 6. Scope mappings that reference dropped roles.
    if let Some(scope_mappings) = realm.scope_mappings.as_mut() {
        for mapping in scope_mappings.iter_mut() {
            if let Some(roles) = mapping.roles.as_mut() {
                roles.retain(|name| realm_role_allowed(name, &allowed_realm_roles));
            }
        }
    }

    // 7. Claim mappers on clients and client scopes: each keeps only the
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

    // 8. Identity-provider mappers: only the ones the template defines survive,
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

    // 9. User profile: users must not be able to edit the attributes the
    //    claims come from.
    reconcile_user_profile(&mut realm, template, &mut discarded)?;

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
    let mut realm_copy = realm.clone();
    super::realm_user_credentials::remove_user_credentials(&mut realm_copy);
    super::realm_identity_providers::remove_demo_identity_providers(&mut realm_copy);

    // Collect well-known clients and their secrets to set.
    let keycloak_client_id = env::var("KEYCLOAK_CLIENT_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .with_context(|| "KEYCLOAK_CLIENT_ID can't be empty")?;
    let keycloak_client_secret = env::var("KEYCLOAK_CLIENT_SECRET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .with_context(|| "KEYCLOAK_CLIENT_SECRET can't be empty")?;
    let mut known_clients = vec![
        // Keycloak client is always known and required, as checked (and failed if required) above.
        (keycloak_client_id, Some(keycloak_client_secret)),
        // IVR specific clients are optional, their secrets may or may not be configured during deployment.
        // This is not considered a failure, it will be ignored, and a new secret will be generated.
        (
            env::var("KEYCLOAK_IVR_SERVICE_CLIENT_ID")
                .unwrap_or(DEFAULT_IVR_SERVICE_CLIENT_ID.to_string()),
            env::var("KEYCLOAK_IVR_SERVICE_CLIENT_SECRET").ok(),
        ),
        (
            IVR_VOTING_CLIENT_ID.to_string(),
            env::var("KEYCLOAK_IVR_VOTING_CLIENT_SECRET").ok(),
        ),
    ];

    // For each IDP that has both clientId and clientSecret configured,
    // look if it is the special CERTIFICATES_IDP_ALIAS, then generate a
    // new secret, update the IDP config, and record (clientId -> newSecret) in the known_clients so the
    // matching Keycloak client can be given the same credential in the client's loop below.
    if let Some(identity_providers) = realm_copy.identity_providers.clone() {
        let new_identity_providers = identity_providers
            .iter()
            .map(|idp| {
                let mut idp_copy = idp.clone();
                match idp_copy.config.clone() {
                    Some(config) if idp.alias.as_deref() == Some(CERTIFICATES_IDP_ALIAS) => {
                        let mut new_config = config.clone();
                        if let Some(idp_client_id) = new_config.get("clientId").cloned() {
                            if new_config.contains_key("clientSecret") {
                                let new_secret = generate_client_secret();
                                new_config.insert("clientSecret".to_string(), new_secret.clone());
                                known_clients.push((idp_client_id, Some(new_secret)));
                            }
                        }
                        idp_copy.config = Some(new_config);
                    }
                    _ => {
                        // no config, nothing to do
                    }
                }
                idp_copy
            })
            .collect();
        realm_copy.identity_providers = Some(new_identity_providers);
    }

    // For each client, assign its secret:
    // 1. The known Keycloak clients (such as service-account, or ivr clients) get the configured env var secret.
    // 2. The client that was configured in IDP CERTIFICATES_IDP_ALIAS gets the generated client secret (becomes a "known client").
    // 3. All others have their secret cleared so Keycloak regenerates it.
    realm_copy.clients = realm_copy.clients.map(|clients| {
        clients
            .iter()
            .map(|client| {
                let mut client_copy = client.clone();
                // Check if the client matches with any known client.
                // If not, we'll leave None, and Keycloak will regenerate it.
                client_copy.secret = client.client_id.as_deref().and_then(|client_id| {
                    known_clients
                        .iter()
                        .find(|(known_id, _)| client_id == known_id)
                        .and_then(|(known_id, known_secret)| {
                            // Don't accept empty values
                            let secret = known_secret.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
                            if secret.is_none() {
                                tracing::warn!("Known client '{known_id}' had no secret configured, regenerating");
                            }
                            secret
                        })
                });
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
    default_locale: Option<String>,
) -> Result<()> {
    let mut realm = if let Some(uploaded) = keycloak_event_realm.clone() {
        let template = read_default_election_event_realm().await?;
        sanitize_imported_event_realm(&uploaded, &template, ImportedRealmReconciliation::default())?
    } else {
        read_default_election_event_realm().await?
    };

    if let Some(default_language) = default_locale {
        // Keycloak uses BCP 47 locale codes; convert from ISO 639-2/T if needed
        let keycloak_locale = iso_639_2t_to_bcp47(&default_language).to_string();
        realm.default_locale = Some(keycloak_locale);
        let mut attrs = realm.attributes.clone().unwrap_or_default();
        attrs.insert(
            "language_detection_policy".to_string(),
            "force-default".to_string(),
        );
        // Store the internal locale code so the login template can set USER_LANGUAGE correctly
        attrs.insert("forced_language_code".to_string(), default_language.clone());
        realm.attributes = Some(attrs);
    }

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

/// Fails when an election event with this id already exists in any tenant, so
/// the create and import paths only ever set up new events and their realms.
/// The id stays locked until the transaction ends, so two attempts with the
/// same id cannot both pass this check.
#[instrument(skip(hasura_transaction), err)]
pub async fn ensure_new_election_event(
    hasura_transaction: &Transaction<'_>,
    election_event_id: &str,
) -> Result<()> {
    lock_election_event_id(hasura_transaction, election_event_id).await?;
    if election_event_id_exists(hasura_transaction, election_event_id).await? {
        return Err(anyhow!("Election event {election_event_id} already exists"));
    }
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
        statistics: Some(json!({
            "num_emails_sent": 0,
            "num_sms_sent": 0
        })),
        external_id: None,
    };

    insert_election_event(&hasura_transaction, &new_election_input).await?;
    Ok(())
}

/// Replaces UUIDs in the import data while preserving specific IDs that should remain unchanged.
///
/// This function is a thin wrapper around `replace_realm_ids()` that processes the election event
/// import schema and replaces most UUIDs with new ones, while keeping certain IDs unchanged
/// (like tenant_id and optionally election_event_id). It also automatically preserves UUIDs
/// referenced in Keycloak authenticator configurations.
///
/// # Arguments
/// * `data_str` - The original JSON string representation of the import data
/// * `original_data` - The parsed ImportElectionEventSchema structure
/// * `event_id` - Optional election event ID to use. If None, a new UUID will be generated
/// * `tenant_id` - The tenant ID to use (may differ from the original)
///
/// # Returns
/// A tuple containing:
/// * The modified ImportElectionEventSchema with replaced UUIDs
/// * A HashMap mapping old UUIDs to their new replacements
#[instrument(err, skip(data_str, original_data))]
pub fn replace_ids(
    data_str: &str,
    original_data: &ImportElectionEventSchema,
    event_id: Option<String>,
    tenant_id: String,
) -> Result<(ImportElectionEventSchema, HashMap<String, String>)> {
    // Prepare tenant_id replacement - always replace to ensure consistency
    let tenant_id_replacement = Some((original_data.tenant_id.to_string(), tenant_id.clone()));

    let election_event_id = event_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let election_event_id_replacement = Some((
        original_data.election_event.id.clone(),
        election_event_id.clone(),
    ));

    // Use replace_realm_ids which handles:
    // - Preserving UUIDs in Keycloak authenticator configurations
    // - Preserving tenant_id and election_event_id in the keep list before UUID replacement
    // - Applying explicit tenant_id and election_event_id replacements after UUID replacement
    let (new_data, mut replacement_map) = replace_realm_ids(
        data_str,
        vec![], // Empty keep list - replace_realm_ids will populate it automatically
        tenant_id_replacement,
        election_event_id_replacement,
    )?;

    // Parse the modified JSON string back into the structured format
    let data: ImportElectionEventSchema = deserialize_str(&new_data)?;
    validate_import_scope(&data, &tenant_id, &election_event_id)?;

    // Explicit realm replacements are applied to the JSON but omitted from its
    // map. Publication imports also need these scopes, including identity maps
    // when importing back into the same tenant or keeping the event ID.
    replacement_map.insert(
        original_data.tenant_id.to_string(),
        data.tenant_id.to_string(),
    );
    replacement_map.insert(
        original_data.election_event.id.clone(),
        data.election_event.id.clone(),
    );

    // Return both the modified schema and the UUID replacement mapping
    Ok((data, replacement_map))
}

/// Id, tenant id and election event id of every row an import inserts.
fn import_row_scopes(data: &ImportElectionEventSchema) -> Vec<(&str, &str, &str)> {
    macro_rules! scopes {
        ($rows:expr) => {
            $rows.iter().map(|row| {
                (
                    row.id.as_str(),
                    row.tenant_id.as_str(),
                    row.election_event_id.as_str(),
                )
            })
        };
    }
    let keys_ceremonies = data.keys_ceremonies.as_deref().unwrap_or_default();
    let applications = data.applications.as_deref().unwrap_or_default();
    scopes!(data.elections)
        .chain(scopes!(data.contests))
        .chain(scopes!(data.candidates))
        .chain(scopes!(data.areas))
        .chain(scopes!(keys_ceremonies))
        .chain(scopes!(applications))
        .collect()
}

/// Checks that the imported election event and every row it carries belong to
/// the target tenant and election event once the ids have been replaced.
fn validate_import_scope(
    data: &ImportElectionEventSchema,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let tenant =
        Uuid::parse_str(tenant_id).with_context(|| format!("Invalid tenant id {tenant_id}"))?;
    let event = Uuid::parse_str(election_event_id)
        .with_context(|| format!("Invalid election event id {election_event_id}"))?;
    let in_scope = |row_tenant_id: &str, row_event_id: &str| -> Result<bool> {
        let row_tenant = Uuid::parse_str(row_tenant_id)
            .with_context(|| format!("Invalid tenant id {row_tenant_id}"))?;
        let row_event = Uuid::parse_str(row_event_id)
            .with_context(|| format!("Invalid election event id {row_event_id}"))?;
        Ok(row_tenant == tenant && row_event == event)
    };

    if data.tenant_id != tenant
        || !in_scope(&data.election_event.tenant_id, &data.election_event.id)
            .context("Invalid imported election event")?
    {
        return Err(anyhow!(
            "Imported election event does not match tenant {tenant} and election event {event}"
        ));
    }

    for (id, row_tenant_id, row_event_id) in import_row_scopes(data) {
        if !in_scope(row_tenant_id, row_event_id)
            .with_context(|| format!("Invalid imported row {id}"))?
        {
            return Err(anyhow!(
                "Imported row {id} does not belong to tenant {tenant} and election event {event}"
            ));
        }
    }

    Ok(())
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

/// Get the election event schma and also:
/// - Check version compatibility
/// - Replace IDs and return a mapping of old to new IDs (for preserving references in other documents like voters)
#[instrument(err, skip_all)]
pub async fn get_election_event_schema(
    data_str: &str,
    event_id: Option<String>,
    tenant_id: String,
) -> Result<(ImportElectionEventSchema, HashMap<String, String>)> {
    // Catch a version missmatch early and return a clear error message about it, rather than having it fail later on
    // with a more obscure error when trying to deserialize data that is incompatible with the current version.
    let raw: serde_json::Value = serde_json::from_str(data_str)
        .map_err(|e| anyhow!("Failed to parse import data as JSON: {e}"))?;
    let default_ver = default_version();
    let imported_version = raw
        .get(VERSION_KEY)
        .and_then(|v| v.as_str())
        .unwrap_or(&default_ver);
    let current_version = std::env::var(ENV_VAR_APP_VERSION)
        .map_err(|_| anyhow!("Environment variable {ENV_VAR_APP_VERSION} should be set"))?;
    check_version_compatibility(imported_version, &current_version)?;
    let original_data: ImportElectionEventSchema = deserialize_str(data_str)?;
    replace_ids(data_str, &original_data, event_id, tenant_id.clone())
}

fn reset_imported_election_status(status: ElectionStatus) -> ElectionStatus {
    ElectionStatus {
        is_published: status.is_published,
        init_report: status.init_report,
        allow_tally: status.allow_tally,
        ..ElectionStatus::default()
    }
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
    .map_err(|err| anyhow!("Error getting document for election event ID {election_event_id} and tenant ID {tenant_id}: {err}"))?;

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

            let status: ElectionStatus = clone
                .status
                .clone()
                .map(|value| deserialize_value::<ElectionStatus>(value))
                .transpose()
                .unwrap_or_default()
                .unwrap_or_default();
            let status = reset_imported_election_status(status);

            clone.status = Some(
                serde_json::to_value(status)
                    .with_context(|| "Error serializing election status")?,
            );
            clone.initialization_report_generated = Some(false);

            Ok(clone)
        })
        .collect::<Result<Vec<Election>>>()
        .with_context(|| "Error processing elections")?;

    let language_detection_policy = data.election_event.get_language_detection_policy();
    let mut default_language = None;
    if language_detection_policy == LanguageDetectionPolicy::FORCE_DEFAULT {
        default_language = Some(data.election_event.get_default_language());
    }

    ensure_new_election_event(hasura_transaction, &election_event_id).await?;

    upsert_keycloak_realm(
        tenant_id.as_str(),
        &election_event_id,
        data.keycloak_event_realm.clone(),
        default_language
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
    may_write_secret_attributes: bool,
    secret_write_initiator: Option<&ElectoralLogAdminContext>,
    privileges: ImportUsersPrivileges,
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
        may_write_secret_attributes,
        secret_write_initiator,
        privileges,
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

    let may_write_secret_attributes = object.may_write_secret_attributes;
    let secret_write_initiator = object.secret_write_initiator.clone();
    let privileges = object.privileges;
    let (election_event_schema, mut replacement_map) = process_election_event_file(
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

    // Audit snapshots are referenced by the tally CSV, not the event JSON.
    for (name, _) in &zip_entries {
        if name.starts_with(&format!("{}/", EDocuments::S3_FILES.to_file_name()))
            && name.ends_with(&format!("_{AUDITABLE_BALLOTS_FILE}"))
        {
            if let Some(id) = extract_document_uuid(name).await? {
                replacement_map
                    .entry(id.to_string())
                    .or_insert_with(|| Uuid::new_v4().to_string());
            }
        }
    }

    // Zip file processing
    if document_type == "application/ezip" || matches_mime("zip", &document_type) {
        for (file_name, file_contents) in &zip_entries {
            info!("Importing file: {:?}", file_name);

            let mut cursor = Cursor::new(&file_contents[..]);

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
                    may_write_secret_attributes,
                    secret_write_initiator.as_ref(),
                    privileges,
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
                    &zip_entries,
                )
                .await
                .with_context(|| "Error importing publications")?;
            }
            if file_name.contains(&format!(
                "{}",
                EDocuments::ELECTION_EVENT_CONFIG.to_file_name()
            )) {
                let mut temp_file = NamedTempFile::new()
                    .context("Failed to create election event config temporary file")?;

                io::copy(&mut cursor, &mut temp_file).context(
                    "Failed to copy contents of election event config file to temporary file",
                )?;
                temp_file.as_file_mut().rewind()?;

                import_election_event_config_file(
                    hasura_transaction,
                    &election_event_schema.tenant_id.to_string(),
                    &election_event_schema.election_event.id,
                    temp_file,
                    replacement_map.clone(),
                )
                .await
                .with_context(|| "Error importing election event config file")?;
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

            if file_name.contains(EDocuments::CERTIFICATES.to_file_name()) {
                let pem_content = String::from_utf8(file_contents.clone())
                    .context("Failed to decode certificates PEM as UTF-8")?;
                let tenant_uuid = election_event_schema.tenant_id;
                let election_event_uuid = Uuid::parse_str(&election_event_schema.election_event.id)
                    .context("Failed to parse election event UUID")?;
                let pem_chunks = split_pem_bundle(&pem_content);
                for pem_chunk in pem_chunks {
                    let pem_chunk_owned = pem_chunk.clone();
                    let parsed = tokio::task::spawn_blocking(move || {
                        parse_certificate_pem(&pem_chunk_owned)
                    })
                    .await
                    .context("Failed to spawn blocking task for cert parsing")?
                    .context("Failed to parse certificate PEM")?;
                    let record = CertificateAuthorityRecord {
                        id: Uuid::new_v4(),
                        tenant_id: tenant_uuid,
                        election_event_id: election_event_uuid,
                        common_name: parsed.common_name,
                        subject: parsed.subject,
                        issuer_common_name: parsed.issuer_common_name,
                        issuer: parsed.issuer,
                        not_before: parsed.not_before,
                        not_after: parsed.not_after,
                        fingerprint_sha256: parsed.fingerprint_sha256,
                        serial_number: parsed.serial_number,
                        pem: parsed.pem,
                    };
                    insert_certificate_authority(hasura_transaction, record)
                        .await
                        .context("Failed to insert certificate authority")?;
                }
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

    for scheduled_event in scheduled_events {
        let Some(
            processor @ (EventProcessors::START_VOTING_PERIOD | EventProcessors::END_VOTING_PERIOD),
        ) = scheduled_event.event_processor
        else {
            continue;
        };
        let payload: ManageElectionDatePayload = serde_json::from_value(
            scheduled_event
                .event_payload
                .unwrap_or_else(|| serde_json::json!({})),
        )?;
        if scheduled_event.tenant_id.as_deref() != Some(data.tenant_id.to_string().as_str())
            || scheduled_event.election_event_id.as_deref() != Some(data.election_event.id.as_str())
            || scheduled_event.task_id.as_deref()
                != Some(
                    generate_manage_date_task_name(
                        &data.tenant_id.to_string(),
                        &data.election_event.id,
                        payload.election_id.as_deref(),
                        &processor,
                    )
                    .as_str(),
                )
        {
            continue;
        }
        if payload
            .election_id
            .as_ref()
            .is_some_and(|id| !data.elections.iter().any(|election| &election.id == id))
        {
            continue;
        }
        let Some(date) = scheduled_event
            .cron_config
            .and_then(|config| config.scheduled_date)
        else {
            continue;
        };
        maybe_create_scheduled_event(
            hasura_transaction,
            &data.tenant_id.to_string(),
            &data.election_event.id,
            processor,
            date,
            payload.election_id.as_deref(),
            payload.voting_channels,
        )
        .await?;
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
    voting_channels: Option<Vec<sequent_core::ballot::VotingStatusChannel>>,
) -> Result<()> {
    let start_task_id =
        generate_manage_date_task_name(tenant_id, election_event_id, election_id, &event_processor);
    let payload = ManageElectionDatePayload {
        election_id: election_id.map(str::to_string),
        voting_channels,
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
mod election_status_import_tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use sequent_core::ballot::{InitReport, PeriodDates, VotingStatus, VotingStatusChannel};

    const CHANNELS: [VotingStatusChannel; 4] = [
        VotingStatusChannel::ONLINE,
        VotingStatusChannel::KIOSK,
        VotingStatusChannel::EARLY_VOTING,
        VotingStatusChannel::TELEPHONE,
    ];

    fn used_period_dates() -> PeriodDates {
        let date = Utc.with_ymd_and_hms(2026, 1, 1, 9, 0, 0).unwrap();
        PeriodDates {
            first_started_at: Some(date),
            last_started_at: Some(date),
            first_paused_at: Some(date),
            last_paused_at: Some(date),
            first_stopped_at: Some(date),
            last_stopped_at: Some(date),
        }
    }

    fn status_with_every_channel_open() -> ElectionStatus {
        ElectionStatus {
            is_published: Some(true),
            voting_status: VotingStatus::OPEN,
            init_report: InitReport::DISALLOWED,
            kiosk_voting_status: VotingStatus::OPEN,
            early_voting_status: VotingStatus::OPEN,
            telephone_voting_status: VotingStatus::OPEN,
            voting_period_dates: used_period_dates(),
            kiosk_voting_period_dates: used_period_dates(),
            early_voting_period_dates: used_period_dates(),
            telephone_voting_period_dates: used_period_dates(),
            allow_tally: AllowTallyStatus::DISALLOWED,
        }
    }

    #[test]
    fn imported_election_status_resets_every_channel() {
        let status = reset_imported_election_status(status_with_every_channel_open());

        for channel in CHANNELS {
            assert_eq!(
                status.status_by_channel(channel),
                VotingStatus::NOT_STARTED,
                "{channel:?} status"
            );
            assert_eq!(
                status.dates_by_channel(channel),
                PeriodDates::default(),
                "{channel:?} dates"
            );
        }
    }

    #[test]
    fn imported_election_status_keeps_publication_and_tally_settings() {
        let status = reset_imported_election_status(status_with_every_channel_open());

        assert_eq!(status.is_published, Some(true));
        assert_eq!(status.init_report, InitReport::DISALLOWED);
        assert_eq!(status.allow_tally, AllowTallyStatus::DISALLOWED);
    }
}

#[cfg(test)]
mod publication_import_mapping_tests {
    use super::*;

    #[test]
    fn includes_scope_mappings_from_the_event_import_path() {
        let tenant = Uuid::new_v4().to_string();
        let event = Uuid::new_v4().to_string();
        let document = Uuid::new_v4().to_string();
        let input = serde_json::json!({
            "tenant_id": tenant,
            "election_event": {
                "id": event, "tenant_id": tenant, "is_archived": false,
                "encryption_protocol": "RSA", "annotations": {"document_id": document}
            },
            "elections": [], "contests": [], "candidates": [], "areas": [],
            "area_contests": [], "reports": []
        });
        let original: ImportElectionEventSchema = serde_json::from_value(input.clone()).unwrap();
        for target_tenant in [tenant.clone(), Uuid::new_v4().to_string()] {
            for target_event in [None, Some(event.clone()), Some(Uuid::new_v4().to_string())] {
                let (imported, ids) = replace_ids(
                    &input.to_string(),
                    &original,
                    target_event.clone(),
                    target_tenant.clone(),
                )
                .unwrap();
                assert_eq!(ids.get(&tenant), Some(&target_tenant));
                assert_eq!(ids.get(&event), Some(&imported.election_event.id));
                assert_eq!(imported.election_event.tenant_id, target_tenant);
                if let Some(expected) = target_event {
                    assert_eq!(imported.election_event.id, expected);
                } else {
                    assert_ne!(imported.election_event.id, event);
                }
                assert_ne!(ids[&document], document);
                assert_eq!(
                    imported.election_event.annotations.unwrap()["document_id"],
                    ids[&document]
                );
            }
        }
    }
}

#[cfg(test)]
mod imported_realm_sanitization_tests {
    use super::*;
    use ::keycloak::types::{
        ClientRepresentation, ClientScopeRepresentation, RoleRepresentation, UserRepresentation,
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

    fn user_profile(realm: &RealmRepresentation) -> serde_json::Value {
        user_profile_component(realm)
            .and_then(user_profile_config)
            .expect("realm has a user profile")
    }

    fn profile_attribute<'a>(
        profile: &'a serde_json::Value,
        name: &str,
    ) -> Option<&'a serde_json::Value> {
        profile["attributes"]
            .as_array()?
            .iter()
            .find(|attribute| attribute["name"] == name)
    }

    /// The template with its user profile edited as an uploaded export could be.
    fn with_user_profile(edit: impl FnOnce(&mut serde_json::Value)) -> RealmRepresentation {
        let mut realm = template_realm();
        let mut profile = user_profile(&realm);
        edit(&mut profile);
        realm
            .components
            .as_mut()
            .and_then(|components| components.get_mut(USER_PROFILE_PROVIDER_COMPONENT))
            .and_then(|components| components.first_mut())
            .and_then(|component| component.config.as_mut())
            .expect("template has a user-profile config")
            .insert(
                USER_PROFILE_CONFIG_KEY.to_string(),
                vec![profile.to_string()],
            );
        realm
    }

    #[test]
    fn aligning_resets_user_profile_permissions_the_template_defines() {
        let tampered = with_user_profile(|profile| {
            for attribute in profile["attributes"].as_array_mut().unwrap() {
                if attribute["name"] == "tenant-id" || attribute["name"] == "area-id" {
                    attribute["permissions"]["edit"] = serde_json::json!(["admin", "user"]);
                }
            }
        });

        let profile = user_profile(&sanitize(&tampered));
        let template_profile = user_profile(&template_realm());
        for name in ["tenant-id", "area-id"] {
            assert_eq!(
                profile_attribute(&profile, name).expect("attribute kept")["permissions"],
                profile_attribute(&template_profile, name).expect("template attribute")
                    ["permissions"],
                "{name} must have the template's permissions",
            );
        }

        let error = sanitize_imported_event_realm(
            &tampered,
            &template_realm(),
            ImportedRealmReconciliation::RejectForeignClaims,
        )
        .expect_err("a user-editable tenant-id is reported");
        assert!(error
            .to_string()
            .contains("user-profile permissions of attribute 'tenant-id'"));
    }

    #[test]
    fn aligning_restores_a_removed_attribute_and_the_unmanaged_policy() {
        let tampered = with_user_profile(|profile| {
            profile["attributes"]
                .as_array_mut()
                .unwrap()
                .retain(|attribute| attribute["name"] != "tenant-id");
            profile[UNMANAGED_ATTRIBUTE_POLICY_KEY] = serde_json::json!("ENABLED");
        });

        let profile = user_profile(&sanitize(&tampered));
        let template_profile = user_profile(&template_realm());
        assert_eq!(
            profile_attribute(&profile, "tenant-id"),
            profile_attribute(&template_profile, "tenant-id"),
        );
        assert_eq!(
            profile.get(UNMANAGED_ATTRIBUTE_POLICY_KEY),
            template_profile.get(UNMANAGED_ATTRIBUTE_POLICY_KEY),
        );
    }

    #[test]
    fn aligning_keeps_an_attribute_only_the_upload_declares() {
        let tampered = with_user_profile(|profile| {
            profile["attributes"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "name": "c31-extra",
                    "permissions": {"view": ["admin", "user"], "edit": ["admin", "user"]}
                }));
        });

        let profile = user_profile(&sanitize(&tampered));
        assert!(profile_attribute(&profile, "c31-extra").is_some());
    }

    #[test]
    fn aligning_replaces_a_missing_user_profile_with_the_template() {
        let mut realm = template_realm();
        realm
            .components
            .as_mut()
            .unwrap()
            .remove(USER_PROFILE_PROVIDER_COMPONENT);

        assert_eq!(
            user_profile(&sanitize(&realm)),
            user_profile(&template_realm())
        );
    }

    #[test]
    fn a_template_without_a_readable_user_profile_fails_the_import() {
        let tampered = with_user_profile(|profile| {
            for attribute in profile["attributes"].as_array_mut().unwrap() {
                if attribute["name"] == "tenant-id" {
                    attribute["permissions"]["edit"] = serde_json::json!(["admin", "user"]);
                }
            }
        });
        let mut without_profile = template_realm();
        without_profile
            .components
            .as_mut()
            .unwrap()
            .remove(USER_PROFILE_PROVIDER_COMPONENT);
        let mut unreadable_profile = template_realm();
        unreadable_profile
            .components
            .as_mut()
            .and_then(|components| components.get_mut(USER_PROFILE_PROVIDER_COMPONENT))
            .and_then(|components| components.first_mut())
            .and_then(|component| component.config.as_mut())
            .expect("template has a user-profile config")
            .insert(
                USER_PROFILE_CONFIG_KEY.to_string(),
                vec!["not a profile".to_string()],
            );

        for template in [without_profile, unreadable_profile] {
            let error = sanitize_imported_event_realm(
                &tampered,
                &template,
                ImportedRealmReconciliation::AlignWithTemplate,
            )
            .expect_err("the uploaded permissions can't be checked without the template's");
            assert!(error.to_string().contains("no readable user profile"));
        }
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

    fn default_role_composites(realm: &RealmRepresentation) -> Option<Composites> {
        realm
            .roles
            .iter()
            .flat_map(|roles| roles.realm.iter().flatten())
            .find(|role| {
                role.name
                    .as_deref()
                    .is_some_and(|name| name.starts_with(DEFAULT_REALM_ROLE_PREFIX))
            })
            .and_then(|role| role.composites.clone())
    }

    fn realm_admin_composites() -> Composites {
        Composites {
            client: Some(HashMap::from([(
                REALM_ADMIN_CLIENT.to_string(),
                vec![REALM_ADMIN_ROLE.to_string()],
            )])),
            ..Default::default()
        }
    }

    fn reject(imported: &RealmRepresentation) -> Result<RealmRepresentation> {
        sanitize_imported_event_realm(
            imported,
            &template_realm(),
            ImportedRealmReconciliation::RejectForeignClaims,
        )
    }

    #[test]
    fn aligning_the_template_keeps_the_default_role_composites() {
        let template = template_realm();
        assert!(default_role_composites(&template).is_some());
        assert_eq!(
            default_role_composites(&sanitize(&template)),
            default_role_composites(&template)
        );
    }

    #[test]
    fn aligning_the_template_does_not_duplicate_its_mappers() {
        let mapper_names = |mappers: &Option<Vec<ProtocolMapperRepresentation>>| {
            let mut names: Vec<_> = mappers
                .iter()
                .flatten()
                .filter_map(|mapper| mapper.name.clone())
                .collect();
            names.sort();
            names
        };
        let template = template_realm();
        let sanitized = sanitize(&template);
        for (sanitized_client, client) in sanitized
            .clients
            .iter()
            .flatten()
            .zip(template.clients.iter().flatten())
        {
            assert_eq!(
                mapper_names(&sanitized_client.protocol_mappers),
                mapper_names(&client.protocol_mappers)
            );
        }
        for (sanitized_scope, scope) in sanitized
            .client_scopes
            .iter()
            .flatten()
            .zip(template.client_scopes.iter().flatten())
        {
            assert_eq!(
                mapper_names(&sanitized_scope.protocol_mappers),
                mapper_names(&scope.protocol_mappers)
            );
        }
    }

    #[test]
    fn a_mapper_adding_a_role_to_the_token_is_claim_authority() {
        let mut realm = template_realm();
        let mapper = ProtocolMapperRepresentation {
            name: Some("grant-realm-admin".to_string()),
            protocol: Some("openid-connect".to_string()),
            protocol_mapper: Some("oidc-hardcoded-role-mapper".to_string()),
            config: Some(HashMap::from([(
                "role".to_string(),
                format!("{REALM_ADMIN_CLIENT}.{REALM_ADMIN_ROLE}"),
            )])),
            ..Default::default()
        };
        realm
            .clients
            .iter_mut()
            .flatten()
            .find(|client| client.client_id.as_deref() == Some("voting-portal"))
            .expect("template has the voting portal")
            .protocol_mappers
            .get_or_insert_with(Vec::new)
            .push(mapper);

        assert!(reject(&realm).is_err());
        let sanitized = sanitize(&realm);
        assert!(!client(&sanitized, "voting-portal")
            .protocol_mappers
            .iter()
            .flatten()
            .any(|mapper| mapper.name.as_deref() == Some("grant-realm-admin")));
    }

    #[test]
    fn a_foreign_composite_on_an_allowed_realm_role_is_reported() {
        let mut realm = template_realm();
        let allowed_role = realm_role_names(&realm)
            .into_iter()
            .find(|name| !name.starts_with(DEFAULT_REALM_ROLE_PREFIX))
            .expect("template has a realm role");
        for role in realm
            .roles
            .iter_mut()
            .flat_map(|roles| roles.realm.iter_mut().flatten())
        {
            if role.name.as_deref() == Some(allowed_role.as_str()) {
                role.composites = Some(realm_admin_composites());
            }
        }

        let error = reject(&realm).expect_err("the composite is foreign");
        assert!(error.to_string().contains(REALM_ADMIN_ROLE), "{error}");
    }

    #[test]
    fn a_foreign_composite_on_the_default_role_is_reported() {
        let mut realm = template_realm();
        realm
            .default_role
            .as_mut()
            .expect("template has a default role")
            .composites = Some(realm_admin_composites());

        let error = reject(&realm).expect_err("the composite is foreign");
        assert!(error.to_string().contains(REALM_ADMIN_ROLE), "{error}");
    }

    #[test]
    fn a_client_role_cannot_inherit_what_it_cannot_be_assigned() {
        let mut realm = template_realm();
        let manage_account = realm
            .roles
            .iter_mut()
            .flat_map(|roles| roles.client.iter_mut().flatten())
            .find(|(client_id, _)| client_id.as_str() == "account")
            .expect("template has account roles")
            .1
            .iter_mut()
            .find(|role| role.name.as_deref() == Some("manage-account"))
            .expect("template has manage-account");
        manage_account.composites = Some(realm_admin_composites());

        let error = reject(&realm).expect_err("the composite is foreign");
        assert!(error.to_string().contains(REALM_ADMIN_ROLE), "{error}");

        let composites_of = |realm: &RealmRepresentation| {
            realm
                .roles
                .iter()
                .flat_map(|roles| roles.client.iter().flatten())
                .filter(|(client_id, _)| client_id.as_str() == "account")
                .flat_map(|(_, roles)| roles.iter())
                .find(|role| role.name.as_deref() == Some("manage-account"))
                .and_then(|role| role.composites.clone())
        };
        assert_eq!(
            composites_of(&sanitize(&realm)),
            composites_of(&template_realm())
        );
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

    #[test]
    fn aligning_restores_a_missing_template_claim_mapper() {
        let mut realm = template_realm();
        realm
            .clients
            .iter_mut()
            .flatten()
            .find(|client| client.client_id.as_deref() == Some("voting-portal"))
            .and_then(|client| client.protocol_mappers.as_mut())
            .expect("voting-portal mappers present")
            .retain(|mapper| mapper.name.as_deref() != Some("x-hasura-tenant-id"));

        let sanitized = sanitize(&realm);

        assert_eq!(
            client_claim_mapper_count(&sanitized, "voting-portal"),
            client_claim_mapper_count(&template_realm(), "voting-portal"),
        );
    }

    #[test]
    fn aligning_leaves_other_mappers_as_uploaded() {
        let mut realm = template_realm();
        realm
            .client_scopes
            .iter_mut()
            .flatten()
            .find(|scope| scope.name.as_deref() == Some("email"))
            .and_then(|scope| scope.protocol_mappers.as_mut())
            .expect("email scope mappers present")
            .retain(|mapper| mapper.name.as_deref() != Some("email verified"));

        let sanitized = sanitize(&realm);

        let names: Vec<_> = sanitized
            .client_scopes
            .iter()
            .flatten()
            .find(|scope| scope.name.as_deref() == Some("email"))
            .and_then(|scope| scope.protocol_mappers.as_deref())
            .unwrap_or_default()
            .iter()
            .filter_map(|mapper| mapper.name.as_deref())
            .collect();
        assert_eq!(names, vec!["email"]);
    }

    /// Composite members that name a role the realm does not define, which
    /// Keycloak refuses when it creates the realm.
    fn undefined_composite_members(realm: &RealmRepresentation) -> Vec<String> {
        let realm_roles = realm_role_names(realm);
        let client_roles: HashMap<String, HashSet<String>> = realm
            .roles
            .iter()
            .flat_map(|roles| roles.client.iter().flatten())
            .map(|(client_id, roles)| {
                let names = roles.iter().filter_map(|role| role.name.clone()).collect();
                (client_id.clone(), names)
            })
            .collect();
        let roles = realm.roles.iter().flat_map(|roles| {
            roles
                .realm
                .iter()
                .flatten()
                .chain(roles.client.iter().flatten().flat_map(|(_, roles)| roles))
        });
        let mut undefined = Vec::new();
        for composites in roles.filter_map(|role| role.composites.as_ref()) {
            for name in composites.realm.iter().flatten() {
                if !realm_roles.contains(name) {
                    undefined.push(name.clone());
                }
            }
            for (client_id, names) in composites.client.iter().flatten() {
                for name in names {
                    if !client_roles
                        .get(client_id)
                        .is_some_and(|defined| defined.contains(name))
                    {
                        undefined.push(format!("{client_id}/{name}"));
                    }
                }
            }
        }
        undefined
    }

    #[test]
    fn aligning_keeps_only_the_composites_the_upload_defines() {
        // Exports from before Keycloak 26.7 have no organization roles, which
        // the template's realm-admin composes.
        let organization_roles = [
            "manage-organizations",
            "query-organizations",
            "view-organizations",
        ];
        let mut realm = template_realm();
        let realm_management = realm
            .roles
            .as_mut()
            .and_then(|roles| roles.client.as_mut())
            .and_then(|clients| clients.get_mut(REALM_ADMIN_CLIENT))
            .expect("template defines realm-management roles");
        realm_management.retain(|role| {
            !role
                .name
                .as_deref()
                .is_some_and(|name| organization_roles.contains(&name))
        });
        for role in realm_management.iter_mut() {
            if let Some(names) = role
                .composites
                .as_mut()
                .and_then(|composites| composites.client.as_mut())
                .and_then(|clients| clients.get_mut(REALM_ADMIN_CLIENT))
            {
                names.retain(|name| !organization_roles.contains(&name.as_str()));
            }
        }
        assert!(undefined_composite_members(&realm).is_empty());

        let sanitized = sanitize(&realm);

        assert_eq!(
            undefined_composite_members(&sanitized),
            Vec::<String>::new()
        );
        let realm_admin_members = sanitized
            .roles
            .iter()
            .flat_map(|roles| roles.client.iter().flatten())
            .filter(|(client_id, _)| client_id.as_str() == REALM_ADMIN_CLIENT)
            .flat_map(|(_, roles)| roles)
            .find(|role| role.name.as_deref() == Some(REALM_ADMIN_ROLE))
            .and_then(|role| role.composites.as_ref())
            .and_then(|composites| composites.client.as_ref())
            .and_then(|clients| clients.get(REALM_ADMIN_CLIENT))
            .cloned()
            .unwrap_or_default();
        assert!(realm_admin_members.contains(&"manage-users".to_string()));
    }

    #[test]
    fn aligning_keeps_the_kiosk_client_claim_mappers() {
        let template = template_realm();
        let kiosk_mappers = client_claim_mapper_count(&template, "voting-portal-kiosk");

        assert_eq!(
            kiosk_mappers,
            client_claim_mapper_count(&template, "voting-portal")
        );
        assert_eq!(
            client_claim_mapper_count(&sanitize(&template), "voting-portal-kiosk"),
            kiosk_mappers
        );
    }
}

#[cfg(test)]
mod import_scope_tests {
    use super::*;

    const ROW_KINDS: [&str; 6] = [
        "elections",
        "contests",
        "candidates",
        "areas",
        "keys_ceremonies",
        "applications",
    ];

    fn new_id() -> String {
        Uuid::new_v4().to_string()
    }

    fn bundle(tenant: &str, event: &str) -> Value {
        let election = new_id();
        json!({
            "tenant_id": tenant,
            "election_event": {
                "id": event, "tenant_id": tenant, "is_archived": false,
                "encryption_protocol": "RSA"
            },
            "elections": [
                {"id": election, "tenant_id": tenant, "election_event_id": event}
            ],
            "contests": [{
                "id": new_id(), "tenant_id": tenant, "election_event_id": event,
                "election_id": election
            }],
            "candidates": [
                {"id": new_id(), "tenant_id": tenant, "election_event_id": event}
            ],
            "areas": [
                {"id": new_id(), "tenant_id": tenant, "election_event_id": event}
            ],
            "area_contests": [],
            "reports": [],
            "keys_ceremonies": [{
                "id": new_id(), "tenant_id": tenant, "election_event_id": event,
                "trustee_ids": [], "threshold": 2
            }],
            "applications": [{
                "id": new_id(), "tenant_id": tenant, "election_event_id": event,
                "applicant_id": new_id(), "applicant_data": {},
                "verification_type": "MANUAL", "status": "PENDING"
            }]
        })
    }

    fn import(
        input: &Value,
        event_id: Option<String>,
        tenant_id: &str,
    ) -> Result<ImportElectionEventSchema> {
        let original: ImportElectionEventSchema = serde_json::from_value(input.clone())?;
        replace_ids(
            &input.to_string(),
            &original,
            event_id,
            tenant_id.to_string(),
        )
        .map(|(data, _)| data)
    }

    #[test]
    fn accepts_rows_of_the_imported_event() {
        let tenant = new_id();
        let event = new_id();
        let input = bundle(&tenant, &event);
        for target_tenant in [tenant.clone(), new_id()] {
            for target_event in [None, Some(new_id())] {
                let imported = import(&input, target_event.clone(), &target_tenant).unwrap();
                if let Some(expected) = target_event {
                    assert_eq!(imported.election_event.id, expected);
                }
                assert_eq!(imported.election_event.tenant_id, target_tenant);
                let scopes = import_row_scopes(&imported);
                assert_eq!(scopes.len(), ROW_KINDS.len());
                for (_, row_tenant, row_event) in scopes {
                    assert_eq!(row_tenant, target_tenant);
                    assert_eq!(row_event, imported.election_event.id);
                }
            }
        }
    }

    #[test]
    fn picks_a_new_event_id_even_when_the_bundle_lists_it_for_keeping() {
        let tenant = new_id();
        let event = new_id();
        let mut input = bundle(&tenant, &event);
        input["authenticatorConfig"] = json!([{"alias": "kept", "config": {"id": event}}]);
        let imported = import(&input, None, &tenant).unwrap();
        assert_ne!(imported.election_event.id, event);
        assert!(!serde_json::to_string(&imported).unwrap().contains(&event));
    }

    #[test]
    fn rejects_rows_of_another_tenant() {
        let tenant = new_id();
        let event = new_id();
        let other_tenant = Uuid::new_v4().simple().to_string();
        for kind in ROW_KINDS {
            let mut input = bundle(&tenant, &event);
            input[kind][0]["tenant_id"] = json!(other_tenant);
            assert!(
                import(&input, Some(new_id()), &tenant).is_err(),
                "{kind} row of another tenant was accepted"
            );
        }
        let mut input = bundle(&tenant, &event);
        input["election_event"]["tenant_id"] = json!(other_tenant);
        assert!(import(&input, Some(new_id()), &tenant).is_err());
    }

    /// A row whose tenant id is not a UUID is reported with that value, not
    /// as a row of another tenant.
    #[test]
    fn names_a_row_tenant_id_that_is_not_a_uuid() {
        let tenant = new_id();
        let mut input = bundle(&tenant, &new_id());
        input["areas"][0]["tenant_id"] = json!("not-a-uuid");
        let error = import(&input, Some(new_id()), &tenant).unwrap_err();
        assert!(
            format!("{error:#}").contains("Invalid tenant id not-a-uuid"),
            "{error:#}"
        );
    }

    #[test]
    fn rejects_rows_of_another_event() {
        let tenant = new_id();
        let event = new_id();
        let other_event = new_id();
        for kind in ROW_KINDS {
            let mut input = bundle(&tenant, &event);
            input["authenticatorConfig"] =
                json!([{"alias": "kept", "config": {"id": other_event}}]);
            input[kind][0]["election_event_id"] = json!(other_event);
            assert!(
                import(&input, Some(new_id()), &tenant).is_err(),
                "{kind} row of another event was accepted"
            );
        }
    }
}

#[cfg(test)]
mod new_election_event_tests {
    use super::*;
    use crate::services::database::generate_hasura_pool;
    use tokio_postgres::error::SqlState;

    const ELECTION_EVENT_TABLE: &str = r#"
        CREATE SCHEMA IF NOT EXISTS sequent_backend;
        CREATE TABLE sequent_backend.election_event (
            id UUID PRIMARY KEY,
            tenant_id UUID NOT NULL
        );
    "#;

    /// Election event ids are unique across tenants, so an id is taken as soon
    /// as any tenant uses it.
    #[tokio::test]
    #[ignore = "requires an empty PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
    async fn rejects_an_election_event_id_used_in_another_tenant() -> Result<()> {
        let pool = generate_hasura_pool().await?;
        let mut client = pool.get().await?;
        let transaction = client.transaction().await?;
        transaction.batch_execute(ELECTION_EVENT_TABLE).await?;
        let used_id = Uuid::new_v4();
        transaction
            .execute(
                "INSERT INTO sequent_backend.election_event (id, tenant_id) VALUES ($1, $2)",
                &[&used_id, &Uuid::new_v4()],
            )
            .await?;

        assert!(
            ensure_new_election_event(&transaction, &used_id.to_string())
                .await
                .is_err()
        );
        assert!(
            ensure_new_election_event(&transaction, &Uuid::new_v4().to_string())
                .await
                .is_ok()
        );

        transaction.rollback().await?;
        Ok(())
    }

    /// A second create or import of the same id waits for the first one's
    /// transaction to end instead of setting the event up alongside it.
    #[tokio::test]
    #[ignore = "requires an empty PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
    async fn holds_the_election_event_id_until_the_transaction_ends() -> Result<()> {
        let pool = generate_hasura_pool().await?;
        let mut first_client = pool.get().await?;
        let mut second_client = pool.get().await?;
        let election_event_id = Uuid::new_v4().to_string();

        let first = first_client.transaction().await?;
        first.batch_execute(ELECTION_EVENT_TABLE).await?;
        ensure_new_election_event(&first, &election_event_id).await?;

        let second = second_client.transaction().await?;
        second
            .batch_execute("SET LOCAL lock_timeout = '200ms'")
            .await?;
        let error = ensure_new_election_event(&second, &election_event_id)
            .await
            .expect_err("a second setup of the same id must wait for the first");
        assert_eq!(
            error
                .downcast_ref::<tokio_postgres::Error>()
                .and_then(tokio_postgres::Error::code),
            Some(&SqlState::LOCK_NOT_AVAILABLE),
            "{error:#}"
        );

        second.rollback().await?;
        first.rollback().await?;
        Ok(())
    }
}
