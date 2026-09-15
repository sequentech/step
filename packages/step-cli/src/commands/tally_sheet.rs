// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    types::hasura_types::*,
    utils::{
        read_config::read_config,
        tally::download_document::{download_file, fetch_document},
        upload_file::GetUploadUrl,
    },
};
use clap::{Args, Subcommand, ValueEnum};
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};
use sequent_core::types::tally_sheets::VotingChannel;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    error::Error,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
use windmill::services::ess_xml_converter::convert_ess_enhanced_xml_to_csv;

#[derive(Args)]
#[command(about = "Manage tally sheets and tally sheet imports", long_about = None)]
pub struct TallySheetCommand {
    #[command(subcommand)]
    command: TallySheetCommands,
}

#[derive(Subcommand)]
enum TallySheetCommands {
    Create(CreateTallySheetCommand),
    Review(ReviewTallySheetCommand),
    ImportPreview(PreviewTallySheetImportCommand),
    ImportCreate(CreateTallySheetImportCommand),
    ImportReview(ReviewTallySheetImportCommand),
    ImportList(ListTallySheetImportsCommand),
    ImportShow(ShowTallySheetImportCommand),
    ImportDownloadSource(DownloadTallySheetImportSourceCommand),
    Recount(RecountTallySessionCommand),
    ConvertEssXml(ConvertEssXmlCommand),
}

#[derive(Args)]
#[command(about = "Create a tally sheet from an AreaContestResults JSON file")]
struct CreateTallySheetCommand {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    area_id: String,

    #[arg(long)]
    contest_id: String,

    #[arg(long, value_enum, default_value = "PAPER")]
    channel: VotingChannelArg,

    #[arg(long)]
    content_file: PathBuf,
}

#[derive(Args)]
#[command(about = "Approve or disapprove a pending tally sheet")]
struct ReviewTallySheetCommand {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    tally_sheet_id: String,

    #[arg(long, value_enum)]
    status: TallySheetStatusArg,
}

#[derive(Args)]
#[command(about = "Upload or reference a source file and preview a tally sheet import")]
struct PreviewTallySheetImportCommand {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    file_path: Option<PathBuf>,

    #[arg(long)]
    document_id: Option<String>,

    #[arg(long)]
    sha256: Option<String>,

    #[arg(long, value_enum, default_value = "ESS_ENHANCED_XML")]
    source_format: TallySheetImportSourceFormatArg,

    #[arg(long, value_enum, default_value = "PAPER")]
    selected_channel: VotingChannelArg,

    #[arg(long, default_value_t = false)]
    is_local: bool,
}

#[derive(Args)]
#[command(about = "Upload or reference a source file and create a tally sheet import")]
struct CreateTallySheetImportCommand {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    file_path: Option<PathBuf>,

    #[arg(long)]
    document_id: Option<String>,

    #[arg(long)]
    sha256: Option<String>,

    #[arg(long, value_enum, default_value = "ESS_ENHANCED_XML")]
    source_format: TallySheetImportSourceFormatArg,

    #[arg(long, value_enum, default_value = "PAPER")]
    selected_channel: VotingChannelArg,

    #[arg(long, default_value_t = false)]
    is_local: bool,
}

#[derive(Args)]
#[command(about = "Approve or disapprove a tally sheet import")]
struct ReviewTallySheetImportCommand {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    import_id: String,

    #[arg(long, value_enum)]
    decision: TallySheetImportDecisionArg,
}

#[derive(Args)]
#[command(about = "List tally sheet imports for an election event")]
struct ListTallySheetImportsCommand {
    #[arg(long)]
    election_event_id: String,

    #[arg(long, default_value_t = 50)]
    limit: i64,
}

#[derive(Args)]
#[command(about = "Show a tally sheet import and its generated items")]
struct ShowTallySheetImportCommand {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    import_id: String,
}

#[derive(Args)]
#[command(about = "Download the original source file for a tally sheet import")]
struct DownloadTallySheetImportSourceCommand {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    import_id: String,

    #[arg(long, default_value = "output")]
    output_dir: PathBuf,
}

#[derive(Args)]
#[command(about = "Recount a completed tally session with fresh result ids")]
struct RecountTallySessionCommand {
    #[arg(long)]
    election_event_id: String,

    #[arg(long)]
    tally_id: String,
}

#[derive(Args)]
#[command(about = "Convert an ES&S Enhanced XML file to canonical tally sheet CSV")]
struct ConvertEssXmlCommand {
    #[arg(long)]
    input: PathBuf,

