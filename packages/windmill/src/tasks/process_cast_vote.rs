// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::adapters::datafix_cast_vote::{
    ElectoralLogDatafixAudit, KeycloakDatafixVoterDirectory, PgDatafixElectionEvents,
    PgDatafixVoterLocks, PgDatafixVotes, SoapVoterView,
};
use crate::adapters::system::{RandomIds, SystemClock};
use crate::services::external::datafix_cast_vote::DatafixCastVoteProcessor;
use crate::types::error::Result;
use celery::error::TaskError;
use tracing::instrument;

/// Processes a single Datafix vote left `in-progress` by the insert path; see
/// `DatafixCastVoteProcessor::process`. `max_retries = 0` because the review
/// beat, not Celery, drives retries.
#[instrument(fields(cast_vote_id = %cast_vote_id), err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn process_cast_vote(
    tenant_id: String,
    election_event_id: String,
    cast_vote_id: String,
) -> Result<()> {
    DatafixCastVoteProcessor {
        votes: PgDatafixVotes,
        election_events: PgDatafixElectionEvents,
        voters: KeycloakDatafixVoterDirectory,
        voter_view: SoapVoterView,
        locks: PgDatafixVoterLocks,
        audit: ElectoralLogDatafixAudit,
        clock: SystemClock,
        ids: RandomIds,
    }
    .process(&tenant_id, &election_event_id, &cast_vote_id)
    .await
}

#[cfg(test)]
mod tests {
    use crate::services::external::utils::external_voter_lock_key;
    use uuid::Uuid;

    #[test]
    fn voter_lock_is_event_wide() {
        let voter = Uuid::new_v4();
        let other_voter = Uuid::new_v4();
        let first = external_voter_lock_key("tenant", "event", &voter);
        let second = external_voter_lock_key("tenant", "event", &voter);
        assert_eq!(first, second);
        assert_ne!(
            first,
            external_voter_lock_key("tenant", "other-event", &voter)
        );
        assert_ne!(
            first,
            external_voter_lock_key("tenant", "event", &other_voter)
        );
    }
}
