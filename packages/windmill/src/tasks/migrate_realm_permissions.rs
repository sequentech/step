// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Adds permissions that a release introduces to the tenant realms made
//! before it. New tenant realms get them from their template; existing ones
//! get the missing realm roles through Keycloak's partial import, skipping
//! any role the realm already has.
//!
//! Signing roles stay a choice made in Users and Roles. The messaging roles
//! replace the `admin-user` reads the release removes, so without a group
//! every administrator of an existing tenant would lose the event Messaging
//! tab, Logs and Send Notification: the groups `admin` and `admin-light`
//! get them as the realm template gives them ([`TEMPLATE_GROUP_GRANTS`]).
//! Readiness verification is granted only to the administrator group.
//! Any group that manages messaging accounts or configurations can read
//! the configuration. Only missing mappings are added; each is logged.

use crate::postgres::tenant::get_tenant_ids;
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context};
use celery::beat::Schedule;
use celery::task::{Task, TaskResult};
use deadpool_postgres::Client as DbClient;
use keycloak::types::{GroupRepresentation, RoleRepresentation};
use keycloak::KeycloakTokenSupplier;
use sequent_core::services::keycloak::{
    get_tenant_realm, partial_import_realm_roles, IfResourceExists, KeycloakAdminClient,
    PartialImportSummary, PubKeycloakAdmin,
};
use sequent_core::types::permissions::Permissions;
use std::collections::BTreeMap;
use std::time::SystemTime;
use tracing::{error, info, instrument};

/// The permissions the migration adds, with the English label Users and
/// Roles shows for each, which becomes the role's description.
pub const MIGRATED_PERMISSIONS: [(Permissions, &str); 28] = [
    (
        Permissions::FORENSIC_READ,
        "Read isolated forensic network data",
    ),
    (
        Permissions::VOTER_PSEUDONYM_RESOLVE,
        "Resolve a voter pseudonym with an audited reason",
    ),
    (
        Permissions::ELECTION_EVENT_SIGNATURES_TAB,
        "Election Event Signatures Tab",
    ),
    (
        Permissions::SIGNING_RULES_READ,
        "Signatures: read protected actions",
    ),
    (
        Permissions::SIGNING_RULES_WRITE,
        "Signatures: edit protected actions",
    ),
    (
        Permissions::SIGNING_CERTIFICATES_READ,
        "Signatures: read certificates",
    ),
    (
        Permissions::SIGNING_ISSUERS_WRITE,
        "Signatures: import and remove trusted issuers",
    ),
    (
        Permissions::SIGNING_CHECKS_WRITE,
        "Signatures: edit certificate checks",
    ),
    (
        Permissions::SIGNING_CERTIFICATES_REGISTER,
        "Signatures: register certificates",
    ),
    (
        Permissions::SIGNING_CERTIFICATES_REVOKE,
        "Signatures: revoke certificates",
    ),
    (
        Permissions::SIGNING_REQUESTS_READ,
        "Signatures: read requests",
    ),
    (
        Permissions::SIGNING_REQUESTS_CANCEL,
        "Signatures: cancel requests",
    ),
    (
        Permissions::SIGNING_REQUESTS_EXPORT,
        "Signatures: export requests",
    ),
    (
        Permissions::SIGN_INITIALIZE_VOTING,
        "Sign: initialize voting",
    ),
    (Permissions::SIGN_OPEN_VOTING, "Sign: open voting"),
    (Permissions::SIGN_CLOSE_VOTING, "Sign: close voting"),
    (
        Permissions::SIGN_GENERATE_ELECTION_RETURNS,
        "Sign: generate election returns",
    ),
    (
        Permissions::SIGN_GENERATE_REPORTS,
        "Sign: generate other election reports",
    ),
    (Permissions::SIGN_TRANSMIT_RESULTS, "Sign: transmit results"),
    (
        Permissions::SIGN_APPROVE_VOTER,
        "Sign: approve a voter manually",
    ),
    (
        Permissions::SIGN_APPROVE_CONFIGURATION,
        "Sign: approve a configuration version",
    ),
    (Permissions::SIGN_KEY_CEREMONY, "Sign: confirm custody"),
    (Permissions::SIGN_TALLY_KEY, "Sign: authorize tally custody"),
    (Permissions::READINESS_CHECK, "Verify election readiness"),
    (
        Permissions::MESSAGING_ACCOUNT_READ,
        "Read messaging accounts",
    ),
    (
        Permissions::MESSAGING_ACCOUNT_WRITE,
        "Manage messaging accounts",
    ),
    (
        Permissions::MESSAGING_CONFIG_WRITE,
        "Configure election event messaging",
    ),
    (
        Permissions::MESSAGING_CONFIG_READ,
        "Read election event messaging",
    ),
];