    #[arg(long)]
    output: PathBuf,

    #[arg(long, value_enum, default_value = "PAPER")]
    selected_channel: VotingChannelArg,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VotingChannelArg {
    Paper,
    Postal,
    InPerson,
}

impl VotingChannelArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Paper => "PAPER",
            Self::Postal => "POSTAL",
            Self::InPerson => "IN_PERSON",
        }
    }

    fn to_core(self) -> VotingChannel {
        match self {
            Self::Paper => VotingChannel::PAPER,
            Self::Postal => VotingChannel::POSTAL,
            Self::InPerson => VotingChannel::IN_PERSON,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TallySheetImportSourceFormatArg {
    EssEnhancedXml,
    CanonicalCsv,
}

impl TallySheetImportSourceFormatArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::EssEnhancedXml => "ESS_ENHANCED_XML",
            Self::CanonicalCsv => "CANONICAL_CSV",
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TallySheetStatusArg {
    Approved,
    Disapproved,
}

impl TallySheetStatusArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "APPROVED",
            Self::Disapproved => "DISAPPROVED",
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TallySheetImportDecisionArg {
    Approve,
    Disapprove,
}

impl TallySheetImportDecisionArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "APPROVE",
            Self::Disapprove => "DISAPPROVE",
        }
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/create_tally_sheet.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct CreateNewTallySheet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/review_tally_sheet.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct ReviewTallySheet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/preview_tally_sheet_import.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct PreviewTallySheetImport;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/create_tally_sheet_import.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct CreateTallySheetImport;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/review_tally_sheet_import.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct ReviewTallySheetImport;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/list_tally_sheet_imports.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct ListTallySheetImports;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_tally_sheet_import.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetTallySheetImport;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/recount_tally_session.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct RecountTallySession;

impl TallySheetCommand {
    pub fn run(&self) {
        match &self.command {
            TallySheetCommands::Create(command) => command.run(),
            TallySheetCommands::Review(command) => command.run(),
            TallySheetCommands::ImportPreview(command) => command.run(),
            TallySheetCommands::ImportCreate(command) => command.run(),
            TallySheetCommands::ImportReview(command) => command.run(),
            TallySheetCommands::ImportList(command) => command.run(),
            TallySheetCommands::ImportShow(command) => command.run(),
            TallySheetCommands::ImportDownloadSource(command) => command.run(),
            TallySheetCommands::Recount(command) => command.run(),
            TallySheetCommands::ConvertEssXml(command) => command.run(),
        }
    }
}

impl CreateTallySheetCommand {
    fn run(&self) {
        let content = match read_json_file(&self.content_file) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Error! Failed to read content file: {}", err);
                return;
            }
        };

        match create_tally_sheet(
            &self.election_event_id,
            &self.area_id,
            &self.contest_id,
            self.channel,
            content,
        ) {
            Ok(sheet) => print_json("Success! Created tally sheet:", &sheet),
            Err(err) => eprintln!("Error! Failed to create tally sheet: {}", err),
        }
    }
}

impl ReviewTallySheetCommand {
    fn run(&self) {
        match review_tally_sheet(&self.election_event_id, &self.tally_sheet_id, self.status) {
            Ok(sheet) => print_json("Success! Reviewed tally sheet:", &sheet),
            Err(err) => eprintln!("Error! Failed to review tally sheet: {}", err),
        }
    }
}

impl PreviewTallySheetImportCommand {
    fn run(&self) {
        let document = match resolve_import_document(
            &self.election_event_id,
            self.file_path.as_deref(),
            self.document_id.as_deref(),
            self.sha256.as_deref(),
            self.is_local,
        ) {
            Ok(document) => document,
            Err(err) => {
                eprintln!("Error! Failed to prepare import source: {}", err);
                return;
            }
        };

        match preview_tally_sheet_import(
            &self.election_event_id,
            &document.document_id,
            document.sha256.as_deref(),
            self.source_format,
            self.selected_channel,
        ) {
            Ok(preview) => print_json("Success! Tally sheet import preview:", &preview),
            Err(err) => eprintln!("Error! Failed to preview tally sheet import: {}", err),
        }
    }
}

