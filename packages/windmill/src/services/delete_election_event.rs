// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::jwks::remove_realm_jwks;
use crate::postgres::election::get_elections;
use crate::postgres::election_event::{
    election_event_id_exists, get_election_event_by_id_if_exist,
};
use crate::services::database::get_hasura_pool;
use crate::services::electoral_log_board::get_event_board;
use crate::services::electoral_log_board::get_event_databases;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use futures::future::try_join_all;
use keycloak::KeycloakError;
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

const VOTING_STATUS_CHANNELS: [VotingStatusChannel; 4] = [
    VotingStatusChannel::ONLINE,
    VotingStatusChannel::KIOSK,
    VotingStatusChannel::EARLY_VOTING,
    VotingStatusChannel::TELEPHONE,
];

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum DeletionContext {
    General,
    AuthorizedPostProcessing,
}

/// Returns why the election event must not be deleted by a user with the
/// given permission labels, or `None` when it can be deleted.
pub fn election_event_deletion_refusal(
    election_event: &ElectionEvent,
    elections: &[Election],
    permission_labels: &[String],
) -> Result<Option<ElectionEventDeletionRefusal>> {
    deletion_refusal_in_context(
        election_event,
        elections,
        permission_labels,
        DeletionContext::General,
    )
}

fn deletion_refusal_in_context(
    election_event: &ElectionEvent,
    elections: &[Election],
    permission_labels: &[String],
    context: DeletionContext,
) -> Result<Option<ElectionEventDeletionRefusal>> {
    let presentation: Option<ElectionEventPresentation> = election_event
        .presentation
        .clone()
        .map(deserialize_value)
        .transpose()
        .map_err(|err| anyhow!("Error parsing election event presentation: {err:?}"))?;
    let locked_down = presentation.and_then(|presentation| presentation.locked_down)
        == Some(LockedDown::LOCKED_DOWN);
    if locked_down && context == DeletionContext::General {
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
        // Closing individual Posts can leave the aggregate gate OPEN. Only a
        // transaction with current export/quorum authority and no voter
        // admission may disregard that summary; every Post is checked below.
        // Still parse the parent, and never infer closure from an empty set.
        if context == DeletionContext::General || elections.is_empty() {
            voting_statuses.extend(
                VOTING_STATUS_CHANNELS
                    .iter()
                    .map(|channel| status.status_by_channel(*channel)),
            );
        }
    }
    for election in elections {
        if let Some(status) = election.status.clone() {
            let status: ElectionStatus = deserialize_value(status).map_err(|err| {
                anyhow!("Error parsing status of election {}: {err:?}", election.id)
            })?;
            voting_statuses.extend(
                VOTING_STATUS_CHANNELS
                    .iter()
                    .map(|channel| status.status_by_channel(*channel)),
            );
        }
    }
    if voting_statuses.iter().any(VotingStatus::is_open) {
        return Ok(Some(ElectionEventDeletionRefusal::VotingInProgress));
    }
    // The authorized task can accept LockedDown, but it must not hide an
    // active/malformed Post or a permission-label refusal behind that result.
    if locked_down {
        return Ok(Some(ElectionEventDeletionRefusal::LockedDown));
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
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id).await?;
    let context = if crate::services::board_operations::is_authorized_deletion(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await?
        && hasura_transaction
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle_access WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND phase='post_processing' AND NOT voter_allowed)",
                &[&tenant_id, &election_event_id],
            )
            .await?
            .try_get::<_, bool>(0)?
    {
        DeletionContext::AuthorizedPostProcessing
    } else {
        DeletionContext::General
    };

    deletion_refusal_in_context(&election_event, &elections, permission_labels, context)
}

#[instrument(err)]
pub async fn delete_keycloak_realm(realm: &str) -> Result<()> {
    let client = KeycloakAdminClient::new().await?;
    remove_realm_jwks(&realm).await?;

    // DELETE is idempotent only when the realm is confirmed absent. A failed
    // existence lookup must never turn authorization or transport errors into success.
    realm_deletion_result(client.client.realm_delete(realm).await)
}

fn realm_deletion_result(result: std::result::Result<(), KeycloakError>) -> Result<()> {
    match result {
        Ok(()) | Err(KeycloakError::HttpFailure { status: 404, .. }) => Ok(()),
        Err(error) => Err(anyhow::Error::new(error)).context("Failed to delete Keycloak realm"),
    }
}

