// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;
use sequent_core::{
    ballot::{Candidate, CandidatesOrder, Contest, StringifiedPeriodDates},
    serialization::deserialize_with_path::{deserialize_str, deserialize_value},
    services::{area_tree::TreeNodeArea, pdf, reports},
    types::to_map::ToMap,
    util::{date_time::get_date_and_time, path::list_subfolders},
};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};
use strand::hash::hash_b64;
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::{
    config::generate_reports::{
        CandidatesOrderPolicy, ContestReportConfig, PipeConfigGenerateReports,
        CONTEST_REPORT_CONFIG,
    },
    pipes::{
        do_tally::{
            list_tally_sheet_subfolders, CandidateResult, ContestResult, OUTPUT_BREAKDOWNS_FOLDER,
            OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER, OUTPUT_CONTEST_RESULT_FILE,
        },
        mark_winners::{WinnerResult, OUTPUT_WINNERS},
        pipe_inputs::{AreaConfig, InputAreaConfig, InputContestConfig, PipeInputs},
        pipe_name::PipeNameOutputDir,
        Pipe,
    },
};
use crate::{
    pipes::error::{Error, Result},
    utils::parse_file,
};

pub const OUTPUT_PDF: &str = "report.pdf";
pub const OUTPUT_HTML: &str = "report.html";
pub const OUTPUT_JSON: &str = "report.json";
pub const PARALLEL_CHUNK_SIZE: usize = 8;

#[derive(Debug)]
pub struct GenerateReports {
    pub pipe_inputs: PipeInputs,
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
}

