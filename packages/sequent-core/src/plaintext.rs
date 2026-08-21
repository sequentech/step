// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot_codec::multi_ballot::{
    votable_contests, BallotChoices, DecodedBallotChoices,
    DecodedContestChoice, DecodedContestChoices,
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

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub enum InvalidPlaintextErrorType {
    Explicit,
    Implicit,
    EncodingError,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub enum PreferencialOrderErrorType {
    PreferenceOrderWithGaps,
    DuplicatedPosition,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct InvalidPlaintextError {
    pub error_type: InvalidPlaintextErrorType,
    pub candidate_id: Option<String>,
    pub message: Option<String>,
    pub message_map: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct DecodedVoteContest {
    pub contest_id: String,
    pub is_explicit_invalid: bool,
    /// Whether the Voter has declined to vote (can be true only for multi-contest ballots with decline to vote policy enabled).
    /// and will be the same for all contests in the ballot.
    pub is_decline_to_vote: bool,
    /// Whether every contest on the ballot was left blank (can be true only
    /// for multi-contest ballots with the blank ballots policy enabled),
    /// and will be the same for all contests in the ballot.
    #[serde(default)]
    pub is_blank_ballot: bool,
    pub invalid_errors: Vec<InvalidPlaintextError>,
    pub invalid_alerts: Vec<InvalidPlaintextError>,
    pub choices: Vec<DecodedVoteChoice>,
}

impl DecodedVoteContest {
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
    /// Get the value of is_decline_to_vote.
    #[must_use]
    pub fn is_decline_to_vote(&self) -> bool {
        self.is_decline_to_vote
    }

    /// Check the validity of the preference order.
    /// Note: PreferenceOrderWithGaps is returned as an error if there are gaps,
    /// but this is generally not considered invalid, so the caller can
    /// handle it depending on the policy or jurisdiction rules.
    /// Returns Ok if the order is valid after sorting it and if it is
    /// contiguous, e.g. 1,2,3,4 or 1,4,2,3.
    /// Returns Err with a Vec of all errors found (may contain multiple variants).
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

/// Checks that a ballot names exactly the contests its ballot style encodes.
///
/// Acclaimed contests carry no ciphertext, so a ballot names exactly the
/// votable ones. Comparing the ids rather than just how many there are is
/// what detects a ballot style whose acclaimed set moved after the ballot was
/// cast: swapping one contest for another keeps the count identical, and the
/// acclaimed flag is deliberately outside the ballot style hash, so no
/// signature or hash check would notice.
fn check_ballot_contests_match_style(
    ballot_contest_ids: &[&str],
    config: &BallotStyle,
) -> Result<(), String> {
    let votable_contest_ids: HashSet<&str> = config
        .votable_contests()
        .map(|contest| contest.id.as_str())
        .collect();
    let named_contest_ids: HashSet<&str> =
        ballot_contest_ids.iter().copied().collect();

    if named_contest_ids.len() != ballot_contest_ids.len() {
        return Err("Ballot names the same contest more than once".to_string());
    }
    if named_contest_ids != votable_contest_ids {
        return Err(format!(
            "Ballot was cast over contests {:?}, but this ballot style \
             encodes {:?}. The ballot style's acclaimed contests changed \
             after the ballot was cast",
            sorted_ids(&named_contest_ids),
            sorted_ids(&votable_contest_ids),
        ));
    }
    Ok(())
}

fn sorted_ids(ids: &HashSet<&str>) -> Vec<String> {
    let mut sorted: Vec<String> =
        ids.iter().map(|id| (*id).to_string()).collect();
    sorted.sort();
    sorted
}

pub fn map_to_decoded_contest<C: Ctx<P = [u8; 30]>>(
    ballot: &AuditableBallot,
) -> Result<Vec<DecodedVoteContest>, String> {
    let mut decoded_contests = vec![];

    let ballot_contests = ballot.deserialize_contests().map_err(|err| {
        format!("Error deserializing auditable ballot contest {:?}", err)
    })?;
    let ballot_contest_ids: Vec<&str> = ballot_contests
        .iter()
        .map(|contest| contest.contest_id.as_str())
        .collect();
    check_ballot_contests_match_style(&ballot_contest_ids, &ballot.config)?;

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

/// Expands a decoded multi-contest ballot back into one entry per contest.
///
/// Acclaimed contests are skipped: they are never encoded, so the decoded
/// ballot has nothing to say about them.
pub fn map_decoded_ballot_choices_to_decoded_contests(
    decoded_ballot_choices: DecodedBallotChoices,
    contests: &Vec<Contest>,
) -> Result<Vec<DecodedVoteContest>, String> {
    let mut decoded_contests = vec![];

    for found_contest in votable_contests(contests) {
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
            is_blank_ballot: decoded_ballot_choices.is_blank_ballot,
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

    let ballot_contest_ids: Vec<&str> = ballot_contests
        .contest_ids
        .iter()
        .map(|contest_id| contest_id.as_str())
        .collect();
    check_ballot_contests_match_style(&ballot_contest_ids, &ballot.config)?;

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
mod acclaimed_contest_set_tests {
    use super::*;
    use crate::ballot::Contest;

    fn contest(id: &str, is_acclaimed: bool) -> Contest {
        Contest {
            id: id.to_string(),
            is_acclaimed: Some(is_acclaimed),
            ..Contest::default()
        }
    }

    fn style(contests: Vec<Contest>) -> BallotStyle {
        BallotStyle {
            id: String::new(),
            tenant_id: String::new(),
            election_event_id: String::new(),
            election_id: String::new(),
            num_allowed_revotes: None,
            description: None,
            public_key: None,
            area_id: String::new(),
            area_presentation: None,
            contests,
            election_event_presentation: None,
            election_presentation: None,
            election_dates: None,
            election_event_annotations: None,
            election_annotations: None,
            area_annotations: None,
            multi_contest_encoding_mode: None,
        }
    }

    /// The acclaimed flag is outside the ballot style hash, so a style whose
    /// acclaimed set moved after a ballot was cast still passes every
    /// signature and hash check. Swapping one contest for another keeps the
    /// contest count identical while shifting every mixed-radix base, so only
    /// comparing the ids catches it.
    #[test]
    fn moved_acclaimed_set_is_rejected_even_though_the_count_matches() {
        let cast_over = ["a", "b"];
        let moved_style = style(vec![
            contest("a", true),
            contest("b", false),
            contest("c", false),
        ]);

        assert_eq!(moved_style.votable_contests().count(), cast_over.len());

        let error = check_ballot_contests_match_style(&cast_over, &moved_style)
            .expect_err("a moved acclaimed set must be rejected");
        assert!(
            error.contains("acclaimed contests changed"),
            "the error must name the cause, got: {error}"
        );
    }

    #[test]
    fn ballot_naming_the_style_s_votable_contests_is_accepted() {
        let unchanged_style = style(vec![
            contest("a", false),
            contest("b", false),
            contest("c", true),
        ]);

        assert!(check_ballot_contests_match_style(
            &["a", "b"],
            &unchanged_style
        )
        .is_ok());
    }

    /// Ballots cast before the flag existed deserialize with `is_acclaimed`
    /// absent, so every contest is votable and the check must pass.
    #[test]
    fn legacy_ballot_without_the_flag_is_accepted() {
        let legacy_style = style(vec![
            Contest {
                id: "a".to_string(),
                ..Contest::default()
            },
            Contest {
                id: "b".to_string(),
                ..Contest::default()
            },
        ]);

        assert!(
            check_ballot_contests_match_style(&["a", "b"], &legacy_style)
                .is_ok()
        );
    }

    #[test]
    fn ballot_naming_a_contest_twice_is_rejected() {
        let one_contest_style = style(vec![contest("a", false)]);

        let error =
            check_ballot_contests_match_style(&["a", "a"], &one_contest_style)
                .expect_err("a repeated contest must be rejected");
        assert!(error.contains("more than once"), "got: {error}");
    }
}
