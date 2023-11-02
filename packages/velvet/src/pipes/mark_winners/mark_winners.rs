// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs;

use super::error::{Error, Result};
use crate::pipes::{
    do_tally::{CandidateResult, ContestResult, OUTPUT_CONTEST_RESULT_FILE},
    pipe_inputs::PipeInputs,
    pipe_name::PipeNameOutputDir,
    Pipe,
};

pub const OUTPUT_WINNERS: &str = "winners.json";

pub struct MarkWinners {
    pub pipe_inputs: PipeInputs,
}

impl MarkWinners {
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }

    fn get_winner(&self, contest_result: &ContestResult) -> CandidateResult {
        let mut max_votes = 0;
        let mut winners = Vec::new();

        for candidate_result in &contest_result.candidate_result {
            if candidate_result.total_count > max_votes {
                max_votes = candidate_result.total_count;
                winners.clear();
                winners.push(candidate_result);
            } else if candidate_result.total_count == max_votes {
                winners.push(candidate_result);
            }
        }

        if winners.len() > 1 {
            // ties resolution
            winners.sort_by(|a, b| a.candidate.name.cmp(&b.candidate.name));
        }

        let winner = winners[0].clone();

        CandidateResult {
            candidate: winner.candidate,
            total_count: winner.total_count,
        }
    }
}

impl Pipe for MarkWinners {
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
                for region_input in &contest_input.region_list {
                    let contest_result_file = PipeInputs::build_path(
                        &input_dir,
                        &contest_input.election_id,
                        &contest_input.id,
                        Some(&region_input.id),
                    )
                    .join(OUTPUT_CONTEST_RESULT_FILE);

                    let f = fs::File::open(&contest_result_file)
                        .map_err(|e| Error::IO(contest_result_file.clone(), e))?;
                    let contest_result: ContestResult = serde_json::from_reader(f)?;

                    let winner = self.get_winner(&contest_result);

                    let mut file = PipeInputs::build_path(
                        &output_dir,
                        &contest_input.election_id,
                        &contest_input.id,
                        Some(&region_input.id),
                    );

                    fs::create_dir_all(&file)?;
                    file.push(OUTPUT_WINNERS);
                    let file = fs::File::create(file)?;

                    serde_json::to_writer(file, &winner)?;
                }

                let contest_result_file = PipeInputs::build_path(
                    &input_dir,
                    &contest_input.election_id,
                    &contest_input.id,
                    None,
                )
                .join(OUTPUT_CONTEST_RESULT_FILE);

                let f = fs::File::open(&contest_result_file)
                    .map_err(|e| Error::IO(contest_result_file.clone(), e))?;
                let contest_result: ContestResult = serde_json::from_reader(f)?;

                let winner = self.get_winner(&contest_result);

                let mut file = PipeInputs::build_path(
                    &output_dir,
                    &contest_input.election_id,
                    &contest_input.id,
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
