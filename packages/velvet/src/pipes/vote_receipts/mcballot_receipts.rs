// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::vote_receipt::{PipeConfigVoteReceipts, DEFAULT_MCBALLOT_TITLE};
use crate::pipes::decode_ballots::decode_mcballots::OUTPUT_DECODED_BALLOTS_FILE;
use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{InputElectionConfig, PipeInputs};
use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};
use crate::pipes::Pipe;
use hex::encode;
use sequent_core::ballot::{Candidate, CandidatesOrder, Contest, StringifiedPeriodDates};
use sequent_core::ballot_codec::multi_ballot::DecodedBallotChoices;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::services::{pdf, reports};
use sequent_core::types::templates::VoteReceiptPipeType;
use sequent_core::util::date_time::get_date_and_time;
use serde::Serialize;
use serde_json::Map;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{default, fs};
use strand::hash::hash_sha256;
use tracing::{info, instrument};
use uuid::Uuid;

pub const OUTPUT_FILE_PDF: &str = "mcballots_receipts.pdf";
pub const OUTPUT_FILE_HTML: &str = "mcballots_receipts.html";

pub const BALLOT_IMAGES_OUTPUT_FILE_PDF: &str = "mcballots_images.pdf";
pub const BALLOT_IMAGES_OUTPUT_FILE_HTML: &str = "mcballots_images.html";

pub struct MCBallotReceipts {
    pub pipe_inputs: PipeInputs,
}

pub struct VoteReceiptsPipeData {
    pub output_file_pdf: String,
    pub output_file_html: String,
    pub pipe_name: String,
    pub pipe_name_output_dir: String,
}

// QR code = containing header of the report and voted candidates per position
// (if no votes, the content of QR code should be header of the report and "ABSTENTION")
pub fn qr_encode_choices(contests: &Vec<ContestData>, title: &str) -> String {
    let is_blank: bool = contests.iter().all(|contest| contest.is_blank());
    let mut data = vec![title.to_string()];
    if is_blank {
        data.push("ABSTENTION".to_string());
    } else {
        for contest in contests {
            data.push(contest.contest.name.clone().unwrap_or_default());
            for candidate in &contest.decoded_choices {
                if !candidate.is_selected() {
                    continue;
                }
                let candidate_name = candidate
                    .candidate
                    .clone()
                    .map(|cand| cand.name)
                    .flatten()
                    .unwrap_or_default();
                data.push(candidate_name);
            }
        }
    }
    data.join(":")
}

