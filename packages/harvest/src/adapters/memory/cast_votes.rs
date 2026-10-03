// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::cast_votes::{CastVoter, CastVotes};
use sequent_core::ballot::VotingStatusChannel;
use std::collections::VecDeque;
use std::sync::Mutex;
use windmill::services::insert_cast_vote::{
    CastVoteError, InsertCastVoteInput, InsertCastVoteResult,
};
use windmill::services::cast_ballot::{CastBallotInput, CastBallotResult};
use windmill::services::receive_ballot::{
    ReceiveBallotInput, ReceiveBallotOutput,
};

/// Who cast a vote, as the insertion received it.
#[derive(Clone, Debug, PartialEq)]
pub struct Attempt {
    pub ballot_id: String,
    pub election_id: String,
    pub content: String,
    pub tenant_id: String,
    pub auth_time: Option<i64>,
    pub username: Option<String>,
    pub voter_id: String,
    pub area_id: String,
    pub voting_channel: VotingStatusChannel,
    pub voter_ip: Option<String>,
    pub voter_country: Option<String>,
}

/// Answers each attempt with the next scripted outcome.
#[derive(Default)]
pub struct ScriptedCastVotes {
    outcomes: Mutex<VecDeque<Result<InsertCastVoteResult, CastVoteError>>>,
    attempts: Mutex<Vec<Attempt>>,
    receipts: Mutex<VecDeque<Result<ReceiveBallotOutput, CastVoteError>>>,
    receive_attempts: Mutex<Vec<Attempt>>,
    casts: Mutex<VecDeque<Result<CastBallotResult, CastVoteError>>>,
    cast_attempts: Mutex<Vec<Attempt>>,
}

impl ScriptedCastVotes {
    pub fn receiving(
        receipts: impl IntoIterator<
            Item = Result<ReceiveBallotOutput, CastVoteError>,
        >,
    ) -> Self {
        Self {
            receipts: Mutex::new(receipts.into_iter().collect()),
            ..Default::default()
        }
    }

    pub fn receive_attempts(&self) -> Vec<Attempt> {
        self.receive_attempts.lock().unwrap().clone()
    }

    pub fn casting(
        casts: impl IntoIterator<Item = Result<CastBallotResult, CastVoteError>>,
    ) -> Self {
        Self {
            casts: Mutex::new(casts.into_iter().collect()),
            ..Default::default()
        }
    }

    /// Casts of received ballots: `content` is the voter's Cast signature.
    pub fn cast_attempts(&self) -> Vec<Attempt> {
        self.cast_attempts.lock().unwrap().clone()
    }

    pub fn answering(
        outcomes: impl IntoIterator<
            Item = Result<InsertCastVoteResult, CastVoteError>,
        >,
    ) -> Self {
        Self {
            outcomes: Mutex::new(outcomes.into_iter().collect()),
            ..Default::default()
        }
    }

    pub fn attempts(&self) -> Vec<Attempt> {
        self.attempts.lock().unwrap().clone()
    }
}

#[rocket::async_trait]
impl CastVotes for ScriptedCastVotes {
    async fn try_insert(
        &self,
        input: InsertCastVoteInput,
        voter: CastVoter<'_>,
    ) -> Result<InsertCastVoteResult, CastVoteError> {
        self.attempts.lock().unwrap().push(Attempt {
            ballot_id: input.ballot_id,
            election_id: input.election_id.to_string(),
            content: input.content,
            tenant_id: voter.tenant_id.to_string(),
            auth_time: *voter.auth_time,
            username: voter.username.clone(),
            voter_id: voter.voter_id.to_string(),
            area_id: voter.area_id.to_string(),
            voting_channel: voter.voting_channel,
            voter_ip: voter.voter_ip.clone(),
            voter_country: voter.voter_country.clone(),
        });
        self.outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("a scripted outcome for every attempt")
    }

    async fn try_receive(
        &self,
        input: ReceiveBallotInput,
        voter: CastVoter<'_>,
    ) -> Result<ReceiveBallotOutput, CastVoteError> {
        self.receive_attempts.lock().unwrap().push(Attempt {
            ballot_id: input.ballot_id,
            election_id: input.election_id.to_string(),
            content: input.content,
            tenant_id: voter.tenant_id.to_string(),
            auth_time: *voter.auth_time,
            username: voter.username.clone(),
            voter_id: voter.voter_id.to_string(),
            area_id: voter.area_id.to_string(),
            voting_channel: voter.voting_channel,
            voter_ip: voter.voter_ip.clone(),
            voter_country: voter.voter_country.clone(),
        });
        self.receipts
            .lock()
            .unwrap()
            .pop_front()
            .expect("a scripted receipt for every received ballot")
    }

    async fn try_cast(
        &self,
        input: CastBallotInput,
        voter: CastVoter<'_>,
    ) -> Result<CastBallotResult, CastVoteError> {
        self.cast_attempts.lock().unwrap().push(Attempt {
            ballot_id: input.ballot_id,
            election_id: input.election_id.to_string(),
            content: input.cast_signature,
            tenant_id: voter.tenant_id.to_string(),
            auth_time: *voter.auth_time,
            username: voter.username.clone(),
            voter_id: voter.voter_id.to_string(),
            area_id: voter.area_id.to_string(),
            voting_channel: voter.voting_channel,
            voter_ip: voter.voter_ip.clone(),
            voter_country: voter.voter_country.clone(),
        });
        self.casts
            .lock()
            .unwrap()
            .pop_front()
            .expect("a scripted outcome for every cast")
    }
}
