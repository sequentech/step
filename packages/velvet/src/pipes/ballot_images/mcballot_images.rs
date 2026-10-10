// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::ballot_images_config::{
    BallotImageSignaturePolicy, PipeConfigBallotImages, BALLOT_IMAGE_SIGNATURE_POLICY_ANNOTATION,
    DEFAULT_MCBALLOT_TITLE,
};
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
use sequent_core::signatures::ecies_encrypt::EciesKeyPair;
use sequent_core::signatures::ecies_encrypt::SignRequest;
use sequent_core::temp_path::generate_temp_file;
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

pub const BALLOT_IMAGES_OUTPUT_FILE: &str = "ballots";
pub const BALLOT_FILES_CSV: &str = "ballots_files.csv";
pub const BALLOT_FILES_CSV_SIGNATURE: &str = "ballots_files.csv.sign";
pub const BALLOT_PAGE_SIGN_PAYLOAD_VERSION: &str = "v2";

pub struct MCBallotImages {
    pub pipe_inputs: PipeInputs,
}

pub struct BallotImagesPipeData {
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

impl MCBallotImages {
    #[instrument(skip_all, name = "MCBallotImages::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }

    #[instrument(skip_all, err)]
    fn print_ballot_images(
        &self,
        ballots: &[Bridge],
        contests: &Vec<Contest>,
        election_input: &InputElectionConfig,
        pipe_config: &PipeConfigBallotImages,
        signature_policy: BallotImageSignaturePolicy,
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
                let (digital_signature, sign_data) = if pipe_config.acm_key.is_some() {
                    let page_ids = BallotPageIds {
                        election_event_id,
                        precinct_id: precint_id,
                        serial_number: ballot.mcballot.serial_number.as_deref().unwrap_or_default(),
                        election_id,
                        page_number,
                    };
                    let data_str = ballot_page_sign_payload(
                        signature_policy,
                        &page_ids,
                        &contest_choices.contest_id,
                        &choices,
                    )?;
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
                } else {
                    (None, None)
                };

                let cd: ContestData = ContestData {
                    contest: contest.clone(),
                    decoded_choices: choices,
                    undervotes,
                    overvotes,
                    digital_signature,
                    sign_data,
                    page_number: Some(page_number),
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
        if let Some(acm_key) = &pipe_config.acm_key {
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
            election_alias: election_input.alias.clone(),
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
    pub fn get_config(&self) -> Result<PipeConfigBallotImages> {
        let pipe_config: PipeConfigBallotImages = self
            .pipe_inputs
            .stage
            .pipe_config(self.pipe_inputs.stage.current_pipe)
            .and_then(|pc| pc.config)
            .map(|value| serde_json::from_value(value))
            .transpose()?
            .unwrap_or(PipeConfigBallotImages::mcballot());
        Ok(pipe_config)
    }
}

struct BallotPageIds<'a> {
    election_event_id: &'a str,
    precinct_id: &'a str,
    serial_number: &'a str,
    election_id: &'a str,
    page_number: i64,
}

/// Encodes each field as `<byte length>:<value>`, joined with `:`.
fn length_prefixed(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|field| format!("{}:{}", field.len(), field))
        .collect::<Vec<String>>()
        .join(":")
}

fn ballot_page_sign_payload(
    policy: BallotImageSignaturePolicy,
    ids: &BallotPageIds,
    contest_id: &str,
    choices: &[DecodedChoice],
) -> Result<String> {
    let page_number = ids.page_number.to_string();
    match policy {
        BallotImageSignaturePolicy::IdentifiersOnly => Ok(format!(
            "{}:{}:{}:{}:{}",
            ids.election_event_id, ids.precinct_id, ids.serial_number, ids.election_id, page_number
        )),
        BallotImageSignaturePolicy::IdentifiersAndChoices => {
            let mut selected: Vec<&str> = choices
                .iter()
                .filter(|choice| choice.is_selected())
                .map(|choice| choice.choice.id.as_str())
                .collect();
            selected.sort_unstable();
            let selections_hash =
                hash_sha256(length_prefixed(&selected).as_bytes()).map_err(|e| {
                    Error::UnexpectedError(format!("Error hashing the page selections: {e}"))
                })?;
            let selections_hash = hex::encode(selections_hash);
            Ok(format!(
                "{}:{}",
                BALLOT_PAGE_SIGN_PAYLOAD_VERSION,
                length_prefixed(&[
                    ids.election_event_id,
                    ids.precinct_id,
                    ids.serial_number,
                    ids.election_id,
                    &page_number,
                    contest_id,
                    &selections_hash,
                ])
            ))
        }
    }
}

fn sign_ballot_files_csv(acm_key: &EciesKeyPair, csv_bytes: Vec<u8>) -> Result<String> {
    let data = String::from_utf8(csv_bytes)
        .map_err(|e| Error::UnexpectedError(format!("Ballot files CSV is not UTF-8: {e}")))?;
    let request = SignRequest {
        id: BALLOT_FILES_CSV.to_string(),
        data,
    };
    let mut signatures = ecies_sign_data_bulk(acm_key, &[request])
        .map_err(|e| Error::UnexpectedError(format!("Error signing the ballot files CSV: {e}")))?;
    signatures
        .remove(BALLOT_FILES_CSV)
        .ok_or_else(|| Error::UnexpectedError("Missing ballot files CSV signature".into()))
}

#[instrument(skip_all)]
fn get_pipe_data() -> BallotImagesPipeData {
    BallotImagesPipeData {
        output_file: BALLOT_IMAGES_OUTPUT_FILE.to_string(),
        pipe_name_output_dir: PipeNameOutputDir::MCBallotImages.as_ref().to_string(),
        pipe_name: PipeName::MCBallotImages.as_ref().to_string(),
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
impl Pipe for MCBallotImages {
    #[instrument(err, skip_all, name = "MultiBallotReceipts::exec")]
    fn exec(&self) -> Result<()> {
        let pipe_config: PipeConfigBallotImages = self.get_config()?;
        let pipe_data = get_pipe_data();
        for election_input in &self.pipe_inputs.election_list {
            let signature_policy = BallotImageSignaturePolicy::from_annotations(
                &election_input.election_event_annotations,
            )
            .map_err(|e| {
                Error::UnexpectedError(format!(
                    "Invalid {BALLOT_IMAGE_SIGNATURE_POLICY_ANNOTATION} annotation: {e}"
                ))
            })?;
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
                                let (bytes_pdf, bytes_html) = self.print_ballot_images(
                                    chunk,
                                    &area_contests.contests,
                                    &election_input,
                                    &pipe_config,
                                    signature_policy,
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
                            let csv_path = path.join(BALLOT_FILES_CSV);
                            let files_lock = files.lock().map_err(|e| {
                                Error::UnexpectedError(format!("Error locking files: {}", e))
                            })?;

                            let rt = Runtime::new()?;
                            let csv_bytes = rt.block_on(async {
                                write_file_hash_csv(files_lock.clone(), csv_path)
                                    .await
                                    .map_err(|e| {
                                        Error::UnexpectedError(format!(
                                            "Error writing file hash CSV: {}",
                                            e
                                        ))
                                    })
                            })?;

                            if let Some(acm_key) = &pipe_config.acm_key {
                                let signature = sign_ballot_files_csv(acm_key, csv_bytes)?;
                                fs::write(path.join(BALLOT_FILES_CSV_SIGNATURE), signature)?;
                            }
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
    pub election_alias: String,
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
                    is_explicit_invalid: contest.is_explicit_invalid,
                    is_decline_to_vote: dbc.is_explicit_invalid,
                    is_blank_ballot: dbc.is_blank_ballot,
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

pub async fn write_file_hash_csv(data: Vec<BallotCsvData>, path: PathBuf) -> Result<Vec<u8>> {
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

    Ok(data_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERIAL_NUMBER: &str = "000000001";
    const PAGE_NUMBER: i64 = 3;

    fn page_ids<'a>(precinct_id: &'a str, election_id: &'a str) -> BallotPageIds<'a> {
        BallotPageIds {
            election_event_id: "event",
            precinct_id,
            serial_number: SERIAL_NUMBER,
            election_id,
            page_number: PAGE_NUMBER,
        }
    }

    fn choice(id: &str, selected: i64) -> DecodedChoice {
        DecodedChoice {
            choice: DecodedVoteChoice {
                id: id.to_string(),
                selected,
                write_in_text: None,
            },
            candidate: None,
        }
    }

    fn build_payload(
        policy: BallotImageSignaturePolicy,
        ids: &BallotPageIds,
        contest_id: &str,
        choices: &[DecodedChoice],
    ) -> String {
        ballot_page_sign_payload(policy, ids, contest_id, choices).expect("payload")
    }

    #[test]
    fn identifiers_only_payload_keeps_original_format() {
        let payload = build_payload(
            BallotImageSignaturePolicy::IdentifiersOnly,
            &page_ids("prec", "elec"),
            "contest",
            &[choice("a", 1), choice("b", -1)],
        );

        assert_eq!(payload, "event:prec:000000001:elec:3");
    }

    #[test]
    fn identifiers_and_choices_payload_matches_documented_encoding() {
        let payload = build_payload(
            BallotImageSignaturePolicy::IdentifiersAndChoices,
            &page_ids("prec", "elec"),
            "contest",
            &[choice("c", 1), choice("b", -1), choice("a", 1)],
        );

        assert_eq!(
            payload,
            "v2:5:event:4:prec:9:000000001:4:elec:1:3:7:contest:64:\
             b2939ab7982fed32555ff3b42e4361401f8a511e9f52c81f58e0fbeea7573fdb"
        );
    }

    #[test]
    fn identifiers_and_choices_payload_covers_selected_candidates() {
        let ids = page_ids("prec", "elec");
        let first = build_payload(
            BallotImageSignaturePolicy::IdentifiersAndChoices,
            &ids,
            "contest",
            &[choice("a", 1), choice("b", -1)],
        );
        let second = build_payload(
            BallotImageSignaturePolicy::IdentifiersAndChoices,
            &ids,
            "contest",
            &[choice("a", -1), choice("b", 1)],
        );
        let blank = build_payload(
            BallotImageSignaturePolicy::IdentifiersAndChoices,
            &ids,
            "contest",
            &[choice("a", -1), choice("b", -1)],
        );

        assert_ne!(first, second);
        assert_ne!(first, blank);
        assert_ne!(second, blank);
    }

    #[test]
    fn identifiers_and_choices_payload_covers_contest_id() {
        let ids = page_ids("prec", "elec");
        let choices = [choice("a", 1)];

        assert_ne!(
            build_payload(
                BallotImageSignaturePolicy::IdentifiersAndChoices,
                &ids,
                "contest-1",
                &choices
            ),
            build_payload(
                BallotImageSignaturePolicy::IdentifiersAndChoices,
                &ids,
                "contest-2",
                &choices
            ),
        );
    }

    #[test]
    fn identifiers_and_choices_payload_keeps_field_boundaries() {
        let choices = [choice("a", 1)];
        let joined_event = BallotPageIds {
            election_event_id: "event:prec",
            precinct_id: "",
            ..page_ids("", "elec")
        };
        let joined_precinct = BallotPageIds {
            election_event_id: "event",
            precinct_id: "prec:",
            ..page_ids("", "elec")
        };

        assert_ne!(
            build_payload(
                BallotImageSignaturePolicy::IdentifiersAndChoices,
                &joined_event,
                "contest",
                &choices
            ),
            build_payload(
                BallotImageSignaturePolicy::IdentifiersAndChoices,
                &joined_precinct,
                "contest",
                &choices
            ),
        );
    }

    #[test]
    fn identifiers_and_choices_payload_ignores_candidate_order() {
        let ids = page_ids("prec", "elec");

        assert_eq!(
            build_payload(
                BallotImageSignaturePolicy::IdentifiersAndChoices,
                &ids,
                "contest",
                &[choice("a", 1), choice("b", 1), choice("c", -1)]
            ),
            build_payload(
                BallotImageSignaturePolicy::IdentifiersAndChoices,
                &ids,
                "contest",
                &[choice("c", -1), choice("b", 1), choice("a", 1)]
            ),
        );
    }
}
