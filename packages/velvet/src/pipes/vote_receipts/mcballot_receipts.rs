// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::vote_receipt::{PipeConfigVoteReceipts, DEFAULT_MCBALLOT_TITLE};
use crate::pipes::decode_ballots::decode_mcballots::OUTPUT_DECODED_BALLOTS_FILE;
use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{InputElectionConfig, PipeInputs};
use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};
use crate::pipes::Pipe;
use anyhow::{anyhow, Context};
use csv::Writer;
use hex::encode;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use sequent_core::ballot::{Candidate, CandidatesOrder, Contest, StringifiedPeriodDates};
use sequent_core::ballot_codec::multi_ballot::DecodedBallotChoices;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::services::{pdf, reports};
use sequent_core::signatures::ecies_encrypt::ecies_sign_data_bulk;
use sequent_core::signatures::ecies_encrypt::SignRequest;
use sequent_core::temp_path::generate_temp_file;
use sequent_core::types::templates::VoteReceiptPipeType;
use sequent_core::util::date_time::get_date_and_time;
use serde::Serialize;
use serde_json::Map;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use strand::hash::{hash_b64, hash_sha256};
use tokio::runtime::Runtime;
use tracing::{info, instrument};

pub const VOTE_RECEIPTS_OUTPUT_FILE: &str = "vote_receipts";
pub const BALLOT_IMAGES_OUTPUT_FILE: &str = "ballots";

pub struct MCBallotReceipts {
    pub pipe_inputs: PipeInputs,
}

pub struct VoteReceiptsPipeData {
    pub output_file: String,
    pub pipe_name: String,
    pub pipe_name_output_dir: String,
}

// QR code = containing header of the report and voted candidates per position
// (if no votes, the content of QR code should be header of the report and "ABSTENTION")

#[instrument(skip_all)]
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

