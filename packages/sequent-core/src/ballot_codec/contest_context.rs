// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::{Candidate, Contest};
use crate::ballot_codec::check_contest_configuration;
use crate::types::ceremonies::CountingAlgType;
use std::collections::HashMap;

/// Precomputed per-contest constants used by the ballot codecs.
///
/// Validating a contest configuration and locating its marker candidates
/// only depend on the immutable contest definition, so batch encode/decode
/// paths build this context once per contest instead of recomputing it for
/// every ballot.
pub struct ContestCodecContext<'a> {
    pub contest: &'a Contest,
    /// The explicit invalid marker candidate, if the contest defines one.
    pub explicit_invalid_candidate: Option<&'a Candidate>,
    /// The explicit blank marker candidate, if the contest defines one.
    pub explicit_blank_candidate: Option<&'a Candidate>,
    /// All candidates sorted by id.
    pub sorted_candidates: Vec<&'a Candidate>,
    /// Non-marker candidates sorted by id. Their positions define the
    /// candidate slots in both encodings.
    pub sorted_normal_candidates: Vec<&'a Candidate>,
    /// All candidates indexed by id.
    pub candidates_by_id: HashMap<&'a str, &'a Candidate>,
    /// Position of each non-marker candidate id in
    /// `sorted_normal_candidates`.
    pub normal_candidate_positions: HashMap<&'a str, usize>,
}

/// Validates the contest configuration.
///
/// Returns the first configuration error message when the contest defines
/// more than one explicit invalid or explicit blank marker candidate.
pub fn validate_contest_configuration(contest: &Contest) -> Result<(), String> {
    let configuration_errors = check_contest_configuration(contest);
    if let Some(error) = configuration_errors.invalid_errors.first() {
        return Err(error.message.clone().unwrap_or_else(|| {
            "contest has an invalid configuration".to_string()
        }));
    }
    Ok(())
}

impl<'a> ContestCodecContext<'a> {
    /// Validates the contest configuration and precomputes the
    /// contest-level constants used by the codecs.
    ///
    /// Returns the first configuration error message when the contest
    /// defines more than one explicit invalid or explicit blank marker
    /// candidate.
    pub fn new(contest: &'a Contest) -> Result<Self, String> {
        validate_contest_configuration(contest)?;

        Ok(Self::new_unchecked(contest))
    }

    /// Precomputes the contest-level constants used by the codecs without
    /// validating the contest configuration.
    ///
    /// Used by decode paths that validate the configuration at a specific
    /// point of the decoding process to preserve the order in which errors
    /// are reported.
    pub fn new_unchecked(contest: &'a Contest) -> Self {
        let explicit_invalid_candidate = contest
            .candidates
            .iter()
            .find(|candidate| candidate.is_explicit_invalid());
        let explicit_blank_candidate = contest
            .candidates
            .iter()
            .find(|candidate| candidate.is_explicit_blank());

        let mut sorted_candidates: Vec<&Candidate> =
            contest.candidates.iter().collect();
        sorted_candidates.sort_by(|a, b| a.id.cmp(&b.id));

        let sorted_normal_candidates: Vec<&Candidate> = sorted_candidates
            .iter()
            .copied()
            .filter(|candidate| {
                !candidate.is_explicit_invalid()
                    && !candidate.is_explicit_blank()
            })
            .collect();

        let candidates_by_id: HashMap<&str, &Candidate> = contest
            .candidates
            .iter()
            .map(|candidate| (candidate.id.as_str(), candidate))
            .collect();

        let normal_candidate_positions: HashMap<&str, usize> =
            sorted_normal_candidates
                .iter()
                .enumerate()
                .map(|(position, candidate)| (candidate.id.as_str(), position))
                .collect();

        ContestCodecContext {
            contest,
            explicit_invalid_candidate,
            explicit_blank_candidate,
            sorted_candidates,
            sorted_normal_candidates,
            candidates_by_id,
            normal_candidate_positions,
        }
    }

    /// Returns the bases of the single-contest (dense) encoding.
    pub fn single_contest_bases(&self) -> Result<Vec<u64>, String> {
        // Calculate the base for candidates. It depends on the
        // `contest.counting_algorithm`:
        // - plurality-at-large: base 2 (value can be either 0 o 1)
        // - preferential (*bordas*): contest.max + 1
        // - cummulative: contest.extra_options.cumulative_number_of_checkboxes
        //   + 1

        let contest = self.contest;
        let candidate_base: u64 = match contest.get_counting_algorithm() {
            CountingAlgType::PluralityAtLarge => 2,
            CountingAlgType::Cumulative => contest
                .cumulative_number_of_checkboxes()
                .checked_add(1)
                .ok_or_else(|| {
                    "cumulative candidate base exceeds u64".to_string()
                })?,
            _ => u64::try_from(contest.max_votes)
                .map_err(|_| {
                    "candidate base requires non-negative max_votes".to_string()
                })?
                .checked_add(1)
                .ok_or_else(|| "candidate base exceeds u64".to_string())?,
        };

        // Set the initial bases and raw ballot, populate bases using the valid
        // candidates list
        let mut bases: Vec<u64> = vec![2];
        if self.explicit_blank_candidate.is_some() {
            bases.push(2);
        }
        for _i in 0..self.sorted_normal_candidates.len() {
            bases.push(candidate_base);
        }

        // Add bases for null terminators.
        if contest.allow_writeins() {
            let char_map = contest.get_char_map();
            let write_in_base = char_map.base();
            for candidate in contest.candidates.iter() {
                if candidate.is_write_in() {
                    bases.push(write_in_base);
                }
            }
        }

        Ok(bases)
    }
}