pub struct GeneratedReportsBytes {
    bytes_pdf: Option<Vec<u8>>,
    bytes_html: Vec<u8>,
    bytes_json: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TemplateData {
    pub execution_annotations: HashMap<String, String>,
    pub reports: Vec<ReportDataComputed>,
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
    pub fn get_config(&self) -> Result<PipeConfigGenerateReports> {
        let pipe_config: PipeConfigGenerateReports = self
            .pipe_inputs
            .stage
            .pipe_config(self.pipe_inputs.stage.current_pipe)
            .and_then(|pc| pc.config)
            .map(|value| serde_json::from_value(value))
            .transpose()?
            .unwrap_or_default();
        Ok(pipe_config)
    }

    #[instrument(err, skip_all)]
    pub fn compute_reports(
        &self,
        reports: Vec<ReportData>,
        areas_map: &HashMap<String, TreeNodeArea>,
    ) -> Result<Vec<ReportDataComputed>> {
        let default_area_annotations: HashMap<String, String> =
            HashMap::from([("registered_voters".to_string(), "0".to_string())]);
        let mut reports = reports
            .iter()
            .map(|report| {
                let area_annotations: HashMap<String, String> = report
                    .area
                    .clone()
                    .map(|area| {
                        areas_map
                            .get(&area.id)
                            .cloned()
                            .map(|area| area.annotations)
                    })
                    .flatten()
                    .flatten()
                    .map(|annotations| deserialize_value::<HashMap<String, String>>(annotations))
                    .transpose()
                    .unwrap_or(Some(default_area_annotations.clone()))
                    .unwrap_or(default_area_annotations.clone());
                let map_winners: HashMap<_, _> = report
                    .winners
                    .iter()
                    .map(|winner_result| {
                        (
                            winner_result.candidate.id.clone(),
                            winner_result.winning_position,
                        )
                    })
                    .collect();

                // We will sort the candidates in contest_result by the same
                // criteria as in the ballot
                let mut contest_result = report.contest_result.clone();

                contest_result.contest.name = contest_result.contest.name.as_ref().map(|name| {
                    name.split('/')
                        .next()
                        .unwrap_or_default()
                        .trim()
                        .to_string()
                });

                sort_candidates(
                    &mut contest_result.candidate_result,
                    contest_result
                        .contest
                        .presentation
                        .clone()
                        .unwrap_or_default()
                        .candidates_order
                        .unwrap_or_default(),
                );

                // And we will sort the candidates in candidate_result by
                // winning position
                let mut candidate_result: Vec<CandidateResultForReport> = contest_result
                    .candidate_result
                    .iter()
                    .map(|candidate_result| CandidateResultForReport {
                        candidate: candidate_result.candidate.clone(),
                        total_count: candidate_result.total_count,
                        percentage_votes: candidate_result.percentage_votes,
                        winning_position: map_winners.get(&candidate_result.candidate.id).cloned(),
                    })
                    .collect();

                let contest_report_config: ContestReportConfig = report
                    .contest_result
                    .contest
                    .annotations
                    .clone()
                    .unwrap_or_default()
                    .get(CONTEST_REPORT_CONFIG)
                    .map(|contest_report_config| {
                        deserialize_str(contest_report_config)
                            .map_err(|err| {
                                warn!("Error deserializing contest_report_config: {err:?}")
                            })
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();

                match contest_report_config.candidates_order {
                    CandidatesOrderPolicy::AsInBallot => {}
                    CandidatesOrderPolicy::SortByWinningPosition => {
                        candidate_result.sort_by(|a, b| {
                            a.winning_position
                                .unwrap_or(usize::MAX)
                                .cmp(&b.winning_position.unwrap_or(usize::MAX))
                                .then_with(|| b.total_count.cmp(&a.total_count))
                                .then_with(|| a.candidate.name.cmp(&b.candidate.name))
                        });
                    }
                };

                ReportDataComputed {
                    election_name: report.election_name.clone(),
                    election_id: report.election_id.clone(),
                    election_description: report.election_description.clone(),
                    election_dates: report.election_dates.clone(),
                    election_annotations: report.election_annotations.clone(),
                    election_event_annotations: report.election_event_annotations.clone(),
                    contest: report.contest.clone(),
                    contest_result,
                    area: report.area.clone(),
                    area_annotations,
                    candidate_result,
                    is_aggregate: false,
                    tally_sheet_id: None,
                    channel_type: report.channel_type.clone(),
                }
            })
            .collect::<Vec<ReportDataComputed>>();

        reports.sort_by(|a, b| {
            b.contest_result
                .contest
                .name
                .cmp(&a.contest_result.contest.name)
        });

        Ok(reports)
    }

    #[instrument(err, skip_all)]
    pub fn generate_report(
        &self,
        reports: Vec<ReportData>,
        enable_pdfs: bool,
        election_hash: Option<String>,
        areas_map: &HashMap<String, TreeNodeArea>,
    ) -> Result<(GeneratedReportsBytes, String)> {
        let config = self.get_config()?;
        let mut execution_annotations = config.execution_annotations;

        let computed_reports = self.compute_reports(reports.clone(), areas_map)?;
        let template_data = TemplateData {
            execution_annotations: execution_annotations.clone(),
            reports: computed_reports.clone(),
        };

        let json_data = serde_json::to_value(template_data)?;
        let bytes_json = json_data.to_string().as_bytes().to_vec();

        // Hash the json results
        let results_hash = if let Some(election_hash) = election_hash {
            election_hash
        } else {
            hash_b64(&bytes_json).map_err(|err| {
                Error::UnexpectedError(format!("Error hashing the results file: {err:?}"))
            })?
        };

        // Insert the results_hash into the execution_annotations and re-render the template for both PDF and HTML
        execution_annotations.insert("results_hash".to_string(), results_hash.clone());
        let template_data = TemplateData {
            execution_annotations,
            reports: computed_reports,
        };

        let template_vars = template_data
            .clone()
            .to_map()
            // TODO: Fix neededing to do a Map Err
            .map_err(|err| Error::UnexpectedError(format!("serialization error: {err:?}")))?;

        let mut template_map = HashMap::new();
        let report_base_html = include_str!("../../resources/report_base_html.hbs");
        template_map.insert("report_base_html".to_string(), report_base_html.to_string());
        let report_base_pdf = include_str!("../../resources/report_base_pdf.hbs");
        template_map.insert("report_base_pdf".to_string(), report_base_pdf.to_string());
        let report_content = config
            .report_content_template
            .unwrap_or(include_str!("../../resources/report_content.hbs").to_string());
        template_map.insert("report_content".to_string(), report_content);

        let render_html_user = reports::render_template(
            "report_base_html",
            template_map.clone(),
            template_vars.clone(),
        )
        .map_err(|e| {
            Error::UnexpectedError(format!(
                "Error during render_template_text from report.hbs template file: {}",
                e
            ))
        })?;

        let mut template_system_vars = Map::new();
        template_system_vars.insert(
            "rendered_user_template".to_string(),
            serde_json::to_value(&render_html_user)?,
        );

        if let serde_json::Value::Object(obj) = &config.extra_data {
            for (key, value) in obj {
                template_system_vars.insert(key.clone(), value.clone());
            }
        }

        let render_html =
            reports::render_template_text(&config.system_template, template_system_vars.clone())
                .map_err(|e| {
                    Error::UnexpectedError(format!(
                        "Error during render_template_text from report.hbs template file: {}",
                        e
                    ))
                })?;

        let bytes_pdf = if enable_pdfs {
            let render_pdf_user: String =
                reports::render_template("report_base_pdf", template_map, template_vars.clone())
                    .map_err(|e| {
                        Error::UnexpectedError(format!(
                            "Error during render_template_text from report.hbs template file: {}",
                            e
                        ))
                    })?;

            template_system_vars.insert(
                "rendered_user_template".to_string(),
                serde_json::to_value(&render_pdf_user)?,
            );

            let render_pdf =
                reports::render_template_text(&config.system_template, template_system_vars)
                    .map_err(|e| {
                        Error::UnexpectedError(format!(
                            "Error during render_template_text from report.hbs template file: {}",
                            e
                        ))
                    })?;

            let pdf_options = config
                .pdf_options
                .map(|val| Some(val.to_print_to_pdf_options()))
                .unwrap_or_default();

            let rt = tokio::runtime::Runtime::new().unwrap();
            let bytes_pdf =
                pdf::sync::PdfRenderer::render_pdf(render_pdf, pdf_options).map_err(|e| {
                    Error::UnexpectedError(format!("Error during html_to_pdf conversion: {}", e))
                })?;

            Some(bytes_pdf)
        } else {
            None
        };

        let generated_report_bytes = GeneratedReportsBytes {
            bytes_pdf: bytes_pdf,
            bytes_html: render_html.as_bytes().to_vec(),
            bytes_json: bytes_json,
        };

        Ok((generated_report_bytes, results_hash))
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

    #[instrument(err, skip(self))]
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

    #[instrument(err, skip(self))]
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

        let file = fs::File::open(&path).map_err(|error| Error::FileAccess(path.clone(), error))?;

        let res: Vec<WinnerResult> = parse_file(file)?;

        Ok(res)
    }

    #[instrument(err, skip(self))]
    pub fn read_reports(&self) -> Result<Vec<ElectionReportDataComputed>> {
        let mut election_reports: Vec<ElectionReportDataComputed> = vec![];

        for election_input in &self.pipe_inputs.election_list {
            let mut reports = vec![];
            let areas_map: HashMap<String, TreeNodeArea> = election_input
                .areas
                .iter()
                .map(|area| (area.id.clone(), area.clone()))
                .collect();
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
                    election_name: election_input.alias.clone(),
                    election_id: election_input.id.to_string(),
                    election_description: election_input.description.clone(),
                    election_dates: election_input.dates.clone(),
                    election_annotations: election_input.annotations.clone(),
                    election_event_annotations: election_input.election_event_annotations.clone(),
                    contest: contest_input.contest.clone(),
                    contest_result,
                    area: None,
                    winners,
                    channel_type: None,
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
                        election_id: election_input.id.to_string(),
                        election_description: election_input.description.clone(),
                        election_dates: election_input.dates.clone(),
                        election_annotations: election_input.annotations.clone(),
                        election_event_annotations: election_input
                            .election_event_annotations
                            .clone(),
                        contest: contest_input.contest.clone(),
                        contest_result,
                        area: Some(BasicArea {
                            id: area.id.to_string(),
                            name: area.area.name.clone(),
                        }),
                        winners,
                        channel_type: None,
                    });
                }
            }

            let computed_reports = self.compute_reports(reports, &areas_map)?;

            election_reports.push(ElectionReportDataComputed {
                election_id: election_input.id.clone().to_string(),
                area: None,
                census: election_input.census,
                total_votes: election_input.total_votes,
                reports: computed_reports,
            });
        }
        Ok(election_reports)
    }

    #[instrument(err, skip_all)]
    fn read_breakdowns(
        &self,
        election_id: &Uuid,
        election_name: &str,
        election_description: &str,
        election_dates: &Option<StringifiedPeriodDates>,
        election_annotations: &HashMap<String, String>,
        election_event_annotations: &HashMap<String, String>,
        contest_id: Option<&Uuid>,
        contest: &Contest,
        area_id: Option<&Uuid>,
        area: &Option<BasicArea>,
        is_aggregate: bool,
        tally_sheet_id: Option<String>,
    ) -> Result<Vec<ReportData>> {
        // read contest results
        let mut contest_base_path = PipeInputs::build_path(
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
        let mut winners_base_path = PipeInputs::build_path(
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
            contest_base_path =
                PipeInputs::build_tally_sheet_path(&contest_base_path, &tally_sheet);
            winners_base_path =
                PipeInputs::build_tally_sheet_path(&winners_base_path, &tally_sheet);
        }

        if is_aggregate {
            contest_base_path =
                contest_base_path.join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);
            winners_base_path =
                winners_base_path.join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);
        }
        let contest_base_breakdown_path = contest_base_path.join(OUTPUT_BREAKDOWNS_FOLDER);
        let winners_base_breakdown_path = winners_base_path.join(OUTPUT_BREAKDOWNS_FOLDER);

        if !contest_base_breakdown_path.exists()
            || !contest_base_breakdown_path.is_dir()
            || !winners_base_breakdown_path.exists()
            || !winners_base_breakdown_path.is_dir()
        {
            return Ok(vec![]);
        }
        let contest_subfolders = list_subfolders(&contest_base_breakdown_path);

        let mut reports: Vec<ReportData> = vec![];

        for subfolder in contest_subfolders {
            let contest_results_file_path = subfolder.join(OUTPUT_CONTEST_RESULT_FILE);
            let contest_results_file = fs::File::open(&contest_results_file_path)
                .map_err(|e| Error::FileAccess(contest_results_file_path.clone(), e))?;
            let contest_result: ContestResult = parse_file(contest_results_file)?;

            let subfolder_name = subfolder.file_name().unwrap();
            let winners_subfolder = winners_base_breakdown_path.join(subfolder_name);
            let winners_file_path = winners_subfolder.join(OUTPUT_WINNERS);
            let winners_file = fs::File::open(&winners_file_path)
                .map_err(|e| Error::FileAccess(winners_file_path.clone(), e))?;
            let winners: Vec<WinnerResult> = parse_file(winners_file)?;

            let report = ReportData {
                election_name: election_name.to_string(),
                election_id: election_id.to_string(),
                election_description: election_description.to_string(),
                election_dates: election_dates.clone(),
                election_annotations: election_annotations.clone(),
                election_event_annotations: election_event_annotations.clone(),
                contest: contest.clone(),
                contest_result,
                area: area.clone(),
                winners,
                channel_type: Some(subfolder_name.to_string_lossy().into_owned()),
            };
            reports.push(report);
        }
        Ok(reports)
    }

    #[instrument(
        skip(
            self,
            contest,
            election_annotations,
            election_event_annotations,
            areas_map
        ),
        err
    )]
    #[instrument(err, skip_all)]
    fn make_report(
        &self,
        election_id: &Uuid,
        election_name: &str,
        election_description: &str,
        election_dates: &Option<StringifiedPeriodDates>,
        election_annotations: &HashMap<String, String>,
        election_event_annotations: &HashMap<String, String>,
        contest_id: Option<&Uuid>,
        area: Option<BasicArea>,
        contest: Contest,
        is_aggregate: bool,
        tally_sheet_id: Option<String>,
        enable_pdfs: bool,
        is_write: bool,
        election_hash: Option<String>,
        areas_map: &HashMap<String, TreeNodeArea>,
    ) -> Result<ReportData> {
        let area_id = area
            .clone()
            .map(|value| Uuid::parse_str(&value.id))
            .transpose()
            .map_err(|err| Error::UnexpectedError(format!("{}", err)))?;
        let contest_result = self.read_contest_result(
            election_id,
            contest_id,
            area_id.as_ref(),
            is_aggregate,
            tally_sheet_id.clone(),
        )?;

        let winners = self.read_winners(
            election_id,
            contest_id,
            area_id.as_ref(),
            is_aggregate,
            tally_sheet_id.clone(),
        )?;

        let breakdowns = self.read_breakdowns(
            election_id,
            election_name,
            election_description,
            election_dates,
            &election_annotations,
            &election_event_annotations,
            contest_id,
            &contest,
            area_id.as_ref(),
            &area,
            is_aggregate,
            tally_sheet_id.clone(),
        )?;

        let report = ReportData {
            election_name: election_name.to_string(),
            election_id: election_id.to_string(),
            election_description: election_description.to_string(),
            election_dates: election_dates.clone(),
            election_annotations: election_annotations.clone(),
            election_event_annotations: election_event_annotations.clone(),
            contest,
            contest_result,
            area: area.clone(),
            winners,
            channel_type: None,
        };

        let mut combined: Vec<ReportData> = Vec::new();
        combined.push(report.clone());
        combined.extend(breakdowns);

        if is_write {
            self.write_report(
                election_id,
                contest_id,
                area_id.as_ref(),
                combined,
                is_aggregate,
                tally_sheet_id.clone(),
                enable_pdfs,
                false,
                election_hash,
                areas_map,
            )?;
        }

        Ok(report)
    }

    #[instrument(err, skip(self, reports, areas_map), err)]
    fn write_report(
        &self,
        election_id: &Uuid,
        contest_id: Option<&Uuid>,
        area_id: Option<&Uuid>,
        reports: Vec<ReportData>,
        is_aggregate: bool,
        tally_sheet_id: Option<String>,
        enable_pdfs: bool,
        area_based: bool,
        election_hash: Option<String>,
        areas_map: &HashMap<String, TreeNodeArea>,
    ) -> Result<String> {
        let (reports, result_hash) =
            self.generate_report(reports, enable_pdfs, election_hash, areas_map)?;

        let mut base_path = match area_based {
            true => {
                PipeInputs::build_path_by_area(&self.output_dir, election_id, contest_id, area_id)
            }
            false => PipeInputs::build_path(&self.output_dir, election_id, contest_id, area_id),
        };

        if let Some(tally_sheet) = tally_sheet_id.clone() {
            base_path = PipeInputs::build_tally_sheet_path(&base_path, &tally_sheet);
        }

        if is_aggregate {
            base_path = base_path.join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);
        }

        fs::create_dir_all(&base_path)?;

        if let Some(bytes_pdf) = reports.bytes_pdf.clone() {
            let pdf_path = base_path.join(OUTPUT_PDF);
            let mut pdf_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(pdf_path)?;
            pdf_file.write_all(&bytes_pdf)?;
        };

        let html_path = base_path.join(OUTPUT_HTML);
        let mut html_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(html_path)?;
        html_file.write_all(&reports.bytes_html)?;

        let json_path = base_path.join(OUTPUT_JSON);
        let mut json_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(json_path)?;
        json_file.write_all(&reports.bytes_json)?;

        Ok(result_hash)
    }
}

