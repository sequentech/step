use crate::postgres::area::{get_areas_by_ids, get_event_areas};
use crate::postgres::area_contest::{export_area_contests, get_area_contests_by_area_contest_ids};
use crate::postgres::candidate::export_candidate_csv;
use crate::postgres::contest::{export_contests, get_contest_by_election_ids};
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{export_elections, get_elections, get_elections_by_ids};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::cast_votes::ElectionCastVotes;
use crate::services::consolidation::acm_json::get_acm_key_pair;
use crate::services::database::get_hasura_pool;
use crate::services::election_dates::get_election_dates;
use crate::services::reports::ballot_images::BallotImagesTemplate;
use crate::services::reports::report_variables::{get_app_hash, get_app_version, get_report_hash};
use crate::services::reports::template_renderer::{
    ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
};
use crate::services::reports::vote_receipt::VoteReceiptTemplate;
use crate::services::tally_sheets::tally::create_tally_sheets_map;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use rusqlite::Connection;
use sequent_core::ballot::{
    Annotations, BallotStyle, Contest, ContestEncryptionPolicy, DecodedBallotsInclusionPolicy,
};
use sequent_core::ballot_codec::PlaintextCodec;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::area_tree::TreeNodeArea;
use sequent_core::services::s3;
use sequent_core::services::translations::Name;
use sequent_core::signatures::ecies_encrypt::EciesKeyPair;
use sequent_core::sqlite::area::create_area_sqlite;
use sequent_core::sqlite::area_contest::create_area_contest_sqlite;
use sequent_core::sqlite::candidate::{create_candidate_sqlite, import_candidate_sqlite};
use sequent_core::sqlite::contests::create_contest_sqlite;
use sequent_core::sqlite::election::create_election_sqlite;
use sequent_core::sqlite::election_event::create_election_event_sqlite;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::{
    Area, Election, ElectionEvent, TallySession, TallySessionContest, TallySheet,
};
use sequent_core::types::scheduled_event::ScheduledEvent;
use sequent_core::types::templates::{
    PrintToPdfOptionsLocal, ReportExtraConfig, SendTemplateBody, VoteReceiptPipeType,
};
pub use sequent_core::util::date_time::get_date_and_time;
use sequent_core::util::temp_path::get_public_assets_path_env_var;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use strand::{backend::ristretto::RistrettoCtx, context::Ctx};
use tempfile::{NamedTempFile, TempPath};
use tokio::runtime::Handle;
use tokio::task;
use tracing::{event, info, instrument, warn, Level};
use uuid::Uuid;
use velvet::cli::state::State;
use velvet::cli::CliRun;
use velvet::config::generate_reports::PipeConfigGenerateReports;
use velvet::config::vote_receipt::PipeConfigVoteReceipts;
use velvet::pipes::generate_db::{PipeConfigGenerateDatabase, DATABASE_FILENAME};
use velvet::pipes::pipe_inputs::{AreaConfig, ElectionConfig};
use velvet::pipes::pipe_inputs::{
    DEFAULT_DIR_BALLOTS, DEFAULT_DIR_CONFIGS, DEFAULT_DIR_DATABASE, DEFAULT_DIR_TALLY_SHEETS,
};
use velvet::pipes::pipe_name::PipeName;

#[derive(Debug, Clone)]
pub struct AreaContestDataType {
    pub plaintexts: Vec<<RistrettoCtx as Ctx>::P>,
    pub last_tally_session_execution: TallySessionContest,
    pub contest: Contest,
    pub ballot_style: BallotStyle,
    pub eligible_voters: u64,
    pub area: Area,
    pub auditable_votes: u64,
}

