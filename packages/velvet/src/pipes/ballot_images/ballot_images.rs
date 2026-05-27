// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::ballot_images_config::PipeConfigBallotImages;
use crate::pipes::decode_ballots::OUTPUT_DECODED_BALLOTS_FILE;
use crate::pipes::do_tally::tally::Tally;
use crate::pipes::error::{Error, Result};
use crate::pipes::pipe_inputs::{InputElectionConfig, PipeInputs};
use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};
use crate::pipes::Pipe;
use sequent_core::ballot::{Candidate, Contest, StringifiedPeriodDates, Weight};
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::services::{pdf, reports};
use sequent_core::types::ceremonies::{ScopeOperation, TallyOperation};
use sequent_core::util::date_time::get_date_and_time;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use tracing::info;
use tracing::instrument;
use uuid::Uuid;

/// Output filename for ballot images PDF file.
pub const BALLOT_IMAGES_OUTPUT_FILE_PDF: &str = "ballot_images.pdf";
/// Output filename for ballot images HTML file.
pub const BALLOT_IMAGES_OUTPUT_FILE_HTML: &str = "ballot_images.html";

/// Ballot images pipe implementation for generating ballot representations.
pub struct BallotImages {
    /// Pipeline input configuration.
    pub pipe_inputs: PipeInputs,
}

/// Ballot images pipe data containing output filenames and paths.
pub struct BallotImagesPipeData {
    /// Output filename for PDF file.
    pub output_file_pdf: String,
    /// Output filename for HTML file.
    pub output_file_html: String,
    /// Pipeline name string.
    pub pipe_name: String,
    /// Pipeline output directory name.
    pub pipe_name_output_dir: String,
}

impl BallotImages {
    /// Creates a new ballot images pipe instance.
    #[instrument(skip_all, name = "BallotImages::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }

    #[instrument(skip_all, err)]
    /// Generates ballot images (PDF and HTML) for a contest.
    ///
    /// # Errors
    /// Returns an error if tally creation, template rendering, or PDF generation fails.
    #[allow(clippy::unused_self)]
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
        .map_err(|e| Error::Unexpected(e.to_string()))?;

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
        let data = compute_data(data);

        let mut map = Map::new();
        map.insert("data".to_string(), serde_json::to_value(&data)?);
        map.insert(
            "extra_data".to_string(),
            serde_json::to_value(&pipe_config.extra_data)?,
        );

        let rendered_user_template = reports::render_template_text(&pipe_config.template, map)
            .map_err(|e| {
                Error::Unexpected(format!(
                    "Error during render_template_text from report.hbs template file: {e}"
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
                Error::Unexpected(format!(
                    "Error during render_template_text from report.hbs template file: {e}"
                ))
            })?;

        let pdf_options = pipe_config
            .pdf_options
            .clone()
            .map(|options| options.to_print_to_pdf_options());

        let bytes_pdf = if pipe_config.enable_pdfs {
            let bytes_html = bytes_html.clone();
            let bytes_pdf = pdf::sync::PdfRenderer::render_pdf(bytes_html, pdf_options)
                .map_err(|e| Error::Unexpected(format!("Error during PDF rendering: {e}")))?;

            Some(bytes_pdf)
        } else {
            None
        };

        Ok((bytes_pdf, bytes_html.into_bytes()))
    }

    #[instrument(err, skip_all)]
    /// Gets the ballot images pipe configuration.
    ///
    /// # Errors
    /// Returns an error if deserialization of the pipe config fails.
    pub fn get_config(&self) -> Result<PipeConfigBallotImages> {
        let pipe_config: PipeConfigBallotImages = self
            .pipe_inputs
            .stage
            .pipe_config(self.pipe_inputs.stage.current_pipe)
            .and_then(|pc| pc.config)
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        Ok(pipe_config)
    }
}

#[instrument(skip_all)]
/// Returns the ballot images pipe metadata.
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
                            election_input,
                            &pipe_config,
                            &area_input.area.name,
                        )?;

                        let path = PipeInputs::build_path(
                            self.pipe_inputs
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
                            file.write_all(some_bytes_pdf)?;
                        }

                        let file = path.join(&pipe_type_data.output_file_html);
                        let mut file = OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(true)
                            .open(file)?;
                        file.write_all(&bytes_html)?;
                    } else {
                        tracing::warn!(
                            "[{}] File not found: {} -- Not processed",
                            pipe_type_data.pipe_name,
                            decoded_ballots_file.display()
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Serialize)]
/// Template data for rendering ballot images.
struct TemplateData {
    /// The contest configuration.
    pub contest: Contest,
    /// The decoded ballot choices.
    pub ballots: Vec<DecodedVoteContest>,
    /// The election name.
    pub election_name: String,
    /// The election alias.
    pub election_alias: String,
    /// The area name.
    pub area: String,
    /// The election dates.
    pub election_dates: Option<StringifiedPeriodDates>,
    /// The election annotations.
    pub election_annotations: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Ballot data for rendering includes decoded votes and invalid/blank status.
pub struct BallotData {
    /// Ballot identifier.
    pub id: String,
    /// Encoded vote representation.
    pub encoded_vote: String,
    /// Whether the ballot is marked as invalid.
    pub is_invalid: bool,
    /// Whether the ballot is blank (no votes).
    pub is_blank: bool,
    /// Contest choices for this ballot.
    pub contest_choices: Vec<ContestData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Contest data for rendering includes decoded choices and vote counts.
pub struct ContestData {
    /// Contest configuration.
    pub contest: Contest,
    /// Decoded choices for this contest.
    pub decoded_choices: Vec<DecodedChoice>,
    /// Number of undervotes for this contest.
    pub undervotes: i64,
    /// Number of overvotes for this contest.
    pub overvotes: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Computed template data prepared for rendering ballot images.
pub struct ComputedTemplateData {
    /// All ballot data to render.
    pub ballot_data: Vec<BallotData>,
    /// Name of the election.
    pub election_name: String,
    /// Alias for the election.
    pub election_alias: String,
    /// Area name for this ballot set.
    pub area: String,
    /// Election start and end dates if specified.
    pub election_dates: Option<StringifiedPeriodDates>,
    /// Election annotations.
    pub election_annotations: HashMap<String, String>,
    /// Extra execution annotations such as date printed.
    pub execution_annotations: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Decoded choice data with candidate information.
pub struct DecodedChoice {
    /// The decoded vote choice.
    pub choice: DecodedVoteChoice,
    /// The candidate if found, or None if not a valid choice.
    pub candidate: Option<Candidate>,
}

#[instrument(skip_all)]
/// Computes template data by processing ballot data for rendering.
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

            let is_blank = selected_candidates.is_empty();
            let num_selected_i64 =
                i64::try_from(num_selected).expect("num_selected should fit in i64");
            let undervotes = data.contest.max_votes.saturating_sub(num_selected_i64);
            let mut overvotes = 0i64;
            if num_selected_i64 > data.contest.max_votes {
                overvotes = num_selected_i64.saturating_sub(data.contest.max_votes);
            }

            let encoded_vote_contest = data
                .contest
                .encode_plaintext_contest_bigint(decoded_vote_contest)
                .expect("Failed to encode plaintext contest")
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
