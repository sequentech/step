// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;
use std::{cmp::Ordering, fs};

use rand::seq::SliceRandom;
use sequent_core::ballot::{Candidate, TieBreakingPolicy};
use sequent_core::types::ceremonies::CountingAlgType;
use sequent_core::util::path::list_subfolders;
use serde::Serialize;
use tracing::{event, instrument, Level};

use crate::pipes::do_tally::counting_algorithm::instant_runoff::RunoffStatus;
use crate::pipes::do_tally::{
    list_tally_sheet_subfolders, CandidateResult, OUTPUT_BREAKDOWNS_FOLDER,
};
use crate::pipes::error::{Error, Result};
use crate::pipes::{
    do_tally::{
        ContestResult, OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER,
        OUTPUT_CONTEST_RESULT_FILE,
    },
    pipe_inputs::PipeInputs,
    pipe_name::PipeNameOutputDir,
    Pipe,
};
use crate::utils::parse_file;

pub const OUTPUT_WINNERS: &str = "winners.json";

pub struct MarkWinners {
    pub pipe_inputs: PipeInputs,
}

impl MarkWinners {
    #[instrument(skip_all, name = "MarkWinners::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }

    #[instrument(err, skip_all)]
    pub fn get_winners(contest_result: &ContestResult) -> Result<Vec<WinnerResult>> {
        if contest_result.contest.is_acclaimed() {
            // There are no vote totals to sort. Preserve the configured
            // candidate order so positions 1..N are deterministic and match
            // the ballot/results ordering rather than alphabetical tie logic.
            return Ok(contest_result
                .candidate_result
                .iter()
                .filter(|result| result.candidate.is_acclamation_eligible())
                .enumerate()
                .map(|(index, result)| WinnerResult {
                    candidate: result.candidate.clone(),
                    total_count: 0,
                    winning_position: index + 1,
                })
                .collect());
        }

        let winning_candidates_num = contest_result.contest.winning_candidates_num as usize;

        if let (CountingAlgType::InstantRunoff, Some(process_results)) = (
            contest_result.contest.get_counting_algorithm(),
            &contest_result.process_results,
        ) {
            let runoff: RunoffStatus = serde_json::from_value(process_results.clone())?;
            let Some(runoff_winner) = runoff.get_winner() else {
                return Ok(vec![]);
            };
            let winner = contest_result
                .candidate_result
                .iter()
                .find(|result| result.candidate.id == runoff_winner.id)
                .ok_or_else(|| {
                    Error::UnexpectedError(format!(
                        "Runoff winner {} is not a candidate of contest {}",
                        runoff_winner.id, contest_result.contest.id
                    ))
                })?;
            return Ok(std::iter::once(winner)
                .take(winning_candidates_num)
                .map(|w| WinnerResult {
                    candidate: w.candidate.clone(),
                    total_count: w.total_count,
                    winning_position: 1,
                })
                .collect());
        }

        let mut winners = contest_result.candidate_result.clone();

        winners.retain(|w| !w.candidate.is_explicit_blank() && !w.candidate.is_explicit_invalid());

        winners.sort_by(|a, b| {
            match b.total_count.cmp(&a.total_count) {
                // order of candidates elected with the same count
                Ordering::Equal => a.candidate.name.cmp(&b.candidate.name),
                other => other,
            }
        });

        Ok(Self::fill_seats(
            winners,
            winning_candidates_num,
            &contest_result.contest.get_tie_breaking_policy(),
        )
        .into_iter()
        .enumerate()
        .map(|(index, w)| WinnerResult {
            candidate: w.candidate,
            total_count: w.total_count,
            winning_position: index + 1,
        })
        .collect())
    }

    /// Takes the first `seats` candidates of a list ranked by count. When
    /// candidates with the same count straddle the last seat, the seats
    /// left for them are drawn by lot, or stay unassigned until an external
    /// procedure decides them.
    fn fill_seats(
        mut ranked: Vec<CandidateResult>,
        seats: usize,
        tie_breaking_policy: &TieBreakingPolicy,
    ) -> Vec<CandidateResult> {
        let tied_count = match (
            seats
                .checked_sub(1)
                .and_then(|last_seat| ranked.get(last_seat)),
            ranked.get(seats),
        ) {
            (Some(last_elected), Some(first_not_elected))
                if last_elected.total_count == first_not_elected.total_count =>
            {
                last_elected.total_count
            }
            _ => {
                ranked.truncate(seats);
                return ranked;
            }
        };

        let tie_start = ranked.partition_point(|result| result.total_count > tied_count);
        let tie_end = ranked.partition_point(|result| result.total_count >= tied_count);
        ranked.truncate(tie_end);
        let mut tied = ranked.split_off(tie_start);

        match tie_breaking_policy {
            TieBreakingPolicy::RANDOM => {
                tied.shuffle(&mut rand::rng());
                tied.truncate(seats.saturating_sub(tie_start));
                ranked.append(&mut tied);
            }
            TieBreakingPolicy::EXTERNAL_PROCEDURE => {}
        }
        ranked
    }

