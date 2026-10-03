// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::ballot::VotingStatusChannel;
use windmill::services::cast_ballot::{CastBallotInput, CastBallotResult};
use windmill::services::insert_cast_vote::{
    CastVoteError, InsertCastVoteInput, InsertCastVoteResult,
};
use windmill::services::receive_ballot::{
    ReceiveBallotInput, ReceiveBallotOutput,
};

/// Authenticated voter context forwarded to Windmill's insertion.
pub struct CastVoter<'a> {
    pub tenant_id: &'a str,
    pub voter_id: &'a str,
    pub area_id: &'a str,
    pub voting_channel: VotingStatusChannel,
    pub auth_time: &'a Option<i64>,
    pub voter_ip: &'a Option<String>,
    pub voter_country: &'a Option<String>,
    pub username: &'a Option<String>,
}

/// Where a voter's ballot is checked and stored.
#[rocket::async_trait]
pub trait CastVotes: Send + Sync {
    /// One attempt; handlers retry the errors it returns as `Err`.
    async fn try_insert(
        &self,
        input: InsertCastVoteInput,
        voter: CastVoter<'_>,
    ) -> Result<InsertCastVoteResult, CastVoteError>;

    /// Stores a ballot at review and answers with the ballot box's receipt.
    async fn try_receive(
        &self,
        input: ReceiveBallotInput,
        voter: CastVoter<'_>,
    ) -> Result<ReceiveBallotOutput, CastVoteError>;

    /// Casts a ballot received at review, on the voter's signed request, and
    /// answers with the ballot box's cast receipt.
    async fn try_cast(
        &self,
        input: CastBallotInput,
        voter: CastVoter<'_>,
    ) -> Result<CastBallotResult, CastVoteError>;
}
