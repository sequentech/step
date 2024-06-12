// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use sequent_core::{
    ballot::{Candidate, Contest},
    services::{pdf, reports},
    types::tally_sheets,
};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use tracing::instrument;
use uuid::Uuid;

use crate::pipes::{
    do_tally::{
        list_tally_sheet_subfolders, ContestResult,
        OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER, OUTPUT_CONTEST_RESULT_FILE,
    },
    mark_winners::{WinnerResult, OUTPUT_WINNERS},
    pipe_inputs::PipeInputs,
    pipe_name::PipeNameOutputDir,
    Pipe,
};
use crate::{
    pipes::error::{Error, Result},
    utils::parse_file,
};

pub const OUTPUT_PDF: &str = "report.pdf";
pub const OUTPUT_RECEIPT_PDF: &str = "vote_receipts.pdf";
pub const OUTPUT_HTML: &str = "report.html";
pub const OUTPUT_JSON: &str = "report.json";
pub const PARALLEL_CHUNK_SIZE: usize = 8;

pub struct GenerateReports {
    pub pipe_inputs: PipeInputs,
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
}

impl GenerateReports {
    #[instrument(skip_all, name = "GenerateReports::new")]
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
                        percentage_votes: cr.percentage_votes,
                        winning_position: map_winners.get(&cr.candidate.id).cloned(),
                    })
                    .collect();

                candidate_result.sort_by(|a, b| {
                    a.winning_position
                        .unwrap_or(usize::MAX)
                        .cmp(&b.winning_position.unwrap_or(usize::MAX))
                        .then_with(|| b.total_count.cmp(&a.total_count))
                        .then_with(|| a.candidate.name.cmp(&b.candidate.name))
                });

                ReportDataComputed {
                    election_name: r.election_name.clone(),
                    contest: r.contest.clone(),
                    contest_result: r.contest_result.clone(),
                    area_id: r.area_id.clone(),
                    candidate_result,
                    is_aggregate: false,
                    tally_sheet_id: None,
                }
            })
            .collect::<Vec<ReportDataComputed>>();

        Ok(reports)
    }

    #[instrument(skip_all)]
    pub fn generate_report(&self, reports: Vec<ReportData>) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        let reports = self.compute_reports(reports)?;
        let reports = serde_json::to_value(reports)?;

        let mut map = Map::new();

        map.insert("reports".to_owned(), reports.clone());

        let mut template_map = HashMap::new();
        let html = include_str!("../../resources/report_base_html.hbs");
        template_map.insert("report_base_html".to_string(), html.to_string());
        let html = include_str!("../../resources/report_base_pdf.hbs");
        template_map.insert("report_base_pdf".to_string(), html.to_string());
        let html = include_str!("../../resources/report_content.hbs");
        template_map.insert("report_content".to_string(), html.to_string());

        let render_html =
            reports::render_template("report_base_html", template_map.clone(), map.clone())
                .map_err(|e| {
                    Error::UnexpectedError(format!(
                        "Error during render_template_text from report.hbs template file: {}",
                        e
                    ))
                })?;

        let render_pdf =
            reports::render_template("report_base_pdf", template_map, map).map_err(|e| {
                Error::UnexpectedError(format!(
                    "Error during render_template_text from report.hbs template file: {}",
                    e
                ))
            })?;

        let bytes_pdf = pdf::html_to_pdf(render_pdf.clone()).map_err(|e| {
            Error::UnexpectedError(format!("Error during html_to_pdf conversion: {}", e))
        })?;

        Ok((
            bytes_pdf,
            render_html.as_bytes().to_vec(),
            reports.to_string().as_bytes().to_vec(),
        ))
    }

    #[instrument(skip(self))]
    pub fn has_aggregate(
        &self,
        election_id: &Uuid,
        contest_id: Option<&Uuid>,
        area_id: &Uuid,
    ) -> bool {
        let base_path = PipeInputs::build_path(
            &self
                .pipe_inputs
                .cli
                .output_dir
                .as_path()
                .join(PipeNameOutputDir::DoTally.as_ref()),
            election_id,
            contest_id,
            Some(area_id.clone()).as_ref(),
        );
        let aggregate_path = base_path.join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);
        aggregate_path.exists() && aggregate_path.is_dir()
    }

    #[instrument(skip(self))]
    fn read_contest_result(
        &self,
        election_id: &Uuid,
        contest_id: Option<&Uuid>,
        area_id: Option<&Uuid>,
        is_aggregate: bool,
        tally_sheet_id: Option<String>,
    ) -> Result<ContestResult> {
        let mut base_path = PipeInputs::build_path(
            &self
                .pipe_inputs
                .cli
                .output_dir
                .as_path()
                .join(PipeNameOutputDir::DoTally.as_ref()),
            election_id,
            contest_id,
            area_id,
        );
        if let Some(tally_sheet) = tally_sheet_id.clone() {
            base_path = PipeInputs::build_tally_sheet_path(&base_path, &tally_sheet);
        }

        if is_aggregate {
            base_path = base_path.join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);
        }

        let path = base_path.join(OUTPUT_CONTEST_RESULT_FILE);

        let file = fs::File::open(&path).map_err(|e| Error::FileAccess(path.clone(), e))?;

        let contest_result: ContestResult = parse_file(file)?;

        Ok(contest_result)
    }

    #[instrument(skip(self))]
    fn read_winners(
        &self,
        election_id: &Uuid,
        contest_id: Option<&Uuid>,
        area_id: Option<&Uuid>,
        is_aggregate: bool,
        tally_sheet_id: Option<String>,
    ) -> Result<Vec<WinnerResult>> {
        let mut base_path = PipeInputs::build_path(
            &self
                .pipe_inputs
                .cli
                .output_dir
                .as_path()
                .join(PipeNameOutputDir::MarkWinners.as_ref()),
            election_id,
            contest_id,
            area_id,
        );

        if let Some(tally_sheet) = tally_sheet_id.clone() {
            base_path = PipeInputs::build_tally_sheet_path(&base_path, &tally_sheet);
        }

        if is_aggregate {
            base_path = base_path.join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);
        }

        let path = base_path.join(OUTPUT_WINNERS);

        let f = fs::File::open(&path).map_err(|e| Error::FileAccess(path.clone(), e))?;

        let res: Vec<WinnerResult> = parse_file(f)?;

        Ok(res)
    }

    #[instrument(skip(self))]
    pub fn read_reports(&self) -> Result<Vec<ElectionReportDataComputed>> {
        let mut election_reports: Vec<ElectionReportDataComputed> = vec![];

        for election_input in &self.pipe_inputs.election_list {
            let mut reports = vec![];
            for contest_input in &election_input.contest_list {
                let contest_result = self.read_contest_result(
                    &election_input.id,
                    Some(&contest_input.id),
                    None,
                    false,
                    None,
                )?;

                let winners = self.read_winners(
                    &election_input.id,
                    Some(&contest_input.id),
                    None,
                    false,
                    None,
                )?;

                reports.push(ReportData {
                    election_name: election_input.name.clone(),
                    contest: contest_input.contest.clone(),
                    contest_result,
                    area_id: None,
                    winners,
                });

                for area in &contest_input.area_list {
                    let contest_result = self.read_contest_result(
                        &election_input.id,
                        Some(&contest_input.id),
                        Some(&area.id),
                        false,
                        None,
                    )?;

                    let winners = self.read_winners(
                        &election_input.id,
                        Some(&contest_input.id),
                        Some(&area.id),
                        false,
                        None,
                    )?;

                    reports.push(ReportData {
                        election_name: election_input.name.clone(),
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
                census: election_input.census,
                total_votes: election_input.total_votes,
                reports: computed_reports,
            });
        }
        Ok(election_reports)
    }

    fn make_report(
        &self,
        election_id: &Uuid,
        election_name: &str,
        contest_id: Option<&Uuid>,
        area_id: Option<&Uuid>,
        contest: Contest,
        is_aggregate: bool,
        tally_sheet_id: Option<String>,
    ) -> Result<ReportData> {
        let contest_result = self.read_contest_result(
            election_id,
            contest_id,
            area_id,
            is_aggregate,
            tally_sheet_id.clone(),
        )?;

        let winners = self.read_winners(
            election_id,
            contest_id,
            area_id,
            is_aggregate,
            tally_sheet_id.clone(),
        )?;

        let report = ReportData {
            election_name: election_name.to_string(),
            contest,
            contest_result,
            area_id: None,
            winners,
        };

        self.write_report(
            election_id,
            contest_id,
            area_id,
            vec![report.clone()],
            is_aggregate,
            tally_sheet_id.clone(),
        )?;

        Ok(report)
    }

    fn write_report(
        &self,
        election_id: &Uuid,
        contest_id: Option<&Uuid>,
        area_id: Option<&Uuid>,
        reports: Vec<ReportData>,
        is_aggregate: bool,
        tally_sheet_id: Option<String>,
    ) -> Result<()> {
        let (bytes_pdf, bytes_html, bytes_json) = self.generate_report(reports)?;

        let mut base_path =
            PipeInputs::build_path(&self.output_dir, election_id, contest_id, area_id);

        if let Some(tally_sheet) = tally_sheet_id.clone() {
            base_path = PipeInputs::build_tally_sheet_path(&base_path, &tally_sheet);
        }

        if is_aggregate {
            base_path = base_path.join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);
        }

        fs::create_dir_all(&base_path)?;

        let pdf_path = base_path.join(OUTPUT_PDF);
        let mut pdf_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(pdf_path)?;
        pdf_file.write_all(&bytes_pdf)?;

        let html_path = base_path.join(OUTPUT_HTML);
        let mut html_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(html_path)?;
        html_file.write_all(&bytes_html)?;

        let json_path = base_path.join(OUTPUT_JSON);
        let mut json_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(json_path)?;
        json_file.write_all(&bytes_json)?;

        Ok(())
    }
}

