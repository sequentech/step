// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::pipes::decode_ballots::OUTPUT_DECODED_BALLOTS_FILE;
use crate::pipes::do_tally::tally::Tally;
use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::PipeInputs;
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

pub const OUTPUT_FILE: &str = "vote_receipts.pdf";

pub struct VoteReceipts {
    pub pipe_inputs: PipeInputs,
}

impl VoteReceipts {
    #[instrument(skip_all, name = "VoteReceipts::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl VoteReceipts {
    fn print_vote_receipts(&self, path: &Path, contest: &Contest) -> Result<()> {
        let tally = Tally::new(contest, vec![path.to_path_buf()], 0)
            .map_err(|e| Error::UnexpectedError(e.to_string()))?;

        let pipe_config = self
            .pipe_inputs
            .stage
            .pipe_config(self.pipe_inputs.stage.current_pipe);

        dbg!(&pipe_config);

        Ok(())
    }
}

impl Pipe for VoteReceipts {
    #[instrument(skip_all, name = "VoteReceipts::exec")]
    fn exec(&self) -> Result<()> {
        let input_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DecodeBallots.as_ref());

        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                for area_input in &contest_input.area_list {
                    let decoded_ballots_file = PipeInputs::build_path(
                        &input_dir,
                        &contest_input.election_id,
                        Some(&contest_input.id),
                        Some(&area_input.id),
                    )
                    .join(OUTPUT_DECODED_BALLOTS_FILE);

                    let res = self.print_vote_receipts(
                        decoded_ballots_file.as_path(),
                        &contest_input.contest,
                    );

                    if let Err(Error::FileAccess(file, _)) = &res {
                        println!(
                            "[{}] File not found: {} -- Not processed",
                            PipeName::VoteReceipts.as_ref(),
                            file.display()
                        );
                    }

                    match res {
                        Ok(decoded_ballots) => {
                            let mut output_path = PipeInputs::build_path(
                                self.pipe_inputs
                                    .cli
                                    .output_dir
                                    .join(PipeNameOutputDir::VoteReceipts.as_ref())
                                    .as_path(),
                                &election_input.id,
                                Some(&contest_input.id),
                                Some(&area_input.id),
                            );

                            fs::create_dir_all(&output_path)?;
                            output_path.push(OUTPUT_FILE);
                            let file = File::create(&output_path)
                                .map_err(|e| Error::FileAccess(output_path, e))?;

                            serde_json::to_writer(file, &decoded_ballots)?;
                        }
                        Err(e) => {
                            if let Error::FileAccess(file, _) = &e {
                                println!(
                                    "[{}] File not found: {} -- Not processed",
                                    PipeName::VoteReceipts.as_ref(),
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