/// Retire protocol boards while preserving their signed records.
pub async fn archive_protocol_boards(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    event_id: &str,
) -> Result<()> {
    let boards = crate::postgres::protocol_board::get_event_protocol_boards(
        transaction,
        tenant_id,
        event_id,
    )
    .await?;
    if boards.is_empty() {
        return Ok(());
    }
    let url = std::env::var("WBRAID_BOARD_DATABASE_URL")
        .context("WBRAID_BOARD_DATABASE_URL must be set")?;
    let mut options: sqlx::postgres::PgConnectOptions = url.parse()?;
    if let Ok(password) = std::env::var("WBRAID_B4_PG_PASSWORD") {
        options = options.password(&password);
    }
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    let db = wbraid_b4::db::BoardDb::from_postgres_pool(pool).await?;
    let pool = db
        .postgres_pool()
        .context("native board archive requires PostgreSQL")?;
    for row in boards {
        sqlx::query("UPDATE boards SET status='archived' WHERE name=$1")
            .bind(&row.name)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Drop the election event's electoral-log database: its board, the logs the board
/// continues and its ballot box.
#[instrument(err)]
pub async fn delete_election_event_electoral_log(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    get_event_databases()
        .await?
        .drop_event(tenant_id, election_event_id)
        .await
}

/// After creating or importing an election event failed, drop the electoral-log
/// database created for it, which the rolled-back Hasura transaction no longer has,
/// also when the failure came before the database was registered. The database is
/// kept when an election event with that ID exists in any tenant, or when it is
/// registered to another tenant. Failures are logged.
pub async fn drop_uncommitted_event_database(tenant_id: &str, election_event_id: &str) {
    let dropped: Result<bool> = async {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction = client.build_transaction().read_only(true).start().await?;
        let exists = election_event_id_exists(&transaction, election_event_id).await?;
        transaction.commit().await?;
        if exists {
            return Ok(false);
        }
        let databases = get_event_databases().await?;
        if let Some(entry) = databases.event(election_event_id).await? {
            if uuid::Uuid::parse_str(&entry.tenant_id)? != uuid::Uuid::parse_str(tenant_id)? {
                return Ok(false);
            }
        }
        databases.drop_event(tenant_id, election_event_id).await?;
        Ok(true)
    }
    .await;
    match dropped {
        Ok(true) => info!(
            "Dropped the electoral-log database, if any, of the uncreated election event {election_event_id}"
        ),
        Ok(false) => {}
        Err(error) => tracing::error!(
            "Could not drop the electoral-log database of the uncreated election event {election_event_id}: {error:?}"
        ),
    }
}

#[instrument(err)]
pub async fn delete_election_event_related_documents(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let documents_prefix = format!("tenant-{}/event-{}/", tenant_id, election_event_id);
    let bucket = s3::get_private_bucket()?;
    s3::delete_files_from_s3(bucket, documents_prefix.clone(), s3::S3Endpoint::Server)
        .await
        .map_err(|err| anyhow!("Error delete private files from s3: {err:?}"))?;

    // Also delete the public files related to the election event, such as the election event config
    let public_bucket = s3::get_public_bucket()?;
    s3::delete_files_from_s3(
        public_bucket.clone(),
        documents_prefix,
        s3::S3Endpoint::Server,
    )
    .await
    .map_err(|err| anyhow!("Error delete public files from s3: {err:?}"))?;

    let results_index_key = format!("results-index/{election_event_id}.json");
    s3::delete_files_from_s3(public_bucket, results_index_key, s3::S3Endpoint::Server)
        .await
        .map_err(|err| anyhow!("Error delete public results index from s3: {err:?}"))?;
    Ok(())
}

#[cfg(test)]
mod realm_deletion_tests {
    use super::*;

    fn http_failure(status: u16) -> KeycloakError {
        KeycloakError::HttpFailure {
            status,
            body: None,
            text: String::new(),
        }
    }

    #[test]
    fn deleted_and_already_absent_realms_complete_cleanup() {
        assert!(realm_deletion_result(Ok(())).is_ok());
        assert!(realm_deletion_result(Err(http_failure(404))).is_ok());
    }

    #[test]
    fn other_http_failures_preserve_the_error_for_the_cleanup_task() {
        for status in [400, 401, 403, 409, 429, 500, 503] {
            let error = realm_deletion_result(Err(http_failure(status))).unwrap_err();
            assert!(matches!(
                error.downcast_ref::<KeycloakError>(),
                Some(KeycloakError::HttpFailure { status: actual, .. }) if *actual == status
            ));
        }
    }

    #[test]
    fn request_failures_do_not_mean_the_realm_is_absent() {
        let request_error = reqwest::Client::new().get("http://[").build().unwrap_err();
        let error =
            realm_deletion_result(Err(KeycloakError::ReqwestFailure(request_error))).unwrap_err();
        assert!(matches!(
            error.downcast_ref::<KeycloakError>(),
            Some(KeycloakError::ReqwestFailure(_))
        ));
    }
}

#[cfg(test)]
mod deletion_refusal_tests {
    use super::*;
    use serde_json::{json, Value};

    const LABEL: &str = "north";

    fn election_event(presentation: Option<Value>, status: Option<Value>) -> ElectionEvent {
        serde_json::from_value(json!({
            "id": "event",
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
        let elections = [election(
            Some(json!({"voting_status": "CLOSED", "telephone_voting_status": "CLOSED"})),
            None,
        )];
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
        for channel in [
            "voting_status",
            "kiosk_voting_status",
            "early_voting_status",
            "telephone_voting_status",
        ] {
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

    #[test]
    fn authorized_teardown_accepts_only_the_stale_parent_summary() {
        for channel in [
            "voting_status",
            "kiosk_voting_status",
            "early_voting_status",
            "telephone_voting_status",
        ] {
            for status in ["OPEN", "PAUSED"] {
                let event = election_event(None, Some(json!({channel: status})));
                let elections = [election(Some(json!({"voting_status":"CLOSED"})), None)];
                assert_eq!(
                    deletion_refusal_in_context(&event, &elections, &[], DeletionContext::General)
                        .unwrap(),
                    Some(ElectionEventDeletionRefusal::VotingInProgress)
                );
                assert_eq!(
                    deletion_refusal_in_context(
                        &event,
                        &elections,
                        &[],
                        DeletionContext::AuthorizedPostProcessing
                    )
                    .unwrap(),
                    None
                );
            }
        }
    }

    #[test]
    fn authorized_teardown_still_refuses_every_active_child_channel_even_if_locked() {
        for locked in [None, Some(json!({"locked_down":"locked-down"}))] {
            for channel in [
                "voting_status",
                "kiosk_voting_status",
                "early_voting_status",
                "telephone_voting_status",
            ] {
                for status in ["OPEN", "PAUSED"] {
                    let event =
                        election_event(locked.clone(), Some(json!({"voting_status":"CLOSED"})));
                    let elections = [
                        election(Some(json!({"voting_status":"CLOSED"})), None),
                        election(Some(json!({channel:status})), None),
                    ];
                    assert_eq!(
                        deletion_refusal_in_context(
                            &event,
                            &elections,
                            &[],
                            DeletionContext::AuthorizedPostProcessing
                        )
                        .unwrap(),
                        Some(ElectionEventDeletionRefusal::VotingInProgress)
                    );
                }
            }
        }
    }

    #[test]
    fn authorized_teardown_preserves_parsing_and_label_boundaries() {
        let context = DeletionContext::AuthorizedPostProcessing;
        let event = election_event(
            Some(json!({"locked_down":"locked-down"})),
            Some(json!({"voting_status":"OPEN"})),
        );
        assert_eq!(
            deletion_refusal_in_context(&event, &[election(None, Some(LABEL))], &[], context)
                .unwrap(),
            Some(ElectionEventDeletionRefusal::PermissionLabels)
        );
        assert!(deletion_refusal_in_context(
            &event,
            &[election(Some(json!({"voting_status":"UNKNOWN"})), None)],
            &[],
            context
        )
        .is_err());
        let malformed = election_event(None, Some(json!({"voting_status":"UNKNOWN"})));
        assert!(deletion_refusal_in_context(&malformed, &[], &[], context).is_err());
        let malformed = election_event(Some(json!({"locked_down":"maybe"})), None);
        assert!(deletion_refusal_in_context(&malformed, &[], &[], context).is_err());
        assert_eq!(
            deletion_refusal_in_context(
                &event,
                &[election(Some(json!({"voting_status":"CLOSED"})), None)],
                &[],
                context
            )
            .unwrap(),
            Some(ElectionEventDeletionRefusal::LockedDown)
        );
        assert_eq!(
            deletion_refusal_in_context(
                &election_event(None, Some(json!({"voting_status":"OPEN"}))),
                &[],
                &[],
                context
            )
            .unwrap(),
            Some(ElectionEventDeletionRefusal::VotingInProgress)
        );
    }
}
