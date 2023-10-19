pub mod ballot_codec;
pub mod error;

use self::ballot_codec::BallotCodec;
use self::error::{Error, Result};
use super::{Pipe, PipeInputs::PipeInputs};
use crate::cli::CliRun;
use crate::pipes::PipeInputs::BALLOTS_FILE;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};

pub const OUTPUT_DECODED_BALLOTS_FILE: &str = "decoded_ballots.json";

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
        // get an election config
        let election_input = &self.pipe_input.election_list[0];
        let election_config_file = fs::File::open(&election_input.config)?;
        let _election_config: serde_json::Value = serde_json::from_reader(election_config_file)?;
        // dbg!(&_election_config);
        // let questions = _election_config
        //     .get("configuration")
        //     .and_then(serde_json::Value::as_object)
        //     .ok_or(Error::ConfigNotValid)?
        //     .get("questions")
        //     .and_then(serde_json::Value::as_array)
        //     .ok_or(Error::ConfigNotValid)?;

        // get contest config
        // TODO: map over election.contest_list
        let contest_input = &election_input.contest_list[0];
        let contest_config_file = fs::File::open(&contest_input.config)?;
        let contest: Contest = serde_json::from_reader(contest_config_file)?;

        // tally_type is plurality at large, same district magnitude
        let bases = vec![2; contest.choices.len() + 1];
        let ballot_codec = BallotCodec::new(bases);

        let mut file = self.pipe_input.get_path_for_contest(
            &self.pipe_input.cli.input_dir,
            &election_input.id,
            &contest_input.id,
        );
        file.push(BALLOTS_FILE);
        let file = fs::File::open(file)?;

        let reader = std::io::BufReader::new(file);

        let mut decoded_ballots: Vec<DecodedVote> = vec![];

        for line in reader.lines() {
            let line = line?;
            let decoded = ballot_codec
                .decode_ballot(line.parse::<u32>().map_err(|_| Error::WrongBallotsFormat)?);

            let choices = decoded
                .iter()
                .zip(contest.choices.iter())
                .map(|(decoded_choice, choice)| SelectedChoice {
                    id: choice.id.to_string(),
                    selected: *decoded_choice as i64,
                })
                .collect::<Vec<SelectedChoice>>();

            let decoded_vote = DecodedVote {
                contest_id: contest.id.to_string(),
                choices,
            };

            decoded_ballots.push(decoded_vote);
        }

        // TODO: output
        // write this json into a output files
        dbg!(&decoded_ballots);

        let mut file = self.pipe_input.get_path_for_contest(
            &self.pipe_input.cli.output_dir,
            &election_input.id,
            &contest_input.id,
        );

        fs::create_dir_all(&file)?;
        file.push(OUTPUT_DECODED_BALLOTS_FILE);
        let file = File::create(file)?;
        
        serde_json::to_writer(file, &decoded_ballots)?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Contest {
    pub id: String,
    pub title: String,
    pub max: i64,
    pub min: i64,
    pub num_winners: i64,
    pub tally_type: String,
    #[serde(rename = "answer_total_votes_percentage")]
    pub total_votes_percentages: String,
    #[serde(rename = "answers")]
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub id: String,
    pub category: String,
    #[serde(rename = "details")]
    pub detail: String,
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

#[derive(Debug, Serialize)]
pub struct DecodedVote {
    pub contest_id: String,
    pub choices: Vec<SelectedChoice>,
}

#[derive(Debug, Serialize)]
pub struct SelectedChoice {
    pub id: String,
    pub selected: i64,
}
