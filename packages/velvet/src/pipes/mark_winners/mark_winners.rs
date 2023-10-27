// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs;

use sequent_core::ballot::Contest;

use super::error::{Error, Result};
use crate::pipes::{
    do_tally::{ContestResult, OUTPUT_CONTEST_RESULT_FILE},
    pipe_inputs::PipeInputs,
    pipe_name::PipeNameOutputDir,
    Pipe,
};

pub struct MarkWinners {
    pub pipe_inputs: PipeInputs,
}

impl MarkWinners {
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
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
                let contest_config_file = fs::File::open(&contest_input.config)
                    .map_err(|e| Error::IO(contest_input.config.clone(), e))?;
                let contest: Contest = serde_json::from_reader(contest_config_file)?;

                for region_input in &contest_input.region_list {
                    let contest_result_file = self
                        .pipe_inputs
                        .get_path_for_data(
                            &input_dir,
                            &contest_input.election_id,
                            &contest_input.id,
                            Some(&region_input.id),
                        )
                        .join(OUTPUT_CONTEST_RESULT_FILE);

                    // Logic here
                    let f = fs::File::open(&contest_result_file)
                        .map_err(|e| Error::IO(contest_result_file.clone(), e))?;
                    let contest_result: ContestResult = serde_json::from_reader(f)?;

                    dbg!(contest_result);

                    let mut file = self.pipe_inputs.get_path_for_data(
                        &output_dir,
                        &contest_input.election_id,
                        &contest_input.id,
                        Some(&region_input.id),
                    );

                    fs::create_dir_all(&file)?;
                    file.push(OUTPUT_CONTEST_RESULT_FILE);
                    let file = fs::File::create(file)?;

                    // serde_json::to_writer(file, &res)?;
                }
            }
        }

        Ok(())
    }
}
