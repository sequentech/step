// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Production adapters for Datafix cast-vote processing. The task holds no
//! database connection across the VoterView call, so every operation takes
//! its own pooled connection.

use crate::ports::datafix_cast_vote::{
    DatafixAudit, DatafixElectionEvents, DatafixVoterDirectory, DatafixVoterLocks, DatafixVotes,
    PreparedSetVoted, VoterView,
};
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::ballot_box_reads::{ballot_box_time, cast_vote_status};
use crate::services::cast_votes::{CastVote, CastVoteStatus};
use crate::services::database::get_hasura_pool;
use crate::services::electoral_log_board::get_event_store;
use crate::services::external::datafix_types::{SoapRequest, SoapRequestResult};
use crate::services::external::utils::post_operation_result_to_electoral_log;
use crate::services::external::voterview_requests::{self, PreparedSoapRequest, SoapSendError};
use crate::services::pg_lock::PgLock;
use crate::types::error::Result;
use anyhow::anyhow;
use chrono::{DateTime, Local};
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::adapters::ballot_box::BallotStatus;
use electoral_log::adapters::ballot_box_seal::is_sealed_refusal;
use electoral_log::messages::newtypes::ExtApiRequestDirection;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::{User, VOTED_CHANNEL, VOTED_CHANNEL_INTERNET_VALUE};
use sequent_core::util::retry::retry_with_exponential_backoff;
use std::collections::HashMap;
use std::time::Duration as StdDuration;
use tracing::{error, instrument};
use uuid::Uuid;

/// The status a vote's ballot has in the ballot box.
fn ballot_status(status: CastVoteStatus) -> BallotStatus {
    match status {
        CastVoteStatus::InProgress => BallotStatus::Pending,
        CastVoteStatus::Valid => BallotStatus::Valid,
        CastVoteStatus::Discarded => BallotStatus::Rejected,
    }
}

/// The election event's ballot box.
#[derive(Clone, Copy, Debug, Default)]
pub struct PgDatafixVotes;

impl DatafixVotes for PgDatafixVotes {
    async fn load(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        cast_vote_id: &Uuid,
    ) -> anyhow::Result<Option<CastVote>> {
        let ballot = get_event_store(election_event_id)
            .await?
            .pending_ballot(election_event_id, &cast_vote_id.to_string())
            .await
            .map_err(|err| anyhow!("Error loading cast vote: {err:?}"))?;
        ballot
            .map(|ballot| {
                let accepted_at = ballot_box_time(ballot.accepted_at)?;
                Ok(CastVote {
                    id: ballot.id,
                    tenant_id: tenant_id.to_string(),
                    election_id: Some(ballot.election_id),
                    area_id: Some(ballot.area_id),
                    created_at: Some(accepted_at),
                    last_updated_at: Some(accepted_at),
                    content: None,
                    voter_id_string: Some(ballot.voter_id),
                    election_event_id: election_event_id.to_string(),
                    ballot_id: Some(ballot.ballot_id),
                    cast_ballot_signature: None,
                    status: cast_vote_status(ballot.status),
                })
            })
            .transpose()
    }

    async fn has_valid_vote(
        &self,
        _tenant_id: &str,
        election_event_id: &str,
        voter_id: &str,
    ) -> anyhow::Result<bool> {
        let state = get_event_store(election_event_id)
            .await?
            .voter_ballot_state(election_event_id, voter_id)
            .await
            .map_err(|err| anyhow!("Error checking prior valid votes: {err:?}"))?;
        Ok(state.has_valid)
    }

