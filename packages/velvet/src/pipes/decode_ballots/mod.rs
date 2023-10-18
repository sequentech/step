pub mod ballot_codec;
pub mod error;

use self::ballot_codec::BallotCodec;
use self::error::{Error, Result};
use super::{Pipe, PipeInputs::PipeInputs};
use crate::cli::CliRun;
use serde::Deserialize;
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

        let questions = json_value
            .get("configuration")
            .and_then(serde_json::Value::as_object)
            .ok_or(Error::ConfigNotValid)?
            .get("questions")
            .and_then(serde_json::Value::as_array)
            .ok_or(Error::ConfigNotValid)?;

        // TODO: this contains multiple questions which will then be dispatch into single contest in multiple sub dirs
        let contest = questions.get(0).ok_or(Error::ConfigNotValid)?;

        let contest: Contest =
            serde_json::from_value(contest.clone()).map_err(|_| Error::ConfigNotValid)?;

        dbg!(contest);

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Contest {
    #[serde(rename = "answer_total_votes_percentage")]
    pub total_votes_percentages: String,
    #[serde(rename = "answers")]
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub category: String,
    #[serde(rename = "details")]
    pub detail: String,
    pub id: i64,
    #[serde(rename = "sort_order")]
    pub sort_order: i64,
    pub text: String,
    pub urls: Vec<Url>,
}

#[derive(Debug, Deserialize)]
pub struct Url {
    pub title: String,
    #[serde(rename = "url")]
    pub content: String,
}
