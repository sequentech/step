// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::update_bulletin_board;
use crate::postgres::tenant::get_tenant_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::delete_election_event::drop_uncommitted_event_database;
use crate::services::election_event_board::BoardSerializable;
use crate::services::import::import_election_event::ensure_election_audit_boards;
use crate::services::import::import_election_event::ensure_new_election_event;
use crate::services::import::import_election_event::insert_election_event_db;
use crate::services::import::import_election_event::upsert_keycloak_realm;
use crate::services::import::import_election_event::EventRealm;
use crate::services::tasks_execution::{update_complete, update_fail};
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
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};

#[instrument(err)]
pub async fn insert_election_event_anyhow(
    object: CreateElectionEventInput,
    id: String,
    task_execution: TasksExecution,
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
    match upsert_keycloak_realm(tenant_id.as_str(), &id, EventRealm::Template, None).await {
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

    match hasura_transaction.commit().await {
        Ok(_) => (),
        Err(err) => {
            update_fail(&task_execution, "Failed to commit Hasura transaction").await?;
            return Err(anyhow!("Failed to commit Hasura transaction: {err}").into());
        }
    };

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

impl CreateElectionEventInput {
    /// The new election event as the caller may describe it where election
    /// data arrives only through the signed import: its tenant, name,
    /// description and alias. Every other attribute, the id included, takes
    /// its default.
    pub fn texts_only(&self) -> Self {
        Self {
            id: None,
            created_at: None,
            updated_at: None,
            labels: None,
            annotations: None,
            tenant_id: self.tenant_id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            presentation: Some(json!({ "i18n": self.texts() })),
            bulletin_board_reference: None,
            is_archived: None,
            voting_channels: None,
            status: None,
            user_boards: None,
            encryption_protocol: None,
            is_audit: None,
            audit_election_event_id: None,
            public_key: None,
            alias: self.alias.clone(),
            statistics: None,
        }
    }

    /// The name, description and alias per language: those `presentation.i18n`
    /// gives, or else the top-level ones, in English.
    fn texts(&self) -> Map<String, Value> {
        /// The texts of a language that the caller gives.
        const TEXTS: [&str; 3] = ["name", "description", "alias"];
        let given: Map<String, Value> = self
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.get("i18n"))
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter_map(|(language, texts)| {
                let texts: Map<String, Value> = texts
                    .as_object()?
                    .iter()
                    .filter(|(key, text)| TEXTS.contains(&key.as_str()) && text.is_string())
                    .map(|(key, text)| (key.clone(), text.clone()))
                    .collect();
                (!texts.is_empty()).then(|| (language.clone(), Value::Object(texts)))
            })
            .collect();
        if !given.is_empty() {
            return given;
        }
        let mut texts = Map::new();
        texts.insert("name".to_string(), json!(self.name));
        if let Some(description) = &self.description {
            texts.insert("description".to_string(), json!(description));
        }
        if let Some(alias) = &self.alias {
            texts.insert("alias".to_string(), json!(alias));
        }
        Map::from_iter([("en".to_string(), Value::Object(texts))])
    }
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_election_event_t(
    object: CreateElectionEventInput,
    id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    let tenant_id = object.tenant_id.clone();
    if let Err(error) = insert_election_event_anyhow(object, id.clone(), task_execution).await {
        drop_uncommitted_event_database(&tenant_id, &id).await;
        return Err(error.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(value: Value) -> CreateElectionEventInput {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn only_the_texts_of_a_new_event_come_from_the_caller() {
        let given = input(json!({
            "id": "4b1c9c33-5bb0-4d8e-9a59-8c3a4f8f9a01",
            "tenant_id": "tenant",
            "name": "Event",
            "description": "About",
            "alias": "Short",
            "is_archived": true,
            "is_audit": true,
            "public_key": "key",
            "encryption_protocol": "OTHER",
            "status": {"voting_status": "OPEN"},
            "voting_channels": {"online": true},
            "labels": {"a": "b"},
            "annotations": {"c": "d"},
            "user_boards": "boards",
            "audit_election_event_id": "4b1c9c33-5bb0-4d8e-9a59-8c3a4f8f9a02",
            "presentation": {
                "i18n": {
                    "en": {"name": "Event", "description": "About", "css": "x"},
                    "es": {"name": "Evento", "alias": 3},
                    "fr": {"other": "x"}
                },
                "language_conf": {"enabled_language_codes": ["en", "es"]},
                "keys_ceremony_policy": "AUTOMATED_CEREMONY"
            }
        }));
        let kept = given.texts_only();
        assert_eq!(
            serde_json::to_value(&kept).unwrap(),
            json!({
                "id": null,
                "created_at": null,
                "updated_at": null,
                "labels": null,
                "annotations": null,
                "tenant_id": "tenant",
                "name": "Event",
                "description": "About",
                "presentation": {"i18n": {
                    "en": {"name": "Event", "description": "About"},
                    "es": {"name": "Evento"}
                }},
                "bulletin_board_reference": null,
                "is_archived": null,
                "voting_channels": null,
                "status": null,
                "user_boards": null,
                "encryption_protocol": null,
                "is_audit": null,
                "audit_election_event_id": null,
                "public_key": null,
                "alias": "Short",
                "statistics": null
            })
        );
    }

    #[test]
    fn without_translations_the_texts_are_the_english_ones() {
        let given = input(json!({
            "tenant_id": "tenant",
            "name": "Event",
            "alias": "Short",
            "presentation": {"css": "x"}
        }));
        assert_eq!(
            given.texts_only().presentation,
            Some(json!({"i18n": {"en": {"name": "Event", "alias": "Short"}}}))
        );
    }
}
