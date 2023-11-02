// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use sequent_core::{
    ballot::{BallotStyle, Candidate, Contest},
    services::{pdf, reports},
};
use serde::Serialize;
use serde_json::Map;
use uuid::Uuid;

use super::error::{Error, Result};
use crate::pipes::{
    do_tally::{ContestResult, OUTPUT_CONTEST_RESULT_FILE},
    mark_winners::{WinnerResult, OUTPUT_WINNERS},
    pipe_inputs::{PipeInputs, PREFIX_ELECTION},
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
        reports: Vec<ReportData>,
    ) -> Result<Vec<u8>> {
        let reports = reports
            .iter()
            .map(|r| {
                let map_winners: HashMap<_, _> = r
                    .winners
                    .iter()
                    .map(|cr| (cr.candidate.id.clone(), cr.winning_position.clone()))
                    .collect();

                let candidate_result: Vec<CandidateResultForReport> = r
                    .contest_result
                    .candidate_result
                    .iter()
                    .map(|cr| CandidateResultForReport {
                        candidate: cr.candidate.clone(),
                        total_count: cr.total_count,
                        winning_position: map_winners.get(&cr.candidate.id).cloned(),
                    })
                    .collect();

                ReportDataComputed {
                    contest: r.contest.clone(),
                    candidate_result,
                }
            })
            .collect::<Vec<ReportDataComputed>>();

        let mut map = Map::new();
        map.insert(
            "ballot_style".to_owned(),
            serde_json::to_value(ballot_style)?,
        );
        map.insert("reports".to_owned(), serde_json::to_value(reports)?);

        let html = include_str!("../../resources/report.hbs");
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
    ) -> Result<Vec<WinnerResult>> {
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

        let res: Vec<WinnerResult> = serde_json::from_reader(f)?;

        Ok(res)
    }
}

impl Pipe for GenerateReports {
    fn exec(&self) -> Result<()> {
        for election_input in &self.pipe_inputs.election_list {
            let mut reports = vec![];
            for contest_input in &election_input.contest_list {
                let contest_result =
                    self.read_contest_result(&election_input.id, &contest_input.id, None)?;

                let winners = self.read_winners(&election_input.id, &contest_input.id, None)?;

                reports.push(ReportData {
                    contest: contest_input.contest.clone(),
                    contest_result,
                    winners,
                })
            }

            let bytes = self.generate_report(&election_input.ballot_style, reports)?;

            let path = &self
                .output_dir
                .join(format!("{}{}", PREFIX_ELECTION, election_input.id));
            fs::create_dir_all(path)?;

            let file = path.join(OUTPUT_PDF);
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&file)?;

            file.write_all(&bytes)?;
            serde_json::to_writer(file, &bytes)?;
        }

        Ok(())
    }
}

pub struct ReportData {
    pub contest: Contest,
    pub contest_result: ContestResult,
    pub winners: Vec<WinnerResult>,
}

#[derive(Serialize)]
pub struct ReportDataComputed {
    pub contest: Contest,
    pub candidate_result: Vec<CandidateResultForReport>,
}

#[derive(Serialize)]
pub struct CandidateResultForReport {
    pub candidate: Candidate,
    pub total_count: u64,
    pub winning_position: Option<usize>,
}
