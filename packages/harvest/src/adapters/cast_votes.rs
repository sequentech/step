// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::cast_votes::{CastVoter, CastVotes};
use windmill::services::insert_cast_vote::{
    try_insert_cast_vote, CastVoteError, InsertCastVoteInput,
    InsertCastVoteResult,
};
use windmill::services::cast_ballot::{
    try_cast_ballot, CastBallotInput, CastBallotResult,
};
use windmill::services::receive_ballot::{
    try_receive_ballot, ReceiveBallotInput, ReceiveBallotOutput,
};

/// Windmill's cast vote insertion, on its process-wide pools.
pub struct WindmillCastVotes;

#[rocket::async_trait]
impl CastVotes for WindmillCastVotes {
    async fn try_insert(
        &self,
        input: InsertCastVoteInput,
        voter: CastVoter<'_>,
    ) -> Result<InsertCastVoteResult, CastVoteError> {
        try_insert_cast_vote(
            input,
            voter.tenant_id,
            voter.voter_id,
            voter.area_id,
            voter.voting_channel,
            voter.auth_time,
            voter.voter_ip,
            voter.voter_country,
            voter.username,
        )
        .await
    }

    async fn try_receive(
        &self,
        input: ReceiveBallotInput,
        voter: CastVoter<'_>,
    ) -> Result<ReceiveBallotOutput, CastVoteError> {
        try_receive_ballot(
            input,
            voter.tenant_id,
            voter.voter_id,
            voter.area_id,
            voter.voting_channel,
            voter.auth_time,
        )
        .await
    }

    async fn try_cast(
        &self,
        input: CastBallotInput,
        voter: CastVoter<'_>,
    ) -> Result<CastBallotResult, CastVoteError> {
        try_cast_ballot(
            input,
            voter.tenant_id,
            voter.voter_id,
            voter.area_id,
            voter.voting_channel,
            voter.auth_time,
            voter.voter_ip,
            voter.voter_country,
            voter.username,
        )
        .await
    }
}
