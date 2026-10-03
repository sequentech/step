// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{batch_file_name, list_batch_files, PipeInputs, BALLOTS_FILE};
use crate::pipes::Pipe;
use num_bigint::BigUint;
use sequent_core::ballot::Contest;
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::DecodedVoteContest;

use std::fs::{self, File};
use std::io::BufRead;
use std::path::Path;

use std::str::FromStr;
use tracing::{instrument, warn};

use rayon::prelude::*;

use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};

pub const OUTPUT_DECODED_BALLOTS_FILE: &str = "decoded_ballots.json";

pub struct DecodeBallots {
    pub pipe_inputs: PipeInputs,
}

impl DecodeBallots {
    #[instrument(skip_all, name = "DecodeBallots::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl DecodeBallots {
    #[instrument(err, skip(contest))]
    fn decode_ballots(path: &Path, contest: &Contest) -> Result<Vec<DecodedVoteContest>> {
        let file = fs::File::open(path).map_err(|e| Error::FileAccess(path.to_path_buf(), e))?;
        let reader = std::io::BufReader::new(file);
        let mut decoded_ballots: Vec<DecodedVoteContest> = vec![];

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

            let decoded_vote = contest
                .decode_plaintext_contest_bigint(&plaintext)
                .map_err(|_| Error::UnexpectedError("Wrong ballot format".into()))?;

            decoded_ballots.push(decoded_vote);
        }

        Ok(decoded_ballots)
    }
}

impl Pipe for DecodeBallots {
    #[instrument(err, skip_all, name = "DecodeBallots::exec")]
    fn exec(&self) -> Result<()> {
        let tasks: Vec<_> = self
            .pipe_inputs
            .election_list
            .iter()
            .flat_map(|election_input| {
                election_input
                    .contest_list
                    .iter()
                    .flat_map(move |contest_input| {
                        contest_input
                            .area_list
                            .iter()
                            .map(move |area_input| (election_input, contest_input, area_input))
                    })
            })
            .collect();

        tasks.par_iter().try_for_each(
            |(election_input, contest_input, area_input)| -> Result<()> {
                let area_ballots_dir = PipeInputs::build_path(
                    self.pipe_inputs.root_path_ballots.as_path(),
                    &election_input.id,
                    Some(&contest_input.id),
                    Some(&area_input.id),
                );

                // One file, unless the area's ballots were split into weight
                // batches: then one per batch, each decoded on its own and
                // written under its batch's name for do_tally to count.
                let ballot_files = list_batch_files(&area_ballots_dir, BALLOTS_FILE)?;
                if ballot_files.is_empty() {
                    warn!(
                        "[{}] File not found: {} -- Not processed",
                        PipeName::DecodeBallots.as_ref(),
                        area_ballots_dir.join(BALLOTS_FILE).display()
                    );
                    return Ok(());
                }

                let output_dir = PipeInputs::build_path(
                    self.pipe_inputs
                        .cli
                        .output_dir
                        .join(PipeNameOutputDir::DecodeBallots.as_ref())
                        .as_path(),
                    &election_input.id,
                    Some(&contest_input.id),
                    Some(&area_input.id),
                );
                fs::create_dir_all(&output_dir)?;

                for (path_ballots, multiplier) in ballot_files {
                    let decoded_ballots = DecodeBallots::decode_ballots(
                        path_ballots.as_path(),
                        &contest_input.contest,
                    )?;

                    let output_path =
                        output_dir.join(batch_file_name(OUTPUT_DECODED_BALLOTS_FILE, multiplier));
                    let file = File::create(&output_path)
                        .map_err(|e| Error::FileAccess(output_path.clone(), e))?;

                    serde_json::to_writer(file, &decoded_ballots)?;
                }
                Ok(())
            },
        )?;

        Ok(())
    }
}