#[instrument(skip_all)]
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
        ballots: &[Bridge],
        contests: &Vec<Contest>,
        election_input: &InputElectionConfig,
        pipe_config: &PipeConfigVoteReceipts,
        area_name: &str,
    ) -> Result<(Option<Vec<u8>>, Vec<u8>)> {
        // 1. Gather the sign_data for all ballots/contests
        let mut bulk_sign_requests = Vec::new();

        // We'll store some structures that map from (ballotIndex, contestIndex, pageNum)
        // to the sign_data string, so later we can fill in the signatures.
        struct ContestLocator {
            sign_id: String,
            ballot_index: usize,
            contest_index: usize,
        }
        let mut locators = Vec::new();

        let contest_map: HashMap<String, Contest> = contests
            .iter()
            .map(|c| (c.id.to_string(), c.clone()))
            .collect();
        let execution_annotations: Option<HashMap<String, String>> =
            pipe_config.execution_annotations.clone();
        let precint_id = election_input
            .annotations
            .get(&"miru:precinct-code".to_string())
            .map(|s| s.as_str())
            .unwrap_or_default();
        let mut page_number = 1;
        let election_event_id = election_input
            .election_event_annotations
            .get("miru:election-event-id")
            .map(|s| s.as_str())
            .unwrap_or_default();
        let election_id = election_input
            .annotations
            .get("miru:election-id")
            .map(|s| s.as_str())
            .unwrap_or_default();

        let mut ballot_data = vec![];
        for (b_idx, ballot) in ballots.iter().enumerate() {
            let mut cds = vec![];
            for (c_idx, contest_choices) in ballot.choices.iter().enumerate() {
                let contest = contest_map
                    .get(&contest_choices.contest_id)
                    .ok_or_else(|| Error::UnexpectedError("Can't get contest".into()))?;

                let mut choices = DecodedChoice::from_dvcs(contest_choices, contest);

                let candidates_order = contest
                    .presentation
                    .clone()
                    .unwrap_or_default()
                    .candidates_order
                    .unwrap_or_default();
                sort_candidates(&mut choices, candidates_order.clone());

                let num_selected = choices.iter().filter(|can| can.is_selected()).count();
                let undervotes = contest.max_votes - (num_selected as i64);
                let overvotes = if (num_selected as i64) > contest.max_votes {
                    (num_selected as i64) - contest.max_votes
                } else {
                    0
                };

                // Instead of calling ecies_sign_data here, we only CREATE the data
                let (digital_signature, sign_data) =
                    match (&pipe_config.acm_key, &pipe_config.pipe_type) {
                        (Some(_acm_key), VoteReceiptPipeType::BALLOT_IMAGES) => {
                            let data_str = format!(
                                "{}:{}:{}:{}:{}",
                                election_event_id,
                                precint_id,
                                ballot.mcballot.serial_number.clone().unwrap_or_default(),
                                election_id,
                                page_number.to_string()
                            );
                            // We'll push this into our bulk_sign_requests
                            // We also need a unique ID to correlate the signature
                            let sign_id = format!("b{}_c{}_p{}", b_idx, c_idx, page_number);

                            bulk_sign_requests.push(SignRequest {
                                id: sign_id.clone(),
                                data: data_str.clone(),
                            });

                            // We'll store so we can insert the signature after we do the bulk sign
                            locators.push(ContestLocator {
                                sign_id: sign_id.clone(),
                                ballot_index: b_idx,
                                contest_index: c_idx,
                            });

                            // We do not have a signature yet, so just placeholders
                            (None, Some(data_str))
                        }
                        _ => (None, None),
                    };

                let cd: ContestData = ContestData {
                    contest: contest.clone(),
                    decoded_choices: choices,
                    undervotes,
                    overvotes,
                    digital_signature,
                    sign_data,
                    page_number: match &pipe_config.pipe_type {
                        VoteReceiptPipeType::BALLOT_IMAGES => Some(page_number),
                        _ => None,
                    },
                };

                page_number += 1;
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
                id: ballot.mcballot.serial_number.clone().unwrap_or_default(),
                encoded_vote,
                is_invalid: ballot.mcballot.is_explicit_invalid,
                is_blank,
                contest_choices: cds,
            };

            ballot_data.push(bd);
            page_number += 1; // inc by one for summary page
        }

        // 2. Now we do exactly one bulk sign if we have any sign_data
        let mut signatures_map: HashMap<String, String> = HashMap::new();
        if let (Some(acm_key), VoteReceiptPipeType::BALLOT_IMAGES) =
            (&pipe_config.acm_key, &pipe_config.pipe_type)
        {
            if !bulk_sign_requests.is_empty() {
                signatures_map = ecies_sign_data_bulk(acm_key, &bulk_sign_requests)
                    .map_err(|e| Error::UnexpectedError(format!("Error in bulk signing: {}", e)))?;
            }
        }

        // 3. Use the `locators` array to stitch the signatures back into `ballot_data`
        for locator in locators {
            // get the actual signature from the map
            if let Some(sig_base64) = signatures_map.get(&locator.sign_id) {
                if ballot_data.len() <= locator.ballot_index {
                    return Err(Error::UnexpectedError(format!(
                        "index out of bounds for ballot_index {} and length {}",
                        locator.ballot_index,
                        ballot_data.len()
                    )));
                }
                let bd = &mut ballot_data[locator.ballot_index];
                if bd.contest_choices.len() <= locator.contest_index {
                    return Err(Error::UnexpectedError(format!(
                        "index out of bounds for contest_index {} and length {}",
                        locator.contest_index,
                        bd.contest_choices.len()
                    )));
                }
                let cd = &mut bd.contest_choices[locator.contest_index];

                cd.digital_signature = Some(sig_base64.clone());
            }
        }

        let td = TemplateData {
            election_name: election_input.name.clone(),
            ballot_data,
            area: area_name.to_string(),
            election_annotations: election_input.annotations.clone(),
            election_dates: election_input.dates.clone(),
            execution_annotations: execution_annotations.unwrap_or_default().clone(),
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
            Some(
                pdf::sync::PdfRenderer::render_pdf(bytes_html.clone(), pdf_options).map_err(
                    |e| Error::UnexpectedError(format!("Error during PDF rendering: {}", e)),
                )?,
            )
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
            output_file: VOTE_RECEIPTS_OUTPUT_FILE.to_string(),
            pipe_name_output_dir: PipeNameOutputDir::MCBallotReceipts.as_ref().to_string(),
            pipe_name: PipeName::VoteReceipts.as_ref().to_string(),
        },
        VoteReceiptPipeType::BALLOT_IMAGES => VoteReceiptsPipeData {
            output_file: BALLOT_IMAGES_OUTPUT_FILE.to_string(),
            pipe_name_output_dir: PipeNameOutputDir::MCBallotImages.as_ref().to_string(),
            pipe_name: PipeName::MCBallotImages.as_ref().to_string(),
        },
    }
}

