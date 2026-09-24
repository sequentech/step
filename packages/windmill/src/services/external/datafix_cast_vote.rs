// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Resolves votes cast in Datafix events: they are inserted in progress and
//! become valid or discarded here.

use crate::domain::datafix_cast_vote::{
    discard_audit, pre_send_decision, screen_voter, set_voted_audit, set_voted_reply_update,
    set_voted_retry, InternetChannelUpdate, PreSendDecision, VoterScreening,
};
use crate::ports::clock::{Clock, IdGenerator};
use crate::ports::datafix_cast_vote::{
    DatafixAudit, DatafixElectionEvents, DatafixVoterDirectory, DatafixVoterLocks, DatafixVotes,
    PreparedSetVoted, VoterView,
};
use crate::services::cast_votes::{CastVote, CastVoteStatus};
use crate::services::external::utils::{
    datafix_annotations, external_voter_lock_key, DATAFIX_VOTER_LOCK_SECS,
};
use crate::types::error::{Error, Result};
use chrono::Duration;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::ElectionEvent;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

pub struct DatafixCastVoteProcessor<V, E, D, S, L, A, C, I> {
    pub votes: V,
    pub election_events: E,
    pub voters: D,
    pub voter_view: S,
    pub locks: L,
    pub audit: A,
    pub clock: C,
    pub ids: I,
}

