// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::vote_receipt::PipeConfigVoteReceipts;
use crate::pipes::decode_ballots::decode_mcballots::OUTPUT_DECODED_BALLOTS_FILE;
use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{InputElectionConfig, PipeInputs};
use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};
use crate::pipes::Pipe;
use sequent_core::ballot::{Candidate, Contest};
use sequent_core::ballot_codec::multi_ballot::DecodedBallotChoices;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::services::{pdf, reports};
use serde::Serialize;
use serde_json::Map;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use tracing::instrument;
use uuid::Uuid;

pub const OUTPUT_FILE_PDF: &str = "mcballots_receipts.pdf";
pub const OUTPUT_FILE_HTML: &str = "mcballots_receipts.html";

pub struct MCBallotReceipts {
    pub pipe_inputs: PipeInputs,
}

impl MCBallotReceipts {
    #[instrument(skip_all, name = "MCBallotReceipts::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }

    #[instrument(skip_all, err)]
    fn print_vote_receipts(
        &self,
        path: &Path,
        contests: &Vec<Contest>,
        election_input: &InputElectionConfig,
        pipe_config: &PipeConfigVoteReceipts,
    ) -> Result<(Option<Vec<u8>>, Vec<u8>)> {
        let f = fs::File::open(&path).map_err(|e| Error::FileAccess(path.to_path_buf(), e))?;
        let mcballots: Vec<DecodedBallotChoices> = crate::utils::parse_file(f)?;
        let contest_map: HashMap<String, Contest> = contests
            .iter()
            .map(|c| (c.id.to_string(), c.clone()))
            .collect();
        let ballots = convert_ballots(election_input, mcballots)?;

        let mut ballot_data = vec![];
        for ballot in ballots {
            let mut cds = vec![];
            for contest_choices in ballot.choices {
                let contest = contest_map.get(&contest_choices.contest_id).unwrap();
                let choices = DecodedChoice::from_dvcs(&contest_choices, &contest);

                let cd = ContestData {
                    contest: contest.clone(),
                    decoded_choices: choices,
                };

                cds.push(cd);
            }

            let bd = BallotData {
                id: Uuid::new_v4(),
                // FIXME
                encoded_vote: "".into(),
                // FIXME
                is_invalid: ballot.mcballot.is_explicit_invalid,
                // FIXME
                is_blank: false,
                contest_choices: cds,
            };

            ballot_data.push(bd);
        }

        let td = TemplateData {
            election_name: election_input.name.clone(),
            ballot_data,
        };

        let mut map = Map::new();
        map.insert("data".to_string(), serde_json::to_value(&td)?);
        map.insert(
            "extra_data".to_string(),
            serde_json::to_value(&pipe_config.extra_data)?,
        );

        let bytes_html =
            reports::render_template_text(&pipe_config.template, map).map_err(|e| {
                Error::UnexpectedError(format!(
                    "Error during render_template_text from report.hbs template file: {}",
                    e
                ))
            })?;

        let bytes_pdf = if pipe_config.enable_pdfs {
            Some(pdf::html_to_pdf(bytes_html.clone(), None).map_err(|e| {
                Error::UnexpectedError(format!("Error during html_to_pdf conversion: {}", e))
            })?)
        } else {
            None
        };

        Ok((bytes_pdf, bytes_html.into_bytes()))
    }

    #[instrument(skip_all)]
    pub fn get_config(&self) -> Result<PipeConfigVoteReceipts> {
        let pipe_config: PipeConfigVoteReceipts = self
            .pipe_inputs
            .stage
            .pipe_config(self.pipe_inputs.stage.current_pipe)
            .and_then(|pc| pc.config)
            .map(|value| serde_json::from_value(value))
            .transpose()?
            .unwrap_or_default();
        Ok(pipe_config)
    }
}

