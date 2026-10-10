// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::ballot_images_config::PipeConfigBallotImages;
use crate::pipes::decode_ballots::OUTPUT_DECODED_BALLOTS_FILE;
use crate::pipes::do_tally::tally::Tally;
use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{InputElectionConfig, PipeInputs};
use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};
use crate::pipes::report_manifest::write_folder_manifest;
use crate::pipes::Pipe;
use sequent_core::ballot::{Candidate, Contest, StringifiedPeriodDates, Weight};
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest, InvalidPlaintextErrorType};
use sequent_core::services::{pdf, reports};
use sequent_core::types::ceremonies::{ScopeOperation, TallyOperation};
use sequent_core::util::date_time::get_date_and_time;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use tracing::info;
use tracing::instrument;
use uuid::Uuid;

/// The platform's report type the ballot images are: what their hash
/// manifest names.
pub const BALLOT_IMAGES_REPORT_TYPE: &str = "BALLOT_IMAGES";
pub const BALLOT_IMAGES_OUTPUT_FILE_PDF: &str = "ballot_images.pdf";
pub const BALLOT_IMAGES_OUTPUT_FILE_HTML: &str = "ballot_images.html";

/// Effective election zones resolved with the event context when this report
/// run was prepared; stored in the existing arbitrary extra_data object.
pub const ELECTION_TIME_ZONES_VAR: &str = "electionTimezones";

pub fn add_ballot_image_time_variables(
    extra_data: &Value,
    election_id: &str,
    variables: &mut Map<String, Value>,
) {
    reports::copy_template_time_variables(extra_data, variables);
    if let Some(zone) = extra_data
        .get(ELECTION_TIME_ZONES_VAR)
        .and_then(|zones| zones.get(election_id))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|zone| !zone.is_empty())
    {
        variables.insert(
            reports::ELECTION_TIMEZONE_VAR.to_string(),
            Value::String(zone.to_string()),
        );
    }
}

pub struct BallotImages {
    pub pipe_inputs: PipeInputs,
}

pub struct BallotImagesPipeData {
    pub output_file_pdf: String,
    pub output_file_html: String,
    pub pipe_name: String,
    pub pipe_name_output_dir: String,
}

impl BallotImages {
    #[instrument(skip_all, name = "BallotImages::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }

    #[instrument(skip_all, err)]
    fn print_ballot_images(
        &self,
        path: &Path,
        contest: &Contest,
        election_input: &InputElectionConfig,
        pipe_config: &PipeConfigBallotImages,
        area_name: &str,
    ) -> Result<(Option<Vec<u8>>, Vec<u8>)> {
        let tally = Tally::new(
            contest,
            ScopeOperation::Area(TallyOperation::ProcessBallotsAll), // TODO: Fix this
            vec![(path.to_path_buf(), Weight::default())],
            0,
            0,
            vec![],
            vec![],
        )
        .map_err(|e| Error::UnexpectedError(e.to_string()))?;

        let ballots = tally
            .ballots
            .iter()
            .map(|(ballot, _weight)| ballot.clone())
            .collect::<Vec<DecodedVoteContest>>();

        let data = TemplateData {
            contest: tally.contest.clone(),
            ballots,
            election_name: election_input.name.clone(),
            election_alias: election_input.alias.clone(),
            election_annotations: election_input.annotations.clone(),
            election_dates: election_input.dates.clone(),
            area: area_name.to_string(),
        };

        info!("election_input: {}", election_input.name);
        let mut data = compute_data(data);
        if let Some(annotations) = &pipe_config.execution_annotations {
            data.execution_annotations.extend(annotations.clone());
        }

        let mut map = Map::new();
        map.insert("data".to_string(), serde_json::to_value(&data)?);
        map.insert(
            "extra_data".to_string(),
            serde_json::to_value(&pipe_config.extra_data)?,
        );
        // The event's zone and timezone texts for the date helpers.
        add_ballot_image_time_variables(
            &pipe_config.extra_data,
            &election_input.id.to_string(),
            &mut map,
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

        add_ballot_image_time_variables(
            &pipe_config.extra_data,
            &election_input.id.to_string(),
            &mut system_map,
        );

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
            let bytes_html = bytes_html.clone();
            let bytes_pdf =
                pdf::sync::PdfRenderer::render_pdf(bytes_html, pdf_options).map_err(|e| {
                    Error::UnexpectedError(format!("Error during PDF rendering: {}", e))
                })?;

            Some(bytes_pdf)
        } else {
            None
        };

        Ok((bytes_pdf, bytes_html.into_bytes()))
    }

