// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election::{get_election_by_id, update_election_presentation};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keycloak_realm;
use crate::postgres::scheduled_event::*;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::date::ISO8601;
use crate::services::pg_lock::PgLock;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::voting_status::{self};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use async_trait::async_trait;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionPresentation, Enrollment, InitReport, VotingStatus};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm, KeycloakAdminClient};
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{error, event, info, Level};
use uuid::Uuid;

pub async fn update_keycloak(
    scheduled_event: &ScheduledEvent,
    registration_allowed: bool,
) -> Result<()> {
    let realm_name = match scheduled_event.election_event_id {
        Some(ref event_id) => get_event_realm(
            scheduled_event
                .tenant_id
                .as_ref()
                .ok_or("scheduled event missing tenant_id")?
                .as_str(),
            scheduled_event
                .election_event_id
                .as_ref()
                .ok_or("scheduled event missing election_event_id")?
                .as_str(),
        ),
        None => get_tenant_realm(
            &scheduled_event
                .tenant_id
                .as_ref()
                .ok_or("scheduled event missing tenant_id")?
                .as_str(),
        ),
    };

    let mut keycloak_db_client: DbClient = match get_keycloak_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            return Err(Error::String(format!(
                "Error getting Keycloak DB pool: {err}"
            )));
        }
    };

    let keycloak_transaction = match keycloak_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            return Err(Error::String(format!(
                "Error starting Keycloak transaction: {err}"
            )));
        }
    };

    let realm_id =
        match keycloak_realm::get_realm_id(&keycloak_transaction, realm_name.to_string()).await {
            Ok(id) => id,
            Err(err) => {
                return Err(Error::String(format!("Error obtaining realm id: {err}")));
            }
        };
    let keycloak_client = KeycloakAdminClient::new().await?;
    let other_client = KeycloakAdminClient::pub_new().await?;
    let mut realm = keycloak_client
        .get_realm(&other_client, &realm_name)
        .await?;
    realm.registration_allowed = Some(registration_allowed);
    let keycloak_client = KeycloakAdminClient::new().await?;
    keycloak_client
        .upsert_realm(
            &realm_name,
            &serde_json::to_string(&realm)?,
            scheduled_event
                .tenant_id
                .as_ref()
                .ok_or("scheduled event missing tenant_id")?
                .as_str(),
            false,
            None,
        )
        .await?;

    Ok(())
}

#[instrument(err)]
pub async fn manage_election_enrollment_wrapped(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
    election_id: String,
) -> AnyhowResult<()> {
    let scheduled_event = find_scheduled_event_by_id(
        hasura_transaction,
        Some(tenant_id.clone()),
        Some(election_event_id.clone()),
        &scheduled_event_id,
    )
    .await
    .with_context(|| "Error obtaining scheduled event by id")?;

    let Some(scheduled_event) = scheduled_event else {
        return Err(anyhow!(
            "Can't find scheduled event with id: {}",
            scheduled_event_id
        ));
    };

    let Some(election) = get_election_by_id(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await
    .with_context(|| "Error obtaining election by id")?
    else {
        return Err(anyhow!("Election not found"));
    };

    let Some(event_payload) = scheduled_event.event_payload.clone() else {
        event!(Level::WARN, "Missing election_event_id");
        return Ok(());
    };
    let event_payload: ManageEnrollmentPayload = deserialize_value(event_payload)?;

    update_keycloak(
        &scheduled_event,
        event_payload.enable_enrollment == Some(true),
    )
    .await?;

    if let Some(election_presentation) = election.presentation {
        let election_presentation: ElectionPresentation = ElectionPresentation {
            enrollment: if (event_payload.enable_enrollment == Some(true)) {
                Enrollment::ENABLED
            } else {
                Enrollment::DISABLED
            },
            ..serde_json::from_value(election_presentation)?
        };
        update_election_presentation(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &election_id,
            serde_json::to_value(election_presentation)?,
        )
        .await?;
    }

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_event.id)
        .await
        .with_context(|| "Error stopping scheduled event")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn manage_election_enrollment(
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
    election_id: String,
) -> Result<()> {
    let lock: PgLock = PgLock::acquire(
        format!(
            "execute_manage_election_enrollment-{}-{}-{}-{}",
            tenant_id, election_event_id, scheduled_event_id, election_id
        ),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(120),
    )
    .await
    .with_context(|| "Error acquiring pglock")?;

    let res = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let scheduled_event_id = scheduled_event_id.clone();
        let election_id = election_id.clone();
        Box::pin(async move {
            // Your async code here
            manage_election_enrollment_wrapped(
                hasura_transaction,
                tenant_id,
                election_event_id,
                scheduled_event_id,
                election_id,
            )
            .await
        })
    })
    .await;

    info!("result: {:?}", res);

    lock.release()
        .await
        .with_context(|| "Error releasing pglock")?;

    Ok(res?)
}
