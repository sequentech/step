// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::cast_votes::CastVotes;
use sequent_core::ballot::VotingStatusChannel;
use windmill::services::insert_cast_vote::{
    try_insert_cast_vote, CastVoteError, InsertCastVoteInput,
    InsertCastVoteResult,
};

/// Windmill's cast vote insertion, on its process-wide pools.
pub struct WindmillCastVotes;

#[rocket::async_trait]
impl CastVotes for WindmillCastVotes {
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
    ) -> Result<InsertCastVoteResult, CastVoteError> {
        try_insert_cast_vote(
            input,
            tenant_id,
            voter_id,
            area_id,
            voting_channel,
            auth_time,
            voter_ip,
            voter_country,
            username,
        )
        .await
    }
}
