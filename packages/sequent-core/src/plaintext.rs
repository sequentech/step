// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot_codec::multi_ballot::{
    BallotChoices, DecodedBallotChoices, DecodedContestChoice,
    DecodedContestChoices,
};
use crate::ballot_codec::PlaintextCodec;
use crate::multi_ballot::AuditableMultiBallotContests;
use crate::types::ceremonies::CountingAlgType;
use crate::{ballot::*, multi_ballot::AuditableMultiBallot};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use strand::context::Ctx;

/// Category of plaintext validation failure.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub enum InvalidPlaintextErrorType {
    /// Voter explicitly marked the ballot invalid.
    Explicit,
    /// Encoding or selection rules were violated.
    Implicit,
    /// Ballot could not be encoded into the available plaintext space.
    EncodingError,
}

/// Preference-order validation failure for ranked-choice contests.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub enum PreferencialOrderErrorType {
    /// Ranked positions skip a number in the sequence (e.g. 1, 3 without 2).
    PreferenceOrderWithGaps,
    /// The same rank was assigned to more than one candidate.
    DuplicatedPosition,
}

/// Validation error attached to a decoded contest vote.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct InvalidPlaintextError {
    /// How the invalid state was detected.
    pub error_type: InvalidPlaintextErrorType,
    /// Candidate associated with the error, when applicable.
    pub candidate_id: Option<String>,
    /// Default-locale error message.
    pub message: Option<String>,
    /// Localized error messages keyed by locale code.
    pub message_map: HashMap<String, String>,
}

/// Decoded voter selections and validation state for one contest.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct DecodedVoteContest {
    /// Contest identifier from the ballot style.
    pub contest_id: String,
    /// Whether the voter explicitly chose invalid options.
    pub is_explicit_invalid: bool,
    /// Whether the Voter has declined to vote (can be true only for multi-contest ballots with decline to vote policy enabled).
    /// and will be the same for all contests in the ballot.
    pub is_decline_to_vote: bool,
    /// Hard validation errors that reject the ballot.
    pub invalid_errors: Vec<InvalidPlaintextError>,
    /// Soft validation alerts shown to the voter but not always blocking.
    pub invalid_alerts: Vec<InvalidPlaintextError>,
    /// Per-candidate selection state and write-in text.
    pub choices: Vec<DecodedVoteChoice>,
}

impl DecodedVoteContest {
    /// Returns whether the contest is explicitly or implicitly invalid.
    pub fn is_invalid(&self) -> bool {
        self.is_explicit_invalid || !self.invalid_errors.is_empty()
    }
    /// Returns whether no candidate was selected in this contest.
    pub fn is_blank(&self) -> bool {
        !self.is_invalid()
            && self
                .choices
                .clone()
                .iter()
                .all(|choice| choice.selected < 0)
    }
    /// Returns the value of `is_decline_to_vote`.
    #[must_use]
    pub fn is_decline_to_vote(&self) -> bool {
        self.is_decline_to_vote
    }

    /// Check the validity of the preference order.
    /// Note: `PreferenceOrderWithGaps` is returned as an error if there are gaps,
    /// but this is generally not considered invalid, so the caller can
    /// handle it depending on the policy or jurisdiction rules.
    /// Returns Ok if the order is valid after sorting it and if it is
    /// contiguous, e.g. 1,2,3,4 or 1,4,2,3.
    /// Returns Err with a Vec of all errors found (may contain multiple variants).
    ///
    /// # Errors
    ///
    /// Returns preference-order validation failures when ranks are duplicated or gapped.
    pub fn validate_preferencial_order(
        &self,
    ) -> Result<(), Vec<PreferencialOrderErrorType>> {
        let mut errors: Vec<PreferencialOrderErrorType> = Vec::new();

        // Discard the unselected choices and sort the selected ones by their preference order
        let choices: Vec<i64> = self
            .choices
            .iter()
            .filter(|choice| choice.selected >= 0)
            .map(|choice| choice.selected)
            .collect();

        // After removing the unselected choices we check that there are no duplicates in
        // the preference order
        let choices_unique_set = choices.iter().collect::<HashSet<_>>();
        if choices.len() != choices_unique_set.len() {
            errors.push(PreferencialOrderErrorType::DuplicatedPosition);
        }

        // Check that there are no gaps in the ordered choices
        let mut ordered_choices = choices_unique_set
            .into_iter()
            .cloned()
            .collect::<Vec<i64>>();
        ordered_choices.sort();
        let expected_order: Vec<i64> =
            (0..ordered_choices.len() as i64).collect();

        if ordered_choices != expected_order {
            errors.push(PreferencialOrderErrorType::PreferenceOrderWithGaps);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Selection state for one candidate within a decoded contest.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct DecodedVoteChoice {
    /// Candidate identifier from the ballot style.
    pub id: String,
    /// Selection weight or rank; negative means not selected.
    pub selected: i64,
    /// Write-in text when the candidate is a write-in option.
    pub write_in_text: Option<String>,
}

impl DecodedVoteChoice {
    /// Returns whether the candidate was selected.
    pub fn is_selected(&self) -> bool {
        self.selected >= 0
    }
}

/// Decodes each contest ciphertext in an auditable ballot into [`DecodedVoteContest`].
///
/// # Errors
///
/// Returns an error when contest counts mismatch or decoding fails.
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

/// Maps decoded multi-contest ballot choices onto ballot-style contests.
///
/// # Errors
///
/// Returns an error when a contest id is missing or mapping fails.
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
            is_explicit_invalid: found_ballot_choices.is_explicit_invalid,
            is_decline_to_vote: decoded_ballot_choices.is_explicit_invalid,
            invalid_errors: found_ballot_choices.invalid_errors.clone(),
            invalid_alerts: found_ballot_choices.invalid_alerts.clone(),
            choices,
        };

        decoded_contests.push(decoded_contest);
    }
    Ok(decoded_contests)
}

/// Decodes each contest in an auditable multi-contest ballot into [`DecodedVoteContest`].
///
/// # Errors
///
/// Returns an error when deserialization, decoding, or contest lookup fails.
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
