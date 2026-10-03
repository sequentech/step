// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{
    batch_file_name, list_batch_files, InputElectionConfig, PipeInputs, BALLOTS_FILE,
};
use crate::pipes::Pipe;
use num_bigint::BigUint;
use sequent_core::ballot::{Contest, MultiContestEncodingMode};
use sequent_core::ballot_codec::multi_ballot::{
    BallotChoices, DecodedBallotChoices, MultiBallotCodecContext,
};
use sequent_core::plaintext::{
    map_decoded_ballot_choices_to_decoded_contests, DecodedVoteChoice, DecodedVoteContest,
};
use uuid::Uuid;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufRead;
use std::path::Path;

use std::str::FromStr;
use tracing::{instrument, warn};

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
        include_decline_to_vote: bool,
        include_blank_ballots: bool,
        mode: MultiContestEncodingMode,
        serial_number_counter: &mut u32,
    ) -> Result<Vec<DecodedBallotChoices>> {
        let file = fs::File::open(path).map_err(|e| Error::FileAccess(path.to_path_buf(), e))?;
        let reader = std::io::BufReader::new(file);
        let mut decoded_ballots: Vec<DecodedBallotChoices> = vec![];

        // The codec context only depends on the contest configurations, so
        // it is built once, on the first ballot, and reused for every other
        // ballot in the file.
        let mut codec_context: Option<MultiBallotCodecContext> = None;

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

            let context = match codec_context.as_mut() {
                Some(context) => context,
                None => {
                    let context = MultiBallotCodecContext::new(
                        contests,
                        include_decline_to_vote,
                        include_blank_ballots,
                        mode,
                    )
                    .map_err(|_| Error::UnexpectedError("Wrong ballot format".into()))?;
                    codec_context.get_or_insert(context)
                }
            };

            let decoded = BallotChoices::decode_from_bigint_with_context(
                context,
                &plaintext,
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
            // (contest_id, area_id, batch multiplier) -> dvc
            let mut output_map: HashMap<(String, Uuid, u64), Vec<DecodedVoteContest>> =
                HashMap::new();

            for (area_id, unsorted_contests) in area_contest_map {
                let mut contests = unsorted_contests.contests.clone();
                contests.sort_by_key(|c| c.id.clone());
                let area_ballots_dir = PipeInputs::mcballots_path(
                    self.pipe_inputs.root_path_ballots.as_path(),
                    &election_input.id,
                    &area_id,
                );

                let include_decline_to_vote = election_input
                    .presentation
                    .as_ref()
                    .and_then(|presentation| presentation.decline_to_vote_policy.clone())
                    == Some(sequent_core::ballot::DeclineToVotePolicy::ENABLED);

                let include_blank_ballots = election_input
                    .presentation
                    .as_ref()
                    .and_then(|presentation| presentation.blank_ballots_policy.clone())
                    == Some(sequent_core::ballot::BlankBallotsPolicy::ENABLED);

                // Resolved election-wide at publication time, so every ballot style for
                // this election carries the same value; look it up by area anyway in case
                // that ever stops being true.
                let area_ballot_style = election_input
                    .ballot_styles
                    .iter()
                    .find(|ballot_style| ballot_style.area_id == area_id.to_string())
                    .ok_or(Error::AreaConfigNotFound(area_id))?;

                // `None` means the style predates the encoding-mode field, so it can only
                // have been produced by the legacy layout. `create_ballot_style` always
                // writes `Some(..)`, so a new style never lands here.
                let multi_contest_encoding_mode = area_ballot_style
                    .multi_contest_encoding_mode
                    .unwrap_or_default();

                // One file, unless the area's ballots were split into weight
                // batches: then one per batch, each decoded on its own and
                // written under its batch's name for do_tally to count.
                let ballot_files = list_batch_files(&area_ballots_dir, BALLOTS_FILE)?;
                if ballot_files.is_empty() {
                    println!(
                        "[{}] File not found: {} -- Not processed",
                        PipeName::DecodeMCBallots.as_ref(),
                        area_ballots_dir.join(BALLOTS_FILE).display()
                    );
                    continue;
                }

                for (path_ballots, multiplier) in ballot_files {
                    let decoded_ballots = Self::decode_ballots(
                        path_ballots.as_path(),
                        &contests,
                        include_decline_to_vote,
                        include_blank_ballots,
                        multi_contest_encoding_mode,
                        &mut serial_number_counter,
                    )?;

                    // output multi contest ballots, will be read by mcballot_receipt pipe

                    let output_dir = PipeInputs::mcballots_path(
                        self.pipe_inputs
                            .cli
                            .output_dir
                            .join(PipeNameOutputDir::DecodeMCBallots.as_ref())
                            .as_path(),
                        &election_input.id,
                        &area_id,
                    );

                    fs::create_dir_all(&output_dir)?;
                    let output_path =
                        output_dir.join(batch_file_name(OUTPUT_DECODED_BALLOTS_FILE, multiplier));
                    let file = File::create(&output_path)
                        .map_err(|e| Error::FileAccess(output_path, e))?;

                    serde_json::to_writer(file, &decoded_ballots)?;

                    // accumulate per-contest ballots

                    for dbc in decoded_ballots {
                        let decoded_contests =
                            map_decoded_ballot_choices_to_decoded_contests(dbc.clone(), &contests)
                                .map_err(|err| Error::UnexpectedError(err))?;

                        for decoded_contest in decoded_contests {
                            output_map
                                .entry((
                                    decoded_contest.contest_id.clone(),
                                    area_id.clone(),
                                    multiplier,
                                ))
                                .or_default()
                                .push(decoded_contest);
                        }
                    }
                }
            }

            // output ballots in the normal format to allow non adapted pipes to execute transparently

            for ((contest_id, area_id, multiplier), dvcs) in output_map {
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
                output_path.push(batch_file_name(
                    OUTPUT_DECODED_CONTEST_BALLOTS_FILE,
                    multiplier,
                ));
                let file =
                    File::create(&output_path).map_err(|e| Error::FileAccess(output_path, e))?;

                serde_json::to_writer(file, &dvcs)?;
            }
        }

        Ok(())
    }
}
