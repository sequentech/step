// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use sequent_core::{
    ballot::{Candidate, Contest},
    services::{pdf, reports},
};
use serde_json::Map;

use super::error::{Error, Result};
use crate::pipes::{
    do_tally::{ContestResult, OUTPUT_CONTEST_RESULT_FILE},
    mark_winners::{WinnerCandidate, OUTPUT_WINNERS},
    pipe_inputs::{PipeInputs, CONTEST_CONFIG_FILE},
    pipe_name::PipeNameOutputDir,
    Pipe,
};

const OUTPUT_PDF: &str = "report.pdf";

pub struct GenerateReports {
    pub pipe_inputs: PipeInputs,
}

impl GenerateReports {
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }

    pub fn generate_report(&self, contest: &Contest, winner: &WinnerCandidate) -> Result<Vec<u8>> {
        let mut map = Map::new();
        map.insert("contest".to_owned(), serde_json::to_value(&contest)?);
        map.insert("winner".to_owned(), serde_json::to_value(&winner)?);

        let html = include_str!("../../resources/report.html");
        let render = reports::render_template_text(html, map)?;

        let bytes = pdf::html_to_pdf(render)?;

        Ok(bytes)
    }
}

impl Pipe for GenerateReports {
    fn exec(&self) -> Result<()> {
        let input_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::MarkWinners.as_ref());
        let output_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::GenerateReports.as_ref());

        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                let f = fs::File::open(&contest_input.config)
                    .map_err(|e| Error::IO(contest_input.config.clone(), e))?;
                let contest: Contest = serde_json::from_reader(f)?;

                for region_input in &contest_input.region_list {
                    let contest_result_file = self
                        .pipe_inputs
                        .get_path_for_data(
                            &input_dir,
                            &contest_input.election_id,
                            &contest_input.id,
                            Some(&region_input.id),
                        )
                        .join(OUTPUT_WINNERS);

                    let f = fs::File::open(&contest_result_file)
                        .map_err(|e| Error::IO(contest_result_file.clone(), e))?;
                    let winner: WinnerCandidate = serde_json::from_reader(f)?;

                    let bytes = self.generate_report(&contest, &winner)?;

                    let mut file = self.pipe_inputs.get_path_for_data(
                        &output_dir,
                        &contest_input.election_id,
                        &contest_input.id,
                        Some(&region_input.id),
                    );

                    fs::create_dir_all(&file)?;
                    file.push(OUTPUT_PDF);

                    let mut file = OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open(&file)?;

                    file.write_all(&bytes)?;
                }

                let contest_result_file = self
                    .pipe_inputs
                    .get_path_for_data(
                        &input_dir,
                        &contest_input.election_id,
                        &contest_input.id,
                        None,
                    )
                    .join(OUTPUT_WINNERS);

                let f = fs::File::open(&contest_result_file)
                    .map_err(|e| Error::IO(contest_result_file.clone(), e))?;
                let winner: WinnerCandidate = serde_json::from_reader(f)?;

                let bytes = self.generate_report(&contest, &winner)?;

                let mut file = self.pipe_inputs.get_path_for_data(
                    &output_dir,
                    &contest_input.election_id,
                    &contest_input.id,
                    None,
                );

                fs::create_dir_all(&file)?;
                file.push(OUTPUT_PDF);

                let mut file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(&file)?;

                file.write_all(&bytes)?;
                serde_json::to_writer(file, &bytes)?;
            }
        }

        Ok(())
    }
}