#[instrument(skip_all)]
fn decode_plaintexts_to_biguints(
    plaintexts: &Vec<<RistrettoCtx as Ctx>::P>,
    contest: &Contest,
) -> Vec<String> {
    plaintexts
        .iter()
        .filter_map(|plaintext| {
            let plaintext_format = plaintext
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<String>>()
                .join(" ");
            let biguint = contest.decode_plaintext_contest_to_biguint(plaintext);

            match biguint {
                Ok(v) => {
                    let biguit_str = v.to_str_radix(10);
                    event!(
                        Level::INFO,
                        "Decoding plaintext {plaintext_format} into string '{biguit_str}'"
                    );

                    Some(biguit_str)
                }
                Err(e) => {
                    event!(
                        Level::WARN,
                        "Decoding plaintext {plaintext_format} has failed: {e}"
                    );
                    None
                }
            }
        })
        .collect::<Vec<_>>()
}

#[instrument(skip_all, err)]
pub fn prepare_tally_for_area_contest(
    base_tempdir: PathBuf,
    area_contest: &AreaContestDataType,
    tally_sheets: &HashMap<(String, String), Vec<TallySheet>>,
    tally_session: &TallySession,
) -> Result<()> {
    let contest_encryption_policy = tally_session
        .configuration
        .clone()
        .unwrap_or_default()
        .get_contest_encryption_policy();
    let area_id = area_contest.last_tally_session_execution.area_id.clone();
    let contest_id = area_contest.contest.id.clone();
    let relevant_sheets = tally_sheets
        .get(&(area_id.clone(), contest_id.clone()))
        .map(|val| val.clone())
        .unwrap_or(vec![]);
    let election_id = area_contest.contest.election_id.clone();

    let biguit_ballots =
        decode_plaintexts_to_biguints(&area_contest.plaintexts, &area_contest.contest);

    let velvet_input_dir = base_tempdir.join("input");
    let _velvet_output_dir = base_tempdir.join("output");

    //// create ballots
    let ballots_path = velvet_input_dir.join(format!(
        "{DEFAULT_DIR_BALLOTS}/election__{election_id}/contest__{contest_id}/area__{area_id}"
    ));
    fs::create_dir_all(&ballots_path)?;

    if ContestEncryptionPolicy::SINGLE_CONTEST == contest_encryption_policy {
        let csv_ballots_path = ballots_path.join("ballots.csv");
        let mut csv_ballots_file = File::create(&csv_ballots_path)?;
        let buffer = biguit_ballots.join("\n").into_bytes();

        csv_ballots_file.write_all(&buffer)?;
    } else if ContestEncryptionPolicy::MULTIPLE_CONTESTS == contest_encryption_policy {
        // For multiple contests, we store ballots in a more aggregated location
        let election_ballots_path = velvet_input_dir.join(format!(
            "{DEFAULT_DIR_BALLOTS}/election__{election_id}/area__{area_id}"
        ));

        fs::create_dir_all(&election_ballots_path)?;
        let csv_ballots_path = election_ballots_path.join("ballots.csv");
        let buffer = biguit_ballots.join("\n").into_bytes();

        // Use OpenOptions to append if file exists, create if not
        // FIXME: This fails here https://github.com/sequentech/step/blob/199d13b20d29bf1ea2bffbbc34fadd6fb35dbf1b/packages/sequent-core/src/ballot_codec/multi_ballot.rs#L687
        let mut csv_ballots_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&csv_ballots_path)?;

        csv_ballots_file.write_all(&buffer)?;
    }

    //// create area folder
    let area_path: PathBuf = velvet_input_dir.join(format!(
        "{DEFAULT_DIR_CONFIGS}/election__{election_id}/contest__{contest_id}/area__{area_id}"
    ));
    fs::create_dir_all(&area_path)?;
    // create area config
    let area_config_path: PathBuf = velvet_input_dir.join(format!(
        "{DEFAULT_DIR_CONFIGS}/election__{election_id}/contest__{contest_id}/area__{area_id}/area-config.json"
    ));

    let area_config = AreaConfig {
        id: Uuid::parse_str(&area_id)?,
        name: area_contest.area.name.clone().unwrap_or("".into()),
        tenant_id: Uuid::parse_str(&area_contest.contest.tenant_id)?,
        election_event_id: Uuid::parse_str(&area_contest.contest.election_event_id)?,
        election_id: Uuid::parse_str(&election_id)?,
        census: area_contest.eligible_voters as u64,
        auditable_votes: area_contest.auditable_votes as u64,
        parent_id: area_contest
            .area
            .parent_id
            .clone()
            .map(|parent_id| Uuid::parse_str(&parent_id))
            .transpose()?,
    };
    let mut area_config_file = fs::File::create(area_config_path)?;
    writeln!(area_config_file, "{}", serde_json::to_string(&area_config)?)?;

    //// create contest config file
    let contest_config_path: PathBuf = velvet_input_dir.join(format!(
        "{DEFAULT_DIR_CONFIGS}/election__{election_id}/contest__{contest_id}/contest-config.json"
    ));
    let mut contest_config_file = fs::File::create(contest_config_path)?;
    writeln!(
        contest_config_file,
        "{}",
        serde_json::to_string(&area_contest.contest)?
    )?;

    //// create tally sheets files
    if relevant_sheets.len() > 0 {
        for tally_sheet in relevant_sheets {
            let Some(content) = tally_sheet.content.clone() else {
                continue;
            };
            //// create tally sheets folder
            let tally_sheet_path: PathBuf = velvet_input_dir.join(format!(
                "{DEFAULT_DIR_TALLY_SHEETS}/election__{}/contest__{}/area__{}/tally_sheet__{}",
                election_id, content.contest_id, content.area_id, tally_sheet.id
            ));
            fs::create_dir_all(&tally_sheet_path)?;
            let tally_sheet_file_path: PathBuf = tally_sheet_path.join("tally-sheet.json");
            let mut tally_sheet_file = fs::File::create(tally_sheet_file_path)?;
            writeln!(tally_sheet_file, "{}", serde_json::to_string(&tally_sheet)?)?;
        }
    }

    Ok(())
}