    #[instrument(err, skip_all)]
    pub fn get_config(&self) -> Result<PipeConfigBallotImages> {
        let pipe_config: PipeConfigBallotImages = self
            .pipe_inputs
            .stage
            .pipe_config(self.pipe_inputs.stage.current_pipe)
            .and_then(|pc| pc.config)
            .map(|value| serde_json::from_value(value))
            .transpose()?
            .unwrap_or_default();
        Ok(pipe_config)
    }
}

#[instrument(skip_all)]
fn get_pipe_data() -> BallotImagesPipeData {
    BallotImagesPipeData {
        output_file_pdf: BALLOT_IMAGES_OUTPUT_FILE_PDF.to_string(),
        output_file_html: BALLOT_IMAGES_OUTPUT_FILE_HTML.to_string(),
        pipe_name_output_dir: PipeNameOutputDir::BallotImages.as_ref().to_string(),
        pipe_name: PipeName::BallotImages.as_ref().to_string(),
    }
}

impl Pipe for BallotImages {
    #[instrument(err, skip_all, name = "BallotImages::exec")]
    fn exec(&self) -> Result<()> {
        let input_dir = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DecodeBallots.as_ref());

        let pipe_config: PipeConfigBallotImages = self.get_config()?;

        let pipe_type_data = get_pipe_data();

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

