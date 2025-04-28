// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{InputElectionConfig, PipeInputs, BALLOTS_FILE};
use crate::pipes::Pipe;
use num_bigint::BigUint;
use sequent_core::ballot::Contest;
use sequent_core::ballot_codec::multi_ballot::{BallotChoices, DecodedBallotChoices};
use sequent_core::plaintext::{
    map_decoded_ballot_choices_to_decoded_contests, DecodedVoteChoice, DecodedVoteContest,
};
use uuid::Uuid;

use std::collections::HashMap;
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
    #[instrument(err, skip(contests))]
    fn decode_ballots(
        path: &Path,
        contests: &Vec<Contest>,
        serial_number_counter: &mut u32,
    ) -> Result<Vec<DecodedBallotChoices>> {
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

            let decoded = BallotChoices::decode_from_bigint(
                &plaintext,
                contests,
                Some(serial_number_counter),
            )
            .map_err(|_| Error::UnexpectedError("Wrong ballot format".into()))?;

            decoded_ballots.push(decoded);
        }

        Ok(decoded_ballots)
    }

    // contest_id -> (area_id -> dvc)
    #[instrument(skip_all)]
    fn get_contest_dvc_map(
        election_input: &InputElectionConfig,
    ) -> HashMap<String, HashMap<String, DecodedVoteChoice>> {
        let mut ret = HashMap::new();

        for contest in &election_input.contest_list {
            let mut map = HashMap::new();
            for candidate in &contest.contest.candidates {
                let choice = DecodedVoteChoice {
                    id: candidate.id.clone(),
                    selected: -1,
                    write_in_text: None,
                };
                map.insert(candidate.id.clone(), choice);
            }

            ret.insert(contest.id.to_string(), map);
        }

        ret
    }
}

impl Pipe for DecodeMCBallots {
    // FIXME This method is horrid
    #[instrument(err, skip_all, name = "DecodeMultiBallots::exec")]
    fn exec(&self) -> Result<()> {
        let mut serial_number_counter = 1;
        for election_input in &self.pipe_inputs.election_list {
            let area_contest_map = election_input.get_area_contest_map();
            // contest_id -> (area_id -> dvc)
            let contest_dvc_map: HashMap<String, HashMap<String, DecodedVoteChoice>> =
                Self::get_contest_dvc_map(election_input);
            // contest_id -> (area_id -> dvc)
            let mut output_map: HashMap<String, HashMap<Uuid, Vec<DecodedVoteContest>>> =
                HashMap::new();

            for (area_id, unsorted_contests) in area_contest_map {
                let mut contests = unsorted_contests.contests.clone();
                contests.sort_by_key(|c| c.id.clone());
                let path_ballots = PipeInputs::mcballots_path(
                    self.pipe_inputs.root_path_ballots.as_path(),
                    &election_input.id,
                    &area_id,
                )
                .join(BALLOTS_FILE);

                let res = Self::decode_ballots(
                    path_ballots.as_path(),
                    &contests,
                    &mut serial_number_counter,
                );

                match res {
                    Ok(decoded_ballots) => {
                        // output multi contest ballots, will be read by mcballot_receipt pipe

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
                            let decoded_contests = map_decoded_ballot_choices_to_decoded_contests(
                                dbc.clone(),
                                &contests,
                            )
                            .map_err(|err| Error::UnexpectedError(err))?;

                            for decoded_contest in decoded_contests {
                                if !output_map.contains_key(&decoded_contest.contest_id) {
                                    output_map
                                        .insert(decoded_contest.contest_id.clone(), HashMap::new());
                                }
                                let area_dvc_map = output_map
                                    .get_mut(&decoded_contest.contest_id)
                                    .expect("impossible");

                                if !area_dvc_map.contains_key(&area_id) {
                                    area_dvc_map.insert(area_id.clone(), vec![]);
                                }
                                let values = area_dvc_map.get_mut(&area_id).expect("impossible");
                                values.push(decoded_contest);
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

            // output ballots in the normal format to allow non adapted pipes to execute transparently

            for (contest_id, area_dcv_map) in output_map {
                for (area_id, dvcs) in area_dcv_map {
                    let contest_uuid = Uuid::from_str(&contest_id).map_err(|e| {
                        Error::UnexpectedError(format!(
                            "Could not parse uuid for contest {}, {}",
                            contest_id, e
                        ))
                    })?;

                    let mut output_path = PipeInputs::build_path(
                        self.pipe_inputs
                            .cli
                            .output_dir
                            // Important: we are outputing decoded votes to the folder where
                            // further pipes are expecting them, but this folder is normally written to
                            // by the decode_ballots pipe (as opposed to this pipe, decode_mcballots)
                            .join(PipeNameOutputDir::DecodeBallots.as_ref())
                            .as_path(),
                        &election_input.id,
                        Some(&contest_uuid),
                        Some(&area_id),
                    );

                    fs::create_dir_all(&output_path)?;
                    output_path.push(OUTPUT_DECODED_CONTEST_BALLOTS_FILE);
                    let file = File::create(&output_path)
                        .map_err(|e| Error::FileAccess(output_path, e))?;

                    serde_json::to_writer(file, &dvcs)?;
                }
            }
        }

        Ok(())
    }
}