#[instrument(skip_all, err)]
pub fn create_election_configs_blocking(
    base_tempdir: PathBuf,
    area_contests: &Vec<AreaContestDataType>,
    cast_votes_count: &Vec<ElectionCastVotes>,
    scheduled_events: &Vec<ScheduledEvent>,
    elections_single_map: HashMap<String, Election>,
    areas: Vec<TreeNodeArea>,
    default_lang: String,
    election_event: ElectionEvent,
) -> Result<()> {
    let mut elections_map: HashMap<String, ElectionConfig> = HashMap::new();

    let election_event_annotations: HashMap<String, String> = election_event
        .annotations
        .clone()
        .map(|annotations| deserialize_value(annotations).unwrap_or(Default::default()))
        .unwrap_or(Default::default());
    for area_contest in area_contests {
        let election_id = area_contest.contest.election_id.clone();
        let election_event_id = area_contest.contest.election_event_id.clone();
        let tenant_id = area_contest.contest.tenant_id.clone();

        let election_opt = elections_single_map.get(&election_id);

        // TODO: Refactor to just extract some Election Config with no subitems
        let election_name_opt = election_opt.map(|election| election.get_name(&default_lang));

        let election_alias_otp =
            election_opt.map(|election| election.alias.clone().unwrap_or("".to_string()));

        let election_description = election_opt
            .map(|election| election.description.clone().unwrap_or("".to_string()))
            .unwrap_or("".to_string());

        let election_annotations: HashMap<String, String> = election_opt
            .map(|election| {
                election
                    .annotations
                    .clone()
                    .map(|annotations| deserialize_value(annotations).unwrap_or(Default::default()))
                    .unwrap_or(Default::default())
            })
            .unwrap_or(Default::default());

        let election_cast_votes_count = cast_votes_count
            .iter()
            .find(|data| data.election_id == election_id);

        let election_dates = if let Some(election) = election_opt {
            Some(
                get_election_dates(&election, scheduled_events.clone())
                    .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?,
            )
        } else {
            None
        };

        let mut velvet_election: ElectionConfig = match elections_map.get(&election_id) {
            Some(election) => election.clone(),
            None => ElectionConfig {
                id: Uuid::parse_str(&election_id)?,
                name: election_name_opt.unwrap_or("".to_string()),
                alias: election_alias_otp.unwrap_or("".to_string()),
                description: election_description,
                annotations: election_annotations.clone(),
                election_event_annotations: election_event_annotations.clone(),
                dates: election_dates,
                tenant_id: Uuid::parse_str(&area_contest.contest.tenant_id)?,
                election_event_id: Uuid::parse_str(&area_contest.contest.election_event_id)?,
                census: election_cast_votes_count
                    .map(|data| data.census as u64)
                    .unwrap_or(0),
                total_votes: election_cast_votes_count
                    .map(|data| data.cast_votes as u64)
                    .unwrap_or(0),
                ballot_styles: vec![],
                areas: areas.clone(),
            },
        };

        velvet_election
            .ballot_styles
            .push(area_contest.ballot_style.clone());

        elections_map.insert(election_id.clone(), velvet_election);
    }

    // deduplicate the ballot styles
    event!(Level::INFO, "elections_map len {}", elections_map.len());
    for (key, value) in &elections_map {
        let mut velvet_election: ElectionConfig = value.clone();
        velvet_election
            .ballot_styles
            .sort_by_key(|ballot_style| ballot_style.id.clone());
        velvet_election
            .ballot_styles
            .dedup_by_key(|ballot_style| ballot_style.id.clone());
    }

    // write the election configs
    event!(
        Level::INFO,
        "writing election configs for n elements {}",
        elections_map.len()
    );
    for (election_id, election) in &elections_map {
        let election_config_path: PathBuf = base_tempdir.join(format!(
            "input/default/configs/election__{election_id}/election-config.json"
        ));
        let mut election_config_file = fs::File::create(election_config_path)?;
        writeln!(
            election_config_file,
            "{}",
            serde_json::to_string(&election)?
        )?;
    }
    event!(Level::INFO, "Finished writing election configs");

    Ok(())
}

