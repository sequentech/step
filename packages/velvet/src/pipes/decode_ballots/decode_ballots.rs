// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{PipeInputs, BALLOTS_FILE};
use crate::pipes::Pipe;
use num_bigint::BigUint;
use sequent_core::ballot::Contest;
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::DecodedVoteContest;

use std::fs::{self, File};
use std::io::BufRead;
use std::path::Path;

use std::str::FromStr;

use crate::pipes::pipe_name::PipeNameOutputDir;

pub const OUTPUT_DECODED_BALLOTS_FILE: &str = "decoded_ballots.json";

pub struct DecodeBallots {
    pub pipe_inputs: PipeInputs,
}

impl DecodeBallots {
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl DecodeBallots {
    fn decode_ballots(path: &Path, contest: &Contest) -> Result<Vec<DecodedVoteContest>> {
        let file = fs::File::open(path).map_err(|e| Error::IO(path.to_path_buf(), e))?;
        let reader = std::io::BufReader::new(file);
        let mut decoded_ballots: Vec<DecodedVoteContest> = vec![];

        for line in reader.lines() {
            let line = line?;
            let plaintext = BigUint::from_str(&line)
                .map_err(|_| Error::FromPipe("Wrong ballot format".into()))?;
            let decoded_vote = contest
                .decode_plaintext_contest_bigint(&plaintext)
                .map_err(|_| Error::FromPipe("Wrong ballot format".into()))?;

            decoded_ballots.push(decoded_vote);
        }

        Ok(decoded_ballots)
    }
}

impl Pipe for DecodeBallots {
    fn exec(&self) -> Result<()> {
        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                for region_input in &contest_input.region_list {
                    let path_ballots = PipeInputs::build_path(
                        self.pipe_inputs.root_path_ballots.as_path(),
                        &election_input.id,
                        &contest_input.id,
                        Some(&region_input.id),
                    )
                    .join(BALLOTS_FILE);

                    let decoded_ballots = DecodeBallots::decode_ballots(
                        path_ballots.as_path(),
                        &contest_input.contest,
                    )?;

                    let mut output_path = PipeInputs::build_path(
                        self.pipe_inputs
                            .cli
                            .output_dir
                            .join(PipeNameOutputDir::DecodeBallots.as_ref())
                            .as_path(),
                        &election_input.id,
                        &contest_input.id,
                        Some(&region_input.id),
                    );

                    fs::create_dir_all(&output_path)?;
                    output_path.push(OUTPUT_DECODED_BALLOTS_FILE);
                    let file = File::create(output_path)?;

                    serde_json::to_writer(file, &decoded_ballots)?;
                }
            }
        }

        Ok(())
    }
}
