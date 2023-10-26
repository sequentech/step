pub mod error;

use self::error::{Error, Result};
use crate::pipes::pipe_inputs::{PipeInputs, BALLOTS_FILE};
use crate::pipes::Pipe;
use num_bigint::{BigUint, ToBigUint};
use sequent_core::ballot::Contest;
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::DecodedVoteContest;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufRead;
use std::path::PathBuf;
use std::str::FromStr;

use super::pipe_name::PipeNameOutputDir;

pub const OUTPUT_DECODED_BALLOTS_FILE: &str = "decoded_ballots.json";

pub struct DecodeBallots {
    pub pipe_inputs: PipeInputs,
}

impl DecodeBallots {
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl Pipe for DecodeBallots {
    fn exec(&self) -> Result<()> {
        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                let contest_config_file = fs::File::open(&contest_input.config)
                    .map_err(|e| Error::IO(contest_input.config.clone(), e))?;
                let contest: Contest = serde_json::from_reader(contest_config_file)?;

                for region_input in &contest_input.region_list {
                    let mut file = self.pipe_inputs.get_path_for_data(
                        &self.pipe_inputs.cli.input_dir,
                        &election_input.id,
                        &contest_input.id,
                        &region_input.id,
                    );
                    file.push(BALLOTS_FILE);
                    let file = fs::File::open(&file).map_err(|e| Error::IO(file.clone(), e))?;

                    let reader = std::io::BufReader::new(file);

                    let mut decoded_ballots: Vec<DecodedVoteContest> = vec![];

                    for line in reader.lines() {
                        let line = line?;
                        let plaintext =
                            BigUint::from_str(&line).map_err(|_| Error::WrongBallotsFormat)?;
                        let decoded_vote = contest
                            .decode_plaintext_contest_bigint(&plaintext)
                            .map_err(|_| Error::WrongBallotsFormat)?;

                        decoded_ballots.push(decoded_vote);
                    }

                    let mut file = self.pipe_inputs.get_path_for_data(
                        &self
                            .pipe_inputs
                            .cli
                            .output_dir
                            .join(PipeNameOutputDir::DecodeBallots.as_ref()),
                        &election_input.id,
                        &contest_input.id,
                        &region_input.id,
                    );

                    fs::create_dir_all(&file)?;
                    file.push(OUTPUT_DECODED_BALLOTS_FILE);
                    let file = File::create(file)?;

                    serde_json::to_writer(file, &decoded_ballots)?;
                }
            }
        }

        Ok(())
    }
}