#[instrument(skip_all, err)]
pub async fn create_election_configs(
    base_tempdir: PathBuf,
    area_contests: &Vec<AreaContestDataType>,
    cast_votes_count: &Vec<ElectionCastVotes>,
    basic_areas: &Vec<TreeNodeArea>,
    election_event: &ElectionEvent,
) -> Result<()> {
    // aggregate all ballot styles for each election
    event!(
        Level::WARN,
        "area_contest_plaintexts len {}",
        area_contests.len()
    );

    let Some(first_area_contest) = area_contests.first() else {
        return Ok(());
    };
    let tenant_id = &first_area_contest.contest.tenant_id;
    let election_event_id = &first_area_contest.contest.election_event_id;

    // Note: for some reason this is needed, if we reuse the existing transaction, we get:
    // AMQP error "IO error: Connection reset by peer (os error 104)"
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura connection pool")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;

    let default_language: String = election_event.get_default_language();
    let elections = export_elections(&hasura_transaction, tenant_id, election_event_id).await?;

    let elections_single_map: HashMap<String, Election> = elections
        .iter()
        .map(|election| (election.id.clone(), election.clone()))
        .collect();
    let area_contests_r = area_contests.clone();
    let cast_votes_count_r = cast_votes_count.clone();

    let areas_clone = basic_areas.clone();

    // Fetch election event data
    let scheduled_events = find_scheduled_event_by_election_event_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await
    .map_err(|e| anyhow!("Error getting scheduled event by election event_id: {e:?}"))?;

    let event = election_event.clone();

    // Spawn the task
    let handle = tokio::task::spawn_blocking(move || {
        create_election_configs_blocking(
            base_tempdir.clone(),
            &area_contests_r,
            &cast_votes_count_r,
            &scheduled_events,
            elections_single_map.clone(),
            areas_clone.clone(),
            default_language.clone(),
            event.clone(),
        )
    });

    // Await the result
    handle.await?
}

#[instrument(err)]
pub fn generate_initial_state(base_tally_path: &PathBuf, pipe_id: &str) -> Result<State> {
    let cli = CliRun {
        stage: "main".to_string(),
        pipe_id: pipe_id.to_string(),
        config: base_tally_path.join("velvet-config.json"),
        input_dir: base_tally_path.join("input"),
        output_dir: base_tally_path.join("output"),
    };

    let config = cli.validate()?;

    State::new(&cli, &config).map_err(|err| anyhow!("{}", err))
}

