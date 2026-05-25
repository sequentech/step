// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::{AuditableBallot, Contest, ReplicationChoice};
use crate::ballot_codec::multi_ballot::{
    BallotChoices, DecodedBallotChoices, DecodedContestChoice,
    DecodedContestChoices,
};
use crate::ballot_codec::PlaintextCodec;
use crate::multi_ballot::AuditableMultiBallot;
use crate::multi_ballot::AuditableMultiBallotContests;
use crate::types::ceremonies::CountingAlgType;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use strand::context::Ctx;

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
/// Represents an invalid plaintext error types.
#[allow(missing_docs)]
pub enum InvalidPlaintextErrorType {
    Explicit,
    Implicit,
    EncodingError,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
/// Represents an error in the preferential order.
pub enum PreferencialOrderErrorType {
    /// Indicates that there are gaps in the preference order, e.g. 1,2,4 or 1,3,4.
    PreferenceOrderWithGaps,
    /// Indicates that there are duplicated positions in the preference order, e.g. 1,2,2 or 1,1,3.
    DuplicatedPosition,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
/// Represents an invalid plaintext error details.
#[allow(missing_docs)]
pub struct InvalidPlaintextError {
    pub error_type: InvalidPlaintextErrorType,
    pub candidate_id: Option<String>,
    pub message: Option<String>,
    pub message_map: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
/// Represents a decoded vote contest.
#[allow(missing_docs)]
pub struct DecodedVoteContest {
    pub contest_id: String,
    pub is_explicit_invalid: bool,
    pub invalid_errors: Vec<InvalidPlaintextError>,
    pub invalid_alerts: Vec<InvalidPlaintextError>,
    pub choices: Vec<DecodedVoteChoice>,
}

impl DecodedVoteContest {
    /// Check if the contest is invalid, which is true if it is explicitly
    ///  marked as invalid or if it has any invalid errors.
    #[must_use]
    pub const fn is_invalid(&self) -> bool {
        self.is_explicit_invalid || !self.invalid_errors.is_empty()
    }
    /// Check if the contest is blank, which is true
    /// if it is not invalid and all choices are unselected.
    #[must_use]
    pub fn is_blank(&self) -> bool {
        !self.is_invalid()
            && self
                .choices
                .clone()
                .iter()
                .all(|choice| choice.selected < 0)
    }

    /// Check the validity of the preference order.
    ///
    /// Note: `PreferenceOrderWithGaps` is returned as an error if there are gaps,
    /// but this is generally not considered invalid, so the caller can
    /// handle it depending on the policy or jurisdiction rules.
    /// Returns Ok if the order is valid after sorting it and if it is
    /// contiguous, e.g. 1,2,3,4 or 1,4,2,3.
    /// Returns Err with a Vec of all errors found (may contain multiple variants).
    ///
    /// # Errors
    /// Returns `Err(Vec<PreferencialOrderErrorType>)` if the order is invalid.
    ///
    /// # Panics
    /// Panics if there are more than `i64::MAX` selected choices, which
    /// would cause an overflow when converting from `usize` to `i64`.
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
            .copied()
            .collect::<Vec<i64>>();
        ordered_choices.sort_unstable();
        let expected_order: Vec<i64> = (0..ordered_choices.len())
            .map(|i| i64::try_from(i).expect("failed to convert usize to i64"))
            .collect();

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

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
/// Represents a decoded vote choice.
pub struct DecodedVoteChoice {
    /// The candidate ID for this choice.
    pub id: String,
    /// The selection value for this choice, where -1 indicates not selected
    /// and any non-negative value indicates selected and depends on the counting algorithm.
    pub selected: i64,
    /// The write-in text for this choice, if applicable.
    pub write_in_text: Option<String>,
}

impl DecodedVoteChoice {
    /// Check if the choice is selected.
    #[must_use]
    pub const fn is_selected(&self) -> bool {
        self.selected >= 0
    }
}

/// Maps an auditable ballot to decoded contests.
///
/// # Errors
/// Returns `Err(String)` if the number of contests does not match or if deserialization fails.
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
        format!("Error deserializing auditable ballot contest {err:?}")
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

/// Maps decoded ballot choices to decoded contests.
///
/// # Errors
/// Returns `Err(String)` if a contest cannot be found in the ballot style.
pub fn map_decoded_ballot_choices_to_decoded_contests(
    decoded_ballot_choices: &DecodedBallotChoices,
    contests: &[Contest],
) -> Result<Vec<DecodedVoteContest>, String> {
    let mut decoded_contests = vec![];

    for found_contest in contests {
        let contest_id = &found_contest.id;
        let found_ballot_choices = decoded_ballot_choices
            .choices
            .iter()
            .find(|ballot_choice| &ballot_choice.contest_id == contest_id)
            .ok_or_else(|| {
                format!(
                    "Can't find contest with id {contest_id} on ballot style"
                )
            })?;

        let mut choices = vec![];

        for candidate in &found_contest.candidates {
            let selected = if found_ballot_choices
                .choices
                .iter()
                .any(|choice| choice.0 == candidate.id)
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
            contest_id: contest_id.clone(),
            is_explicit_invalid: decoded_ballot_choices.is_explicit_invalid,
            invalid_errors: found_ballot_choices.invalid_errors.clone(),
            invalid_alerts: found_ballot_choices.invalid_alerts.clone(),
            choices,
        };

        decoded_contests.push(decoded_contest);
    }
    Ok(decoded_contests)
}

/// Maps an auditable multi-ballot to decoded contests.
///
/// # Errors
/// Returns `Err(String)` if the number of contests does not match or if deserialization fails.
pub fn map_to_decoded_multi_contest<C: Ctx<P = [u8; 30]>>(
    ballot: &AuditableMultiBallot,
) -> Result<Vec<DecodedVoteContest>, String> {
    let ballot_contests: AuditableMultiBallotContests<C> =
        ballot.deserialize_contests().map_err(|err| {
            format!(
                "Error deserializing auditable multi ballot contest {err:?}"
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
    .map_err(|err| format!("Error decoding multi ballot plaintext {err:?}"))?;

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
                        "Can't find contest with id {contest_id} on ballot style"
                    )
                })
        })
        .collect::<Result<Vec<_>, String>>()?;
    map_decoded_ballot_choices_to_decoded_contests(
        &decoded_ballot_choices,
        &mapped_contests,
    )
}