impl Pipe for GenerateReports {
    #[instrument(skip_all, name = "GenerateReports::exec")]
    fn exec(&self) -> Result<()> {
        let mark_winners_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::MarkWinners.as_ref());

        self.pipe_inputs
            .election_list
            .iter()
            .try_for_each(|election_input| {
                let contest_reports: Result<Vec<_>> = election_input
                    .contest_list
                    .iter()
                    .map(|contest_input| {
                        let chunks = contest_input
                            .area_list
                            .chunks(PARALLEL_CHUNK_SIZE)
                            .enumerate();
                        for (index, area_list_chunk) in chunks {
                            area_list_chunk
                                .par_iter()
                                .map(|area_input| -> Result<ReportData> {
                                    // process tally sheets
                                    let base_tally_sheet_path = PipeInputs::build_path(
                                        &mark_winners_dir,
                                        &area_input.election_id,
                                        Some(&area_input.contest_id),
                                        Some(&area_input.id),
                                    );
                                    let tally_sheet_paths =
                                        list_tally_sheet_subfolders(&base_tally_sheet_path);
                                    let tally_sheet_ids =
                                        tally_sheet_paths
                                            .iter()
                                            .map(|tally_sheet_path| -> Result<String> {
                                                PipeInputs::get_tally_sheet_id_from_path(
                                                    &tally_sheet_path,
                                                )
                                                .ok_or(Error::UnexpectedError(
                                                    "Can't read tally sheet id from path".into(),
                                                ))
                                            })
                                            .collect::<Result<Vec<String>>>()?;
                                    if tally_sheet_ids.len() > 0 {
                                        for tally_sheet_id in tally_sheet_ids {
                                            self.make_report(
                                                &election_input.id,
                                                &election_input.name,
                                                Some(&contest_input.id),
                                                Some(&area_input.id),
                                                contest_input.contest.clone(),
                                                true,
                                                Some(tally_sheet_id),
                                            )?;
                                        }
                                    }

                                    // area aggregates if it has children
                                    let has_aggregate = self.has_aggregate(
                                        &election_input.id,
                                        Some(&contest_input.id),
                                        &area_input.id,
                                    );
                                    if has_aggregate {
                                        self.make_report(
                                            &election_input.id,
                                            &election_input.name,
                                            Some(&contest_input.id),
                                            Some(&area_input.id),
                                            contest_input.contest.clone(),
                                            true,
                                            None,
                                        )?;
                                    }
                                    self.make_report(
                                        &election_input.id,
                                        &election_input.name,
                                        Some(&contest_input.id),
                                        Some(&area_input.id),
                                        contest_input.contest.clone(),
                                        false,
                                        None,
                                    )
                                })
                                .collect::<Result<Vec<ReportData>>>()?;
                        }

                        let contest_report = self.make_report(
                            &election_input.id,
                            &election_input.name,
                            Some(&contest_input.id),
                            None,
                            contest_input.contest.clone(),
                            false,
                            None,
                        )?;

                        Ok(contest_report)
                    })
                    .collect();

                // write report for the current election
                self.write_report(
                    &election_input.id,
                    None,
                    None,
                    contest_reports?,
                    false,
                    None,
                )?;

                Ok(())
            })
    }
}

#[derive(Debug, Clone)]
pub struct ReportData {
    pub election_name: String,
    pub contest: Contest,
    pub area_id: Option<String>,
    pub contest_result: ContestResult,
    pub winners: Vec<WinnerResult>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ElectionReportDataComputed {
    pub election_id: String,
    pub area_id: Option<String>,
    pub census: u64,
    pub total_votes: u64,
    pub reports: Vec<ReportDataComputed>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportDataComputed {
    pub election_name: String,
    pub contest: Contest,
    pub area_id: Option<String>,
    pub is_aggregate: bool,
    pub tally_sheet_id: Option<String>,
    pub contest_result: ContestResult,
    pub candidate_result: Vec<CandidateResultForReport>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CandidateResultForReport {
    pub candidate: Candidate,
    pub total_count: u64,
    pub percentage_votes: f64,
    pub winning_position: Option<usize>,
}