#[instrument(err)]
pub async fn call_velvet(base_tally_path: PathBuf, pipe_id: &str) -> Result<State> {
    let mut state_opt = Some(generate_initial_state(&base_tally_path, pipe_id)?);

    // Use a loop to handle state processing
    loop {
        // Extract the next stage, or return an error if not found
        let next_stage = {
            let state_ref = state_opt
                .as_ref()
                .ok_or_else(|| anyhow!("State should not be None during processing"))?;

            if let Some(stage) = state_ref.get_next() {
                stage.to_string()
            } else {
                break; // Exit loop if no next stage is found
            }
        };

        event!(Level::INFO, "Exec {}", next_stage);

        // Move the state into a block for mutable borrow
        let handle = tokio::task::spawn_blocking({
            let mut state = state_opt
                .take()
                .ok_or_else(|| anyhow!("Failed to take state for execution"))?;

            move || {
                let result = state.exec_next();
                (state, result)
            }
        });

        // Await the result and handle JoinError explicitly
        let (new_state, result) = handle.await.map_err(|err| anyhow!("{}", err))?;
        result?; // Check the result of exec_next()
        state_opt = Some(new_state); // Restore state for the next iteration
    }

    state_opt.ok_or_else(|| anyhow!("State unexpectedly None at the end of processing"))
}

#[derive(Debug, Serialize, Clone)]
struct VelvetTemplateData {
    pub title: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
}

#[instrument(skip_all, err)]
pub async fn build_vote_receipe_pipe_config(
    tally_session: &TallySession,
    hasura_transaction: &Transaction<'_>,
    minio_endpoint_base: String,
    public_asset_path: String,
) -> Result<PipeConfigVoteReceipts> {
    let vote_receipt_renderer = VoteReceiptTemplate::new(ReportOrigins {
        tenant_id: tally_session.tenant_id.clone(),
        election_event_id: tally_session.election_event_id.clone(),
        election_id: None,
        template_alias: None,
        voter_id: None,
        report_origin: ReportOriginatedFrom::ExportFunction,
        executer_username: None,
        tally_session_id: None,
        user_timezone: None,
    });

    let (user_tpl_document, ext_cfg) = vote_receipt_renderer
        .user_tpl_and_extra_cfg_provider(hasura_transaction)
        .await
        .map_err(|e| anyhow!("Error providing the user template and extra config: {e:?}"))?;

    let vote_receipt_system_template = vote_receipt_renderer
        .get_system_template()
        .await
        .map_err(|e| anyhow!("Error getting the system template: {e:?}"))?;

    let vote_receipt_extra_data = VelvetTemplateData {
        title: VELVET_VOTE_RECEIPTS_TEMPLATE_TITLE.to_string(),
        file_logo: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_LOGO_IMG
        ),
        file_qrcode_lib: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_QRCODE_LIB
        ),
    };

    let report_hash = get_report_hash(&ReportType::VOTE_RECEIPT.to_string()).await?;

    let execution_annotations = HashMap::from([
        ("date_printed".to_string(), get_date_and_time()),
        ("app_hash".to_string(), get_app_hash()),
        ("app_version".to_string(), get_app_version()),
        ("report_hash".to_string(), report_hash),
    ]);

    let vote_receipt_pipe_config = PipeConfigVoteReceipts {
        template: user_tpl_document,
        system_template: vote_receipt_system_template,
        extra_data: serde_json::to_value(vote_receipt_extra_data)?,
        enable_pdfs: true,
        pipe_type: VoteReceiptPipeType::VOTE_RECEIPT,
        pdf_options: Some(ext_cfg.pdf_options),
        report_options: Some(ext_cfg.report_options),
        execution_annotations: Some(execution_annotations),
        acm_key: None,
    };
    Ok(vote_receipt_pipe_config)
}

