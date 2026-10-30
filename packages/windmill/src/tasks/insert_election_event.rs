// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::update_bulletin_board;
use crate::postgres::tenant::get_tenant_by_id;
use crate::services::audit_log::{
    diff_values, stage, AdminAction, AdminChange, AuditActor, AuditObject,
};
use crate::services::database::get_hasura_pool;
use crate::services::delete_election_event::drop_uncommitted_event_database;
use crate::services::election_event_board::BoardSerializable;
use crate::services::import::import_election_event::ensure_election_audit_boards;
use crate::services::import::import_election_event::ensure_new_election_event;
use crate::services::import::import_election_event::insert_election_event_db;
use crate::services::import::import_election_event::upsert_keycloak_realm;
use crate::services::task_audit;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::tasks::audit_log_outbox::kick_audit_log_outbox;
use crate::types::error::Result;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use keycloak::types::RealmRepresentation;
use sequent_core;
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::{get_client_credentials, KeycloakAdminClient};
use sequent_core::types::hasura::core::{TasksExecution, VotingChannels};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::fs;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};

/// The fields of a new election event its entry lists.
const AUDITED_FIELDS: [&str; 8] = [
    "name",
    "alias",
    "description",
    "encryption_protocol",
    "voting_channels",
    "is_audit",
    "audit_election_event_id",
    "presentation",
];

/// The tenant log's entry of a created election event.
fn created_entry(
    actor: Option<AuditActor>,
    event: &CreateElectionEventInput,
    id: &str,
) -> AnyhowResult<AdminChange> {
    let mut fields = serde_json::to_value(event)?;
    if let Some(fields) = fields.as_object_mut() {
        fields.retain(|name, _| AUDITED_FIELDS.contains(&name.as_str()));
    }
    Ok(task_audit::change(
        actor,
        task_audit::target(&event.tenant_id, None)?,
        AuditObject::new(ELECTION_EVENT, id, Some(&event.name)),
        AdminAction::Create,
    )
    .with_changes(diff_values("", None, Some(&fields))))
}

#[instrument(err)]
pub async fn insert_election_event_anyhow(
    object: CreateElectionEventInput,
    id: String,
    task_execution: TasksExecution,
    audit_actor: Option<AuditActor>,
) -> AnyhowResult<()> {
    let mut final_object = object.clone();
    final_object.id = Some(id.clone());
    let tenant_id = object.tenant_id.clone();

    let mut db_client = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            update_fail(&task_execution, "Failed to get Hasura DB pool").await?;
            return Err(anyhow!("Failed to get Hasura DB pool: {err}").into());
        }
    };

    let hasura_transaction = match db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            update_fail(&task_execution, "Failed to start Hasura transaction").await?;
            return Err(anyhow!("Failed to start Hasura transaction: {err}").into());
        }
    };

    if let Err(err) = ensure_new_election_event(&hasura_transaction, &id).await {
        update_fail(&task_execution, &format!("{err:#}")).await?;
        return Err(err);
    }

    final_object.id = Some(id.clone());
    if final_object.voting_channels.is_none() {
        // Seed a new event's channels from its tenant, so a channel the tenant
        // has disabled is not silently re-enabled on every event it creates.
        // This runs only when the caller supplied none, which is the create
        // path; existing events keep whatever they already store. META-12778.
        // Only an absent configuration falls back to the default: defaulting on
        // a failed lookup or a malformed value could re-enable a disabled channel.
        let tenant_channels = match get_tenant_by_id(&hasura_transaction, tenant_id.as_str())
            .await
            .and_then(|tenant| match tenant.voting_channels {
                Some(channels) if !channels.is_null() => {
                    serde_json::from_value::<VotingChannels>(channels)
                        .context("Invalid tenant voting channels")
                }
                _ => Ok(VotingChannels::default()),
            }) {
            Ok(channels) => channels,
            Err(err) => {
                update_fail(&task_execution, "Failed to read tenant voting channels").await?;
                return Err(err.context(format!(
                    "Failed to read voting channels for tenant {tenant_id}"
                )));
            }
        };
        final_object.voting_channels = Some(serde_json::to_value(tenant_channels)?);
    }

    crate::postgres::scheduled_event::lock_scheduling_event(&hasura_transaction, &tenant_id, &id)
        .await?;
    match upsert_keycloak_realm(tenant_id.as_str(), &id.as_ref(), None, None).await {
        Ok(realm) => Some(realm),
        Err(err) => {
            update_fail(
                &task_execution,
                "Failed to update task execution status to COMPLETED",
            )
            .await?;
            return Err(anyhow!(
                "Failed to update task execution status to COMPLETED {err}"
            ));
        }
    };

    match insert_election_event_db(&hasura_transaction, &final_object).await {
        Ok(_) => (),
        Err(err) => {
            update_fail(
                &task_execution,
                "Failed to update task execution status to COMPLETED",
            )
            .await?;
            return Err(
                anyhow!("Failed to update task execution status to COMPLETED {err}").into(),
            );
        }
    };

    let board = ensure_election_audit_boards(
        &hasura_transaction,
        tenant_id.as_str(),
        &id.as_ref(),
        &vec![],
        false,
    )
    .await?;

    update_bulletin_board(
        &hasura_transaction,
        tenant_id.as_str(),
        &id.as_ref(),
        &board,
    )
    .await
    .with_context(|| {
        format!(
            "Error updating bulletin board reference for tenant ID {} and election event ID {:?}",
            tenant_id, &id,
        )
    })?;

    if let Err(error) = crate::services::monitoring::audit::initialize_new_event(
        &hasura_transaction,
        &tenant_id,
        &id,
    )
    .await
    {
        update_fail(&task_execution, "Failed to initialize monitoring defaults").await?;
        return Err(error.context("Failed to initialize the new event's monitoring defaults"));
    }

    if let Err(err) = stage(
        &hasura_transaction,
        &created_entry(audit_actor, &final_object, &id)?,
    )
    .await
    {
        update_fail(&task_execution, "Failed to queue the audit entry").await?;
        return Err(err.context("Failed to queue the audit entry"));
    }

    match hasura_transaction.commit().await {
        Ok(_) => (),
        Err(err) => {
            update_fail(&task_execution, "Failed to commit Hasura transaction").await?;
            return Err(anyhow!("Failed to commit Hasura transaction: {err}").into());
        }
    };
    kick_audit_log_outbox();

    update_complete(&task_execution, None)
        .await
        .context("Failed to update task execution status to COMPLETED")
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateElectionEventInput {
    pub id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub tenant_id: String,
    pub name: String,
    pub description: Option<String>,
    pub presentation: Option<Value>,
    pub bulletin_board_reference: Option<Value>,
    pub is_archived: Option<bool>,
    pub voting_channels: Option<Value>,
    pub status: Option<Value>,
    pub user_boards: Option<String>,
    pub encryption_protocol: Option<String>,
    pub is_audit: Option<bool>,
    pub audit_election_event_id: Option<String>,
    pub public_key: Option<String>,
    pub alias: Option<String>,
    pub statistics: Option<Value>,
}

