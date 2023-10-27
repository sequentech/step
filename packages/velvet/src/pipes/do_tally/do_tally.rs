// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{error::Result, tally};
use crate::pipes::{
    decode_ballots::OUTPUT_DECODED_BALLOTS_FILE, pipe_inputs::PipeInputs, pipe_name::PipeName,
    pipe_name::PipeNameOutputDir, Pipe,
};
use sequent_core::{ballot::Contest, plaintext::DecodedVoteContest};
use serde::Serialize;
use std::{collections::HashMap, error::Error as StdError, fs, path::Path};

pub const OUTPUT_CONTEST_RESULT_FILE: &str = "contest_result.json";

pub struct DoTally {
    pub pipe_inputs: PipeInputs,
}

impl DoTally {
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl Pipe for DoTally {
    fn exec(&self) -> Result<()> {
        let input_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DecodeBallots.as_ref());
        let output_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DoTally.as_ref());

        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                let mut contest_ballot_files = vec![];
                for region_input in &contest_input.region_list {
                    let decoded_ballots_file = self
                        .pipe_inputs
                        .get_path_for_data(
                            &input_dir,
                            &contest_input.election_id,
                            &contest_input.id,
                            Some(&region_input.id),
                        )
                        .join(OUTPUT_DECODED_BALLOTS_FILE);

                    let ca = tally::create_tally(
                        contest_input.config.as_path(),
                        vec![decoded_ballots_file.clone()],
                    )?;

                    let res = ca.tally()?;

                    let mut file = self.pipe_inputs.get_path_for_data(
                        &output_dir,
                        &contest_input.election_id,
                        &contest_input.id,
                        Some(&region_input.id),
                    );

                    fs::create_dir_all(&file)?;
                    file.push(OUTPUT_CONTEST_RESULT_FILE);
                    let file = fs::File::create(file)?;

                    serde_json::to_writer(file, &res)?;

                    contest_ballot_files.push(decoded_ballots_file);
                }

                let ca = tally::create_tally(contest_input.config.as_path(), contest_ballot_files)?;

                let res = ca.tally()?;

                let mut file = self.pipe_inputs.get_path_for_data(
                    &output_dir,
                    &contest_input.election_id,
                    &contest_input.id,
                    None,
                );

                file.push(OUTPUT_CONTEST_RESULT_FILE);
                let file = fs::File::create(file)?;

                serde_json::to_writer(file, &res)?;
            }
        }

        Ok(())
    }
}