impl CreateTallySheetImportCommand {
    fn run(&self) {
        let document = match resolve_import_document(
            &self.election_event_id,
            self.file_path.as_deref(),
            self.document_id.as_deref(),
            self.sha256.as_deref(),
            self.is_local,
        ) {
            Ok(document) => document,
            Err(err) => {
                eprintln!("Error! Failed to prepare import source: {}", err);
                return;
            }
        };

        match create_tally_sheet_import(
            &self.election_event_id,
            &document.document_id,
            document.sha256.as_deref(),
            self.source_format,
            self.selected_channel,
        ) {
            Ok(import) => print_json("Success! Created tally sheet import:", &import),
            Err(err) => eprintln!("Error! Failed to create tally sheet import: {}", err),
        }
    }
}

impl ReviewTallySheetImportCommand {
    fn run(&self) {
        match review_tally_sheet_import(&self.election_event_id, &self.import_id, self.decision) {
            Ok(import) => print_json("Success! Reviewed tally sheet import:", &import),
            Err(err) => eprintln!("Error! Failed to review tally sheet import: {}", err),
        }
    }
}

impl ListTallySheetImportsCommand {
    fn run(&self) {
        match list_tally_sheet_imports(&self.election_event_id, self.limit) {
            Ok(imports) => print_json("Success! Tally sheet imports:", &imports),
            Err(err) => eprintln!("Error! Failed to list tally sheet imports: {}", err),
        }
    }
}

impl ShowTallySheetImportCommand {
    fn run(&self) {
        match get_tally_sheet_import(&self.election_event_id, &self.import_id) {
            Ok(import) => print_json("Success! Tally sheet import:", &import),
            Err(err) => eprintln!("Error! Failed to show tally sheet import: {}", err),
        }
    }
}

impl DownloadTallySheetImportSourceCommand {
    fn run(&self) {
        match download_tally_sheet_import_source(
            &self.election_event_id,
            &self.import_id,
            &self.output_dir,
        ) {
            Ok(output_path) => println!(
                "{} {}",
                "Success! Downloaded tally sheet import source:".green(),
                output_path.display().to_string().cyan()
            ),
            Err(err) => eprintln!(
                "Error! Failed to download tally sheet import source: {}",
                err
            ),
        }
    }
}

impl RecountTallySessionCommand {
    fn run(&self) {
        match recount_tally_session(&self.election_event_id, &self.tally_id) {
            Ok(tally_id) => {
                println!(
                    "{} {}",
                    "Success! Recounted tally session:".green(),
                    tally_id.cyan()
                );
            }
            Err(err) => eprintln!("Error! Failed to recount tally session: {}", err),
        }
    }
}

impl ConvertEssXmlCommand {
    fn run(&self) {
        match convert_ess_xml_to_tally_csv(&self.input, &self.output, self.selected_channel) {
            Ok(()) => println!(
                "{} {}",
                "Success! Wrote canonical tally CSV:".green(),
                self.output.display().to_string().cyan()
            ),
            Err(err) => eprintln!("Error! Failed to convert ES&S XML: {}", err),
        }
    }
}

pub fn create_tally_sheet(
    election_event_id: &str,
    area_id: &str,
    contest_id: &str,
    channel: VotingChannelArg,
    content: Value,
) -> Result<Value, Box<dyn Error>> {
    let variables = create_new_tally_sheet::Variables {
        election_event_id: election_event_id.to_string(),
        channel: channel.as_str().to_string(),
        content,
        contest_id: contest_id.to_string(),
        area_id: area_id.to_string(),
    };
    let request_body = CreateNewTallySheet::build_query(variables);
    let data: create_new_tally_sheet::ResponseData = response_data(post_graphql(&request_body)?)?;
    let sheet = data
        .create_new_tally_sheet
        .ok_or("failed creating tally sheet")?;

    Ok(serde_json::to_value(sheet)?)
}

pub fn review_tally_sheet(
    election_event_id: &str,
    tally_sheet_id: &str,
    status: TallySheetStatusArg,
) -> Result<Value, Box<dyn Error>> {
    let variables = review_tally_sheet::Variables {
        election_event_id: election_event_id.to_string(),
        tally_sheet_id: tally_sheet_id.to_string(),
        new_status: status.as_str().to_string(),
    };
    let request_body = ReviewTallySheet::build_query(variables);
    let data: review_tally_sheet::ResponseData = response_data(post_graphql(&request_body)?)?;
    let sheet = data
        .review_tally_sheet
        .ok_or("failed reviewing tally sheet")?;

    Ok(serde_json::to_value(sheet)?)
}