pub const ELECTION_EVENT: &str = "election_event";

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_election_event_t(
    object: CreateElectionEventInput,
    id: String,
    task_execution: TasksExecution,
    audit_actor: Option<AuditActor>,
) -> Result<()> {
    let tenant_id = object.tenant_id.clone();
    if let Err(error) =
        insert_election_event_anyhow(object, id.clone(), task_execution, audit_actor).await
    {
        drop_uncommitted_event_database(&tenant_id, &id).await;
        return Err(error.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::audit_log::AuditOrigin;

    const TENANT_ID: &str = "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";
    const EVENT_ID: &str = "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43";

    fn event() -> CreateElectionEventInput {
        serde_json::from_value(json!({
            "tenant_id": TENANT_ID, "name": "Barangay 2026", "alias": "brgy-2026",
            "encryption_protocol": "RSA256", "labels": {"internal": true},
            "voting_channels": {"online": true}, "public_key": "key",
        }))
        .unwrap()
    }

    #[test]
    fn a_created_event_is_an_entry_of_the_tenant_log_with_its_fields() {
        let admin = AuditActor {
            user_id: Some("admin-id".into()),
            username: Some("ada".into()),
            role: Some("admin-user".into()),
        };
        let entry = created_entry(Some(admin.clone()), &event(), EVENT_ID).unwrap();

        assert_eq!(entry.target.election_event_id, None);
        assert_eq!(entry.target.tenant_id.to_string(), TENANT_ID);
        assert_eq!(
            entry.object,
            AuditObject::new(ELECTION_EVENT, EVENT_ID, Some("Barangay 2026"))
        );
        assert_eq!(entry.action, AdminAction::Create);
        assert_eq!(entry.actor, admin);
        assert_eq!(entry.origin, AuditOrigin::Windmill);
        assert_eq!(
            serde_json::to_value(&entry.changes).unwrap(),
            json!([
                {"path": "alias", "new": "brgy-2026"},
                {"path": "encryption_protocol", "new": "RSA256"},
                {"path": "name", "new": "Barangay 2026"},
                {"path": "voting_channels.online", "new": true},
            ])
        );
        assert_eq!(entry.describe(), "Created election event \"Barangay 2026\"");
    }

    #[test]
    fn a_task_queued_without_its_admin_still_reads_and_logs_no_actor() {
        let params: insert_election_event_tParams = serde_json::from_value(json!({
            "object": event(), "id": EVENT_ID,
            "task_execution": {
                "id": "task", "tenant_id": TENANT_ID, "election_event_id": EVENT_ID,
                "name": "Create", "task_type": "CREATE_ELECTION_EVENT",
                "execution_status": "IN_PROGRESS", "created_at": "2026-10-13T00:00:00Z",
                "start_at": null, "end_at": null, "annotations": null, "labels": null,
                "logs": null, "executed_by_user": "Ada",
            },
        }))
        .unwrap();
        assert_eq!(params.audit_actor, None);
        let entry = created_entry(params.audit_actor, &event(), EVENT_ID).unwrap();
        assert_eq!(entry.actor, AuditActor::default());
    }

    #[test]
    fn an_event_of_an_unreadable_tenant_cannot_be_audited() {
        let mut event = event();
        event.tenant_id = "tenant".into();
        assert!(created_entry(None, &event, EVENT_ID).is_err());
    }
}
