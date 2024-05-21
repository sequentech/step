// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::{cmp::Ordering, fs};

use sequent_core::ballot::Candidate;
use serde::Serialize;
use tracing::{event, instrument, Level};

use crate::pipes::error::{Error, Result};
use crate::pipes::{
    do_tally::{ContestResult, OUTPUT_CONTEST_RESULT_FILE},
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
    fn get_winners(&self, contest_result: &ContestResult) -> Vec<WinnerResult> {
        let mut winners = contest_result.candidate_result.clone();

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
}

impl Pipe for MarkWinners {
    #[instrument(skip_all, name = "MarkWinners::new")]
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
                    let contest_result_file = PipeInputs::build_path(
                        &input_dir,
                        &contest_input.election_id,
                        Some(&contest_input.id),
                        Some(&area_input.id),
                    )
                    .join(OUTPUT_CONTEST_RESULT_FILE);

                    let f = fs::File::open(&contest_result_file)
                        .map_err(|e| Error::FileAccess(contest_result_file.clone(), e))?;
                    let contest_result: ContestResult = parse_file(f)?;

                    let winner = self.get_winners(&contest_result);

                    let mut file = PipeInputs::build_path(
                        &output_dir,
                        &contest_input.election_id,
                        Some(&contest_input.id),
                        Some(&area_input.id),
                    );

                    fs::create_dir_all(&file)?;
                    file.push(OUTPUT_WINNERS);
                    let file = fs::File::create(file)?;

                    serde_json::to_writer(file, &winner)?;
                }

                let contest_result_file = PipeInputs::build_path(
                    &input_dir,
                    &contest_input.election_id,
                    Some(&contest_input.id),
                    None,
                )
                .join(OUTPUT_CONTEST_RESULT_FILE);

                let f = fs::File::open(&contest_result_file)
                    .map_err(|e| Error::FileAccess(contest_result_file.clone(), e))?;
                let contest_result: ContestResult = parse_file(f)?;

                let winner = self.get_winners(&contest_result);

                let mut file = PipeInputs::build_path(
                    &output_dir,
                    &contest_input.election_id,
                    Some(&contest_input.id),
                    None,
                );

                fs::create_dir_all(&file)?;
                file.push(OUTPUT_WINNERS);
                let file = fs::File::create(file)?;

                serde_json::to_writer(file, &winner)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct WinnerResult {
    pub candidate: Candidate,
    pub total_count: u64,
    pub winning_position: usize,
}