/// Roles each template group receives when upgrading an existing realm;
/// existing realms' groups get the ones they lack.
pub const TEMPLATE_GROUP_GRANTS: [(&str, &[Permissions]); 2] = [
    (
        "admin",
        &[
            Permissions::MESSAGING_ACCOUNT_READ,
            Permissions::MESSAGING_ACCOUNT_WRITE,
            Permissions::MESSAGING_CONFIG_WRITE,
            Permissions::MESSAGING_CONFIG_READ,
            Permissions::READINESS_CHECK,
        ],
    ),
    (
        "admin-light",
        &[
            Permissions::MESSAGING_ACCOUNT_READ,
            Permissions::MESSAGING_CONFIG_READ,
        ],
    ),
];

/// A group holding one of these can also read the event messaging
/// configuration: writing or reading accounts without it hides the tab.
const READ_FOLLOWS: [Permissions; 2] = [
    Permissions::MESSAGING_CONFIG_WRITE,
    Permissions::MESSAGING_ACCOUNT_READ,
];

/// The roles one group was granted, by its path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupGrant {
    pub group: String,
    pub roles: Vec<String>,
}

/// What one realm's migration did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealmMigration {
    pub realm: String,
    pub roles: PartialImportSummary,
    pub granted: Vec<GroupGrant>,
}

pub(crate) async fn admin_get<T: serde::de::DeserializeOwned>(
    client: &PubKeycloakAdmin,
    path: &str,
    query: &[(&str, &str)],
) -> anyhow::Result<T> {
    let response = client
        .client
        .get(format!("{}/admin/realms/{path}", client.url))
        .query(query)
        .bearer_auth(client.token_supplier.get(&client.url).await?)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("GET {path}: HTTP {status}"));
    }
    Ok(response.json().await?)
}

/// The top-level group named `name`, if the realm has one.
pub(crate) async fn top_level_group(
    client: &PubKeycloakAdmin,
    realm: &str,
    name: &str,
) -> anyhow::Result<Option<GroupRepresentation>> {
    let found: Vec<GroupRepresentation> = admin_get(
        client,
        &format!("{realm}/groups"),
        &[
            ("search", name),
            ("exact", "true"),
            ("briefRepresentation", "true"),
        ],
    )
    .await?;
    let path = format!("/{name}");
    Ok(found.into_iter().find(|group| {
        group.name.as_deref() == Some(name) && group.path.as_deref() == Some(path.as_str())
    }))
}

/// Grants each group the release roles it lacks: the template's to
/// `admin` and `admin-light`, and the configuration read to the groups
/// holding a role in [`READ_FOLLOWS`]. Returns what was granted.
#[instrument(skip(client), err)]
pub async fn grant_template_roles(
    client: &PubKeycloakAdmin,
    realm: &str,
) -> anyhow::Result<Vec<GroupGrant>> {
    // Group id → (path, roles it should hold), in a stable order.
    let mut wanted: Vec<(String, String, Vec<String>)> = vec![];
    let mut want = |id: String, path: String, roles: Vec<String>| match wanted
        .iter_mut()
        .find(|(known, _, _)| *known == id)
    {
        Some((_, _, held)) => {
            for role in roles {
                if !held.contains(&role) {
                    held.push(role);
                }
            }
        }
        None => wanted.push((id, path, roles)),
    };
    for (name, roles) in TEMPLATE_GROUP_GRANTS {
        if let Some(group) = top_level_group(client, realm, name).await? {
            if let Some(id) = group.id {
                want(
                    id.to_string(),
                    group
                        .path
                        .map(|path| path.to_string())
                        .unwrap_or_else(|| format!("/{name}")),
                    roles.iter().map(|role| role.to_string()).collect(),
                );
            }
        }
    }
    for role in READ_FOLLOWS {
        let mut holders: Vec<GroupRepresentation> = admin_get(
            client,
            &format!("{realm}/roles/{role}/groups"),
            &[("briefRepresentation", "true")],
        )
        .await?;
        holders.sort_by(|a, b| a.path.cmp(&b.path));
        for group in holders {
            if let (Some(id), Some(path)) = (group.id, group.path) {
                want(
                    id.to_string(),
                    path.to_string(),
                    vec![Permissions::MESSAGING_CONFIG_READ.to_string()],
                );
            }
        }
    }

    let mut roles: BTreeMap<String, RoleRepresentation> = BTreeMap::new();
    let mut granted = vec![];
    for (id, path, should_hold) in wanted {
        let held: Vec<RoleRepresentation> = admin_get(
            client,
            &format!("{realm}/groups/{id}/role-mappings/realm"),
            &[],
        )
        .await?;
        let missing: Vec<String> = should_hold
            .into_iter()
            .filter(|role| !held.iter().any(|h| h.name.as_deref() == Some(role)))
            .collect();
        if missing.is_empty() {
            continue;
        }
        let mut mapping = vec![];
        for name in &missing {
            if !roles.contains_key(name) {
                let role: RoleRepresentation =
                    admin_get(client, &format!("{realm}/roles/{name}"), &[]).await?;
                roles.insert(name.clone(), role);
            }
            let role = &roles[name];
            mapping.push(serde_json::json!({"id": role.id, "name": role.name}));
        }
        let url = format!(
            "{}/admin/realms/{realm}/groups/{id}/role-mappings/realm",
            client.url
        );
        let response = client
            .client
            .post(&url)
            .bearer_auth(client.token_supplier.get(&client.url).await?)
            .json(&mapping)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(anyhow!("granting {missing:?} to {path}: HTTP {status}"));
        }
        info!(realm, group = %path, roles = ?missing, "Release roles granted to a group");
        granted.push(GroupGrant {
            group: path,
            roles: missing,
        });
    }
    Ok(granted)
}

