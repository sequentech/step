mod error;

use super::{pipe_inputs::PipeInputs, Pipe};
use crate::pipes::{decode_ballots::OUTPUT_DECODED_BALLOTS_FILE, pipe_name::PipeNameOutputDir};
use std::{error::Error as StdError, fs};

pub struct DoTally {
    pub pipe_inputs: PipeInputs,
}

impl DoTally {
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl Pipe for DoTally {
    fn exec(&self) -> Result<(), Box<dyn StdError>> {
        let input_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DecodeBallots.as_ref());

        dbg!(&input_dir);

        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                let decoded_ballots_file = self
                    .pipe_inputs
                    .get_path_for_contest(&input_dir, &contest_input.election_id, &contest_input.id)
                    .join(OUTPUT_DECODED_BALLOTS_FILE);

                let file = fs::File::open(decoded_ballots_file)?;
                // serde_json::from_reader(file);
            }
        }
        Ok(())
    }
}
