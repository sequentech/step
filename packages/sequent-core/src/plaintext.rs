// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot_codec::multi_ballot::{
    BallotChoices, DecodedBallotChoices, DecodedContestChoice,
    DecodedContestChoices,
};
use crate::ballot_codec::PlaintextCodec;
use crate::multi_ballot::AuditableMultiBallotContests;
use crate::{ballot::*, multi_ballot::AuditableMultiBallot};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strand::context::Ctx;

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub enum InvalidPlaintextErrorType {
    Explicit,
    Implicit,
    EncodingError,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct InvalidPlaintextError {
    pub error_type: InvalidPlaintextErrorType,
    pub candidate_id: Option<String>,
    pub message: Option<String>,
    pub message_map: HashMap<String, String>,
}

/// Message key of the encoding error for a plaintext that holds more than
/// its ballot layout can.
pub const BALLOT_TOO_LARGE_ERROR_MESSAGE: &str =
    "errors.encoding.ballotTooLarge";

impl InvalidPlaintextError {
    /// Encoding error for a plaintext that holds more than its ballot layout
    /// can.
    pub fn ballot_too_large() -> Self {
        InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::EncodingError,
            candidate_id: None,
            message: Some(BALLOT_TOO_LARGE_ERROR_MESSAGE.to_string()),
            message_map: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct DecodedVoteContest {
    pub contest_id: String,
    pub is_explicit_invalid: bool,
    pub invalid_errors: Vec<InvalidPlaintextError>,
    pub invalid_alerts: Vec<InvalidPlaintextError>,
    pub choices: Vec<DecodedVoteChoice>,
}

impl DecodedVoteContest {
    /// A contest plaintext that does not decode under the contest's ballot
    /// layout. It has no selections and counts as an implicitly invalid vote.
    pub fn undecodable(contest: &Contest) -> Self {
        DecodedVoteContest {
            contest_id: contest.id.clone(),
            is_explicit_invalid: false,
            invalid_errors: vec![InvalidPlaintextError::ballot_too_large()],
            invalid_alerts: vec![],
            choices: contest
                .candidates
                .iter()
                .map(|candidate| DecodedVoteChoice {
                    id: candidate.id.clone(),
                    selected: -1,
                    write_in_text: None,
                })
                .collect(),
        }
    }

    pub fn is_invalid(&self) -> bool {
        self.is_explicit_invalid || !self.invalid_errors.is_empty()
    }
    pub fn is_blank(&self) -> bool {
        !self.is_invalid()
            && self
                .choices
                .clone()
                .iter()
                .all(|choice| choice.selected < 0)
    }
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct DecodedVoteChoice {
    pub id: String,
    pub selected: i64,
    pub write_in_text: Option<String>,
}

impl DecodedVoteChoice {
    pub fn is_selected(&self) -> bool {
        self.selected >= 0
    }
}

pub fn map_to_decoded_contest<C: Ctx<P = [u8; 30]>>(
    ballot: &AuditableBallot,
) -> Result<Vec<DecodedVoteContest>, String> {
    let mut decoded_contests = vec![];
    if ballot.config.contests.len() != ballot.contests.len() {
        return Err(format!(
            "Invalid number of contests {} != {}",
            ballot.config.contests.len(),
            ballot.contests.len()
        ));
    }

    let ballot_contests = ballot.deserialize_contests().map_err(|err| {
        format!("Error deserializing auditable ballot contest {:?}", err)
    })?;
    for contest in &ballot_contests {
        let found_contest = ballot
            .config
            .contests
            .iter()
            .find(|contest_el| contest_el.id == contest.contest_id)
            .ok_or_else(|| {
                format!(
                    "Can't find contest with id {} on ballot style",
                    contest.contest_id
                )
            })?;
        let replication_choice: &ReplicationChoice<C> = &contest.choice;
        let decoded_plaintext = found_contest
            .decode_plaintext_contest(&replication_choice.plaintext)?;
        decoded_contests.push(decoded_plaintext);
    }
    Ok(decoded_contests)
}

pub fn map_decoded_ballot_choices_to_decoded_contests(
    decoded_ballot_choices: DecodedBallotChoices,
    contests: &Vec<Contest>,
) -> Result<Vec<DecodedVoteContest>, String> {
    let mut decoded_contests = vec![];

    for found_contest in contests {
        let contest_id = found_contest.id.clone();
        let found_ballot_choices = decoded_ballot_choices
            .choices
            .iter()
            .find(|ballot_choice| ballot_choice.contest_id == contest_id)
            .ok_or_else(|| {
                format!(
                    "Can't find contest with id {} on ballot style",
                    contest_id
                )
            })?;

        let mut choices = vec![];

        for candidate in &found_contest.candidates {
            let selected = if found_ballot_choices
                .choices
                .iter()
                .find(|choice| choice.0 == candidate.id)
                .is_some()
            {
                0
            } else {
                -1
            };

            let decoded_vote_choice = DecodedVoteChoice {
                id: candidate.id.clone(),
                selected,
                write_in_text: None,
            };

            choices.push(decoded_vote_choice);
        }

        let decoded_contest = DecodedVoteContest {
            contest_id: contest_id,
            is_explicit_invalid: decoded_ballot_choices.is_explicit_invalid,
            invalid_errors: found_ballot_choices.invalid_errors.clone(),
            invalid_alerts: found_ballot_choices.invalid_alerts.clone(),
            choices,
        };

        decoded_contests.push(decoded_contest);
    }
    Ok(decoded_contests)
}

pub fn map_to_decoded_multi_contest<C: Ctx<P = [u8; 30]>>(
    ballot: &AuditableMultiBallot,
) -> Result<Vec<DecodedVoteContest>, String> {
    let ballot_contests: AuditableMultiBallotContests<C> =
        ballot.deserialize_contests().map_err(|err| {
            format!(
                "Error deserializing auditable multi ballot contest {:?}",
                err
            )
        })?;

    if ballot.config.contests.len() != ballot_contests.contest_ids.len() {
        return Err(format!(
            "Invalid number of contests {} != {}",
            ballot.config.contests.len(),
            ballot_contests.contest_ids.len()
        ));
    }

    let decoded_ballot_choices = BallotChoices::decode_from_30_bytes(
        &ballot_contests.choice.plaintext,
        &ballot.config,
    )
    .map_err(|err| {
        format!("Error decoding multi ballot plaintext {:?}", err)
    })?;

    let ballot_contests: AuditableMultiBallotContests<C> =
        ballot.deserialize_contests().map_err(|err| {
            format!("Error deserializing auditable ballot contest {:?}", err)
        })?;

    let mapped_contests: Vec<Contest> = ballot_contests
        .contest_ids
        .clone()
        .into_iter()
        .map(|contest_id| -> Result<Contest, String> {
            ballot
                .config
                .contests
                .clone()
                .into_iter()
                .find(|contest_el| contest_el.id == contest_id)
                .ok_or_else(|| {
                    format!(
                        "Can't find contest with id {} on ballot style",
                        contest_id
                    )
                })
        })
        .collect::<Result<Vec<_>, String>>()?;
    map_decoded_ballot_choices_to_decoded_contests(
        decoded_ballot_choices,
        &mapped_contests,
    )
}

#[cfg(test)]
mod undecodable_contest_tests {
    use super::*;
    use crate::ballot_codec::BigUIntCodec;
    use crate::fixtures::ballot_codec::get_fixtures;

    /// For every contest layout of the codec fixtures, the placeholder for a
    /// plaintext that does not decode is an implicitly invalid vote without
    /// selections, and it still encodes like any other vote.
    #[test]
    fn undecodable_contest_is_implicitly_invalid_and_encodes() {
        for fixture in get_fixtures() {
            let contest = &fixture.contest;
            if contest
                .encode_plaintext_contest_bigint(&fixture.plaintext)
                .is_err()
            {
                continue;
            }

            let decoded = DecodedVoteContest::undecodable(contest);

            assert_eq!(decoded.contest_id, contest.id, "{}", fixture.title);
            assert!(decoded.is_invalid(), "{}", fixture.title);
            assert!(!decoded.is_explicit_invalid, "{}", fixture.title);
            assert_eq!(
                decoded.invalid_errors,
                vec![InvalidPlaintextError::ballot_too_large()],
                "{}",
                fixture.title
            );
            assert_eq!(
                decoded.choices.len(),
                contest.candidates.len(),
                "{}",
                fixture.title
            );
            assert!(
                decoded.choices.iter().all(|choice| !choice.is_selected()),
                "{}",
                fixture.title
            );
            assert!(
                contest.encode_plaintext_contest_bigint(&decoded).is_ok(),
                "{}",
                fixture.title
            );
        }
    }
}
