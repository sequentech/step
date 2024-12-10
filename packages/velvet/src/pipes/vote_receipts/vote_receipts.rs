// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::vote_receipt::PipeConfigVoteReceipts;
use crate::pipes::decode_ballots::OUTPUT_DECODED_BALLOTS_FILE;
use crate::pipes::do_tally::tally::Tally;
use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{InputElectionConfig, PipeInputs};
use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};
use crate::pipes::Pipe;

use sequent_core::ballot::{Candidate, Contest,StringifiedPeriodDates};
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::services::{pdf, reports};
use sequent_core::util::date_time::get_date_and_time;
use serde::Serialize;
use serde_json::Map;
use std::fs::{self, OpenOptions};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use tracing::info;
use tracing::instrument;
use uuid::Uuid;

pub const OUTPUT_FILE_PDF: &str = "vote_receipts.pdf";
pub const OUTPUT_FILE_HTML: &str = "vote_receipts.html";

pub struct VoteReceipts {
    pub pipe_inputs: PipeInputs,
}

impl VoteReceipts {
    #[instrument(skip_all, name = "VoteReceipts::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }

    #[instrument(skip_all, err)]
    fn print_vote_receipts(
        &self,
        path: &Path,
        contest: &Contest,
        election_input: &InputElectionConfig,
        pipe_config: &PipeConfigVoteReceipts,
        area_name: &str,
    ) -> Result<(Option<Vec<u8>>, Vec<u8>)> {
        let tally = Tally::new(contest, vec![path.to_path_buf()], 0, 0, vec![])
            .map_err(|e| Error::UnexpectedError(e.to_string()))?;

        let data = TemplateData {
            contest: tally.contest.clone(),
            ballots: tally.ballots.clone(),
            election_name: election_input.name.clone(),
            election_annotations: election_input.annotations.clone(),
            election_dates: election_input.dates.clone(),
            area: area_name.clone().to_string(),
        };

        info!("election_input: {}", election_input.name);
        let data = compute_data(data);

        let mut map = Map::new();
        map.insert("data".to_string(), serde_json::to_value(&data)?);
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

impl Pipe for VoteReceipts {
    #[instrument(skip_all, name = "VoteReceipts::exec")]
    fn exec(&self) -> Result<()> {
        let input_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DecodeBallots.as_ref());

        let pipe_config: PipeConfigVoteReceipts = self.get_config()?;

        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                for area_input in &contest_input.area_list {
                    let decoded_ballots_file = PipeInputs::build_path(
                        &input_dir,
                        &contest_input.election_id,
                        Some(&contest_input.id),
                        Some(&area_input.id),
                    )
                    .join(OUTPUT_DECODED_BALLOTS_FILE);

                    if decoded_ballots_file.exists() {
                        let (bytes_pdf, bytes_html) = self.print_vote_receipts(
                            decoded_ballots_file.as_path(),
                            &contest_input.contest,
                            &election_input,
                            &pipe_config,
                            &area_input.area.name,
                        )?;

                        let path = PipeInputs::build_path(
                            &self
                                .pipe_inputs
                                .cli
                                .output_dir
                                .join(PipeNameOutputDir::VoteReceipts.as_ref())
                                .as_path(),
                            &election_input.id,
                            Some(&contest_input.id),
                            Some(&area_input.id),
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
                            PipeName::VoteReceipts.as_ref(),
                            decoded_ballots_file.display()
                        )
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Serialize)]
struct TemplateData {
    pub contest: Contest,
    pub ballots: Vec<DecodedVoteContest>,
    pub election_name: String,
    pub area: String,
    pub election_dates: Option<StringifiedPeriodDates>,
    pub election_annotations: HashMap<String, String>,
}

#[derive(Serialize, Debug)]
struct ComputedTemplateData {
    pub contest: Contest,
    pub receipts: Vec<ReceiptData>,
    pub election_name: String,
    pub area: String,
    pub election_dates: Option<StringifiedPeriodDates>,
    pub election_annotations: HashMap<String, String>,
    pub execution_annotations: HashMap<String, String>,
}

#[derive(Serialize, Debug)]
struct DecodedChoice {
    pub choice: DecodedVoteChoice,
    pub candidate: Option<Candidate>,
}

#[derive(Serialize, Debug)]
struct ReceiptData {
    pub id: Uuid,
    pub encoded_vote: String,
    pub is_invalid: bool,
    pub is_blank: bool,
    pub is_blank_or_invalid: bool,
    pub decoded_choices: Vec<DecodedChoice>,
    pub is_undervote: bool,
    pub is_overvote: bool,
}

fn compute_data(data: TemplateData) -> ComputedTemplateData {
    let receipts = data
        .ballots
        .iter()
        .map(|decoded_vote_contest| {
            let is_invalid = decoded_vote_contest.is_invalid();
            let selected_candidates = decoded_vote_contest
                .choices
                .iter()
                .filter(|choice| choice.selected >= 0)
                .filter_map(|choice| {
                    data.contest
                        .candidates
                        .iter()
                        .find(|c| c.id == choice.id)
                        .cloned()
                })
                .collect::<Vec<Candidate>>();
            let is_blank = selected_candidates.len() == 0;
            let is_undervote = (selected_candidates.len() as i64) < data.contest.max_votes;
            let is_overvote = (selected_candidates.len() as i64) > data.contest.max_votes;

            let encoded_vote_contest = data
                .contest
                .encode_plaintext_contest_bigint(decoded_vote_contest)
                .unwrap()
                .to_string();

            let decoded_choices = decoded_vote_contest
                .choices
                .iter()
                .map(|choice| DecodedChoice {
                    choice: choice.clone(),
                    candidate: data
                        .contest
                        .candidates
                        .iter()
                        .find(|c| c.id == choice.id)
                        .cloned(),
                })
                .collect::<Vec<DecodedChoice>>();

            ReceiptData {
                id: Uuid::new_v4(),
                encoded_vote: encoded_vote_contest,
                is_invalid,
                is_blank,
                is_blank_or_invalid: is_invalid || is_blank,
                decoded_choices: decoded_choices,
                is_undervote,
                is_overvote,
            }
        })
        .collect::<Vec<ReceiptData>>();

    ComputedTemplateData {
        contest: data.contest,
        receipts,
        election_name: data.election_name,
        area: data.area,
        election_annotations: data.election_annotations,
        election_dates: data.election_dates,
        execution_annotations: HashMap::from([("date_printed".to_string(), get_date_and_time())]),
    }
}
