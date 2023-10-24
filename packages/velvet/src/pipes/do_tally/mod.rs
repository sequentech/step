mod error;

use sequent_core::{ballot::Contest, plaintext::DecodedVoteContest};

use super::{pipe_inputs::PipeInputs, Pipe};
use crate::pipes::{decode_ballots::OUTPUT_DECODED_BALLOTS_FILE, pipe_name::PipeNameOutputDir};
use std::{collections::HashMap, error::Error as StdError, fs, path::Path};

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

        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                let decoded_ballots_file = self
                    .pipe_inputs
                    .get_path_for_contest(&input_dir, &contest_input.election_id, &contest_input.id)
                    .join(OUTPUT_DECODED_BALLOTS_FILE);

                let file = fs::File::open(decoded_ballots_file)?;
                let res: Vec<DecodedVoteContest> = serde_json::from_reader(file)?;
                let res = DoTally::count_choices(
                    &contest_input.id.to_string(),
                    contest_input.config.as_path(),
                    res,
                )?;
            }
        }

        Ok(())
    }
}

impl DoTally {
    fn count_choices(
        contest_id: &str,
        config: &Path,
        votes: Vec<DecodedVoteContest>,
    ) -> Result<ContestResult, Box<dyn StdError>> {
        let file = fs::File::open(config).unwrap();
        let contest: Contest = serde_json::from_reader(file)?;

        let mut vote_count: HashMap<String, u64> = HashMap::new();
        let mut count_valid: u64 = 0;
        let mut count_invalid: u64 = 0;

        for vote in votes {
            // TODO: how to handle invalid ballots?
            if vote.invalid_errors.len() > 0 || vote.is_explicit_invalid {
                count_invalid += 1;
            } else {
                for choice in vote.choices {
                    if choice.selected >= 0 {
                        *vote_count.entry(choice.id).or_insert(0) += 1;
                        count_valid += 1;
                    }
                }
            }
        }

        let result: Vec<ContestChoiceResult> = vote_count
            .into_iter()
            .map(|(choice_id, total_count)| ContestChoiceResult {
                choice_id,
                total_count,
            })
            .collect();

        let result = contest
            .candidates
            .iter()
            .map(|c| {
                result
                    .iter()
                    .find(|r| r.choice_id == c.id)
                    .cloned()
                    .unwrap_or(ContestChoiceResult {
                        choice_id: c.id.clone(),
                        total_count: 0,
                    })
            })
            .collect::<Vec<ContestChoiceResult>>();

        let contest_result = ContestResult {
            contest_id: contest_id.to_string(),
            total_valid_votes: count_valid,
            total_invalid_votes: 1,
            choice_result: result,
        };

        Ok(contest_result)
    }
}

#[derive(Debug, Clone)]
pub struct ContestResult {
    pub contest_id: String,
    pub total_valid_votes: u64,
    pub total_invalid_votes: u64,
    pub choice_result: Vec<ContestChoiceResult>,
}

#[derive(Debug, Clone)]
pub struct ContestChoiceResult {
    pub choice_id: String,
    pub total_count: u64,
}