#[instrument(skip_all, err)]
pub async fn build_ballot_images_pipe_config(
    tally_session: &TallySession,
    hasura_transaction: &Transaction<'_>,
    minio_endpoint_base: String,
    public_asset_path: String,
) -> Result<PipeConfigVoteReceipts> {
    let tenant_id = &tally_session.tenant_id;
    let election_event_id = &tally_session.election_event_id;

    let ballot_images_renderer = BallotImagesTemplate::new(ReportOrigins {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: None,
        template_alias: None,
        voter_id: None,
        report_origin: ReportOriginatedFrom::ExportFunction,
        executer_username: None,
        tally_session_id: None,
        user_timezone: None,
    });

    let (user_tpl_document, ext_cfg) = ballot_images_renderer
        .user_tpl_and_extra_cfg_provider(hasura_transaction)
        .await
        .map_err(|e| anyhow!("Error providing the user template and extra config: {e:?}"))?;

    let ballot_imagest_system_template = ballot_images_renderer
        .get_system_template()
        .await
        .map_err(|e| anyhow!("Error getting the system template: {e:?}"))?;

    let ballot_images_extra_data = VelvetTemplateData {
        title: VELVET_BALLOT_IMAGES_TEMPLATE_TITLE.to_string(),
        file_logo: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_LOGO_IMG
        ),
        file_qrcode_lib: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_QRCODE_LIB
        ),
    };

    let acm_key = get_acm_key_pair(hasura_transaction, &tenant_id, &election_event_id).await?;

    let ballot_images_pipe_config = PipeConfigVoteReceipts {
        template: user_tpl_document,
        system_template: ballot_imagest_system_template,
        extra_data: serde_json::to_value(ballot_images_extra_data)?,
        enable_pdfs: true,
        pipe_type: VoteReceiptPipeType::BALLOT_IMAGES,
        pdf_options: Some(ext_cfg.pdf_options),
        report_options: Some(ext_cfg.report_options),
        execution_annotations: None,
        acm_key: Some(acm_key),
    };
    Ok(ballot_images_pipe_config)
}

async fn build_reports_pipe_config(
    tally_session: &TallySession,
    minio_endpoint_base: String,
    public_asset_path: String,
    report_content_template: Option<String>,
    report_system_template: String,
    pdf_options: Option<PrintToPdfOptionsLocal>,
    tally_type: TallyType,
) -> Result<PipeConfigGenerateReports> {
    let extra_data = VelvetTemplateData {
        title: String::new(),
        file_logo: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_LOGO_IMG
        ),
        file_qrcode_lib: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_QRCODE_LIB
        ),
    };

    let tally_annotations_js = tally_session
        .annotations
        .clone()
        .ok_or_else(|| anyhow!("Missing tally session annotations"))?;

    let tally_annotations: Annotations = deserialize_value(tally_annotations_js)?;

    let tally_executer_username = tally_annotations
        .get("executer_username")
        .cloned()
        .unwrap_or(String::new());

    let report_hash = get_report_hash(&tally_type.to_string()).await?;

    let execution_annotations = HashMap::from([
        ("date_printed".to_string(), get_date_and_time()),
        ("app_hash".to_string(), get_app_hash()),
        ("app_version".to_string(), get_app_version()),
        ("report_hash".to_string(), report_hash),
        ("executer_username".to_string(), tally_executer_username),
    ]);

    Ok(PipeConfigGenerateReports {
        enable_pdfs: false,
        report_content_template,
        execution_annotations,
        system_template: report_system_template,
        pdf_options,
        extra_data: serde_json::to_value(extra_data)?,
    })
}

