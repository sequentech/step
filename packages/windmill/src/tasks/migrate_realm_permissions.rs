// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Adds permissions that a release introduces to the tenant realms made
//! before it. New tenant realms get them from their template; existing ones
//! get the missing realm roles through Keycloak's partial import, skipping
//! any role the realm already has. Only roles are added: which groups hold
//! them stays a choice made in Users and Roles.

use crate::postgres::tenant::get_tenant_ids;
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context};
use celery::beat::Schedule;
use celery::task::{Task, TaskResult};
use deadpool_postgres::Client as DbClient;
use keycloak::types::RoleRepresentation;
use sequent_core::services::keycloak::{
    get_tenant_realm, partial_import_realm_roles, IfResourceExists, KeycloakAdminClient,
    PartialImportSummary, PubKeycloakAdmin,
};
use sequent_core::types::permissions::Permissions;
use std::time::SystemTime;
use tracing::{error, info, instrument};

/// The permissions the migration adds, with the English label Users and
/// Roles shows for each, which becomes the role's description.
pub const MIGRATED_PERMISSIONS: [(Permissions, &str); 21] = [
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
    (Permissions::SIGN_KEY_CEREMONY, "Sign: confirm a key share"),
    (Permissions::SIGN_TALLY_KEY, "Sign: contribute a key share"),
];

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
/// roles a realm already has. Every realm is tried; if any failed the
/// result is an error naming them, after the others were migrated.
#[instrument(skip(client), err)]
pub async fn migrate_realms(
    client: &PubKeycloakAdmin,
    tenant_ids: &[String],
) -> anyhow::Result<Vec<(String, PartialImportSummary)>> {
    let roles = migrated_roles();
    let mut migrated = vec![];
    let mut failed = vec![];
    for tenant_id in tenant_ids {
        let realm = get_tenant_realm(tenant_id);
        match partial_import_realm_roles(client, &realm, &roles, IfResourceExists::Skip).await {
            Ok(summary) => {
                info!(
                    realm,
                    added = summary.added,
                    skipped = summary.skipped,
                    "Realm permissions migrated"
                );
                migrated.push((realm, summary));
            }
            Err(err) => {
                error!(realm, "Realm permissions not migrated: {err:?}");
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
/// and the Keycloak admin credentials the environment names. The tenants
/// are read first and the database connection is released before Keycloak
/// is called.
#[instrument(err)]
pub async fn migrate_realm_permissions_now() -> anyhow::Result<Vec<(String, PartialImportSummary)>>
{
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
        get_tenant_ids(&hasura_transaction).await?
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
