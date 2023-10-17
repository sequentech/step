pub mod ballot_codec;
pub mod error;

use self::ballot_codec::BallotCodec;
use self::error::{Error, Result};
use super::{Pipe, PipeInputs::PipeInputs};
use crate::cli::CliRun;
use serde_json::Value;
use std::fs;

pub struct DecodeBallots {
    pub pipe_input: PipeInputs,
}

impl Pipe for DecodeBallots {
    type Error = Error;

    fn new(cli: &CliRun) -> Result<Self, Error> {
        Ok(Self {
            pipe_input: PipeInputs::new(cli)?,
        })
    }

    fn exec(&self) -> Result<(), Error> {
        let choices = vec![0, 0, 0, 1, 0, 0];

        let ballot_codec = BallotCodec::new(vec![2, 2, 2, 2, 2, 2]);
        let encoded_ballot = ballot_codec.encode_ballot(choices.clone());
        let _decoded_ballot = ballot_codec.decode_ballot(encoded_ballot);

        let election = &self.pipe_input.election_list[0];
        let election_config_file = fs::File::open(&election.config).unwrap();
        let json_value: Value = serde_json::from_reader(election_config_file).unwrap();

        // dbg!(json_value);

        let name_value = json_value
            .get("configuration")
            .ok_or(Error::ConfigNotValid)?
            .get("questions")
            .ok_or(Error::ConfigNotValid)?;

        dbg!(name_value);

        Ok(())
    }
}
