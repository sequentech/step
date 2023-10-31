// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::tally;
use super::{error::Result, invalid_vote::InvalidVote};
use crate::pipes::{
    decode_ballots::OUTPUT_DECODED_BALLOTS_FILE, pipe_inputs::PipeInputs,
    pipe_name::PipeNameOutputDir, Pipe,
};
use crate::utils::HasId;
use sequent_core::ballot::Candidate;
use sequent_core::ballot::Contest;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs};

pub const OUTPUT_CONTEST_RESULT_FILE: &str = "contest_result.json";

pub struct DoTally {
    pub pipe_inputs: PipeInputs,
}

impl DoTally {
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl Pipe for DoTally {
    fn exec(&self) -> Result<()> {
        let input_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DecodeBallots.as_ref());
        let output_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DoTally.as_ref());

        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                let mut contest_ballot_files = vec![];

                for region_input in &contest_input.region_list {
                    let decoded_ballots_file = PipeInputs::build_path(
                        &input_dir,
                        &contest_input.election_id,
                        &contest_input.id,
                        Some(&region_input.id),
                    )
                    .join(OUTPUT_DECODED_BALLOTS_FILE);

                    let ca = tally::create_tally(
                        &contest_input.contest,
                        vec![decoded_ballots_file.clone()],
                    )?;
                    let res = ca.tally()?;

                    let mut file = PipeInputs::build_path(
                        &output_dir,
                        &contest_input.election_id,
                        &contest_input.id,
                        Some(&region_input.id),
                    );

                    fs::create_dir_all(&file)?;
                    file.push(OUTPUT_CONTEST_RESULT_FILE);

                    let file = fs::File::create(file)?;

                    serde_json::to_writer(file, &res)?;

                    contest_ballot_files.push(decoded_ballots_file);
                }

                let ca = tally::create_tally(&contest_input.contest, contest_ballot_files)?;
                let res = ca.tally()?;

                let mut file = PipeInputs::build_path(
                    &output_dir,
                    &contest_input.election_id,
                    &contest_input.id,
                    None,
                );
                file.push(OUTPUT_CONTEST_RESULT_FILE);

                let file = fs::File::create(file)?;

                serde_json::to_writer(file, &res)?;
            }
        }

        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContestResult {
    // #[serde(serialize_with = "to_id")]
    pub contest: Contest,
    pub total_valid_votes: u64,
    pub total_invalid_votes: HashMap<InvalidVote, u64>,
    pub candidate_result: Vec<CandidateResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateResult {
    // #[serde(serialize_with = "to_id")]
    pub candidate: Candidate,
    pub total_count: u64,
}

impl HasId for Contest {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for Candidate {
    fn id(&self) -> &str {
        &self.id
    }
}
