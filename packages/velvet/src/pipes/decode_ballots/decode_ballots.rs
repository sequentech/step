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
use tracing::instrument;

use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};

pub const OUTPUT_DECODED_BALLOTS_FILE: &str = "decoded_ballots.json";

pub struct DecodeBallots {
    pub pipe_inputs: PipeInputs,
}

impl DecodeBallots {
    #[instrument(skip_all, name = "DecodeBallots::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl DecodeBallots {
    #[instrument(err, skip(contest))]
    fn decode_ballots(path: &Path, contest: &Contest) -> Result<Vec<DecodedVoteContest>> {
        let file = fs::File::open(path).map_err(|e| Error::FileAccess(path.to_path_buf(), e))?;
        let reader = std::io::BufReader::new(file);
        let mut decoded_ballots: Vec<DecodedVoteContest> = vec![];

        for line in reader.lines() {
            let line = line?;

            let plaintext = BigUint::from_str(&line);

            if let Err(error) = &plaintext {
                if error.to_string() == "cannot parse integer from empty string" {
                    continue;
                }
            }

            let plaintext =
                plaintext.map_err(|_| Error::UnexpectedError("Wrong ballot format".into()))?;

            let decoded_vote = contest
                .decode_plaintext_contest_bigint(&plaintext)
                .map_err(|_| Error::UnexpectedError("Wrong ballot format".into()))?;

            decoded_ballots.push(decoded_vote);
        }

        Ok(decoded_ballots)
    }
}

impl Pipe for DecodeBallots {
    #[instrument(err, skip_all, name = "DecodeBallots::exec")]
    fn exec(&self) -> Result<()> {
        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                for area_input in &contest_input.area_list {
                    let path_ballots = PipeInputs::build_path(
                        self.pipe_inputs.root_path_ballots.as_path(),
                        &election_input.id,
                        Some(&contest_input.id),
                        Some(&area_input.id),
                    )
                    .join(BALLOTS_FILE);

                    let res = DecodeBallots::decode_ballots(
                        path_ballots.as_path(),
                        &contest_input.contest,
                    );

                    match res {
                        Ok(decoded_ballots) => {
                            let mut output_path = PipeInputs::build_path(
                                self.pipe_inputs
                                    .cli
                                    .output_dir
                                    .join(PipeNameOutputDir::DecodeBallots.as_ref())
                                    .as_path(),
                                &election_input.id,
                                Some(&contest_input.id),
                                Some(&area_input.id),
                            );

                            fs::create_dir_all(&output_path)?;
                            output_path.push(OUTPUT_DECODED_BALLOTS_FILE);
                            let file = File::create(&output_path)
                                .map_err(|e| Error::FileAccess(output_path, e))?;

                            serde_json::to_writer(file, &decoded_ballots)?;
                        }
                        Err(e) => {
                            if let Error::FileAccess(file, _) = &e {
                                println!(
                                    "[{}] File not found: {} -- Not processed",
                                    PipeName::DecodeBallots.as_ref(),
                                    file.display()
                                )
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
