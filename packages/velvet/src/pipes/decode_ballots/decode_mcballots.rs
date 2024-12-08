// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{InputElectionConfig, PipeInputs, BALLOTS_FILE};
use crate::pipes::Pipe;
use num_bigint::BigUint;
use sequent_core::ballot::Contest;
use sequent_core::ballot_codec::multi_ballot::{BallotChoices, DecodedBallotChoices};
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use uuid::Uuid;

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::BufRead;
use std::path::Path;

use std::str::FromStr;
use tracing::instrument;

use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};

pub const OUTPUT_DECODED_BALLOTS_FILE: &str = "decoded_mcballots.json";
pub const OUTPUT_DECODED_CONTEST_BALLOTS_FILE: &str = "decoded_ballots.json";

pub struct DecodeMCBallots {
    pub pipe_inputs: PipeInputs,
}

impl DecodeMCBallots {
    #[instrument(skip_all, name = "DecodeMCBallots::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl DecodeMCBallots {
    #[instrument(skip(contests))]
    fn decode_ballots(path: &Path, contests: &Vec<Contest>) -> Result<Vec<DecodedBallotChoices>> {
        // println!("reading file at {:?}", path);
        
        let file = fs::File::open(path).map_err(|e| Error::FileAccess(path.to_path_buf(), e))?;
        let reader = std::io::BufReader::new(file);
        let mut decoded_ballots: Vec<DecodedBallotChoices> = vec![];
        

        for line in reader.lines() {
            let line = line?;

            let plaintext = BigUint::from_str(&line);

            if let Err(error) = &plaintext {
                if error.to_string() == "cannot parse integer from empty string" {
                    continue;
                }
            }

            let plaintext =
                plaintext.map_err(|_| Error::UnexpectedError("Wrong ballot format".into()))?;

            let decoded = BallotChoices::decode_from_bigint(&plaintext, contests)
                .map_err(|_| Error::UnexpectedError("Wrong ballot format".into()))?;

            decoded_ballots.push(decoded);
        }

        Ok(decoded_ballots)
    }

    fn get_contest_dvc_map(election_input: &InputElectionConfig) -> HashMap<String, HashMap<String, DecodedVoteChoice>> {
        let mut ret = HashMap::new();
        
        for contest in &election_input.contest_list {
            let mut map = HashMap::new();
            for candidate in &contest.contest.candidates {
                let choice = DecodedVoteChoice {
                    id: candidate.id.clone(),
                    selected: -1,
                    write_in_text: None
                };
                map.insert(candidate.id.clone(), choice);
            }
                
            ret.insert(contest.id.to_string(), map);
        }

        ret
    }
}

impl Pipe for DecodeMCBallots {
    #[instrument(skip_all, name = "DecodeMultiBallots::exec")]
    fn exec(&self) -> Result<()> {
        
        for election_input in &self.pipe_inputs.election_list {
            let area_contests = election_input.get_area_contest_map();
            let contest_dvc_map: HashMap<String, HashMap<String, DecodedVoteChoice>> = Self::get_contest_dvc_map(election_input);
            let mut output_map: HashMap<String, HashMap<Uuid, Vec<DecodedVoteContest>>> = HashMap::new();
            
            for (area_id, contests) in area_contests {
                let path_ballots = PipeInputs::mcballots_path(
                    self.pipe_inputs.root_path_ballots.as_path(),
                    &election_input.id,
                    &area_id,
                )
                .join(BALLOTS_FILE);
                
                let res = Self::decode_ballots(
                    path_ballots.as_path(),
                    &contests,
                );

                match res {
                    Ok(decoded_ballots) => {
                        
                        // multi contest ballots

                        let mut output_path = PipeInputs::mcballots_path(
                            self.pipe_inputs
                                .cli
                                .output_dir
                                .join(PipeNameOutputDir::DecodeMCBallots.as_ref())
                                .as_path(),
                            &election_input.id,
                            &area_id,
                        );

                        fs::create_dir_all(&output_path)?;
                        output_path.push(OUTPUT_DECODED_BALLOTS_FILE);
                        let file = File::create(&output_path)
                            .map_err(|e| Error::FileAccess(output_path, e))?;

                        serde_json::to_writer(file, &decoded_ballots)?;

                        // accumulate per-contest ballots

                        for dbc in decoded_ballots {
                            for contest in dbc.choices {
                                
                                let blank = contest_dvc_map.get(&contest.contest_id);
                                if let Some(blank) = blank {
                                    let mut next = blank.clone();
                                    for choice in contest.choices {
                                        let blank = next.get(&choice.0);
                                        if let Some(blank) = blank {
                                            let mut marked = blank.clone();
                                            marked.selected = 1;
                                            next.insert(choice.0, marked);
                                        }   
                                        else {
                                            
                                            return Err(Error::UnexpectedError(format!("could not find candidate for choice")));
                                        }
                                    }
                                    let values: Vec<DecodedVoteChoice> = next.into_values().collect();

                                    let marked = DecodedVoteContest {
                                        contest_id: contest.contest_id.clone(),
                                        is_explicit_invalid: dbc.is_explicit_invalid,
                                        // FIXME
                                        invalid_alerts: vec![],
                                        // FIXME
                                        invalid_errors: vec![],
                                        choices: values
                                    };

                                    if !output_map.contains_key(&contest.contest_id) {
                                        output_map.insert(contest.contest_id.clone(), HashMap::new());
                                    }
                                    let area_dvc_map = output_map.get_mut(&contest.contest_id).expect("impossible");

                                    if !area_dvc_map.contains_key(&area_id) {
                                        area_dvc_map.insert(area_id.clone(), vec![]);
                                    }
                                    let values = area_dvc_map.get_mut(&area_id).expect("impossible");
                                    values.push(marked);
                                }
                                else {
                                    return Err(Error::UnexpectedError(format!("could not find choices for contest")));
                                }
                            }
                        }

                    }
                    Err(e) => {
                        if let Error::FileAccess(file, _) = &e {
                            println!(
                                "[{}] File not found: {} -- Not processed",
                                PipeName::DecodeMCBallots.as_ref(),
                                file.display()
                            )
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            
            for (contest_id, area_dcv_map) in output_map {
                for (area_id, dvcs) in area_dcv_map {
                    let contest_uuid = Uuid::from_str(&contest_id)
                        .map_err(|e| Error::UnexpectedError(format!("Could not parse uuid for contest {}", contest_id)))?;
                    
                    let mut output_path = PipeInputs::build_path(
                        self.pipe_inputs
                            .cli
                            .output_dir
                            // explicitly overwriting normal decode path ballots
                            .join(PipeNameOutputDir::DecodeBallots.as_ref())
                            .as_path(),
                        &election_input.id,
                        Some(&contest_uuid),
                        Some(&area_id),
                    );

                    fs::create_dir_all(&output_path)?;
                    output_path.push(OUTPUT_DECODED_BALLOTS_FILE);
                    let file = File::create(&output_path)
                        .map_err(|e| Error::FileAccess(output_path, e))?;

                    serde_json::to_writer(file, &dvcs)?;
                }
            }
        }

        Ok(())
    }
}