pub fn preview_tally_sheet_import(
    election_event_id: &str,
    document_id: &str,
    sha256: Option<&str>,
    source_format: TallySheetImportSourceFormatArg,
    selected_channel: VotingChannelArg,
) -> Result<Value, Box<dyn Error>> {
    let variables = preview_tally_sheet_import::Variables {
        election_event_id: election_event_id.to_string(),
        document_id: document_id.to_string(),
        sha256: sha256.map(String::from),
        source_format: source_format.as_str().to_string(),
        selected_channel: selected_channel.as_str().to_string(),
    };
    let request_body = PreviewTallySheetImport::build_query(variables);
    let data: preview_tally_sheet_import::ResponseData =
        response_data(post_graphql(&request_body)?)?;
    let preview = data
        .preview_tally_sheet_import
        .ok_or("failed previewing tally sheet import")?
        .preview;

    Ok(preview)
}

pub fn create_tally_sheet_import(
    election_event_id: &str,
    document_id: &str,
    sha256: Option<&str>,
    source_format: TallySheetImportSourceFormatArg,
    selected_channel: VotingChannelArg,
) -> Result<Value, Box<dyn Error>> {
    let variables = create_tally_sheet_import::Variables {
        election_event_id: election_event_id.to_string(),
        document_id: document_id.to_string(),
        sha256: sha256.map(String::from),
        source_format: source_format.as_str().to_string(),
        selected_channel: selected_channel.as_str().to_string(),
    };
    let request_body = CreateTallySheetImport::build_query(variables);
    let data: create_tally_sheet_import::ResponseData =
        response_data(post_graphql(&request_body)?)?;
    let import = data
        .create_tally_sheet_import
        .ok_or("failed creating tally sheet import")?
        .tally_sheet_import;

    Ok(import)
}

pub fn review_tally_sheet_import(
    election_event_id: &str,
    import_id: &str,
    decision: TallySheetImportDecisionArg,
) -> Result<Value, Box<dyn Error>> {
    let variables = review_tally_sheet_import::Variables {
        election_event_id: election_event_id.to_string(),
        import_id: import_id.to_string(),
        decision: decision.as_str().to_string(),
    };
    let request_body = ReviewTallySheetImport::build_query(variables);
    let data: review_tally_sheet_import::ResponseData =
        response_data(post_graphql(&request_body)?)?;
    let import = data
        .review_tally_sheet_import
        .ok_or("failed reviewing tally sheet import")?
        .tally_sheet_import;

    Ok(import)
}

pub fn list_tally_sheet_imports(
    election_event_id: &str,
    limit: i64,
) -> Result<Value, Box<dyn Error>> {
    let variables = list_tally_sheet_imports::Variables {
        election_event_id: ::uuid::Uuid::parse_str(election_event_id)?.to_string(),
        limit,
    };
    let request_body = ListTallySheetImports::build_query(variables);
    let data: list_tally_sheet_imports::ResponseData = response_data(post_graphql(&request_body)?)?;

    Ok(serde_json::to_value(
        data.sequent_backend_tally_sheet_import,
    )?)
}

pub fn get_tally_sheet_import(
    election_event_id: &str,
    import_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let variables = get_tally_sheet_import::Variables {
        election_event_id: ::uuid::Uuid::parse_str(election_event_id)?.to_string(),
        import_id: ::uuid::Uuid::parse_str(import_id)?.to_string(),
    };
    let request_body = GetTallySheetImport::build_query(variables);
    let data: get_tally_sheet_import::ResponseData = response_data(post_graphql(&request_body)?)?;
    let tally_sheet_import = data
        .sequent_backend_tally_sheet_import
        .into_iter()
        .next()
        .ok_or("tally sheet import not found")?;

    Ok(serde_json::to_value(tally_sheet_import)?)
}

pub fn download_tally_sheet_import_source(
    election_event_id: &str,
    import_id: &str,
    output_dir: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let tally_sheet_import = get_tally_sheet_import(election_event_id, import_id)?;
    let document_id = tally_sheet_import
        .get("source_document_id")
        .and_then(Value::as_str)
        .ok_or("tally sheet import has no source document")?;
    let file_name = tally_sheet_import
        .get("source_file_name")
        .and_then(Value::as_str)
        .and_then(|name| Path::new(name).file_name())
        .and_then(|name| name.to_str())
        .map(String::from)
        .unwrap_or_else(|| format!("tally-sheet-import-{import_id}"));
    let output_path = output_dir.join(file_name);
    let document = fetch_document(election_event_id, document_id)?;
    download_file(&document.url, &output_path.to_string_lossy())?;

    Ok(output_path)
}