                    if decoded_ballots_file.exists() {
                        let (bytes_pdf, bytes_html) = self.print_ballot_images(
                            decoded_ballots_file.as_path(),
                            &contest_input.contest,
                            &election_input,
                            &pipe_config,
                            &area_input.area.name,
                        )?;

                        let path = PipeInputs::build_path(
                            &self
                                .pipe_inputs
                                .cli
                                .output_dir
                                .join(&pipe_type_data.pipe_name_output_dir)
                                .as_path(),
                            &election_input.id,
                            Some(&contest_input.id),
                            Some(&area_input.id),
                        );

                        fs::create_dir_all(&path)?;

                        if let Some(ref some_bytes_pdf) = bytes_pdf {
                            let file = path.join(&pipe_type_data.output_file_pdf);
                            let mut file = OpenOptions::new()
                                .write(true)
                                .truncate(true)
                                .create(true)
                                .open(file)?;
                            file.write_all(&some_bytes_pdf)?;
                        }

                        let file = path.join(&pipe_type_data.output_file_html);
                        let mut file = OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(true)
                            .open(file)?;
                        file.write_all(&bytes_html)?;

                        if let Some(stamp) = &pipe_config.configuration {
                            write_folder_manifest(&path, BALLOT_IMAGES_REPORT_TYPE, stamp)?;
                        }
                    } else {
                        println!(
                            "[{}] File not found: {} -- Not processed",
                            pipe_type_data.pipe_name,
                            decoded_ballots_file.display()
                        )
                    }
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
    pub election_name: String,
    pub election_alias: String,
    pub area: String,
    pub election_dates: Option<StringifiedPeriodDates>,
    pub election_annotations: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BallotData {
    pub id: String,
    pub encoded_vote: String,
    pub is_invalid: bool,
    pub is_blank: bool,
    pub contest_choices: Vec<ContestData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContestData {
    pub contest: Contest,
    pub decoded_choices: Vec<DecodedChoice>,
    pub undervotes: i64,
    pub overvotes: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComputedTemplateData {
    pub ballot_data: Vec<BallotData>,
    pub election_name: String,
    pub election_alias: String,
    pub area: String,
    pub election_dates: Option<StringifiedPeriodDates>,
    pub election_annotations: HashMap<String, String>,
    pub execution_annotations: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DecodedChoice {
    pub choice: DecodedVoteChoice,
    pub candidate: Option<Candidate>,
}

#[instrument(skip_all)]
fn compute_data(data: TemplateData) -> ComputedTemplateData {
    let receipts = data
        .ballots
        .iter()
        .map(|decoded_vote_contest| {
            let is_invalid = decoded_vote_contest.is_invalid();
            let selected_candidates = decoded_vote_contest
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

            let num_selected = decoded_vote_contest
                .choices
                .iter()
                .filter(|can| can.is_selected())
                .count();

            // A contest that did not decode has no marks, but it is not a
            // blank vote.
            let has_encoding_error = decoded_vote_contest
                .invalid_errors
                .iter()
                .any(|error| error.error_type == InvalidPlaintextErrorType::EncodingError);
            let is_blank = !has_encoding_error && selected_candidates.len() == 0;
            let undervotes = data.contest.max_votes - (num_selected as i64);
            let mut overvotes = 0;
            if (num_selected as i64) > data.contest.max_votes {
                overvotes = (num_selected as i64) - data.contest.max_votes;
            }

            let encoded_vote_contest = data
                .contest
                .encode_plaintext_contest_bigint(decoded_vote_contest)
                .unwrap()
                .to_string();

            let decoded_choices = decoded_vote_contest
                .choices
                .iter()
                .map(|choice| DecodedChoice {
                    choice: choice.clone(),
                    candidate: data
                        .contest
                        .candidates
                        .iter()
                        .find(|c| c.id == choice.id)
                        .cloned(),
                })
                .collect::<Vec<DecodedChoice>>();

            BallotData {
                contest_choices: vec![ContestData {
                    contest: data.contest.clone(),
                    decoded_choices,
                    undervotes,
                    overvotes,
                }],
                id: Uuid::new_v4().to_string(),
                encoded_vote: encoded_vote_contest,
                is_invalid,
                is_blank,
            }
        })
        .collect::<Vec<BallotData>>();

    ComputedTemplateData {
        ballot_data: receipts,
        election_name: data.election_name,
        election_alias: data.election_alias,
        area: data.area,
        election_annotations: data.election_annotations,
        election_dates: data.election_dates,
        execution_annotations: HashMap::from([("date_printed".to_string(), get_date_and_time())]),
    }
}

#[cfg(test)]
mod timezone_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn report_content_and_outer_template_use_the_resolved_election_zone() {
        let source = json!({
            "electionEventTimezone": "Europe/Madrid",
            "electionTimezone": "Europe/Madrid",
            "electionTimezones": {"office": "Atlantic/Canary"},
            "timezoneTexts": {"defaults": {"timezones.abbrDaylight.Atlantic/Canary": "WEST"}, "overrides": {}}
        });
        let mut variables = Map::new();
        add_ballot_image_time_variables(&source, "office", &mut variables);
        variables.insert("at".into(), json!("2028-04-09T00:00:00Z"));
        assert_eq!(variables["electionEventTimezone"], json!("Europe/Madrid"));
        assert_eq!(
            reports::render_template_text(
                r#"{{datetime_zone at output_format="%H:%M"}}"#,
                variables
            )
            .unwrap(),
            "01:00 WEST"
        );
    }

    #[test]
    fn older_inputs_keep_the_existing_timezone_fallback() {
        let source = json!({"electionEventTimezone": "Europe/Madrid", "electionTimezone": "UTC"});
        let mut variables = Map::new();
        add_ballot_image_time_variables(&source, "office", &mut variables);
        assert_eq!(variables["electionTimezone"], json!("UTC"));
    }
}
