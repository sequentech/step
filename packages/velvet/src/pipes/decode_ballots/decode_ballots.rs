// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{PipeInputs, BALLOTS_FILE};
use crate::pipes::Pipe;
use num_bigint::BigUint;
use sequent_core::ballot::Contest;
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::{
    DecodedVoteChoice, DecodedVoteContest, InvalidPlaintextError, InvalidPlaintextErrorType,
};

use std::fs::{self, File};
use std::io::BufRead;
use std::path::Path;

use std::str::FromStr;
use tracing::{instrument, warn};

use rayon::prelude::*;

use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};

pub const OUTPUT_DECODED_BALLOTS_FILE: &str = "decoded_ballots.json";

/// The line of a ballots file that stands for a decrypted payload the ballot
/// codec cannot read. Such a ballot is still counted, as an invalid ballot, so
/// that every decrypted ballot is either valid or invalid.
pub const UNDECODABLE_BALLOT_LINE: &str = "undecodable";

/// The implicitly invalid, unmarked ballot of `contest` that an undecodable
/// decrypted payload is counted as.
pub fn undecodable_ballot(contest: &Contest) -> DecodedVoteContest {
    DecodedVoteContest {
        contest_id: contest.id.clone(),
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: vec![InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::EncodingError,
            candidate_id: None,
            message: Some("errors.encoding.undecodable".to_string()),
            message_map: Default::default(),
        }],
        invalid_alerts: vec![],
        choices: contest
            .candidates
            .iter()
            .map(|candidate| DecodedVoteChoice {
                id: candidate.id.clone(),
                selected: -1,
                write_in_text: None,
            })
            .collect(),
    }
}

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
    pub fn decode_ballots(path: &Path, contest: &Contest) -> Result<Vec<DecodedVoteContest>> {
        let file = fs::File::open(path).map_err(|e| Error::FileAccess(path.to_path_buf(), e))?;
        let reader = std::io::BufReader::new(file);
        let mut decoded_ballots: Vec<DecodedVoteContest> = vec![];

        for line in reader.lines() {
            let line = line?;

            if line == UNDECODABLE_BALLOT_LINE {
                decoded_ballots.push(undecodable_ballot(contest));
                continue;
            }

            let plaintext = BigUint::from_str(&line);

            if let Err(error) = &plaintext {
                if error.to_string() == "cannot parse integer from empty string" {
                    continue;
                }
            }

            let plaintext =
                plaintext.map_err(|_| Error::UnexpectedError("Wrong ballot format".into()))?;

            // A plaintext the codec cannot decode is an invalid ballot, not
            // a reason to leave it, or the whole count, out.
            let decoded_vote = match contest.decode_plaintext_contest_bigint(&plaintext) {
                Ok(decoded_vote) => decoded_vote,
                Err(error) => {
                    warn!("undecodable plaintext counted as an invalid ballot: {error}");
                    undecodable_ballot(contest)
                }
            };

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
                let path_ballots = PipeInputs::build_path(
                    self.pipe_inputs.root_path_ballots.as_path(),
                    &election_input.id,
                    Some(&contest_input.id),
                    Some(&area_input.id),
                )
                .join(BALLOTS_FILE);

                let res =
                    DecodeBallots::decode_ballots(path_ballots.as_path(), &contest_input.contest);

                match res {
                    Ok(decoded_ballots) => {
                        let mut output_path = PipeInputs::build_path(
                            self.pipe_inputs
                                .cli
                                .output_dir
                                .join(PipeNameOutputDir::DecodeBallots.as_ref())
                                .as_path(),
                            &election_input.id,
                            Some(&contest_input.id),
                            Some(&area_input.id),
                        );

                        fs::create_dir_all(&output_path)?;
                        output_path.push(OUTPUT_DECODED_BALLOTS_FILE);
                        let file = File::create(&output_path)
                            .map_err(|e| Error::FileAccess(output_path.clone(), e))?;

                        serde_json::to_writer(file, &decoded_ballots)?;
                        Ok(())
                    }
                    Err(e) => {
                        if let Error::FileAccess(file, _) = &e {
                            warn!(
                                "[{}] File not found: {} -- Not processed",
                                PipeName::DecodeBallots.as_ref(),
                                file.display()
                            );
                            Ok(())
                        } else {
                            Err(e)
                        }
                    }
                }
            },
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::ballot::Candidate;
    use std::io::Write;

    #[test]
    fn undecodable_lines_are_decoded_as_invalid_ballots() {
        let contest = Contest {
            id: "contest".to_string(),
            candidates: vec![Candidate {
                id: "candidate".to_string(),
                ..Candidate::default()
            }],
            ..Contest::default()
        };
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        write!(file, "{UNDECODABLE_BALLOT_LINE}\n\n{UNDECODABLE_BALLOT_LINE}").expect("write");

        let decoded =
            DecodeBallots::decode_ballots(file.path(), &contest).expect("decoded ballots");

        assert_eq!(decoded.len(), 2);
        assert!(decoded.iter().all(|ballot| ballot.is_invalid()
            && !ballot.is_explicit_invalid
            && ballot.contest_id == "contest"
            && ballot.choices.iter().all(|choice| choice.selected < 0)));
    }
}