fn sort_candidates(candidates: &mut Vec<DecodedChoice>, order_field: CandidatesOrder) {
    match order_field {
        CandidatesOrder::Alphabetical => candidates.sort_by(|a, b| {
            let name_a = match &a.candidate {
                Some(candidate) => candidate
                    .alias
                    .as_ref()
                    .or(candidate.name.as_ref())
                    .unwrap_or(&String::new())
                    .to_lowercase(),
                None => String::new(),
            };

            let name_b = match &b.candidate {
                Some(candidate) => candidate
                    .alias
                    .as_ref()
                    .or(candidate.name.as_ref())
                    .unwrap_or(&String::new())
                    .to_lowercase(),
                None => String::new(),
            };

            name_a.cmp(&name_b)
        }),
        CandidatesOrder::Custom => {
            candidates.sort_by(|a, b| {
                let sort_order_a = match &a.candidate {
                    Some(candidate) => candidate
                        .presentation
                        .as_ref()
                        .and_then(|p| p.sort_order)
                        .unwrap_or(-1),
                    None => -1, // Default value when `a.candidate` is `None`
                };

                let sort_order_b = match &b.candidate {
                    Some(candidate) => candidate
                        .presentation
                        .as_ref()
                        .and_then(|p| p.sort_order)
                        .unwrap_or(-1),
                    None => -1, // Default value when `b.candidate` is `None`
                };

                sort_order_a.cmp(&sort_order_b)
            })
        }

        CandidatesOrder::Random => {
            // We don't randomize in results
        }
    }
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
        area_name: &str,
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
                let mut choices = DecodedChoice::from_dvcs(&contest_choices, &contest);

                let candidates_order = contest
                    .presentation
                    .clone()
                    .unwrap_or_default()
                    .candidates_order
                    .unwrap_or_default();

                sort_candidates(&mut choices, candidates_order.clone());

                let num_selected = choices.iter().filter(|can| can.is_selected()).count();

                let undervotes = contest.max_votes - (num_selected as i64);
                let mut overvotes = 0;
                if (num_selected as i64) > contest.max_votes {
                    overvotes = (num_selected as i64) - contest.max_votes;
                }

                // contest.
                let cd: ContestData = ContestData {
                    contest: contest.clone(),
                    decoded_choices: choices,
                    undervotes,
                    overvotes,
                };

                cds.push(cd);
            }

            cds.sort_by(|a, b| b.contest.name.cmp(&a.contest.name));

            let title = pipe_config.extra_data["title"]
                .as_str()
                .map(|val| val.to_string())
                .unwrap_or(DEFAULT_MCBALLOT_TITLE.to_string());
            let encoded_vote = qr_encode_choices(&cds, &title);
            let is_blank = cds.iter().all(|choice| choice.is_blank());

            let bd = BallotData {
                id: ballot.mcballot.serial_number.unwrap_or_default(),
                encoded_vote: encoded_vote,
                is_invalid: ballot.mcballot.is_explicit_invalid,
                is_blank: is_blank,
                contest_choices: cds,
            };

            ballot_data.push(bd);
        }

        let execution_annotations = pipe_config.execution_annotations.clone();

        let td = TemplateData {
            election_name: election_input.name.clone(),
            ballot_data,
            area: area_name.to_string(),
            election_annotations: election_input.annotations.clone(),
            election_dates: election_input.dates.clone(),
            execution_annotations: execution_annotations.unwrap_or_default(),
        };

        let mut map = Map::new();
        map.insert("data".to_string(), serde_json::to_value(&td)?);
        map.insert(
            "extra_data".to_string(),
            serde_json::to_value(&pipe_config.extra_data)?,
        );

        let rendered_user_template = reports::render_template_text(&pipe_config.template, map)
            .map_err(|e| {
                Error::UnexpectedError(format!(
                    "Error during render_template_text from report.hbs template file: {}",
                    e
                ))
            })?;

        let mut system_map = Map::new();
        system_map.insert(
            "rendered_user_template".to_string(),
            serde_json::to_value(&rendered_user_template)?,
        );

        if let serde_json::Value::Object(obj) = &pipe_config.extra_data {
            for (key, value) in obj {
                system_map.insert(key.clone(), value.clone());
            }
        }

        let bytes_html = reports::render_template_text(&pipe_config.system_template, system_map)
            .map_err(|e| {
                Error::UnexpectedError(format!(
                    "Error during render_template_text from report.hbs template file: {}",
                    e
                ))
            })?;

        let pdf_options = match pipe_config.pdf_options.clone() {
            Some(options) => Some(options.to_print_to_pdf_options()),
            None => None,
        };

        let bytes_pdf = if pipe_config.enable_pdfs {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let bytes_pdf = rt
                .block_on(async {
                    pdf::PdfRenderer::render_pdf(bytes_html.clone(), pdf_options)
                        .await
                        .map_err(|e| {
                            Error::UnexpectedError(format!("Error during PDF rendering: {}", e))
                        })
                })
                .unwrap();

            Some(bytes_pdf)
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
            .unwrap_or(PipeConfigVoteReceipts::mcballot(None));
        Ok(pipe_config)
    }
}

#[instrument(skip_all)]
fn get_pipe_data(pipe_type: VoteReceiptPipeType) -> VoteReceiptsPipeData {
    match pipe_type {
        VoteReceiptPipeType::VOTE_RECEIPT => VoteReceiptsPipeData {
            output_file_pdf: OUTPUT_FILE_PDF.to_string(),
            output_file_html: OUTPUT_FILE_HTML.to_string(),
            pipe_name_output_dir: PipeNameOutputDir::MCBallotReceipts.as_ref().to_string(),
            pipe_name: PipeName::VoteReceipts.as_ref().to_string(),
        },
        VoteReceiptPipeType::BALLOT_IMAGES => VoteReceiptsPipeData {
            output_file_pdf: BALLOT_IMAGES_OUTPUT_FILE_PDF.to_string(),
            output_file_html: BALLOT_IMAGES_OUTPUT_FILE_HTML.to_string(),
            pipe_name_output_dir: PipeNameOutputDir::MCBallotImages.as_ref().to_string(),
            pipe_name: PipeName::MCBallotImages.as_ref().to_string(),
        },
    }
}

