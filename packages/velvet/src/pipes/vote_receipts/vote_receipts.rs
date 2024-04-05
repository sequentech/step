// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::vote_receipt::PipeConfigVoteReceipts;
use crate::pipes::decode_ballots::OUTPUT_DECODED_BALLOTS_FILE;
use crate::pipes::do_tally::tally::Tally;
use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::PipeInputs;
use crate::pipes::Pipe;
use num_bigint::BigUint;
use sequent_core::ballot::{Candidate, CandidatesOrder, Contest};
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::DecodedVoteContest;
use sequent_core::services::{pdf, reports};
use serde::Serialize;
use serde_json::Map;

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use std::str::FromStr;
use tracing::instrument;

use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};

pub const OUTPUT_FILE: &str = "vote_receipts.pdf";

pub struct VoteReceipts {
    pub pipe_inputs: PipeInputs,
}

impl VoteReceipts {
    #[instrument(skip_all, name = "VoteReceipts::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl VoteReceipts {
    fn print_vote_receipts(&self, path: &Path, contest: &Contest) -> Result<Vec<u8>> {
        let tally = Tally::new(contest, vec![path.to_path_buf()], 0)
            .map_err(|e| Error::UnexpectedError(e.to_string()))?;

        let pipe_config = self
            .pipe_inputs
            .stage
            .pipe_config(self.pipe_inputs.stage.current_pipe)
            .and_then(|pc| pc.config)
            .ok_or(Error::UnexpectedError(
                "Pipe config for VoteReceipts not found".to_string(),
            ))?;

        let pipe_config: PipeConfigVoteReceipts = serde_json::from_value(pipe_config)?;
        let template = pipe_config.template;

        let data = TemplateData {
            contest: tally.contest.clone(),
            ballots: tally.ballots.clone(),
        };
        let data = compute_data(data);

        let mut map = Map::new();
        map.insert("data".to_string(), serde_json::to_value(&data)?);

        let bytes_html = reports::render_template_text(&template, map).map_err(|e| {
            Error::UnexpectedError(format!(
                "Error during render_template_text from report.hbs template file: {}",
                e
            ))
        })?;

        let bytes_pdf = pdf::html_to_pdf(bytes_html).map_err(|e| {
            Error::UnexpectedError(format!("Error during html_to_pdf conversion: {}", e))
        })?;

        Ok(bytes_pdf)
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

                    let res = self.print_vote_receipts(
                        decoded_ballots_file.as_path(),
                        &contest_input.contest,
                    )?;

                    let file = PathBuf::from("./toto.pdf");
                    let mut file = OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open(file)?;
                    file.write_all(&res)?;

                    // let path =
                    //     PipeInputs::build_path(&self.pipe_inputs.cli.output_dir.as, election_id, contest_id, area_id);
                    //
                    // fs::create_dir_all(&path)?;
                    //
                    // let file = path.join(OUTPUT_PDF);
                    // let mut file = OpenOptions::new()
                    //     .write(true)
                    //     .truncate(true)
                    //     .create(true)
                    //     .open(file)?;
                    // file.write_all(&bytes_pdf)?;
                    //
                    // let file = path.join(OUTPUT_HTML);
                    // let mut file = OpenOptions::new()
                    //     .write(true)
                    //     .truncate(true)
                    //     .create(true)
                    //     .open(file)?;
                    // file.write_all(&bytes_html)?;
                    //
                    // let file = path.join(OUTPUT_JSON);
                    // let mut file = OpenOptions::new()
                    //     .write(true)
                    //     .truncate(true)
                    //     .create(true)
                    //     .open(file)?;
                    // file.write_all(&bytes_json)?;

                    // if let Err(Error::FileAccess(file, _)) = &res {
                    //     println!(
                    //         "[{}] File not found: {} -- Not processed",
                    //         PipeName::VoteReceipts.as_ref(),
                    //         file.display()
                    //     );
                    // }
                    //
                    // match res {
                    //     Ok(decoded_ballots) => {
                    //         let mut output_path = PipeInputs::build_path(
                    //             self.pipe_inputs
                    //                 .cli
                    //                 .output_dir
                    //                 .join(PipeNameOutputDir::VoteReceipts.as_ref())
                    //                 .as_path(),
                    //             &election_input.id,
                    //             Some(&contest_input.id),
                    //             Some(&area_input.id),
                    //         );
                    //
                    //         fs::create_dir_all(&output_path)?;
                    //         output_path.push(OUTPUT_FILE);
                    //         let file = File::create(&output_path)
                    //             .map_err(|e| Error::FileAccess(output_path, e))?;
                    //
                    //         serde_json::to_writer(file, &decoded_ballots)?;
                    //     }
                    //     Err(e) => {
                    //         if let Error::FileAccess(file, _) = &e {
                    //             println!(
                    //                 "[{}] File not found: {} -- Not processed",
                    //                 PipeName::VoteReceipts.as_ref(),
                    //                 file.display()
                    //             )
                    //         } else {
                    //             return Err(e);
                    //         }
                    //     }
                    // }
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
}

#[derive(Serialize, Debug)]
struct ComputedTemplateData {
    pub contest: Contest,
    pub receipts: Vec<ReceiptData>,
}

#[derive(Serialize, Debug)]
struct ReceiptData {
    pub is_invalid: bool,
    pub is_blank: bool,
    pub selected_candidates: Vec<Candidate>,
}

pub fn compute_data(data: TemplateData) -> ComputedTemplateData {
    let receipts = data
        .ballots
        .iter()
        .map(|decoded_vote_contest| {
            let mut candidates = decoded_vote_contest
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

            let mut is_invalid = !decoded_vote_contest.invalid_errors.is_empty();
            let is_blank = candidates.len() == 0;

            ReceiptData {
                is_invalid,
                is_blank,
                selected_candidates: candidates,
            }
        })
        .collect::<Vec<ReceiptData>>();

    ComputedTemplateData {
        contest: data.contest,
        receipts,
    }
}
