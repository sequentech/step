// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::jwks::remove_realm_jwks;
use super::protocol_manager::{get_b3_pgsql_client, get_election_board};
use crate::postgres::election::get_elections;
use crate::postgres::election_event::get_election_event_by_id_if_exist;
use crate::services::protocol_manager::get_event_board;
use crate::services::protocol_manager::get_immudb_client;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use futures::future::try_join_all;
use sequent_core::ballot::{
    ElectionEventPresentation, ElectionEventStatus, ElectionStatus, LockedDown, VotingStatus,
    VotingStatusChannel,
};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::s3;
use sequent_core::types::hasura::core::{Election, ElectionEvent};
use thiserror::Error;
use tracing::info;
use tracing::{event, instrument, Level};

const VOTING_STATUS_CHANNELS: [VotingStatusChannel; 2] =
    [VotingStatusChannel::ONLINE, VotingStatusChannel::KIOSK];

/// Why an election event cannot be deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ElectionEventDeletionRefusal {
    #[error("The election event is locked down")]
    LockedDown,
    #[error("The election event has elections outside the user's permission labels")]
    PermissionLabels,
    #[error("Voting is open or paused in the election event; close it before deleting the event")]
    VotingInProgress,
}

/// Returns why the election event must not be deleted by a user with the
/// given permission labels, or `None` when it can be deleted.
pub fn election_event_deletion_refusal(
    election_event: &ElectionEvent,
    elections: &[Election],
    permission_labels: &[String],
) -> Result<Option<ElectionEventDeletionRefusal>> {
    let presentation: Option<ElectionEventPresentation> = election_event
        .presentation
        .clone()
        .map(deserialize_value)
        .transpose()
        .map_err(|err| anyhow!("Error parsing election event presentation: {err:?}"))?;
    if presentation.and_then(|presentation| presentation.locked_down)
        == Some(LockedDown::LOCKED_DOWN)
    {
        return Ok(Some(ElectionEventDeletionRefusal::LockedDown));
    }

    let outside_permission_labels = elections
        .iter()
        .filter_map(|election| election.permission_label.as_ref())
        .any(|label| !permission_labels.contains(label));
    if outside_permission_labels {
        return Ok(Some(ElectionEventDeletionRefusal::PermissionLabels));
    }

    let mut voting_statuses: Vec<VotingStatus> = vec![];
    if let Some(status) = election_event.status.clone() {
        let status: ElectionEventStatus = deserialize_value(status)
            .map_err(|err| anyhow!("Error parsing election event status: {err:?}"))?;
        voting_statuses.extend(
            VOTING_STATUS_CHANNELS
                .iter()
                .map(|channel| status.status_by_channel(channel)),
        );
    }
    for election in elections {
        if let Some(status) = election.status.clone() {
            let status: ElectionStatus = deserialize_value(status).map_err(|err| {
                anyhow!("Error parsing status of election {}: {err:?}", election.id)
            })?;
            voting_statuses.extend(
                VOTING_STATUS_CHANNELS
                    .iter()
                    .map(|channel| status.status_by_channel(channel)),
            );
        }
    }
    if voting_statuses
        .iter()
        .any(|status| matches!(status, VotingStatus::OPEN | VotingStatus::PAUSED))
    {
        return Ok(Some(ElectionEventDeletionRefusal::VotingInProgress));
    }

    Ok(None)
}

/// Loads the election event and its elections and returns why they must not
/// be deleted, if they must not. An election event that no longer exists has
/// nothing left to protect.
#[instrument(skip(hasura_transaction), err)]
pub async fn find_election_event_deletion_refusal(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    permission_labels: &[String],
) -> Result<Option<ElectionEventDeletionRefusal>> {
    let Some(election_event) =
        get_election_event_by_id_if_exist(hasura_transaction, tenant_id, election_event_id).await?
    else {
        return Ok(None);
    };
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id, None).await?;

    election_event_deletion_refusal(&election_event, &elections, permission_labels)
}

#[instrument(err)]
pub async fn delete_keycloak_realm(realm: &str) -> Result<()> {
    let client = KeycloakAdminClient::new().await?;
    remove_realm_jwks(&realm).await?;

    let realm_exists = client
        .client
        .realm_get(&realm)
        .await
        .map_err(|err| anyhow!("Keycloak error: {err:?}"));

    info!("realm_exists? {:?}", realm_exists.is_ok());

    if realm_exists.is_ok() {
        client
            .client
            .realm_delete(&realm)
            .await
            .map_err(|err| anyhow!("Keycloak error: {err:?}"))?;
    }
    Ok(())
}

#[instrument(err)]
pub async fn delete_election_event_b3(
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &Vec<String>,
) -> Result<()> {
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);
    let mut board_client = get_b3_pgsql_client().await?;
    let existing: Option<b3::client::pgsql::B3IndexRow> =
        board_client.get_board(board_name.as_str()).await?;

    if existing.is_some() {
        board_client.delete_board(board_name.as_str()).await?;
    }

    for election_id in election_ids {
        let board_name = get_election_board(tenant_id, &election_id, &slug);
        let existing: Option<b3::client::pgsql::B3IndexRow> =
            board_client.get_board(board_name.as_str()).await?;

        if existing.is_some() {
            board_client.delete_board(board_name.as_str()).await?;
        }
    }
    Ok(())
}