fn generate_hashed_filename(
    path: &PathBuf,
    hash_bytes: &[u8],
    default_extension: &str,
) -> Result<PathBuf> {
    let path = path.as_path();

    let hash_hex = hex::encode(hash_bytes);

    let file_stem = path
        .file_stem()
        .ok_or("Invalid file name: No stem found")
        .map_err(|e| Error::UnexpectedError(format!("Error get path file_stem: {}", e)))?
        .to_string_lossy();

    let extension = path
        .extension()
        .map(|ext| ext.to_string_lossy())
        .unwrap_or_else(|| default_extension.to_string().into());

    let new_filename = if extension.is_empty() {
        format!("{file_stem}_{hash_hex}")
    } else {
        format!("{file_stem}_{hash_hex}.{extension}")
    };

    Ok(path.with_file_name(new_filename))
}

impl Pipe for MCBallotReceipts {
    #[instrument(skip_all, name = "MultiBallotReceipts::exec")]
    fn exec(&self) -> Result<()> {
        let pipe_config: PipeConfigVoteReceipts = self.get_config()?;

        let pipe_data = get_pipe_data(pipe_config.pipe_type.clone());

        for election_input in &self.pipe_inputs.election_list {
            let area_contests_map = election_input.get_area_contest_map();

            for (area_id, area_contests) in area_contests_map {
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
                        &area_contests.contests,
                        &election_input,
                        &pipe_config,
                        &area_contests.area_name,
                    )?;

                    let path = PipeInputs::mcballots_path(
                        &self
                            .pipe_inputs
                            .cli
                            .output_dir
                            .join(&pipe_data.pipe_name_output_dir)
                            .as_path(),
                        &election_input.id,
                        &area_id,
                    );

                    fs::create_dir_all(&path)?;

                    if let Some(ref some_bytes_pdf) = bytes_pdf {
                        let file = match &pipe_config.pipe_type {
                            VoteReceiptPipeType::VOTE_RECEIPT => {
                                path.join(&pipe_data.output_file_pdf)
                            }
                            VoteReceiptPipeType::BALLOT_IMAGES => {
                                let pdf_hash =
                                    hash_sha256(some_bytes_pdf.as_slice()).map_err(|e| {
                                        Error::UnexpectedError(format!(
                                            "Error during hash pdf bytes: {}",
                                            e
                                        ))
                                    })?;

                                let file = path.join(&pipe_data.output_file_pdf);

                                generate_hashed_filename(&file, &pdf_hash, "pdf").map_err(|e| {
                                    Error::UnexpectedError(format!(
                                        "Error during hash pdf bytes: {}",
                                        e
                                    ))
                                })?
                            }
                        };

                        let mut file = OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(true)
                            .open(file)?;
                        file.write_all(&some_bytes_pdf)?;
                    }

                    let file = path.join(&pipe_data.output_file_html);
                    let mut file = OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open(file)?;
                    file.write_all(&bytes_html)?;
                } else {
                    println!(
                        "[{}] File not found: {} -- Not processed",
                        &pipe_data.pipe_name,
                        path_ballots.display()
                    )
                }
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Debug)]
pub struct TemplateData {
    pub ballot_data: Vec<BallotData>,
    pub election_name: String,
    pub area: String,
    pub election_dates: Option<StringifiedPeriodDates>,
    pub election_annotations: HashMap<String, String>,
    pub execution_annotations: HashMap<String, String>,
}

#[derive(Serialize, Debug)]
pub struct BallotData {
    pub id: String,
    pub encoded_vote: String,
    pub is_invalid: bool,
    pub is_blank: bool,
    pub contest_choices: Vec<ContestData>,
}

#[derive(Serialize, Debug)]
pub struct ContestData {
    pub contest: Contest,
    pub decoded_choices: Vec<DecodedChoice>,
    pub undervotes: i64,
    pub overvotes: i64,
}

impl ContestData {
    pub fn is_blank(&self) -> bool {
        self.decoded_choices
            .iter()
            .all(|choice| !choice.is_selected())
    }
}

#[derive(Serialize, Debug)]
struct DecodedChoice {
    pub choice: DecodedVoteChoice,
    pub candidate: Option<Candidate>,
}
impl DecodedChoice {
    pub fn is_selected(&self) -> bool {
        self.choice.is_selected()
    }
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
