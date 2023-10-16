pub mod ballot_codec;

use std::fs;

use crate::{cli::CliRun, pipes::error::Error};

use self::ballot_codec::BallotCodec;
use super::{error::Result, Pipe};

pub struct DecodeBallots {
    pub cli: CliRun,
}

impl Pipe for DecodeBallots {
    fn new(cli: &CliRun) -> Self {
        Self { cli: cli.clone() }
    }

    fn cli(&self) -> &CliRun {
        &self.cli
    }

    fn exec(&self) -> Result<()> {
        let choices = vec![0, 0, 0, 1, 0, 0];

        let ballot_codec = BallotCodec::new(vec![2, 2, 2, 2, 2, 2]);
        let encoded_ballot = ballot_codec.encode_ballot(choices.clone());
        let _decoded_ballot = ballot_codec.decode_ballot(encoded_ballot);

        dbg!(&self.cli().config);
        dbg!(&self.cli().input_dir);
        dbg!(&self.cli().output_dir);

        // TODO:
        // dbg!(&self.output_log_file);

        self.read_input_dir_config()?;

        Ok(())
    }
}

impl DecodeBallots {
}