#[instrument(err)]
pub async fn delete_election_event_immudb(tenant_id: &str, election_event_id: &str) -> Result<()> {
    let mut client = get_immudb_client().await?;
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);

    event!(Level::INFO, "database name = {board_name}");

    let has_database = client
        .has_database(&board_name)
        .await
        .map_err(|err| anyhow!("error reading immudb database: {err:?}"))?;

    if has_database {
        client
            .delete_database(&board_name)
            .await
            .map_err(|err| anyhow!("error delete immudb database: {err:?}"))?;
    }
    Ok(())
}

#[instrument(err)]
pub async fn delete_election_event_related_documents(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let documents_prefix = format!("tenant-{}/event-{}/", tenant_id, election_event_id);
    let bucket = s3::get_private_bucket()?;
    s3::delete_files_from_s3(bucket, documents_prefix, false)
        .await
        .map_err(|err| anyhow!("Error delete private files from s3: {err:?}"))?;
    Ok(())
}

#[cfg(test)]
mod deletion_refusal_tests {
    use super::*;
    use serde_json::{json, Value};

    const LABEL: &str = "north";

    fn election_event(presentation: Option<Value>, status: Option<Value>) -> ElectionEvent {
        serde_json::from_value(json!({
            "id": "event",
            "name": "event",
            "tenant_id": "tenant",
            "is_archived": false,
            "encryption_protocol": "RSA256",
            "presentation": presentation,
            "status": status,
        }))
        .unwrap()
    }

    fn election(status: Option<Value>, permission_label: Option<&str>) -> Election {
        serde_json::from_value(json!({
            "id": "election",
            "name": "election",
            "tenant_id": "tenant",
            "election_event_id": "event",
            "status": status,
            "permission_label": permission_label,
        }))
        .unwrap()
    }

    fn refusal(
        election_event: &ElectionEvent,
        elections: &[Election],
        permission_labels: &[&str],
    ) -> Option<ElectionEventDeletionRefusal> {
        let permission_labels: Vec<String> = permission_labels
            .iter()
            .map(|label| label.to_string())
            .collect();
        election_event_deletion_refusal(election_event, elections, &permission_labels).unwrap()
    }

    #[test]
    fn allows_an_event_that_never_opened_voting() {
        let elections = [
            election(Some(json!({"voting_status": "NOT_STARTED"})), None),
            election(None, Some(LABEL)),
        ];
        assert_eq!(
            refusal(&election_event(None, None), &elections, &[LABEL]),
            None
        );
        assert_eq!(refusal(&election_event(None, None), &[], &[]), None);
    }

    #[test]
    fn allows_an_event_whose_voting_is_closed() {
        let event = election_event(
            Some(json!({"locked_down": "not-locked-down"})),
            Some(json!({"voting_status": "CLOSED", "kiosk_voting_status": "CLOSED"})),
        );
        let elections = [election(Some(json!({"voting_status": "CLOSED"})), None)];
        assert_eq!(refusal(&event, &elections, &[]), None);
    }

    #[test]
    fn refuses_a_locked_down_event() {
        let event = election_event(Some(json!({"locked_down": "locked-down"})), None);
        assert_eq!(
            refusal(&event, &[], &[]),
            Some(ElectionEventDeletionRefusal::LockedDown)
        );
    }

    #[test]
    fn refuses_while_voting_is_open_or_paused_on_any_channel() {
        for channel in ["voting_status", "kiosk_voting_status"] {
            for status in ["OPEN", "PAUSED"] {
                let elections = [election(Some(json!({ channel: status })), None)];
                assert_eq!(
                    refusal(&election_event(None, None), &elections, &[]),
                    Some(ElectionEventDeletionRefusal::VotingInProgress),
                    "election {channel} {status}"
                );

                let event = election_event(None, Some(json!({ channel: status })));
                assert_eq!(
                    refusal(&event, &[], &[]),
                    Some(ElectionEventDeletionRefusal::VotingInProgress),
                    "event {channel} {status}"
                );
            }
        }
    }

    #[test]
    fn refuses_elections_outside_the_permission_labels() {
        let elections = [election(None, None), election(None, Some(LABEL))];
        assert_eq!(
            refusal(&election_event(None, None), &elections, &[]),
            Some(ElectionEventDeletionRefusal::PermissionLabels)
        );
        assert_eq!(
            refusal(&election_event(None, None), &elections, &["south"]),
            Some(ElectionEventDeletionRefusal::PermissionLabels)
        );
        assert_eq!(
            refusal(&election_event(None, None), &elections, &["south", LABEL]),
            None
        );
    }

    #[test]
    fn unreadable_status_is_an_error_instead_of_allowing_the_deletion() {
        let elections = [election(Some(json!({"voting_status": "UNKNOWN"})), None)];
        assert!(
            election_event_deletion_refusal(&election_event(None, None), &elections, &[]).is_err()
        );

        let event = election_event(Some(json!({"locked_down": "maybe"})), None);
        assert!(election_event_deletion_refusal(&event, &[], &[]).is_err());
    }
}