#[instrument(err, skip_all)]
fn generate_hashed_filename(
    path: &PathBuf,
    name: &str,
    hash_bytes: &[u8],
    area_id: &str,
    election_input: &InputElectionConfig,
    from_ballot: Option<&Bridge>,
    to_ballot: Option<&Bridge>,
) -> Result<PathBuf> {
    let path = path.as_path();
    let country_code = election_input
        .areas
        .iter()
        .find(|area| area.id == area_id.to_string())
        .and_then(|area| {
            area.annotations
                .as_ref()
                .and_then(|annotations| annotations.get("miru:area-station-id"))
                .and_then(|value| value.as_str())
        })
        .unwrap_or("");
    let post_code = election_input
        .annotations
        .get("miru:precinct-code")
        .map(|s| s.as_str())
        .unwrap_or("");
    let clustered_precint_id = election_input
        .annotations
        .get("clustered_precint_id")
        .map(|s| s.as_str())
        .unwrap_or("");

    let from_ballot_id = match from_ballot {
        Some(from_ballot) => from_ballot.mcballot.serial_number.as_deref().unwrap_or(""),
        None => "000000000",
    };
    let to_ballot_id = match to_ballot {
        Some(to_ballot) => to_ballot.mcballot.serial_number.as_deref().unwrap_or(""),
        None => "000000000",
    };

    let hash_hex = hex::encode(hash_bytes);

    let new_filename = format!(
        "{name}_{post_code}_{country_code}_{clustered_precint_id}_{from_ballot_id}-{to_ballot_id}_{hash_hex}.pdf"
    );

    Ok(path.join(new_filename))
}