    #[instrument(err, skip_all)]
    pub fn create_breakdown_winners(
        base_input_path: &PathBuf,
        base_output_path: &PathBuf,
    ) -> Result<()> {
        let base_input_breakdown_path = base_input_path.join(OUTPUT_BREAKDOWNS_FOLDER);
        let base_output_breakdown_path = base_output_path.join(OUTPUT_BREAKDOWNS_FOLDER);
        let subfolders = list_subfolders(&base_input_breakdown_path);
        for subfolder in subfolders {
            let contest_results_file_path = subfolder.join(OUTPUT_CONTEST_RESULT_FILE);
            let contest_results_file = fs::File::open(&contest_results_file_path)
                .map_err(|e| Error::FileAccess(contest_results_file_path.clone(), e))?;
            let contest_result: ContestResult = parse_file(contest_results_file)?;

            let winners = MarkWinners::get_winners(&contest_result)?;

            let subfolder_name = subfolder.file_name().unwrap();
            let output_subfolder = base_output_breakdown_path.join(subfolder_name);
            fs::create_dir_all(&output_subfolder)?;
            let winners_file_path = output_subfolder.join(OUTPUT_WINNERS);
            let winners_file = fs::File::create(winners_file_path)?;
            serde_json::to_writer(winners_file, &winners)?;
        }
        Ok(())
    }
}