pub fn recount_tally_session(
    election_event_id: &str,
    tally_id: &str,
) -> Result<String, Box<dyn Error>> {
    let variables = recount_tally_session::Variables {
        election_event_id: ::uuid::Uuid::parse_str(election_event_id)?.to_string(),
        tally_session_id: ::uuid::Uuid::parse_str(tally_id)?.to_string(),
    };
    let request_body = RecountTallySession::build_query(variables);
    let data: recount_tally_session::ResponseData = response_data(post_graphql(&request_body)?)?;
    let output = data
        .recount_tally_session
        .ok_or("failed recounting tally session")?;

    Ok(output.tally_session_id)
}

pub fn convert_ess_xml_to_tally_csv(
    input: &Path,
    output: &Path,
    selected_channel: VotingChannelArg,
) -> Result<(), Box<dyn Error>> {
    let xml_bytes = fs::read(input)?;
    // Standalone, offline conversion: with no election event there is no
    // contest configuration to look up, so an empty map is passed and the
    // converter falls back to the bounds declared in the file itself.
    let (csv_bytes, validation_errors) =
        convert_ess_enhanced_xml_to_csv(&xml_bytes, selected_channel.to_core(), &HashMap::new())?;
    fs::write(output, csv_bytes)?;
    for error in &validation_errors {
        let context = match &error.contest_external_id {
            Some(contest_external_id) => format!("[{contest_external_id}] "),
            None => String::new(),
        };
        eprintln!("{} {context}{}", "Warning:".yellow(), error.message);
    }
    Ok(())
}

struct ImportDocument {
    document_id: String,
    sha256: Option<String>,
}

fn resolve_import_document(
    election_event_id: &str,
    file_path: Option<&Path>,
    document_id: Option<&str>,
    sha256: Option<&str>,
    is_local: bool,
) -> Result<ImportDocument, Box<dyn Error>> {
    match (file_path, document_id) {
        (Some(_), Some(_)) => Err(Box::from(
            "provide either --file-path or --document-id, not both",
        )),
        (None, None) => Err(Box::from("provide --file-path or --document-id")),
        (Some(path), None) => {
            let actual_sha256 = sha256_file(path)?;
            if let Some(expected_sha256) = normalize_sha256(sha256)? {
                if expected_sha256 != actual_sha256 {
                    return Err(Box::from(format!(
                        "sha256 mismatch: expected {}, got {}",
                        expected_sha256, actual_sha256
                    )));
                }
            }
            let uploaded_document_id = GetUploadUrl::upload_for_election_event(
                path.to_string_lossy().to_string(),
                is_local,
                Some(election_event_id.to_string()),
            )?;

            Ok(ImportDocument {
                document_id: uploaded_document_id,
                sha256: Some(actual_sha256),
            })
        }
        (None, Some(existing_document_id)) => Ok(ImportDocument {
            document_id: existing_document_id.to_string(),
            sha256: normalize_sha256(sha256)?,
        }),
    }
}

fn read_json_file(path: &Path) -> Result<Value, Box<dyn Error>> {
    let file = File::open(path)?;
    Ok(serde_json::from_reader(file)?)
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

fn normalize_sha256(value: Option<&str>) -> Result<Option<String>, Box<dyn Error>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(None);
    }
    if normalized.len() != 64
        || !normalized
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(Box::from("sha256 must be a 64-character hex string"));
    }

    Ok(Some(normalized))
}

fn post_graphql<T, B>(request_body: &B) -> Result<Response<T>, Box<dyn Error>>
where
    T: DeserializeOwned,
    B: Serialize + ?Sized,
{
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(request_body)
        .send()?;

    if response.status().is_success() {
        Ok(response.json()?)
    } else {
        let status = response.status();
        let error_message = response.text()?;
        Err(Box::from(format!(
            "HTTP Status: {}\nError Message: {}",
            status, error_message
        )))
    }
}

fn response_data<T>(response_body: Response<T>) -> Result<T, Box<dyn Error>> {
    if let Some(data) = response_body.data {
        Ok(data)
    } else if let Some(errors) = response_body.errors {
        let error_messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
        Err(Box::from(error_messages.join(", ")))
    } else {
        Err(Box::from("Unknown error occurred"))
    }
}

fn print_json(message: &str, value: &Value) {
    println!("{}", message.green());
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{}", json),
        Err(err) => eprintln!("Error! Failed to render JSON: {}", err),
    }
}
