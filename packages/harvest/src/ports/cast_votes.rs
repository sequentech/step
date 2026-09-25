// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::ballot::VotingStatusChannel;
use windmill::services::insert_cast_vote::{
    CastVoteError, InsertCastVoteInput, InsertCastVoteResult,
};

/// Where a voter's ballot is checked and stored.
#[rocket::async_trait]
pub trait CastVotes: Send + Sync {
    /// One attempt; handlers retry the errors it returns as `Err`.
    async fn try_insert(
        &self,
        input: InsertCastVoteInput,
        tenant_id: &str,
        voter_id: &str,
        area_id: &str,
        voting_channel: VotingStatusChannel,
        auth_time: &Option<i64>,
        voter_ip: &Option<String>,
        voter_country: &Option<String>,
        username: &Option<String>,
    ) -> Result<InsertCastVoteResult, CastVoteError>;
}
