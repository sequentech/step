// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;
use std::{cmp::Ordering, fs};

use sequent_core::ballot::Candidate;
use sequent_core::util::path::list_subfolders;
use serde::Serialize;
use tracing::{event, instrument, Level};

use crate::pipes::do_tally::CandidateResult;
use crate::pipes::do_tally::{list_tally_sheet_subfolders, OUTPUT_BREAKDOWNS_FOLDER};
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

    #[instrument(skip_all)]
    pub fn get_winners(contest_result: &ContestResult) -> Vec<WinnerResult> {
        let mut winners = contest_result.candidate_result.clone();

        winners.retain(|w| !w.candidate.is_explicit_blank() && !w.candidate.is_explicit_invalid());

        winners.sort_by(|a, b| {
            match b.total_count.cmp(&a.total_count) {
                // ties resolution
                Ordering::Equal => a.candidate.name.cmp(&b.candidate.name),
                other => other,
            }
        });

        winners
            .into_iter()
            .take(contest_result.contest.winning_candidates_num as usize)
            .enumerate()
            .map(|(index, w)| WinnerResult {
                candidate: w.candidate.clone(),
                total_count: w.total_count,
                winning_position: index + 1,
            })
            .collect()
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

            let winners = MarkWinners::get_winners(&contest_result);

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

                        let winners = MarkWinners::get_winners(&contest_result);

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

                        let winners = MarkWinners::get_winners(&contest_result);

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

                    let winners = MarkWinners::get_winners(&contest_result);

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

                let winner = MarkWinners::get_winners(&contest_result);

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