#[instrument(skip_all, err)]
pub async fn create_config_file(
    base_tally_path: PathBuf,
    report_content_template: Option<String>,
    report_system_template: String,
    pdf_options: Option<PrintToPdfOptionsLocal>,
    tally_session: &TallySession,
    tally_type: TallyType,
) -> Result<()> {
    let contest_encryption_policy = tally_session
        .configuration
        .clone()
        .unwrap_or_default()
        .get_contest_encryption_policy();
    let decoded_ballots_policy = tally_session
        .configuration
        .clone()
        .unwrap_or_default()
        .get_decoded_ballots_policy();
    let public_asset_path = get_public_assets_path_env_var()?;

    let minio_endpoint_base = s3::get_minio_url()?;

    let gen_report_pipe_config = build_reports_pipe_config(
        &tally_session,
        minio_endpoint_base,
        public_asset_path,
        report_content_template,
        report_system_template,
        pdf_options,
        tally_type,
    )
    .await?;

    let gen_db_pipe_config = PipeConfigGenerateDatabase {
        include_decoded_ballots: decoded_ballots_policy == DecodedBallotsInclusionPolicy::INCLUDED,
        tenant_id: tally_session.tenant_id.clone(),
        election_event_id: tally_session.election_event_id.clone(),
        database_filename: DATABASE_FILENAME.to_string(),
    };

    info!("FFF enable pdfs: {}", gen_report_pipe_config.enable_pdfs);

    let stages_def = {
        let mut map = HashMap::new();
        map.insert(
            "main".to_string(),
            velvet::config::Stage {
                pipeline: vec![
                    velvet::config::PipeConfig {
                        id: "decode-ballots".to_string(),
                        pipe: match contest_encryption_policy {
                            ContestEncryptionPolicy::MULTIPLE_CONTESTS => PipeName::DecodeMCBallots,
                            ContestEncryptionPolicy::SINGLE_CONTEST => PipeName::DecodeBallots,
                        },
                        config: Some(serde_json::Value::Null),
                    },
                    velvet::config::PipeConfig {
                        id: "do-tally".to_string(),
                        pipe: PipeName::DoTally,
                        config: Some(serde_json::Value::Null),
                    },
                    velvet::config::PipeConfig {
                        id: "mark-winners".to_string(),
                        pipe: PipeName::MarkWinners,
                        config: Some(serde_json::Value::Null),
                    },
                    velvet::config::PipeConfig {
                        id: "gen-report".to_string(),
                        pipe: PipeName::GenerateReports,
                        config: Some(serde_json::to_value(gen_report_pipe_config)?),
                    },
                    velvet::config::PipeConfig {
                        id: "gen-db".to_string(),
                        pipe: PipeName::GenerateDatabase,
                        config: Some(serde_json::to_value(gen_db_pipe_config)?),
                    },
                ],
            },
        );
        map
    };

    let stages = velvet::config::Stages {
        order: vec!["main".to_string()],
        stages_def,
    };

    let velvet_config = velvet::config::Config {
        version: "0.0.0".to_string(),
        stages,
    };

    let config_path = base_tally_path.join("velvet-config.json");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&config_path)?;

    writeln!(file, "{}", serde_json::to_string(&velvet_config)?)?;

    Ok(())
}