    async fn compare_and_set(
        &self,
        _tenant_id: &str,
        election_event_id: &str,
        voter_id: &str,
        cast_vote_id: &Uuid,
        expected: CastVoteStatus,
        next: CastVoteStatus,
    ) -> anyhow::Result<bool> {
        get_event_store(election_event_id)
            .await?
            .set_ballot_status(
                election_event_id,
                voter_id,
                &cast_vote_id.to_string(),
                ballot_status(expected),
                ballot_status(next),
            )
            .await
            .map_err(|err| {
                // A sealed ballot box takes no change (VOTE-FREEZE).
                if is_sealed_refusal(&err) {
                    anyhow!(
                        "Error transitioning cast vote status: {}",
                        crate::services::ballot_box_seal::BALLOT_BOX_SEALED_MESSAGE
                    )
                } else {
                    anyhow!("Error transitioning cast vote status: {err:?}")
                }
            })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PgDatafixElectionEvents;

impl DatafixElectionEvents for PgDatafixElectionEvents {
    async fn get(&self, tenant_id: &str, election_event_id: &str) -> anyhow::Result<ElectionEvent> {
        let mut client = hasura_client().await?;
        let transaction = begin(&mut client).await?;
        get_election_event_by_id(&transaction, tenant_id, election_event_id)
            .await
            .map_err(|err| anyhow!("Error loading election event: {err:?}"))
    }
}

async fn hasura_client() -> anyhow::Result<DbClient> {
    get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting Hasura DB client: {err:?}"))
}

async fn begin(client: &mut DbClient) -> anyhow::Result<Transaction<'_>> {
    client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting Hasura transaction: {err:?}"))
}

#[derive(Clone, Copy, Debug, Default)]
pub struct KeycloakDatafixVoterDirectory;

impl DatafixVoterDirectory for KeycloakDatafixVoterDirectory {
    async fn voter(&self, realm: &str, voter_id: &str) -> anyhow::Result<User> {
        let keycloak = KeycloakAdminClient::new()
            .await
            .map_err(|err| anyhow!("Error obtaining Keycloak client: {err:?}"))?;
        keycloak
            .get_user(realm, voter_id)
            .await
            .map_err(|err| anyhow!("Error fetching voter from Keycloak: {err:?}"))
    }

