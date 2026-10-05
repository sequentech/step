// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::update_bulletin_board;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::BoardSerializable;
use crate::services::import::import_election_event::insert_election_event_db;
use crate::services::import::import_election_event::upsert_b3_and_elog;
use crate::services::import::import_election_event::upsert_keycloak_realm;
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
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::fs;
use strum_macros::Display;
use thiserror::Error;
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

    final_object.id = Some(id.clone());

    match upsert_keycloak_realm(tenant_id.as_str(), &id.as_ref(), None).await {
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

    let board = upsert_b3_and_elog(
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

/// Fields of a new election event that only the server sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
pub enum ServerOwnedField {
    CreatedAt,
    UpdatedAt,
    BulletinBoardReference,
    Status,
    UserBoards,
    IsAudit,
    AuditElectionEventId,
    PublicKey,
    Statistics,
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum CreateElectionEventInputError {
    #[error("id is not a valid UUID v4")]
    InvalidId,
    #[error("{0} is set by the server and cannot be provided when creating an election event")]
    ServerOwnedField(ServerOwnedField),
}

impl CreateElectionEventInput {
    /// A new event carries a well-formed id, if any, and nothing the server
    /// sets itself.
    pub fn validate_new_event(&self) -> std::result::Result<(), CreateElectionEventInputError> {
        if let Some(id) = &self.id {
            parse_uuid_v4(id).map_err(|_| CreateElectionEventInputError::InvalidId)?;
        }

        let supplied = [
            (ServerOwnedField::CreatedAt, self.created_at.is_some()),
            (ServerOwnedField::UpdatedAt, self.updated_at.is_some()),
            (
                ServerOwnedField::BulletinBoardReference,
                self.bulletin_board_reference.is_some(),
            ),
            (ServerOwnedField::Status, self.status.is_some()),
            (ServerOwnedField::UserBoards, self.user_boards.is_some()),
            (ServerOwnedField::IsAudit, self.is_audit == Some(true)),
            (
                ServerOwnedField::AuditElectionEventId,
                self.audit_election_event_id.is_some(),
            ),
            (ServerOwnedField::PublicKey, self.public_key.is_some()),
            (ServerOwnedField::Statistics, self.statistics.is_some()),
        ];
        match supplied.into_iter().find(|(_, is_supplied)| *is_supplied) {
            Some((field, _)) => Err(CreateElectionEventInputError::ServerOwnedField(field)),
            None => Ok(()),
        }
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
    insert_election_event_anyhow(object, id, task_execution).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT_ID: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";

    fn admin_portal_input() -> CreateElectionEventInput {
        CreateElectionEventInput {
            id: Some("d9ecdb61-f799-4d64-8f60-4251cb7dc8cf".to_string()),
            created_at: None,
            updated_at: None,
            labels: None,
            annotations: None,
            tenant_id: TENANT_ID.to_string(),
            name: "Event".to_string(),
            description: None,
            presentation: Some(json!({"i18n": {"en": {"name": "Event"}}})),
            bulletin_board_reference: None,
            is_archived: Some(false),
            voting_channels: None,
            status: None,
            user_boards: None,
            encryption_protocol: Some("RSA256".to_string()),
            is_audit: None,
            audit_election_event_id: None,
            public_key: None,
            alias: None,
            statistics: None,
        }
    }

    #[test]
    fn accepts_what_the_admin_portal_sends() {
        assert_eq!(admin_portal_input().validate_new_event(), Ok(()));
    }

    #[test]
    fn accepts_a_request_without_id() {
        let input = CreateElectionEventInput {
            id: None,
            ..admin_portal_input()
        };
        assert_eq!(input.validate_new_event(), Ok(()));
    }

    #[test]
    fn accepts_an_explicit_non_audit_event() {
        let input = CreateElectionEventInput {
            is_audit: Some(false),
            ..admin_portal_input()
        };
        assert_eq!(input.validate_new_event(), Ok(()));
    }

    #[test]
    fn rejects_an_id_that_is_not_a_uuid() {
        let input = CreateElectionEventInput {
            id: Some("not-a-uuid".to_string()),
            ..admin_portal_input()
        };
        assert_eq!(
            input.validate_new_event(),
            Err(CreateElectionEventInputError::InvalidId)
        );
    }

    #[test]
    fn rejects_each_server_owned_field() {
        let cases = [
            (
                CreateElectionEventInput {
                    created_at: Some("2026-01-01T00:00:00Z".to_string()),
                    ..admin_portal_input()
                },
                ServerOwnedField::CreatedAt,
            ),
            (
                CreateElectionEventInput {
                    updated_at: Some("2026-01-01T00:00:00Z".to_string()),
                    ..admin_portal_input()
                },
                ServerOwnedField::UpdatedAt,
            ),
            (
                CreateElectionEventInput {
                    bulletin_board_reference: Some(json!({"id": 1})),
                    ..admin_portal_input()
                },
                ServerOwnedField::BulletinBoardReference,
            ),
            (
                CreateElectionEventInput {
                    status: Some(json!({"voting_status": "OPEN"})),
                    ..admin_portal_input()
                },
                ServerOwnedField::Status,
            ),
            (
                CreateElectionEventInput {
                    user_boards: Some("board".to_string()),
                    ..admin_portal_input()
                },
                ServerOwnedField::UserBoards,
            ),
            (
                CreateElectionEventInput {
                    is_audit: Some(true),
                    ..admin_portal_input()
                },
                ServerOwnedField::IsAudit,
            ),
            (
                CreateElectionEventInput {
                    audit_election_event_id: Some(
                        "3af34059-6bc9-4c89-b99e-c75c1eec9ac5".to_string(),
                    ),
                    ..admin_portal_input()
                },
                ServerOwnedField::AuditElectionEventId,
            ),
            (
                CreateElectionEventInput {
                    public_key: Some("key".to_string()),
                    ..admin_portal_input()
                },
                ServerOwnedField::PublicKey,
            ),
            (
                CreateElectionEventInput {
                    statistics: Some(json!({})),
                    ..admin_portal_input()
                },
                ServerOwnedField::Statistics,
            ),
        ];

        for (input, field) in cases {
            assert_eq!(
                input.validate_new_event(),
                Err(CreateElectionEventInputError::ServerOwnedField(field)),
                "{field} should be rejected"
            );
        }
    }

    #[test]
    fn server_owned_field_errors_name_the_graphql_field() {
        assert_eq!(
            CreateElectionEventInputError::ServerOwnedField(ServerOwnedField::PublicKey)
                .to_string(),
            "public_key is set by the server and cannot be provided when creating an election event"
        );
    }
}
