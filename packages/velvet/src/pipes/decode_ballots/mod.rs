pub mod ballot_codec;
pub mod error;

use self::error::Error;
use crate::pipes::pipe_inputs::{PipeInputs, BALLOTS_FILE};
use crate::pipes::Pipe;
use num_bigint::{BigUint, ToBigUint};
use sequent_core::ballot::*;
use sequent_core::ballot_codec::*;
use sequent_core::plaintext::*;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::fs::{self, File};
use std::io::BufRead;
use std::str::FromStr;

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
    fn exec(&self) -> Result<(), Box<dyn StdError>> {
        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                let contest_config_file = fs::File::open(&contest_input.config)?;
                let contest: Contest = serde_json::from_reader(contest_config_file)?;

                let mut file = self.pipe_inputs.get_path_for_contest(
                    &self.pipe_inputs.cli.input_dir,
                    &election_input.id,
                    &contest_input.id,
                );
                file.push(BALLOTS_FILE);
                let file = fs::File::open(file)?;

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

                let mut file = self.pipe_inputs.get_path_for_contest(
                    &self.pipe_inputs.cli.output_dir,
                    &election_input.id,
                    &contest_input.id,
                );

                fs::create_dir_all(&file)?;
                file.push(OUTPUT_DECODED_BALLOTS_FILE);
                let file = File::create(file)?;

                serde_json::to_writer(file, &decoded_ballots)?;
            }
        }

        Ok(())
    }
}