impl Pipe for MCBallotReceipts {
    #[instrument(skip_all, name = "MultiBallotReceipts::exec")]
    fn exec(&self) -> Result<()> {
        let pipe_config: PipeConfigVoteReceipts = self.get_config()?;

        for election_input in &self.pipe_inputs.election_list {
            let area_contests = election_input.get_area_contest_map();

            for (area_id, contests) in area_contests {
                let path_ballots = PipeInputs::mcballots_path(
                    &self
                        .pipe_inputs
                        .cli
                        .output_dir
                        .join(PipeNameOutputDir::DecodeMCBallots.as_ref())
                        .as_path(),
                    &election_input.id,
                    &area_id,
                )
                .join(OUTPUT_DECODED_BALLOTS_FILE);

                if path_ballots.exists() {
                    let (bytes_pdf, bytes_html) = self.print_vote_receipts(
                        path_ballots.as_path(),
                        &contests,
                        &election_input,
                        &pipe_config,
                    )?;

                    let path = PipeInputs::mcballots_path(
                        &self
                            .pipe_inputs
                            .cli
                            .output_dir
                            .join(PipeNameOutputDir::MCBallotReceipts.as_ref())
                            .as_path(),
                        &election_input.id,
                        &area_id,
                    );

                    fs::create_dir_all(&path)?;

                    if let Some(ref some_bytes_pdf) = bytes_pdf {
                        let file = path.join(OUTPUT_FILE_PDF);
                        let mut file = OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(true)
                            .open(file)?;
                        file.write_all(&some_bytes_pdf)?;
                    }

                    let file = path.join(OUTPUT_FILE_HTML);
                    let mut file = OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open(file)?;
                    file.write_all(&bytes_html)?;
                } else {
                    println!(
                        "[{}] File not found: {} -- Not processed",
                        PipeName::MCBallotReceipts.as_ref(),
                        path_ballots.display()
                    )
                }
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Debug)]
struct TemplateData {
    pub ballot_data: Vec<BallotData>,
    pub election_name: String,
}

#[derive(Serialize, Debug)]
struct BallotData {
    pub id: Uuid,
    pub encoded_vote: String,
    pub is_invalid: bool,
    pub is_blank: bool,
    pub contest_choices: Vec<ContestData>,
}

#[derive(Serialize, Debug)]
struct ContestData {
    pub contest: Contest,
    pub decoded_choices: Vec<DecodedChoice>,
}

#[derive(Serialize, Debug)]
struct DecodedChoice {
    pub choice: DecodedVoteChoice,
    pub candidate: Option<Candidate>,
}
impl DecodedChoice {
    fn from_dvcs(dvc: &DecodedVoteContest, contest: &Contest) -> Vec<Self> {
        dvc.choices
            .iter()
            .map(|choice| DecodedChoice {
                choice: choice.clone(),
                candidate: contest
                    .candidates
                    .iter()
                    .find(|c| c.id == choice.id)
                    .cloned(),
            })
            .collect::<Vec<DecodedChoice>>()
    }
}

#[derive(Serialize, Debug)]
struct Bridge {
    pub mcballot: DecodedBallotChoices,
    pub choices: Vec<DecodedVoteContest>,
}
impl Bridge {
    fn new(mcballot: DecodedBallotChoices, choices: Vec<DecodedVoteContest>) -> Self {
        Bridge { mcballot, choices }
    }
}

// We are reusing some functionality from the standard receipts pipe/template, so it helps
// to convert mcballots to dcv format
fn convert_ballots(
    election_input: &InputElectionConfig,
    mcballots: Vec<DecodedBallotChoices>,
) -> Result<Vec<Bridge>> {
    let mut ret = vec![];

    let contest_dvc_map = crate::utils::get_contest_dvc_map(election_input);

    for dbc in mcballots {
        let mut ballot_dvcs = vec![];
        for contest in &dbc.choices {
            let blank: Option<&HashMap<String, DecodedVoteChoice>> =
                contest_dvc_map.get(&contest.contest_id);
            if let Some(blank) = blank {
                let mut next = blank.clone();
                for choice in &contest.choices {
                    let blank = next.get(&choice.0);
                    if let Some(blank) = blank {
                        let mut marked = blank.clone();
                        marked.selected = 1;
                        next.insert(choice.0.clone(), marked);
                    } else {
                        return Err(Error::UnexpectedError(format!(
                            "could not find candidate for choice"
                        )));
                    }
                }
                let mut values: Vec<DecodedVoteChoice> = next.into_values().collect();
                values.sort_by(|a, b| a.id.cmp(&b.id));

                let marked_contest = DecodedVoteContest {
                    contest_id: contest.contest_id.clone(),
                    is_explicit_invalid: dbc.is_explicit_invalid,
                    // FIXME
                    invalid_alerts: vec![],
                    // FIXME
                    invalid_errors: vec![],
                    choices: values,
                };

                ballot_dvcs.push(marked_contest);
            } else {
                return Err(Error::UnexpectedError(format!(
                    "could not find choices for contest"
                )));
            }
        }

        ret.push(Bridge::new(dbc, ballot_dvcs));
    }

    Ok(ret)
}