    async fn mark_voted_via_internet(&self, realm: &str, voter_id: &str) -> anyhow::Result<()> {
        mark_voted_via_internet(realm, voter_id)
            .await
            .map_err(anyhow::Error::new)
    }
}

#[instrument(err)]
async fn mark_voted_via_internet(realm: &str, voter_id: &str) -> Result<()> {
    let mut attributes = HashMap::new();
    attributes.insert(
        VOTED_CHANNEL.to_string(),
        vec![VOTED_CHANNEL_INTERNET_VALUE.to_string()],
    );
    retry_with_exponential_backoff(
        || async {
            let client = KeycloakAdminClient::new()
                .await
                .map_err(|err| format!("Error obtaining Keycloak client: {err:?}"))?;
            client
                .edit_user(
                    realm,
                    voter_id,
                    None,
                    Some(attributes.clone()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(|err| format!("Error editing voter Internet channel: {err:?}"))
        },
        3,
        StdDuration::from_millis(500),
    )
    .await
    .map(|_| ())
    .map_err(|err| format!("Error editing voter Internet channel after retries: {err}").into())
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SoapVoterView;

impl PreparedSetVoted for PreparedSoapRequest {
    fn template_sha256(&self) -> &str {
        PreparedSoapRequest::template_sha256(self)
    }
}

impl VoterView for SoapVoterView {
    type Prepared = PreparedSoapRequest;

    async fn prepare_set_voted(
        &self,
        election_event: ElectionEvent,
        username: &str,
    ) -> anyhow::Result<PreparedSoapRequest> {
        voterview_requests::prepare(
            SoapRequest::SetVoted,
            ElectionEventDatafix(election_event),
            &Some(username.to_string()),
        )
        .await
    }

    async fn send(
        &self,
        prepared: PreparedSoapRequest,
    ) -> Result<SoapRequestResult, SoapSendError> {
        voterview_requests::send_prepared(prepared).await
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PgDatafixVoterLocks;

impl DatafixVoterLocks for PgDatafixVoterLocks {
    type Lock = PgLock;

    async fn acquire(
        &self,
        key: String,
        value: String,
        expiry_date: DateTime<Local>,
    ) -> anyhow::Result<PgLock> {
        PgLock::acquire(key, value, expiry_date).await
    }

    async fn extend(&self, lock: &PgLock, seconds: i64) -> anyhow::Result<()> {
        lock.update_expiry_for(seconds).await
    }

    async fn release(&self, lock: PgLock) -> anyhow::Result<()> {
        lock.release().await
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ElectoralLogDatafixAudit;

impl DatafixAudit for ElectoralLogDatafixAudit {
    async fn record(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        voter_id: &str,
        username: &str,
        operation: String,
    ) {
        let Ok(mut client) = get_hasura_pool().await.get().await else {
            error!("Unable to get a DB connection for the Datafix audit entry");
            return;
        };
        let Ok(transaction) = client.transaction().await else {
            error!("Unable to start a transaction for the Datafix audit entry");
            return;
        };
        if let Err(err) = post_operation_result_to_electoral_log(
            &transaction,
            tenant_id,
            election_event_id,
            Some(voter_id),
            username,
            ExtApiRequestDirection::Outbound,
            operation,
        )
        .await
        {
            error!("Unable to record the Datafix audit entry: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vote_statuses_are_ballot_box_statuses() {
        for status in [
            CastVoteStatus::InProgress,
            CastVoteStatus::Valid,
            CastVoteStatus::Discarded,
        ] {
            assert_eq!(cast_vote_status(ballot_status(status)), status);
        }
    }
}

#[cfg(test)]
mod postgres_wiring_tests {
    use super::*;
    use crate::services::ballot_box::BallotFormat;
    use crate::services::electoral_log_board::get_event_databases;
    use electoral_log::adapters::ballot_box::{AcceptBallot, AcceptOutcome};

    #[tokio::test]
    #[ignore = "requires a disposable ELECTORAL_LOG_PG_* database"]
    async fn a_pending_vote_is_loaded_and_resolved_once_in_the_ballot_box() {
        let (tenant, event) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
        let (election, area) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
        let store = get_event_databases()
            .await
            .unwrap()
            .create_event(&tenant, &event)
            .await
            .unwrap();
        store.create_ballot_box(&event).await.unwrap();
        let format = BallotFormat::HashableBallot.to_string();
        let AcceptOutcome::Accepted { id, .. } = store
            .accept_ballot(&AcceptBallot {
                election_event_id: &event,
                election_id: &election,
                area_id: &area,
                voter_id: "voter",
                ballot_id: "0abc12",
                format: &format,
                content: "ciphertext",
                voter_signature: None,
                admission_record: &format!(r#"{{"payload":{{"accepted_at":"2026-10-27T12:00:00.000Z","sequence":{}}},"signature":""}}"#, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64).into_bytes(),
                pseudonym_hash: &[0; 64],
                ballot_hash: &[0; 64],
                voting_channel: "ONLINE",
                status: BallotStatus::Pending,
                voter_ip: None,
                voter_country: None,
                username: None,
                allowed_votes: 0,
            })
            .await
            .unwrap()
        else {
            panic!("the ballot is accepted");
        };
        let id = Uuid::parse_str(&id).unwrap();

        let vote = PgDatafixVotes
            .load(&tenant, &event, &id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(vote.status, CastVoteStatus::InProgress);
        assert_eq!(vote.voter_id_string.as_deref(), Some("voter"));
        assert_eq!(vote.ballot_id.as_deref(), Some("0abc12"));
        assert!(!PgDatafixVotes
            .has_valid_vote(&tenant, &event, "voter")
            .await
            .unwrap());

        let resolve = |expected, next| {
            PgDatafixVotes.compare_and_set(&tenant, &event, "voter", &id, expected, next)
        };
        assert!(!resolve(CastVoteStatus::Valid, CastVoteStatus::Discarded)
            .await
            .unwrap());
        assert!(resolve(CastVoteStatus::InProgress, CastVoteStatus::Valid)
            .await
            .unwrap());
        assert!(
            !resolve(CastVoteStatus::InProgress, CastVoteStatus::Discarded)
                .await
                .unwrap()
        );
        assert!(PgDatafixVotes
            .has_valid_vote(&tenant, &event, "voter")
            .await
            .unwrap());
        // Only a pending vote is loaded.
        assert_eq!(
            PgDatafixVotes.load(&tenant, &event, &id).await.unwrap(),
            None
        );
    }
}