#[derive(Debug, Clone)]
struct InputConfigAreaContest<'a> {
    area: &'a InputAreaConfig,
    contests: Vec<&'a InputContestConfig>,
}

impl Pipe for GenerateReports {
    #[instrument(err, skip_all, name = "GenerateReports::exec")]
    fn exec(&self) -> Result<()> {
        let mark_winners_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::MarkWinners.as_ref());

        let config = self.get_config()?; // Assuming config is shareable (Sync+Clone) or created/cloned per thread if needed

        // 1. Parallelize processing of each election_input
        self.pipe_inputs
            .election_list
            .par_iter() // <- PARALLELIZED
            .try_for_each(|election_input| -> Result<()> {
                // Added Result<()> for try_for_each
                let areas_map: HashMap<String, TreeNodeArea> = election_input
                    .areas
                    .iter()
                    .map(|area| (area.id.clone(), area.clone()))
                    .collect();

                // 2. Parallelize processing of contest_list for generating initial contest_reports
                let contest_reports: Vec<ReportData> = election_input // Ensure ReportData is Send
                    .contest_list
                    .par_iter() // <- PARALLELIZED
                    .map(|contest_input| {
                        // OPTIMIZATION/CLARIFICATION for inner area processing:
                        // The original code collected ReportData from area_list_chunk.par_iter().map()
                        // but then discarded the Vec<ReportData>. If the goal is side-effects
                        // (calling self.make_report) and error propagation, try_for_each is clearer.
                        // If ReportData from these calls IS needed, this part needs to be reworked
                        // to properly collect all generated ReportData items.

                        // Assuming make_report is primarily for side-effects (e.g., writing files)
                        // and its Result is for error checking.
                        contest_input.area_list.par_iter().try_for_each(
                            |area_input| -> Result<()> {
                                let base_tally_sheet_path = PipeInputs::build_path(
                                    &mark_winners_dir,
                                    &area_input.election_id,
                                    Some(&area_input.contest_id),
                                    Some(&area_input.id),
                                );
                                let tally_sheet_paths =
                                    list_tally_sheet_subfolders(&base_tally_sheet_path);
                                let tally_sheet_ids = tally_sheet_paths
                                    .iter()
                                    .map(|tally_sheet_path| -> Result<String> {
                                        PipeInputs::get_tally_sheet_id_from_path(&tally_sheet_path)
                                            .ok_or(Error::UnexpectedError(
                                                "Can't read tally sheet id from path".into(),
                                            ))
                                    })
                                    .collect::<Result<Vec<String>>>()?;

                                if !tally_sheet_ids.is_empty() {
                                    // Changed from tally_sheet_ids.len() > 0
                                    for tally_sheet_id in tally_sheet_ids {
                                        self.make_report(
                                            &election_input.id,
                                            &election_input.name,
                                            &election_input.description,
                                            &election_input.dates,
                                            &election_input.annotations,
                                            &election_input.election_event_annotations,
                                            Some(&contest_input.id),
                                            Some(area_input.area.clone().into()),
                                            contest_input.contest.clone(),
                                            false,
                                            Some(tally_sheet_id), // Note: This was &tally_sheet_id, ensure make_report expects String or &str
                                            config.enable_pdfs,
                                            true,
                                            None,
                                            &areas_map,
                                        )?; // Propagate error
                                    }
                                }

                                let has_aggregate = self.has_aggregate(
                                    &election_input.id,
                                    Some(&contest_input.id),
                                    &area_input.id,
                                );
                                if has_aggregate {
                                    self.make_report(
                                        &election_input.id,
                                        &election_input.name,
                                        &election_input.description,
                                        &election_input.dates,
                                        &election_input.annotations,
                                        &election_input.election_event_annotations,
                                        Some(&contest_input.id),
                                        Some(area_input.area.clone().into()),
                                        contest_input.contest.clone(),
                                        true,
                                        None,
                                        config.enable_pdfs,
                                        true,
                                        None,
                                        &areas_map,
                                    )?; // Propagate error
                                }
                                self.make_report(
                                    &election_input.id,
                                    &election_input.name,
                                    &election_input.description,
                                    &election_input.dates,
                                    &election_input.annotations,
                                    &election_input.election_event_annotations,
                                    Some(&contest_input.id),
                                    Some(area_input.area.clone().into()),
                                    contest_input.contest.clone(),
                                    false,
                                    None,
                                    config.enable_pdfs,
                                    true,
                                    None,
                                    &areas_map,
                                )?; // Propagate error (Original code implicitly returned this)
                                Ok(())
                            },
                        )?; // End of par_iter().try_for_each over area_list for a contest_input

                        // This report is for the contest itself, after its areas are processed
                        let contest_report = self.make_report(
                            &election_input.id,
                            &election_input.name,
                            &election_input.description,
                            &election_input.dates,
                            &election_input.annotations,
                            &election_input.election_event_annotations,
                            Some(&contest_input.id),
                            None,
                            contest_input.contest.clone(),
                            false,
                            None,
                            config.enable_pdfs,
                            true,
                            None,
                            &areas_map,
                        )?;
                        Ok(contest_report)
                    })
                    .collect::<Result<Vec<ReportData>>>()?; // Collect contest-level reports

                // write report for the current election (remains sequential for this election_input task)
                let result_hash = self.write_report(
                    &election_input.id,
                    None,
                    None,
                    contest_reports, // Now correctly using the collected contest_reports
                    false,
                    None,
                    config.enable_pdfs,
                    false,
                    None,
                    &areas_map,
                )?;

                // make area reports with all contests related to each area
                // Construction of area_contests_map remains sequential for simplicity here.
                // If election_input.contest_list is very large, this part could also be a candidate
                // for more complex parallel collection if it becomes a bottleneck.
                let mut area_contests_map: HashMap<String, InputConfigAreaContest> = HashMap::new();
                election_input.contest_list.iter().for_each(|contest| {
                    contest.area_list.iter().for_each(|area| {
                        area_contests_map
                            .entry(area.id.to_string())
                            .and_modify(|entry| entry.contests.push(contest)) // Ensure contest is cloneable or references are fine
                            .or_insert_with(|| InputConfigAreaContest {
                                area: area, // Ensure lifetime of area is suitable or it's cloned
                                contests: vec![contest],
                            });
                    });
                });

                // 3. Parallelize processing of each area in area_contests_map
                area_contests_map
                    .par_iter() // <- PARALLELIZED iteration over areas
                    .try_for_each(|(_area_id, area_contests)| -> Result<()> {
                        // area_id is from map key
                        let matching_area_contests = area_contests.contests.clone(); // Clone if needed for parallel tasks
                        let area_config: &InputAreaConfig = area_contests.area; // area_config is a more descriptive name

                        // The chunking logic here is kept, but you could also flatten and par_iter directly
                        // if intermediate Vec allocation is not an issue.
                        // This collects reports for all contests related to the current area.
                        let contests_report: Vec<ReportData> = matching_area_contests
                            .chunks(PARALLEL_CHUNK_SIZE) // This chunks the contests for the current area
                            .map(|contest_list_chunk| {
                                contest_list_chunk
                                    .par_iter() // Parallel processing of contests within this chunk for this area
                                    .map(|contest_input| {
                                        self.make_report(
                                            &election_input.id,
                                            &election_input.name,
                                            &election_input.description,
                                            &election_input.dates,
                                            &election_input.annotations,
                                            &election_input.election_event_annotations,
                                            Some(&contest_input.id),
                                            Some(area_config.area.clone().into()),
                                            contest_input.contest.clone(),
                                            false,
                                            None,
                                            config.enable_pdfs,
                                            false,
                                            Some(result_hash.clone()),
                                            &areas_map,
                                        )
                                    })
                                    .collect::<Result<Vec<ReportData>>>() // Collect ReportData for this chunk
                            })
                            .collect::<Result<Vec<Vec<ReportData>>>>()? // Collect results from all chunks
                            .into_iter()
                            .flatten() // Flatten Vec<Vec<ReportData>> to Vec<ReportData>
                            .collect();

                        self.write_report(
                            &election_input.id,
                            None,
                            Some(&area_config.id), // Use area_config.id
                            contests_report,
                            false,
                            None,
                            config.enable_pdfs,
                            true,
                            Some(result_hash.clone()),
                            &areas_map,
                        )?;
                        Ok(())
                    })?; // End of par_iter().try_for_each over area_contests_map

                Ok(())
            }) // End of par_iter().try_for_each over election_list
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicArea {
    pub id: String,
    pub name: String,
}

impl From<AreaConfig> for BasicArea {
    fn from(item: AreaConfig) -> Self {
        BasicArea {
            id: item.id.to_string(),
            name: item.name.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReportData {
    pub election_name: String,
    pub election_id: String,
    pub election_description: String,
    pub election_dates: Option<StringifiedPeriodDates>,
    pub election_annotations: HashMap<String, String>,
    pub election_event_annotations: HashMap<String, String>,
    pub contest: Contest,
    pub area: Option<BasicArea>,
    pub contest_result: ContestResult,
    pub winners: Vec<WinnerResult>,
    pub channel_type: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ElectionReportDataComputed {
    pub election_id: String,
    pub area: Option<BasicArea>,
    pub census: u64,
    pub total_votes: u64,
    pub reports: Vec<ReportDataComputed>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportDataComputed {
    pub election_name: String,
    pub election_id: String,
    pub election_description: String,
    pub election_dates: Option<StringifiedPeriodDates>,
    pub election_annotations: HashMap<String, String>,
    pub election_event_annotations: HashMap<String, String>,
    pub contest: Contest,
    pub area: Option<BasicArea>,
    pub area_annotations: HashMap<String, String>,
    pub is_aggregate: bool,
    pub tally_sheet_id: Option<String>,
    pub contest_result: ContestResult,
    pub candidate_result: Vec<CandidateResultForReport>,
    pub channel_type: Option<String>,
}

impl From<ReportDataComputed> for ReportData {
    fn from(item: ReportDataComputed) -> Self {
        ReportData {
            election_name: item.election_name.clone(),
            election_id: item.election_id.clone(),
            election_description: item.election_description.clone(),
            election_dates: item.election_dates.clone(),
            election_annotations: item.election_annotations.clone(),
            election_event_annotations: item.election_event_annotations.clone(),
            contest: item.contest.clone(),
            area: item.area.clone(),
            contest_result: item.contest_result.clone(),
            winners: item
                .candidate_result
                .into_iter()
                .filter_map(|winner| winner.into())
                .collect(),
            channel_type: item.channel_type.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CandidateResultForReport {
    pub candidate: Candidate,
    pub total_count: u64,
    pub percentage_votes: f64,
    pub winning_position: Option<usize>,
}

impl From<CandidateResultForReport> for Option<WinnerResult> {
    fn from(item: CandidateResultForReport) -> Self {
        let Some(winning_position) = item.winning_position.clone() else {
            return None;
        };
        Some(WinnerResult {
            candidate: item.candidate,
            total_count: item.total_count,
            winning_position,
        })
    }
}

#[instrument(skip_all)]
fn sort_candidates(candidates: &mut Vec<CandidateResult>, order_field: CandidatesOrder) {
    match order_field {
        CandidatesOrder::Alphabetical => {
            candidates.sort_by(|a, b| {
                let name_a = a
                    .candidate
                    .alias
                    .as_ref()
                    .or(a.candidate.name.as_ref())
                    .unwrap_or(&String::new())
                    .to_lowercase();
                let name_b = b
                    .candidate
                    .alias
                    .as_ref()
                    .or(b.candidate.name.as_ref())
                    .unwrap_or(&String::new())
                    .to_lowercase();
                name_a.cmp(&name_b)
            });
        }
        CandidatesOrder::Custom => {
            candidates.sort_by(|a, b| {
                let sort_order_a = a
                    .candidate
                    .presentation
                    .as_ref()
                    .and_then(|p| p.sort_order)
                    .unwrap_or(-1);
                let sort_order_b = b
                    .candidate
                    .presentation
                    .as_ref()
                    .and_then(|p| p.sort_order)
                    .unwrap_or(-1);
                sort_order_a.cmp(&sort_order_b)
            });
        }

        CandidatesOrder::Random => {
            // We don't randomize in results
        }
    }
}
