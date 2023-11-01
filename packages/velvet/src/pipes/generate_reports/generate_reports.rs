// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use sequent_core::{
    ballot::{BallotStyle, Contest},
    services::{pdf, reports},
};
use serde::Serialize;
use serde_json::Map;
use uuid::Uuid;

use super::error::{Error, Result};
use crate::pipes::{
    do_tally::{ContestResult, OUTPUT_CONTEST_RESULT_FILE},
    mark_winners::{WinnerCandidate, OUTPUT_WINNERS},
    pipe_inputs::PipeInputs,
    pipe_name::PipeNameOutputDir,
    Pipe,
};

const OUTPUT_PDF: &str = "report.pdf";

pub struct GenerateReports {
    pub pipe_inputs: PipeInputs,
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
}

impl GenerateReports {
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        let input_dir = pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::MarkWinners.as_ref());
        let output_dir = pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::GenerateReports.as_ref());

        Self {
            pipe_inputs,
            input_dir,
            output_dir,
        }
    }

    pub fn generate_report(
        &self,
        ballot_style: &BallotStyle,
        contest: &Contest,
        contest_result: &ContestResult,
        winner: &WinnerCandidate,
    ) -> Result<Vec<u8>> {
        let mut map = Map::new();

        map.insert(
            "ballot_style".to_owned(),
            serde_json::to_value(ballot_style)?,
        );
        map.insert(
            "report_list".to_owned(),
            serde_json::to_value(vec![ReportForContest {
                contest: contest.clone(),
                contest_result: contest_result.clone(),
            }])?,
        );
        map.insert("winner".to_owned(), serde_json::to_value(winner)?);

        let html = include_str!("../../resources/report.html");
        let render = reports::render_template_text(html, map)?;

        let bytes = pdf::html_to_pdf(render)?;

        Ok(bytes)
    }

    fn read_contest_result(
        &self,
        election_id: &Uuid,
        contest_id: &Uuid,
        region_id: Option<&Uuid>,
    ) -> Result<ContestResult> {
        let path = PipeInputs::build_path(
            &self
                .pipe_inputs
                .cli
                .output_dir
                .as_path()
                .join(PipeNameOutputDir::DoTally.as_ref()),
            election_id,
            contest_id,
            region_id,
        )
        .join(OUTPUT_CONTEST_RESULT_FILE);

        let f = fs::File::open(&path).map_err(|e| Error::IO(path.clone(), e))?;

        let res: ContestResult = serde_json::from_reader(f)?;

        Ok(res)
    }

    fn read_winners(
        &self,
        election_id: &Uuid,
        contest_id: &Uuid,
        region_id: Option<&Uuid>,
    ) -> Result<WinnerCandidate> {
        let path = PipeInputs::build_path(
            &self
                .pipe_inputs
                .cli
                .output_dir
                .as_path()
                .join(PipeNameOutputDir::MarkWinners.as_ref()),
            election_id,
            contest_id,
            region_id,
        )
        .join(OUTPUT_WINNERS);

        let f = fs::File::open(&path).map_err(|e| Error::IO(path.clone(), e))?;

        let res: WinnerCandidate = serde_json::from_reader(f)?;

        Ok(res)
    }
}

impl Pipe for GenerateReports {
    fn exec(&self) -> Result<()> {
        for election_input in &self.pipe_inputs.election_list {
            for contest_input in &election_input.contest_list {
                for region_input in &contest_input.region_list {
                    let contest_result = self.read_contest_result(
                        &election_input.id,
                        &contest_input.id,
                        Some(&region_input.id),
                    )?;

                    let winner_candidate = self.read_winners(
                        &election_input.id,
                        &contest_input.id,
                        Some(&region_input.id),
                    )?;

                    let bytes = self.generate_report(
                        &election_input.ballot_style,
                        &contest_input.contest,
                        &contest_result,
                        &winner_candidate,
                    )?;

                    let mut file = PipeInputs::build_path(
                        &self.output_dir,
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

                let contest_result =
                    self.read_contest_result(&election_input.id, &contest_input.id, None)?;

                let winner_candidate =
                    self.read_winners(&election_input.id, &contest_input.id, None)?;

                let bytes = self.generate_report(
                    &election_input.ballot_style,
                    &contest_input.contest,
                    &contest_result,
                    &winner_candidate,
                )?;

                let mut file = PipeInputs::build_path(
                    &self.output_dir,
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

#[derive(Serialize)]
pub struct ReportForContest {
    pub contest: Contest,
    pub contest_result: ContestResult,
}
