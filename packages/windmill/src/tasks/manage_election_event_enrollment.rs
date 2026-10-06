// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::{
    get_election_event_by_id, update_election_event_presentation,
};
use crate::postgres::scheduled_event::*;
use crate::services::enrollment_windows;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::voting_status::{self};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use async_trait::async_trait;
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionEventPresentation, Enrollment};
use sequent_core::serialization::deserialize_with_path::{self, deserialize_value};
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient};
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{error, event, info, Level};

pub async fn update_keycloak_otp(
    tenant_id: Option<String>,
    election_event_id: Option<String>,
    new_otp_state: String,
) -> Result<()> {
    let Some(ref tenant_id) = tenant_id else {
        return Ok(());
    };

    let realm_name = get_event_realm(
        tenant_id,
        election_event_id
            .as_ref()
            .ok_or_else(|| anyhow!("Missing election_event_id"))?,
    );

    // Define authentication flows to update
    let authentication_flows = vec![
        "comelec-registration",
        "sequent browser flow",
        "reset credentials",
    ];

    // Loop through each flow to update its execution
    for flow_name in authentication_flows {
        let keycloak_client = KeycloakAdminClient::new().await?;
        let pub_client = KeycloakAdminClient::pub_new().await?;

        let flow_executions = keycloak_client
            .get_flow_executions(&pub_client, &realm_name, flow_name)
            .await
            .with_context(|| format!("Error fetching flow executions for '{}'", flow_name))?;

        for mut execution in flow_executions {
            if execution.provider_id.as_deref() == Some("message-otp-authenticator") {
                execution.requirement = Some(new_otp_state.clone());

                keycloak_client
                    .upsert_flow_execution(
                        &pub_client,
                        &realm_name,
                        flow_name,
                        &serde_json::to_string(&execution)?,
                    )
                    .await
                    .with_context(|| {
                        format!("Error updating flow execution for '{}'", flow_name)
                    })?;
            }
        }
    }

    Ok(())
}

pub async fn update_keycloak_enrollment(
    tenant_id: Option<String>,
    election_event_id: Option<String>,
    enable_enrollment: bool,
) -> Result<()> {
    let Some(ref tenant_id) = tenant_id else {
        return Ok(());
    };

    let realm_name = get_event_realm(
        &tenant_id,
        election_event_id
            .as_ref()
            .ok_or("scheduled event missing election_event_id")?
            .as_str(),
    );

    let keycloak_client = KeycloakAdminClient::new().await?;
    let other_client = KeycloakAdminClient::pub_new().await?;
    let mut realm = keycloak_client
        .get_realm(&other_client, &realm_name)
        .await
        .with_context(|| "Error obtaining realm")?;
    let state_result = crate::tasks::migrate_registration_flows::apply_registration_desire(
        &mut realm,
        enable_enrollment,
    );

    let keycloak_client = KeycloakAdminClient::new().await?;
    keycloak_client
        .upsert_realm(
            &realm_name,
            &serde_json::to_string(&realm)?,
            &tenant_id,
            false,
            None,
            None,
        )
        .await?;
    // Even an unreadable private recovery state persists the denial before
    // reporting failure; it must never become an enabled ordinary update.
    state_result?;

    Ok(())
}

/// Loads the current database targets before any external enrollment effect.
/// Kept separate so inactive and misrouted queued tasks can be checked without
/// contacting the identity provider.
pub async fn load_enrollment_task(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    scheduled_event_id: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> AnyhowResult<Option<(ScheduledEvent, ElectionEvent)>> {
    // Writer order: signing advisory lock, event row, target schedule row.
    // Hold them through the external effect and refresh; re-read only after
    // acquiring them so a queued task cannot restore stale allowing policy.
    lock_scheduled_event(
        hasura_transaction,
        tenant_id,
        election_event_id,
        scheduled_event_id,
    )
    .await?;
    let scheduled_event = find_scheduled_event_by_id(
        hasura_transaction,
        Some(tenant_id.to_owned()),
        Some(election_event_id.to_owned()),
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
    if !matches!(
        scheduled_event.event_processor.as_ref(),
        Some(EventProcessors::START_ENROLLMENT_PERIOD | EventProcessors::END_ENROLLMENT_PERIOD)
    ) {
        return Err(anyhow!(
            "Scheduled event {scheduled_event_id} is not an enrollment processor"
        ));
    }
    // Queued before the row moved to a later time: it runs then.
    if crate::tasks::scheduled_events::fires_later(&scheduled_event, now) {
        info!("Scheduled event {scheduled_event_id} was moved to a later time; it runs then");
        return Ok(None);
    }

    let election_event =
        get_election_event_by_id(hasura_transaction, &tenant_id, &election_event_id)
            .await
            .with_context(|| "Error obtaining election by id")?;

    if election_event.is_archived {
        info!("Event {election_event_id} is archived; enrollment remains unchanged");
        return Ok(None);
    }
    Ok(Some((scheduled_event, election_event)))
}

#[instrument(err)]
pub async fn manage_election_event_enrollment_wrapped(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
) -> AnyhowResult<()> {
    let Some((scheduled_event, election_event)) = load_enrollment_task(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &scheduled_event_id,
        chrono::Utc::now(),
    )
    .await?
    else {
        return Ok(());
    };
    let enable_enrollment =
        scheduled_event.event_processor == Some(EventProcessors::START_ENROLLMENT_PERIOD);

    update_keycloak_enrollment(
        scheduled_event.tenant_id.clone(),
        scheduled_event.election_event_id.clone(),
        enable_enrollment.clone(),
    )
    .await?;

    if let Some(election_event_presentation) = election_event.presentation {
        let election_event_presentation = ElectionEventPresentation {
            enrollment: if (enable_enrollment) {
                Some(Enrollment::ENABLED)
            } else {
                Some(Enrollment::DISABLED)
            },
            ..deserialize_with_path::deserialize_value(election_event_presentation)?
        };
        update_election_event_presentation(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            serde_json::to_value(election_event_presentation)?,
        )
        .await?;
    }

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_event.id)
        .await
        .with_context(|| "Error stopping scheduled event")?;

    // The per-Post windows the registration form enforces (VOTE-LIFECYCLE §8),
    // in a savepoint: the realm switch has already changed, so a failure here
    // must not roll back stopping this event (it would fire again every tick).
    // It is logged; the next enrollment change or publication writes them.
    if let Err(err) =
        enrollment_windows::refresh_in_savepoint(hasura_transaction, &tenant_id, &election_event_id)
            .await
    {
        error!(
            "Event {election_event_id}: the enrollment windows were not updated after scheduled \
             event {scheduled_event_id}: {err:?}"
        );
    }

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn manage_election_event_enrollment(
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
) -> Result<()> {
    let res = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let scheduled_event_id = scheduled_event_id.clone();
        Box::pin(async move {
            // Your async code here
            manage_election_event_enrollment_wrapped(
                hasura_transaction,
                tenant_id,
                election_event_id,
                scheduled_event_id,
            )
            .await
        })
    })
    .await?;

    info!("result: {:?}", res);

    Ok(res)
}