impl<V, E, D, S, L, A, C, I> DatafixCastVoteProcessor<V, E, D, S, L, A, C, I>
where
    V: DatafixVotes,
    E: DatafixElectionEvents,
    D: DatafixVoterDirectory,
    S: VoterView,
    L: DatafixVoterLocks,
    A: DatafixAudit,
    C: Clock,
    I: IdGenerator,
{
    /// Processes a single Datafix vote left `in-progress` by the insert path:
    /// reloads the row, skips it unless it is still `in-progress`, then takes
    /// the event-wide per-voter lock and delegates the actual send to
    /// `process_locked`. The lock is always released, and its release error
    /// is only surfaced after the processing result so a failed send is not
    /// masked.
    pub async fn process(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        cast_vote_id: &str,
    ) -> Result<()> {
        let cast_vote_id =
            Uuid::parse_str(cast_vote_id).map_err(|err| format!("Invalid cast_vote_id: {err}"))?;
        let Some(cast_vote) = self
            .load_cast_vote(tenant_id, election_event_id, &cast_vote_id)
            .await?
        else {
            info!("Cast vote no longer exists; skipping");
            return Ok(());
        };
        if cast_vote.status != CastVoteStatus::InProgress {
            info!("Cast vote is no longer in-progress; skipping");
            return Ok(());
        }

        let voter_id = cast_vote
            .voter_id_string
            .as_deref()
            .ok_or("Voter id not found")?;
        let voter_id =
            Uuid::parse_str(voter_id).map_err(|err| format!("Invalid voter id: {err}"))?;
        let lock = match self
            .locks
            .acquire(
                external_voter_lock_key(
                    &cast_vote.tenant_id,
                    &cast_vote.election_event_id,
                    &voter_id,
                ),
                self.ids.new_id().to_string(),
                self.clock.now() + Duration::seconds(DATAFIX_VOTER_LOCK_SECS),
            )
            .await
        {
            Ok(lock) => lock,
            Err(err) => {
                info!("Another Datafix voter operation owns the lock: {err}");
                return Ok(());
            }
        };

        let result = self
            .process_locked(tenant_id, election_event_id, &cast_vote_id, &lock)
            .await;
        let release_result = self.locks.release(lock).await;
        result?;
        release_result.map_err(|err| format!("Error releasing Datafix voter lock: {err}"))?;
        Ok(())
    }

    /// Runs the Datafix send while the per-voter lock is held: validates the
    /// event's Datafix configuration, resolves the voter, and sends `SetVoted`,
    /// transitioning the row to its terminal status. A request without a
    /// usable reply leaves the vote `in-progress`: it is retried on the next
    /// beat and, if the situation persists, requires manual reconciliation.
    #[instrument(skip(self, lock), fields(cast_vote_id = %cast_vote_id), err)]
    async fn process_locked(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        cast_vote_id: &Uuid,
        lock: &L::Lock,
    ) -> Result<()> {
        let Some(cast_vote) = self
            .load_cast_vote(tenant_id, election_event_id, cast_vote_id)
            .await?
        else {
            return Ok(());
        };
        if cast_vote.status != CastVoteStatus::InProgress {
            info!("Cast vote is no longer in-progress; skipping");
            return Ok(());
        }

        let voter_id = cast_vote
            .voter_id_string
            .as_deref()
            .ok_or("Voter id not found")?;
        let election_event = self.load_election_event(&cast_vote).await?;
        datafix_annotations(&election_event)
            .map_err(|err| format!("Invalid Datafix configuration: {err}"))?
            .ok_or("Cast vote is pending but the election event is not configured for Datafix")?;

        let realm = get_event_realm(&cast_vote.tenant_id, &cast_vote.election_event_id);
        let user = self
            .voters
            .voter(&realm, voter_id)
            .await
            .map_err(task_error)?;
        let username = user.username.clone().ok_or("Username is None")?;
        self.locks
            .extend(lock, DATAFIX_VOTER_LOCK_SECS)
            .await
            .map_err(|err| format!("Datafix voter lock was lost after Keycloak lookup: {err}"))?;

        let channel = match screen_voter(&user) {
            VoterScreening::Discard => {
                let changed = self
                    .transition_cast_vote(
                        &cast_vote,
                        CastVoteStatus::InProgress,
                        CastVoteStatus::Discarded,
                    )
                    .await?;
                self.audit_operation(&cast_vote, voter_id, &username, discard_audit(changed))
                    .await;
                return Ok(());
            }
            VoterScreening::Eligible(channel) => channel,
        };

        let prior_valid_vote = self.has_prior_valid_vote(&cast_vote, voter_id).await?;
        match pre_send_decision(channel, prior_valid_vote) {
            PreSendDecision::Validate(update) => {
                let changed = self
                    .transition_cast_vote(
                        &cast_vote,
                        CastVoteStatus::InProgress,
                        CastVoteStatus::Valid,
                    )
                    .await?;
                if changed && update == InternetChannelUpdate::Mark {
                    self.mark_voted_via_internet(&realm, voter_id).await;
                }
                return Ok(());
            }
            PreSendDecision::SendSetVoted => {}
        }

        let prepared = self
            .voter_view
            .prepare_set_voted(election_event, &username)
            .await
            .map_err(|err| format!("Unable to prepare SetVoted before dispatch: {err}"))?;

        self.locks
            .extend(lock, DATAFIX_VOTER_LOCK_SECS)
            .await
            .map_err(|err| format!("Datafix voter lock was lost before SetVoted: {err}"))?;

        let template_sha256 = prepared.template_sha256().to_string();
        let result = self.voter_view.send(prepared).await;
        match &result {
            Ok(result) => info!(
                template_sha256 = %result.template_sha256,
                response = result.response.classification(),
                "Datafix API response received"
            ),
            Err(err) => info!(
                template_sha256 = %template_sha256,
                "Datafix API request failed: {err}"
            ),
        }

        match result {
            Ok(result) => {
                let changed = match set_voted_reply_update(&result.response) {
                    InternetChannelUpdate::Mark => {
                        self.transition_cast_vote_and_mark_internet(&cast_vote, &realm, voter_id)
                            .await?
                    }
                    InternetChannelUpdate::Leave => {
                        self.transition_cast_vote(
                            &cast_vote,
                            CastVoteStatus::InProgress,
                            CastVoteStatus::Valid,
                        )
                        .await?
                    }
                };
                let operation = set_voted_audit(&result.response, changed, &result.template_sha256);
                self.audit_operation(&cast_vote, voter_id, &username, operation)
                    .await;
                Ok(())
            }
            Err(err) => {
                let retry = set_voted_retry(&err, &template_sha256);
                self.audit_operation(&cast_vote, voter_id, &username, retry.audit)
                    .await;
                Err(retry.error.into())
            }
        }
    }

    /// Loads the cast vote by id in its own short transaction, or `None` if it
    /// no longer exists.
    #[instrument(skip(self), fields(cast_vote_id = %cast_vote_id), err)]
    async fn load_cast_vote(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        cast_vote_id: &Uuid,
    ) -> Result<Option<CastVote>> {
        self.votes
            .load(tenant_id, election_event_id, cast_vote_id)
            .await
            .map_err(task_error)
    }

    /// Loads the election event that owns the cast vote, needed for its
    /// Datafix configuration and realm.
    #[instrument(skip(self, cast_vote), fields(election_event_id = %cast_vote.election_event_id), err)]
    async fn load_election_event(&self, cast_vote: &CastVote) -> Result<ElectionEvent> {
        self.election_events
            .get(&cast_vote.tenant_id, &cast_vote.election_event_id)
            .await
            .map_err(task_error)
    }

    /// Returns whether the voter already has a `valid` vote for the event; a
    /// prior valid vote means this ballot must not be counted a second time.
    #[instrument(skip(self, cast_vote), fields(cast_vote_id = %cast_vote.id), err)]
    async fn has_prior_valid_vote(&self, cast_vote: &CastVote, voter_id: &str) -> Result<bool> {
        self.votes
            .has_valid_vote(&cast_vote.tenant_id, &cast_vote.election_event_id, voter_id)
            .await
            .map_err(task_error)
    }

    /// Compare-and-sets the vote from `expected` to `next` in its own
    /// transaction, returning whether the row moved. A `false` result is
    /// logged (not an error): it means another worker already advanced the
    /// row past `expected`.
    #[instrument(skip(self, cast_vote), fields(cast_vote_id = %cast_vote.id), err)]
    async fn transition_cast_vote(
        &self,
        cast_vote: &CastVote,
        expected: CastVoteStatus,
        next: CastVoteStatus,
    ) -> Result<bool> {
        let cast_vote_id = Uuid::parse_str(&cast_vote.id)
            .map_err(|err| format!("Invalid cast_vote_id in stored row: {err}"))?;
        let changed = self
            .votes
            .compare_and_set(
                &cast_vote.tenant_id,
                &cast_vote.election_event_id,
                &cast_vote_id,
                expected,
                next,
            )
            .await
            .map_err(task_error)?;
        if !changed {
            warn!("Cast vote status changed concurrently; terminal status was not overwritten");
        }
        Ok(changed)
    }

    /// Promotes an in-progress vote and then records the Internet channel. A
    /// Keycloak failure is traced and left for the reconciliation process; it
    /// does not roll back the terminal Hasura status.
    #[instrument(skip(self, cast_vote), fields(cast_vote_id = %cast_vote.id), err)]
    async fn transition_cast_vote_and_mark_internet(
        &self,
        cast_vote: &CastVote,
        realm: &str,
        voter_id: &str,
    ) -> Result<bool> {
        let changed = self
            .transition_cast_vote(cast_vote, CastVoteStatus::InProgress, CastVoteStatus::Valid)
            .await?;
        if changed {
            self.mark_voted_via_internet(realm, voter_id).await;
        }
        Ok(changed)
    }

    async fn mark_voted_via_internet(&self, realm: &str, voter_id: &str) {
        if let Err(err) = self.voters.mark_voted_via_internet(realm, voter_id).await {
            error!("Could not mark the voter Internet channel: {err}");
        }
    }

    /// Records the outcome of an outbound Datafix operation in the electoral
    /// log.
    #[instrument(skip(self, cast_vote), fields(cast_vote_id = %cast_vote.id))]
    async fn audit_operation(
        &self,
        cast_vote: &CastVote,
        voter_id: &str,
        username: &str,
        operation: String,
    ) {
        let operation = format!("cast_vote_id={}; {operation}", cast_vote.id);
        self.audit
            .record(
                &cast_vote.tenant_id,
                &cast_vote.election_event_id,
                voter_id,
                username,
                operation,
            )
            .await;
    }
}

/// The adapters word their errors as the task has always reported them; the
/// task error keeps that text as a plain message.
fn task_error(err: anyhow::Error) -> Error {
    Error::String(err.to_string())
}
