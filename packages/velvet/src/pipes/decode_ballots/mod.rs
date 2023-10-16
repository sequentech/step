pub mod ballot_codec;

use crate::cli::CliRun;

use self::ballot_codec::BallotCodec;
use super::{
    error::Result,
    Pipe,
    PipeInputs::{PipeInputs, PipeInputsRead},
};

pub struct DecodeBallots {
    pub pipe_input: PipeInputs,
}

impl Pipe for DecodeBallots {
    fn new(cli: &CliRun) -> Self {
        Self {
            pipe_input: PipeInputs { cli: cli.clone() },
        }
    }

    fn exec(&self) -> Result<()> {
        let choices = vec![0, 0, 0, 1, 0, 0];

        let ballot_codec = BallotCodec::new(vec![2, 2, 2, 2, 2, 2]);
        let encoded_ballot = ballot_codec.encode_ballot(choices.clone());
        let _decoded_ballot = ballot_codec.decode_ballot(encoded_ballot);

        dbg!(&self.pipe_input.cli.config);
        dbg!(&self.pipe_input.cli.input_dir);
        dbg!(&self.pipe_input.cli.output_dir);

        // TODO:
        // dbg!(&self.output_log_file);

        self.read_input_dir_config()?;

        Ok(())
    }
}

impl PipeInputsRead for DecodeBallots {
    fn read_input_dir_config(&self) -> Result<()> {
        self.pipe_input.read_input_dir_config()
    }
}
