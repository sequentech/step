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
use tracing::instrument;
use uuid::Uuid;

use crate::pipes::{
    do_tally::invalid_vote::InvalidVote,
    error::{Error, Result},
};
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
    #[instrument]
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

    #[instrument(skip_all)]
    pub fn compute_reports(&self, reports: Vec<ReportData>) -> Result<Vec<ReportDataComputed>> {
        let reports = reports
            .iter()
            .map(|r| {
                let map_winners: HashMap<_, _> = r
                    .winners
                    .iter()
                    .map(|cr| (cr.candidate.id.clone(), cr.winning_position))
                    .collect();

                let mut candidate_result: Vec<CandidateResultForReport> = r
                    .contest_result
                    .candidate_result
                    .iter()
                    .map(|cr| CandidateResultForReport {
                        candidate: cr.candidate.clone(),
                        total_count: cr.total_count,
                        winning_position: map_winners.get(&cr.candidate.id).cloned(),
                    })
                    .collect();

                candidate_result.sort_by(|a, b| {
                    a.winning_position
                        .unwrap_or(usize::MAX)
                        .cmp(&b.winning_position.unwrap_or(usize::MAX))
                        .then_with(|| a.total_count.cmp(&b.total_count))
                        .then_with(|| a.candidate.name.cmp(&b.candidate.name))
                });

                ReportDataComputed {
                    contest: r.contest.clone(),
                    contest_result: r.contest_result.clone(),
                    area_id: r.area_id.clone(),
                    candidate_result,
                }
            })
            .collect::<Vec<ReportDataComputed>>();
        Ok(reports)
    }

    #[instrument(skip_all)]
    pub fn generate_report(
        &self,
        ballot_style: &BallotStyle,
        reports: Vec<ReportData>,
    ) -> Result<Vec<u8>> {
        let reports = self.compute_reports(reports)?;

        let mut map = Map::new();
        map.insert(
            "ballot_style".to_owned(),
            serde_json::to_value(ballot_style)?,
        );
        map.insert("reports".to_owned(), serde_json::to_value(reports)?);

        let html = include_str!("../../resources/report.hbs");
        let render = reports::render_template_text(html, map).map_err(|e| {
            Error::UnexpectedError(format!(
                "Error during render_template_text from report.hbs template file: {}",
                e.to_string()
            ))
        })?;

        let bytes = pdf::html_to_pdf(render).map_err(|e| {
            Error::UnexpectedError(format!(
                "Error during html_to_pdf conversion: {}",
                e.to_string()
            ))
        })?;

        Ok(bytes)
    }

    #[instrument(skip(self))]
    fn read_contest_result(
        &self,
        election_id: &Uuid,
        contest_id: &Uuid,
        area_id: Option<&Uuid>,
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
            area_id,
        )
        .join(OUTPUT_CONTEST_RESULT_FILE);

        let f = fs::File::open(&path).map_err(|e| Error::FileAccess(path.clone(), e))?;

        let res: ContestResult = serde_json::from_reader(f)?;

        Ok(res)
    }

    #[instrument(skip(self))]
    fn read_winners(
        &self,
        election_id: &Uuid,
        contest_id: &Uuid,
        area_id: Option<&Uuid>,
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
            area_id,
        )
        .join(OUTPUT_WINNERS);

        let f = fs::File::open(&path).map_err(|e| Error::FileAccess(path.clone(), e))?;

        let res: Vec<WinnerResult> = serde_json::from_reader(f)?;

        Ok(res)
    }

    #[instrument(skip(self))]
    pub fn read_reports(&self) -> Result<Vec<ElectionReportDataComputed>> {
        let mut election_reports: Vec<ElectionReportDataComputed> = vec![];
        for election_input in &self.pipe_inputs.election_list {
            let mut reports = vec![];
            for contest_input in &election_input.contest_list {
                let contest_result =
                    self.read_contest_result(&election_input.id, &contest_input.id, None)?;

                let winners = self.read_winners(&election_input.id, &contest_input.id, None)?;

                reports.push(ReportData {
                    contest: contest_input.contest.clone(),
                    contest_result,
                    area_id: None,
                    winners,
                });
                for area in &contest_input.area_list {
                    let contest_result = self.read_contest_result(
                        &election_input.id,
                        &contest_input.id,
                        Some(&area.id),
                    )?;

                    let winners =
                        self.read_winners(&election_input.id, &contest_input.id, Some(&area.id))?;

                    reports.push(ReportData {
                        contest: contest_input.contest.clone(),
                        contest_result,
                        area_id: Some(area.id.to_string()),
                        winners,
                    });
                }
            }
            let computed_reports = self.compute_reports(reports)?;
            election_reports.push(ElectionReportDataComputed {
                election_id: election_input.id.clone().to_string(),
                area_id: None,
                reports: computed_reports,
            });
        }
        Ok(election_reports)
    }
}

impl Pipe for GenerateReports {
    #[instrument(skip_all)]
    fn exec(&self) -> Result<()> {
        for election_input in &self.pipe_inputs.election_list {
            let mut reports = vec![];
            for contest_input in &election_input.contest_list {
                let mut contest_result =
                    self.read_contest_result(&election_input.id, &contest_input.id, None)?;

                let defaults = default_invalid_votes();
                for (key, value) in defaults {
                    contest_result.invalid_votes.entry(key).or_insert(value);
                }

                let winners = self.read_winners(&election_input.id, &contest_input.id, None)?;

                reports.push(ReportData {
                    contest: contest_input.contest.clone(),
                    contest_result,
                    area_id: None,
                    winners,
                })
            }

            // FIXME: now we have multitple ballot styles
            let bytes = self.generate_report(&election_input.ballot_styles[0], reports)?;

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

fn default_invalid_votes() -> HashMap<InvalidVote, u64> {
    let mut map = HashMap::new();
    map.insert(InvalidVote::Implicit, 0);
    map.insert(InvalidVote::Explicit, 0);
    map.insert(InvalidVote::Blank, 0);
    map
}

#[derive(Debug)]
pub struct ReportData {
    pub contest: Contest,
    pub area_id: Option<String>,
    pub contest_result: ContestResult,
    pub winners: Vec<WinnerResult>,
}

#[derive(Debug, Serialize)]
pub struct ElectionReportDataComputed {
    pub election_id: String,
    pub area_id: Option<String>,
    pub reports: Vec<ReportDataComputed>,
}

#[derive(Debug, Serialize)]
pub struct ReportDataComputed {
    pub contest: Contest,
    pub area_id: Option<String>,
    pub contest_result: ContestResult,
    pub candidate_result: Vec<CandidateResultForReport>,
}

#[derive(Debug, Serialize)]
pub struct CandidateResultForReport {
    pub candidate: Candidate,
    pub total_count: u64,
    pub winning_position: Option<usize>,
}