/// The realm roles the migration imports: plain realm roles without ids,
/// so Keycloak gives each realm its own.
pub fn migrated_roles() -> Vec<RoleRepresentation> {
    MIGRATED_PERMISSIONS
        .iter()
        .map(|(permission, label)| RoleRepresentation {
            name: Some(permission.to_string()),
            description: Some(label.to_string()),
            composite: Some(false),
            client_role: Some(false),
            ..Default::default()
        })
        .collect()
}

/// Imports [`migrated_roles`] into the realm of each tenant, skipping the
/// roles a realm already has, then grants the template roles to its groups
/// ([`grant_template_roles`]). Every realm is tried; if any failed the
/// result is an error naming them, after the others were migrated.
#[instrument(skip(client), err)]
pub async fn migrate_realms(
    client: &PubKeycloakAdmin,
    tenant_ids: &[String],
) -> anyhow::Result<Vec<RealmMigration>> {
    let roles = migrated_roles();
    let mut migrated = vec![];
    let mut failed = vec![];
    for tenant_id in tenant_ids {
        let realm = get_tenant_realm(tenant_id);
        let summary = match partial_import_realm_roles(
            client,
            &realm,
            &roles,
            IfResourceExists::Skip,
        )
        .await
        {
            Ok(summary) => summary,
            Err(err) => {
                error!(realm, "Realm permissions not migrated: {err:?}");
                failed.push(realm);
                continue;
            }
        };
        match grant_template_roles(client, &realm).await {
            Ok(granted) => {
                info!(
                    realm,
                    added = summary.added,
                    skipped = summary.skipped,
                    groups = granted.len(),
                    "Realm permissions migrated"
                );
                migrated.push(RealmMigration {
                    realm,
                    roles: summary,
                    granted,
                });
            }
            Err(err) => {
                error!(
                    realm,
                    "Messaging roles not granted to the realm's groups: {err:?}"
                );
                failed.push(realm);
            }
        }
    }
    if failed.is_empty() {
        Ok(migrated)
    } else {
        Err(anyhow!(
            "Realm permissions not migrated in {}",
            failed.join(", ")
        ))
    }
}

/// [`migrate_realms`] for every tenant in the database, with the database
/// and the Keycloak admin credentials the environment names. First marks
/// the migrated event messaging configurations a save would refuse
/// ([`crate::services::messaging::config::mark_migrated_configs`]). The tenants
/// are read first and the database connection is released before Keycloak
/// is called.
#[instrument(err)]
pub async fn migrate_realm_permissions_now() -> anyhow::Result<Vec<RealmMigration>> {
    let tenant_ids = {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .context("Error getting hasura client")?;
        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .context("Error starting hasura transaction")?;
        let tenant_ids = get_tenant_ids(&hasura_transaction).await?;
        // The event configurations the migration copied from annotations
        // were never validated: once, mark those a save would refuse.
        let marked = crate::services::messaging::config::mark_migrated_configs(&hasura_transaction)
            .await
            .context("Error validating migrated messaging configurations")?;
        hasura_transaction
            .commit()
            .await
            .context("Error committing the migrated messaging configurations")?;
        if !marked.is_empty() {
            info!(
                events = marked.len(),
                "Migrated messaging configurations need saving again"
            );
        }
        tenant_ids
    };
    let client = KeycloakAdminClient::pub_new().await?;
    migrate_realms(&client, &tenant_ids).await
}

/// Seconds before the first retry of a failed migration; each next retry
/// waits twice as long.
const FIRST_RETRY_SECONDS: u32 = 30;

/// Seconds to wait before retry number `retries + 1`: 30 s, 1, 2, 4 and
/// 8 minutes for the task's five retries.
pub fn retry_countdown(retries: u32) -> u32 {
    FIRST_RETRY_SECONDS.saturating_mul(2u32.saturating_pow(retries))
}

/// Retried with a growing wait when the migration fails, as when Keycloak
/// is still starting with beat; every run skips the roles a realm has.
#[instrument(skip(task))]
#[celery::task(bind = true, max_retries = 5)]
pub async fn migrate_realm_permissions(task: &Self) -> TaskResult<()> {
    match migrate_realm_permissions_now().await {
        Ok(_) => Ok(()),
        Err(err) => {
            let retries = task.request().retries;
            error!(retries, "Realm permission migration failed: {err:?}");
            task.retry_with_countdown(retry_countdown(retries))
        }
    }
}

/// A beat schedule that sends its task once, when beat starts.
pub struct RunOnce;

impl Schedule for RunOnce {
    fn next_call_at(&self, last_run_at: Option<SystemTime>) -> Option<SystemTime> {
        match last_run_at {
            None => Some(SystemTime::now()),
            Some(_) => None,
        }
    }
}