#[instrument(skip_all, err)]
async fn populate_sqlite_election_event_data(
    base_tempdir: &Path,
    hasura_transaction: &Transaction<'_>,
    tally_session: &TallySession,
) -> Result<String> {
    let document_id = Uuid::new_v4().to_string();
    let velvet_input_dir = base_tempdir.join("input");

    let base_database_path = velvet_input_dir.join(format!("{DEFAULT_DIR_DATABASE}/"));
    let database_path = base_database_path.join(format!("results.db"));

    let tenant_id = &tally_session.tenant_id;
    let election_event_id = &tally_session.election_event_id;
    let election_ids = tally_session.election_ids.clone();
    let areas_ids = tally_session.area_ids.clone();

    let database_path_ref = &database_path;

    task::block_in_place(move || -> anyhow::Result<()> {
        Handle::current().block_on(async move {
            // Make sure the directory exists
            fs::create_dir_all(&base_database_path)?;

            let mut sqlite_connection = Connection::open(database_path_ref)?;
            let sqlite_transaction = sqlite_connection.transaction()?;

            let election_event =
                get_election_event_by_id(hasura_transaction, tenant_id, election_event_id)
                    .await
                    .context("Failed to get election event by ID")?;
            create_election_event_sqlite(&sqlite_transaction, election_event)
                .await
                .context("Failed to create election event table")?;

            let elections = match election_ids.clone() {
                Some(ids) => {
                    get_elections_by_ids(hasura_transaction, tenant_id, election_event_id, &ids)
                        .await
                }
                None => get_elections(hasura_transaction, tenant_id, election_event_id, None).await,
            }
            .context("Failed to get elections")?;

            create_election_sqlite(&sqlite_transaction, elections)
                .await
                .context("Failed to create election table")?;

            let contests = match election_ids {
                Some(ids) => get_contest_by_election_ids(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    &ids,
                )
                .await
                .context("Failed to export contests")?,
                None => export_contests(hasura_transaction, tenant_id, election_event_id)
                    .await
                    .context("Failed to export contests")?,
            };

            create_contest_sqlite(&sqlite_transaction, contests.clone())
                .await
                .context("Failed to create contest table")?;

            let contests_ids: Vec<String> = contests.iter().map(|c| c.id.clone()).collect();

            // TODO Create csv with candidates

            create_candidate_sqlite(&sqlite_transaction)
                .await
                .context("Failed to create candidate table")?;

            let contests_csv_temp = NamedTempFile::new()
                .context("Failed to create temporary file for candidates csv")?;

            let contests_csv = contests_csv_temp.path();

            export_candidate_csv(
                hasura_transaction,
                contests_csv,
                &contests_ids,
                tenant_id,
                election_event_id,
            )
            .await
            .context("Failed exporting candidates to csv")?;

            import_candidate_sqlite(&sqlite_transaction, contests_csv)
                .await
                .context("Failed importing candidates to sqlite database")?;

            let areas = match areas_ids.clone() {
                Some(ids) => {
                    get_areas_by_ids(hasura_transaction, tenant_id, election_event_id, &ids)
                        .await
                        .context("Failed to get event areas by IDs")?
                }
                None => get_event_areas(hasura_transaction, tenant_id, election_event_id)
                    .await
                    .context("Failed to get event areas")?,
            };

            create_area_sqlite(&sqlite_transaction, areas)
                .await
                .context("Failed to create area table")?;

            let area_contests = match areas_ids {
                Some(ids) => get_area_contests_by_area_contest_ids(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    &ids,
                    &contests_ids,
                )
                .await
                .context("Failed to get areas contestby IDs")?,
                None => export_area_contests(hasura_transaction, tenant_id, election_event_id)
                    .await
                    .context("Failed to export area contests")?,
            };

            create_area_contest_sqlite(
                &sqlite_transaction,
                tenant_id,
                election_event_id,
                area_contests,
            )
            .await
            .context("Failed to create area contest table")?;

            sqlite_transaction.commit()?;
            Ok(())
        })
    })?;

    Ok(document_id)
}

#[instrument(skip_all, err)]
pub async fn run_velvet_tally(
    base_tally_path: PathBuf,
    area_contests: &Vec<AreaContestDataType>,
    cast_votes_count: &Vec<ElectionCastVotes>,
    tally_sheets: &Vec<TallySheet>,
    report_content_template: Option<String>,
    report_system_template: String,
    pdf_options: Option<PrintToPdfOptionsLocal>,
    areas: &Vec<Area>,
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
    tally_session: &TallySession,
    tally_type: TallyType,
) -> Result<State> {
    let basic_areas: Vec<TreeNodeArea> = areas.into_iter().map(|area| area.into()).collect();
    // map<(area_id,contest_id), tally_sheet>
    let tally_sheet_map = create_tally_sheets_map(tally_sheets);
    for area_contest in area_contests {
        prepare_tally_for_area_contest(
            base_tally_path.clone(),
            area_contest,
            &tally_sheet_map,
            tally_session,
        )?;
    }
    create_election_configs(
        base_tally_path.clone(),
        area_contests,
        cast_votes_count,
        &basic_areas,
        election_event,
    )
    .await?;

    let database_document_id = populate_sqlite_election_event_data(
        base_tally_path.as_path(),
        hasura_transaction,
        tally_session,
    )
    .await?;

    create_config_file(
        base_tally_path.clone(),
        report_content_template,
        report_system_template,
        pdf_options,
        tally_session,
        tally_type,
    )
    .await?;
    call_velvet(base_tally_path.clone(), "decode-ballots").await
}