#[derive(Serialize, Debug, Clone)]
struct BallotCsvData {
    pub file_name: String,
    pub hash: String,
}
impl Pipe for MCBallotReceipts {
    #[instrument(err, skip_all, name = "MultiBallotReceipts::exec")]
    fn exec(&self) -> Result<()> {
        let pipe_config: PipeConfigVoteReceipts = self.get_config()?;
        let pipe_data = get_pipe_data(pipe_config.pipe_type.clone());
        for election_input in &self.pipe_inputs.election_list {
            let area_contests_map = election_input.get_area_contest_map();

            let files = Mutex::new(vec![]);

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
                    let f = fs::File::open(path_ballots.as_path())
                        .map_err(|e| Error::FileAccess(path_ballots.as_path().to_path_buf(), e))?;
                    let mcballots: Vec<DecodedBallotChoices> = crate::utils::parse_file(f)?;

                    let ballots = convert_ballots(election_input, mcballots)?;
                    let report_options = pipe_config.report_options.clone().unwrap_or_default();
                    let max_threads = report_options.max_threads.unwrap_or_else(|| 3);
                    let pool = ThreadPoolBuilder::new()
                        .num_threads(max_threads)
                        .build()
                        .map_err(|e| {
                            Error::UnexpectedError(format!("Error building thread pool: {}", e))
                        })?;

                    let max_items_per_report =
                        report_options.max_items_per_report.unwrap_or_else(|| 100);

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

                    let chunks: Vec<&[Bridge]> = match ballots.is_empty() {
                        true => vec![&[] as &[Bridge]],
                        false => {
                            info!("ballots len = {len}", len = ballots.len());
                            ballots.chunks(max_items_per_report).collect()
                        }
                    };

                    let result: Result<(), Error> = pool.install(|| {
                        chunks.into_par_iter().enumerate().try_for_each(
                            |(chunk_index, chunk)| {
                                info!(
                                    "processing batch {chunk_index} len = {len}",
                                    len = chunk.len()
                                );
                                let (bytes_pdf, bytes_html) = self.print_vote_receipts(
                                    chunk,
                                    &area_contests.contests,
                                    &election_input,
                                    &pipe_config,
                                    &area_contests.area_name,
                                )?;

                                fs::create_dir_all(&path)?;

                                if let Some(ref some_bytes_pdf) = bytes_pdf {
                                    // pdf file creation
                                    let pdf_hash =
                                        hash_sha256(some_bytes_pdf.as_slice()).map_err(|e| {
                                            Error::UnexpectedError(format!(
                                                "Error during hash pdf bytes: {}",
                                                e
                                            ))
                                        })?;

                                    let base_file_name = pipe_data.output_file.clone();
                                    let from_ballot = match ballots.is_empty() {
                                        true => None,
                                        false => Some(chunk.first().ok_or(
                                            Error::UnexpectedError("Can't get first chunk".into()),
                                        )?),
                                    };

                                    let to_ballot = match ballots.is_empty() {
                                        true => None,
                                        false => Some(chunk.last().ok_or(
                                            Error::UnexpectedError("Can't get last chunk".into()),
                                        )?),
                                    };

                                    let file = generate_hashed_filename(
                                        &path,
                                        &base_file_name.clone(),
                                        &pdf_hash,
                                        &area_id.to_string(),
                                        election_input,
                                        from_ballot,
                                        to_ballot,
                                    )
                                    .map_err(|e| {
                                        Error::UnexpectedError(format!(
                                            "Error during hash pdf bytes: {}",
                                            e
                                        ))
                                    })?;

                                    let file_name = file
                                        .file_name()
                                        .ok_or(Error::UnexpectedError(
                                            "Can't get file name".into(),
                                        ))?
                                        .to_str()
                                        .ok_or(Error::UnexpectedError(
                                            "Can't get file name".into(),
                                        ))?;
                                    let bytes_json = file_name.as_bytes().to_vec();
                                    let file_hash = hash_b64(&bytes_json).map_err(|err| {
                                        Error::UnexpectedError(format!(
                                            "Error hashing the results file: {err:?}"
                                        ))
                                    })?;

                                    // Lock the mutex before modifying the vector
                                    let mut files_lock = files.lock().map_err(|e| {
                                        Error::UnexpectedError(format!(
                                            "Error locking files: {}",
                                            e
                                        ))
                                    })?;
                                    files_lock.push(BallotCsvData {
                                        file_name: file_name.to_string(),
                                        hash: file_hash,
                                    });

                                    let mut file = OpenOptions::new()
                                        .write(true)
                                        .truncate(true)
                                        .create(true)
                                        .open(file)?;
                                    file.write_all(some_bytes_pdf)?;
                                }

                                let file = path.join(format!(
                                    "{}_batch-{}.html",
                                    pipe_data.output_file, chunk_index,
                                ));

                                let mut file = OpenOptions::new()
                                    .write(true)
                                    .truncate(true)
                                    .create(true)
                                    .open(file)?;
                                file.write_all(&bytes_html)?;
                                Ok::<(), Error>(())
                            },
                        )?;

                        // Write the CSV file of file names and hashes ONLY for `ballot` type
                        if pipe_data.output_file.clone() == BALLOT_IMAGES_OUTPUT_FILE {
                            let csv_filename = format!("ballots_files.csv");
                            let csv_path = path.join(csv_filename);
                            let files_lock = files.lock().map_err(|e| {
                                Error::UnexpectedError(format!("Error locking files: {}", e))
                            })?;

                            let rt = Runtime::new()?;
                            rt.block_on(async {
                                write_file_hash_csv(files_lock.clone(), csv_path)
                                    .await
                                    .map_err(|e| {
                                        Error::UnexpectedError(format!(
                                            "Error writing file hash CSV: {}",
                                            e
                                        ))
                                    })
                            })?;
                        }

                        Ok(())
                    });

                    if let Err(e) = result {
                        eprintln!("Error processing: {}", e);
                    }
                } else {
                    println!(
                        "[{}] File not found: {} -- Not processed",
                        &pipe_data.pipe_name,
                        path_ballots.display()
                    );
                };
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
    pub digital_signature: Option<String>,
    pub sign_data: Option<String>,
    pub page_number: Option<i64>,
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

// We are reusing some functionality from the standard receipts pipe/template,
// so it helps to convert mcballots to dcv format
#[instrument(err, skip_all)]
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

pub async fn write_file_hash_csv(data: Vec<BallotCsvData>, path: PathBuf) -> Result<()> {
    let headers = vec!["file_name".to_string(), "hash".to_string()];

    let mut writer = Writer::from_writer(vec![]);

    writer.write_record(&headers).map_err(|e| {
        Error::UnexpectedError(format!("Failed to write headers to CSV file: {}", e))
    })?;

    for entry in data {
        writer
            .write_record(&[entry.file_name, entry.hash])
            .map_err(|e| Error::UnexpectedError(format!("Failed to write record: {}", e)))?;
    }

    let data_bytes = writer
        .into_inner()
        .map_err(|e| Error::UnexpectedError(format!("Failed to flush CSV writer: {}", e)))?;

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;
    file.write_all(&data_bytes)?;

    Ok(())
}