impl Pipe for MarkWinners {
    #[instrument(err, skip_all, name = "MarkWinners::new")]
    fn exec(&self) -> Result<()> {
        let input_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DoTally.as_ref());
        let output_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::MarkWinners.as_ref());

        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                for area_input in &contest_input.area_list {
                    let base_input_path = PipeInputs::build_path(
                        &input_dir,
                        &contest_input.election_id,
                        Some(&contest_input.id),
                        Some(&area_input.id),
                    );

                    let base_output_path = PipeInputs::build_path(
                        &output_dir,
                        &contest_input.election_id,
                        Some(&contest_input.id),
                        Some(&area_input.id),
                    );
                    // do aggregate winners
                    let base_input_aggregate_path =
                        base_input_path.join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);
                    if base_input_aggregate_path.exists() && base_input_aggregate_path.is_dir() {
                        let contest_result_file =
                            base_input_aggregate_path.join(OUTPUT_CONTEST_RESULT_FILE);

                        let contest_results_file = fs::File::open(&contest_result_file)
                            .map_err(|e| Error::FileAccess(contest_result_file.clone(), e))?;
                        let contest_result: ContestResult = parse_file(contest_results_file)?;

                        let winners = MarkWinners::get_winners(&contest_result)?;

                        let aggregate_output_path = base_output_path
                            .join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);

                        fs::create_dir_all(&aggregate_output_path)?;
                        let winners_file_path = aggregate_output_path.join(OUTPUT_WINNERS);
                        let winners_file = fs::File::create(winners_file_path)?;

                        serde_json::to_writer(winners_file, &winners)?;
                    }

                    // do tally sheet winners
                    let tally_sheet_folders = list_tally_sheet_subfolders(&base_input_path);
                    for tally_sheet_folder in tally_sheet_folders {
                        let contest_result_file =
                            tally_sheet_folder.join(OUTPUT_CONTEST_RESULT_FILE);

                        let contest_results_file = fs::File::open(&contest_result_file)
                            .map_err(|e| Error::FileAccess(contest_result_file.clone(), e))?;
                        let contest_result: ContestResult = parse_file(contest_results_file)?;

                        let winners = MarkWinners::get_winners(&contest_result)?;

                        let Some(tally_sheet_id) =
                            PipeInputs::get_tally_sheet_id_from_path(&tally_sheet_folder)
                        else {
                            return Err(Error::UnexpectedError(
                                "Can't read tally sheet id from path".into(),
                            ));
                        };
                        let tally_sheet_folder =
                            PipeInputs::build_tally_sheet_path(&base_output_path, &tally_sheet_id);
                        fs::create_dir_all(&tally_sheet_folder)?;

                        let winners_file_path = tally_sheet_folder.join(OUTPUT_WINNERS);
                        let winners_file = fs::File::create(winners_file_path)?;

                        serde_json::to_writer(winners_file, &winners)?;
                    }

                    // do area winners

                    let contest_result_file = base_input_path.join(OUTPUT_CONTEST_RESULT_FILE);

                    let contest_results_file = fs::File::open(&contest_result_file)
                        .map_err(|e| Error::FileAccess(contest_result_file.clone(), e))?;
                    let contest_result: ContestResult = parse_file(contest_results_file)?;

                    let winners = MarkWinners::get_winners(&contest_result)?;

                    fs::create_dir_all(&base_output_path)?;
                    let winners_file_path = base_output_path.join(OUTPUT_WINNERS);
                    let winners_file = fs::File::create(winners_file_path)?;

                    serde_json::to_writer(winners_file, &winners)?;
                }

                let contest_result_path = PipeInputs::build_path(
                    &input_dir,
                    &contest_input.election_id,
                    Some(&contest_input.id),
                    None,
                );
                let contest_result_file = contest_result_path.join(OUTPUT_CONTEST_RESULT_FILE);

                let f = fs::File::open(&contest_result_file)
                    .map_err(|e| Error::FileAccess(contest_result_file.clone(), e))?;
                let contest_result: ContestResult = parse_file(f)?;

                let winner = MarkWinners::get_winners(&contest_result)?;

                let winner_folder = PipeInputs::build_path(
                    &output_dir,
                    &contest_input.election_id,
                    Some(&contest_input.id),
                    None,
                );

                fs::create_dir_all(&winner_folder)?;
                let winner_file_path = winner_folder.join(OUTPUT_WINNERS);
                let winner_file = fs::File::create(winner_file_path)?;

                serde_json::to_writer(winner_file, &winner)?;

                // do breakdown winners
                MarkWinners::create_breakdown_winners(&contest_result_path, &winner_folder)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct WinnerResult {
    pub candidate: Candidate,
    pub total_count: u64,
    pub winning_position: usize,
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use sequent_core::ballot::{CandidatePresentation, Contest};

    use super::*;

    fn candidate_result(
        id: &str,
        name: &str,
        configure: impl FnOnce(&mut CandidatePresentation),
    ) -> CandidateResult {
        let mut presentation = CandidatePresentation::new();
        configure(&mut presentation);
        CandidateResult {
            candidate: Candidate {
                id: id.to_string(),
                name: Some(name.to_string()),
                presentation: Some(presentation),
                ..Candidate::default()
            },
            percentage_votes: 0.0,
            total_count: 0,
        }
    }

    #[test]
    fn acclaimed_winners_preserve_configuration_order_and_exclude_placeholders() {
        let contest_result = ContestResult {
            contest: Contest {
                is_acclaimed: Some(true),
                winning_candidates_num: 1,
                ..Contest::default()
            },
            candidate_result: vec![
                candidate_result("configured-first", "Zulu", |_| {}),
                candidate_result("blank", "Blank", |p| p.is_explicit_blank = Some(true)),
                candidate_result("disabled", "Disabled", |p| p.is_disabled = Some(true)),
                candidate_result("write-in", "Write in", |p| p.is_write_in = Some(true)),
                candidate_result("configured-second", "Alpha", |_| {}),
            ],
            ..ContestResult::default()
        };

        let winners = MarkWinners::get_winners(&contest_result).expect("winners");

        assert_eq!(
            winners
                .iter()
                .map(|winner| winner.candidate.id.as_str())
                .collect::<Vec<_>>(),
            vec!["configured-first", "configured-second"]
        );
        assert_eq!(
            winners
                .iter()
                .map(|winner| winner.winning_position)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert!(winners.iter().all(|winner| winner.total_count == 0));
    }

    fn plurality_result(
        tie_breaking_policy: TieBreakingPolicy,
        winning_candidates_num: i64,
        counts: &[(&str, &str, u64)],
    ) -> ContestResult {
        ContestResult {
            contest: Contest {
                winning_candidates_num,
                tie_breaking_policy: Some(tie_breaking_policy),
                ..Contest::default()
            },
            candidate_result: counts
                .iter()
                .map(|(id, name, total_count)| CandidateResult {
                    total_count: *total_count,
                    ..candidate_result(id, name, |_| {})
                })
                .collect(),
            ..ContestResult::default()
        }
    }

    fn winner_ids(contest_result: &ContestResult) -> Vec<String> {
        MarkWinners::get_winners(contest_result)
            .expect("winners")
            .into_iter()
            .map(|winner| winner.candidate.id)
            .collect()
    }

    #[test]
    fn plurality_last_seat_tie_is_drawn_by_lot() {
        let contest_result = plurality_result(
            TieBreakingPolicy::RANDOM,
            2,
            &[("a", "A", 10), ("b", "B", 7), ("c", "C", 7)],
        );

        let mut last_seat_winners = HashSet::new();
        for _ in 0..64 {
            let winners = winner_ids(&contest_result);
            assert_eq!(winners.len(), 2);
            assert_eq!(winners[0], "a");
            last_seat_winners.insert(winners[1].clone());
        }

        assert_eq!(
            last_seat_winners,
            HashSet::from(["b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn plurality_last_seat_tie_stays_open_under_external_procedure() {
        let contest_result = plurality_result(
            TieBreakingPolicy::EXTERNAL_PROCEDURE,
            2,
            &[("a", "A", 10), ("b", "B", 7), ("c", "C", 7)],
        );

        assert_eq!(winner_ids(&contest_result), vec!["a"]);
    }

    #[test]
    fn plurality_tie_within_the_seats_elects_every_tied_candidate() {
        let contest_result = plurality_result(
            TieBreakingPolicy::EXTERNAL_PROCEDURE,
            2,
            &[("a", "A", 7), ("b", "B", 7), ("c", "C", 3)],
        );

        let winners = MarkWinners::get_winners(&contest_result).expect("winners");

        assert_eq!(
            winners
                .iter()
                .map(|winner| winner.candidate.id.as_str())
                .collect::<HashSet<_>>(),
            HashSet::from(["a", "b"])
        );
        assert_eq!(
            winners
                .iter()
                .map(|winner| winner.winning_position)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }
}
