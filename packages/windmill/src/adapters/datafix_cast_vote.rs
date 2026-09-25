// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Production adapters for Datafix cast-vote processing. The task holds no
//! database connection across the VoterView call, so every operation takes
//! its own pooled connection and transaction.

use crate::ports::datafix_cast_vote::{
    DatafixAudit, DatafixElectionEvents, DatafixVoterDirectory, DatafixVoterLocks, DatafixVotes,
    PreparedSetVoted, VoterView,
};
use crate::postgres::cast_vote::{
    compare_and_set_cast_vote_status, get_cast_vote_by_id, has_valid_cast_vote,
};
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::cast_votes::{CastVote, CastVoteStatus};
use crate::services::database::get_hasura_pool;
use crate::services::external::datafix_types::{SoapRequest, SoapRequestResult};
use crate::services::external::utils::post_operation_result_to_electoral_log;
use crate::services::external::voterview_requests::{self, PreparedSoapRequest, SoapSendError};
use crate::services::pg_lock::PgLock;
use crate::types::error::Result;
use anyhow::anyhow;
use chrono::{DateTime, Local};
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::messages::newtypes::ExtApiRequestDirection;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::{User, VOTED_CHANNEL, VOTED_CHANNEL_INTERNET_VALUE};
use sequent_core::util::retry::retry_with_exponential_backoff;
use std::collections::HashMap;
use std::time::Duration as StdDuration;
use tracing::{error, instrument};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default)]
pub struct PgDatafixVotes;

impl DatafixVotes for PgDatafixVotes {
    async fn load(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        cast_vote_id: &Uuid,
    ) -> anyhow::Result<Option<CastVote>> {
        let mut client = hasura_client().await?;
        let transaction = begin(&mut client).await?;
        get_cast_vote_by_id(&transaction, tenant_id, election_event_id, cast_vote_id)
            .await
            .map_err(|err| anyhow!("Error loading cast vote: {err:?}"))
    }

    async fn has_valid_vote(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        voter_id: &str,
    ) -> anyhow::Result<bool> {
        let mut client = hasura_client().await?;
        let transaction = begin(&mut client).await?;
        has_valid_cast_vote(&transaction, tenant_id, election_event_id, voter_id)
            .await
            .map_err(|err| anyhow!("Error checking prior valid votes: {err:?}"))
    }

    async fn compare_and_set(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        cast_vote_id: &Uuid,
        expected: CastVoteStatus,
        next: CastVoteStatus,
    ) -> anyhow::Result<bool> {
        let mut client = hasura_client().await?;
        let transaction = begin(&mut client).await?;
        let changed = compare_and_set_cast_vote_status(
            &transaction,
            tenant_id,
            election_event_id,
            cast_vote_id,
            expected,
            next,
        )
        .await
        .map_err(|err| anyhow!("Error transitioning cast vote status: {err:?}"))?;
        transaction
            .commit()
            .await
            .map_err(|err| anyhow!("Error committing cast vote status: {err:?}"))?;
        Ok(changed)
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
