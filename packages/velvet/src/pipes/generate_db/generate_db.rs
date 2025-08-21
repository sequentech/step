// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rusqlite::Connection;
use sequent_core::types::hasura::core::{Area, TallySession};
use sequent_core::types::results::{
    ResultsAreaContest, ResultsAreaContestCandidate, ResultsContest, ResultsContestCandidate,
    ResultsElection,
};
use serde_json::json;
use tempfile::{NamedTempFile, TempPath};
use tracing::info;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::pipes::generate_reports::{ElectionReportDataComputed, GenerateReports};
use crate::pipes::pipe_inputs::{self, PipeInputs};
use crate::pipes::pipe_name::{PipeName, PipeNameOutputDir};
use crate::pipes::Pipe;
use core::cmp;
use sequent_core::sqlite::results_area_contest::create_results_area_contests_sqlite;
use sequent_core::sqlite::results_area_contest_candidate::create_results_area_contest_candidates_sqlite;
use sequent_core::sqlite::results_contest::create_results_contest_sqlite;
use sequent_core::sqlite::results_contest_candidate::create_results_contest_candidates_sqlite;
use sequent_core::sqlite::results_election::create_results_election_sqlite;
use sequent_core::sqlite::results_event::create_results_event_sqlite;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{instrument, warn};

use tokio::runtime::Runtime;

use crate::{
    pipes::error::{Error, Result},
    utils::parse_file,
};

use anyhow::anyhow;
use anyhow::Context;
use rusqlite::Transaction as SqliteTransaction;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PipeConfigGenerateDatabase {
    pub include_decoded_ballots: bool,
    pub tenant_id: String,
    pub election_event_id: String,
    pub database_filename: String,
}

impl PipeConfigGenerateDatabase {
    #[instrument(skip_all, name = "PipeConfigGenerateDatabase::new")]
    pub fn new() -> Self {
        Self::default()
    }
}

pub const DATABASE_FILENAME: &str = "results.db";

#[derive(Debug)]
pub struct GenerateDatabase {
    pub pipe_inputs: PipeInputs,
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
}

impl GenerateDatabase {
    #[instrument(skip_all, name = "GenerateDatabase::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        let input_dir = pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::GenerateReports.as_ref());
        let output_dir = pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::GenerateDatabase.as_ref());

        Self {
            pipe_inputs,
            input_dir,
            output_dir,
        }
    }

    #[instrument(skip_all)]
    pub fn get_config(&self) -> Result<PipeConfigGenerateDatabase> {
        let pipe_config: PipeConfigGenerateDatabase = self
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

impl Pipe for GenerateDatabase {
    #[instrument(err, skip_all, name = "GenerateDatabase::exec")]
    fn exec(&self) -> crate::pipes::error::Result<()> {
        // TODO review this figure out a better way to get reports
        let mut stage = self.pipe_inputs.stage.clone();
        stage.current_pipe = Some(PipeName::GenerateReports);
        let cli = self.pipe_inputs.cli.clone();
        let pipe_inputs = PipeInputs::new(cli, stage.clone())?;

        let gen_reports = GenerateReports::new(pipe_inputs);
        let reports = gen_reports.read_reports()?;

        let config = self.get_config()?;

        populate_results_tables(
            &self.pipe_inputs.root_path_database,
            &self.output_dir,
            reports,
            &config,
        )?;

        Ok(())
    }
}

#[instrument(skip(state_opt, config))]
pub fn populate_results_tables(
    input_database_path: &Path,
    output_database_path: &Path,
    state_opt: Vec<ElectionReportDataComputed>,
    config: &PipeConfigGenerateDatabase,
) -> Result<()> {
    let input_database_path = input_database_path.join(&config.database_filename);
    let database_path = output_database_path.join(&config.database_filename);

    let rt = Runtime::new()?;

    let _ = tokio::task::block_in_place(|| -> anyhow::Result<String> {
        let process_result = rt.block_on(async {
            // Make sure the directory exists
            fs::create_dir_all(&output_database_path)?;

            if fs::exists(&input_database_path)? {
                fs::copy(input_database_path, &database_path).map_err(|error| anyhow!("Could not copy file: {error}"))?;
            } else {
                warn!("No input database found. A new database will be created only with result tables.")
            }

            let mut sqlite_connection = Connection::open(&database_path)
                .map_err(|error| anyhow!("Error opening sqlite database: {error}"))?;
            let sqlite_transaction = sqlite_connection
                .transaction()
                .map_err(|error| anyhow!("Error starting sqlite database transaction: {error}"))?;

            if config.include_decoded_ballots {
                let parent_output_path = output_database_path.parent().ok_or(anyhow!("Invalid parent folder"))?;
                let decoded_ballots_path = parent_output_path.join(PipeNameOutputDir::DecodeBallots.as_ref());

                process_decoded_ballots(
                    &sqlite_transaction,
                    &decoded_ballots_path,
                ).await?;
            }

            let result = process_results_tables(
                state_opt,
                &config.tenant_id,
                &config.election_event_id,
                &sqlite_transaction,
            )
            .await;

            sqlite_transaction
                .commit()
                .map_err(|error| anyhow!("Error commiting sqlite database transaction: {error}"))?;

            result
        })?;

        Ok(process_result)
    })?;

    Ok(())
}

/// Processes decoded ballot files found within the specified path.
///
/// This function recursively traverses the `decoded_ballots_path` to find all
/// `decoded_ballots.json` files. For each file found, it extracts the
/// `election_id`, `contest_id`, and `area_id` from the directory structure
/// (e.g., `election__<id>/contest__<id>/area__<id>/decoded_ballots.json`).
///
/// The content of each `decoded_ballots.json` file is then read and inserted
/// into the `ballot` table in the provided SQLite transaction.
///
/// The `ballot` table is created if it does not already exist with the schema:
/// `(election_id TEXT NOT NULL, contest_id TEXT NOT NULL, area_id TEXT NOT NULL, decoded_ballot_json BLOB, PRIMARY KEY (election_id, contest_id, area_id))`
///
/// If a row with the same `(election_id, contest_id, area_id)` already exists,
/// it will be replaced (`INSERT OR REPLACE`).
///
/// # Arguments
/// * `sqlite_transaction` - A mutable reference to an active `rusqlite::Transaction`.
/// * `decoded_ballots_path` - The root path from which to start searching for ballot files.
///
/// # Returns
/// `Result<()>` - Returns `Ok(())` on success, or an `anyhow::Error` if any operation fails.
#[instrument(skip_all)]
pub async fn process_decoded_ballots(
    sqlite_transaction: &SqliteTransaction<'_>,
    decoded_ballots_path: &Path,
) -> anyhow::Result<()> {
    print!("-------------- {decoded_ballots_path:?}");

    // 1. Create the 'ballot' table if it does not already exist.
    // The table stores election, contest, and area IDs as text, and the JSON content as a BLOB.
    // The primary key ensures uniqueness for each combination of election, contest, and area.
    sqlite_transaction
        .execute(
            "CREATE TABLE IF NOT EXISTS ballot (
            election_id TEXT NOT NULL,
            contest_id TEXT NOT NULL,
            area_id TEXT NOT NULL,
            decoded_ballot_json BLOB,
            PRIMARY KEY (election_id, contest_id, area_id)
        );",
            [], // No parameters for table creation
        )
        .context("Failed to create 'ballot' table")?;

    // 2. Iterate through the directory structure to find 'decoded_ballots.json' files.
    // WalkDir provides an iterator that recursively goes through directories.
    for entry in WalkDir::new(decoded_ballots_path)
        .into_iter()
        .filter_map(|e| e.ok())
    // Filter out any errors during directory traversal
    {
        let path = entry.path();

        // Check if the current entry is a file and its name is "decoded_ballots.json".
        if path.is_file()
            && path
                .file_name()
                .map_or(false, |name| name == "decoded_ballots.json")
        {
            // A 'decoded_ballots.json' file has been found.
            tracing::info!("Found decoded_ballots.json at: {:?}", path);

            // Extract the election, contest, and area IDs from the file's path.
            // This helper function parses the directory names to get the required IDs.
            let (election_id, contest_id, area_id) = extract_ids_from_path(path)
                .with_context(|| format!("Failed to extract IDs from path: {:?}", path))?;

            // Read the entire content of the decoded_ballots.json file asynchronously.
            // The content will be stored as a byte vector (Vec<u8>), suitable for BLOB.
            let decoded_ballot_json_content = fs::read(path)
                .with_context(|| format!("Failed to read file content: {:?}", path))?;

            // Insert or replace the data into the 'ballot' table.
            // `INSERT OR REPLACE` is used to handle cases where the same ballot might be
            // processed multiple times, ensuring the latest content is always stored.
            sqlite_transaction.execute(
                "INSERT OR REPLACE INTO ballot (election_id, contest_id, area_id, decoded_ballot_json) VALUES (?, ?, ?, ?)",
                rusqlite::params![
                    election_id,                  // Parameter for election_id
                    contest_id,                   // Parameter for contest_id
                    area_id,                      // Parameter for area_id
                    decoded_ballot_json_content   // Parameter for decoded_ballot_json BLOB
                ],
            ).with_context(|| format!(
                "Failed to insert/replace ballot data for election: {}, contest: {}, area: {}",
                election_id, contest_id, area_id
            ))?;

            tracing::info!(
                "Successfully processed ballot for election: {}, contest: {}, area: {}",
                election_id,
                contest_id,
                area_id
            );
        }
    }

    Ok(())
}

/// Helper function to extract election, contest, and area IDs from a file path.
///
/// This function iterates through the components of a given `Path` and looks for
/// segments that start with "election__", "contest__", or "area__". It then
/// extracts the UUID part following these prefixes.
///
/// # Arguments
/// * `path` - A reference to the `Path` from which to extract IDs.
///
/// # Returns
/// `Result<(String, String, String)>` - A tuple containing (election_id, contest_id, area_id)
///                                      as Strings on success, or an `anyhow::Error` if any
///                                      of the required IDs cannot be found.
#[instrument(skip_all)]
fn extract_ids_from_path(path: &Path) -> anyhow::Result<(String, String, String)> {
    let mut election_id: Option<String> = None;
    let mut contest_id: Option<String> = None;
    let mut area_id: Option<String> = None;

    // Iterate over the components of the path (e.g., "election__uuid", "contest__uuid", etc.)
    for component in path.iter().map(|c| c.to_string_lossy()) {
        if let Some(id) = component.strip_prefix("election__") {
            election_id = Some(id.to_string());
        } else if let Some(id) = component.strip_prefix("contest__") {
            contest_id = Some(id.to_string());
        } else if let Some(id) = component.strip_prefix("area__") {
            area_id = Some(id.to_string());
        }
    }

    // Check if all three IDs were successfully extracted.
    match (election_id, contest_id, area_id) {
        (Some(e), Some(c), Some(a)) => Ok((e, c, a)),
        _ => Err(anyhow::anyhow!(
            "Could not extract all required IDs (election, contest, area) from path: {:?}",
            path
        )),
    }
}

#[instrument(skip_all)]
pub async fn process_results_tables(
    results: Vec<ElectionReportDataComputed>,
    tenant_id: &str,
    election_event_id: &str,
    sqlite_transaction: &SqliteTransaction<'_>,
) -> Result<String> {
    let results_event_id =
        generate_results_id_if_necessary(sqlite_transaction, tenant_id, election_event_id).await?;

    save_results(
        sqlite_transaction,
        results.clone(),
        tenant_id,
        election_event_id,
        &results_event_id,
    )
    .await?;

    Ok(results_event_id)
}

#[instrument(skip_all)]
pub async fn generate_results_id_if_necessary(
    sqlite_transaction: &SqliteTransaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<String> {
    let results_event_id = Uuid::new_v4().to_string();

    let results_event_id = create_results_event_sqlite(
        sqlite_transaction,
        tenant_id,
        election_event_id,
        &results_event_id,
    )
    .await
    .context("Failed to create results event table")?;
    Ok(results_event_id)
}

#[instrument(skip_all)]
pub async fn save_results(
    sqlite_transaction: &SqliteTransaction<'_>,
    results: Vec<ElectionReportDataComputed>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
) -> anyhow::Result<()> {
    let mut results_contests: Vec<ResultsContest> = Vec::new();
    let mut results_area_contests: Vec<ResultsAreaContest> = Vec::new();
    let mut results_elections: Vec<ResultsElection> = Vec::new();
    let mut results_contest_candidates: Vec<ResultsContestCandidate> = Vec::new();
    let mut results_area_contest_candidates: Vec<ResultsAreaContestCandidate> = Vec::new();
    for election in &results {
        let total_voters_percent: f64 =
            (election.total_votes as f64) / (cmp::max(election.census, 1) as f64);
        results_elections.push(ResultsElection {
            id: Uuid::new_v4().into(),
            tenant_id: tenant_id.into(),
            election_event_id: election_event_id.into(),
            election_id: election.election_id.clone(),
            results_event_id: results_event_id.into(),
            name: None,
            elegible_census: Some(election.census as i64),
            total_voters: Some(election.total_votes as i64),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            total_voters_percent: Some(total_voters_percent.clamp(0.0, 1.0).try_into()?),
            documents: None,
        });

        for contest in &election.reports {
            let total_votes_percent: f64 = contest.contest_result.percentage_total_votes / 100.0;
            let auditable_votes_percent: f64 =
                contest.contest_result.percentage_auditable_votes / 100.0;
            let total_valid_votes_percent: f64 =
                contest.contest_result.percentage_total_valid_votes / 100.0;
            let total_invalid_votes_percent: f64 =
                contest.contest_result.percentage_total_invalid_votes / 100.0;
            let explicit_invalid_votes_percent: f64 =
                contest.contest_result.percentage_invalid_votes_explicit / 100.0;
            let implicit_invalid_votes_percent: f64 =
                contest.contest_result.percentage_invalid_votes_implicit / 100.0;
            let total_blank_votes_percent: f64 =
                contest.contest_result.percentage_total_blank_votes / 100.0;

            let extended_metrics_value = serde_json::to_value(
                contest
                    .contest_result
                    .extended_metrics
                    .clone()
                    .unwrap_or_default(),
            )
            .expect("Failed to convert to JSON");
            let mut annotations = json!({});
            annotations["extended_metrics"] = extended_metrics_value;

            if let Some(area) = &contest.area {
                results_area_contests.push(ResultsAreaContest {
                    id: Uuid::new_v4().into(),
                    tenant_id: tenant_id.into(),
                    election_event_id: election_event_id.into(),
                    election_id: election.election_id.clone(),
                    contest_id: contest.contest.id.clone(),
                    area_id: area.id.clone(),
                    results_event_id: results_event_id.into(),
                    elegible_census: Some(contest.contest_result.census as i64),
                    total_votes: Some(contest.contest_result.total_votes as i64),
                    total_votes_percent: Some(total_votes_percent.clamp(0.0, 1.0).try_into()?),
                    total_auditable_votes: Some(contest.contest_result.auditable_votes as i64),
                    total_auditable_votes_percent: Some(
                        auditable_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    total_valid_votes: Some(contest.contest_result.total_valid_votes as i64),
                    total_valid_votes_percent: Some(
                        total_valid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    total_invalid_votes: Some(contest.contest_result.total_invalid_votes as i64),
                    total_invalid_votes_percent: Some(
                        total_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    explicit_invalid_votes: Some(
                        contest.contest_result.invalid_votes.explicit as i64,
                    ),
                    explicit_invalid_votes_percent: Some(
                        explicit_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    implicit_invalid_votes: Some(
                        contest.contest_result.invalid_votes.implicit as i64,
                    ),
                    implicit_invalid_votes_percent: Some(
                        implicit_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    blank_votes: Some(contest.contest_result.total_blank_votes as i64),
                    blank_votes_percent: Some(
                        total_blank_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    created_at: None,
                    last_updated_at: None,
                    labels: None,
                    annotations: Some(annotations),
                    documents: None,
                });

                let votes_base: f64 = cmp::max(
                    contest.contest_result.total_votes
                        - contest.contest_result.total_invalid_votes
                        - contest.contest_result.total_blank_votes,
                    1,
                ) as f64;

                for candidate in &contest.candidate_result {
                    let cast_votes_percent: f64 = (candidate.total_count as f64) / votes_base;
                    results_area_contest_candidates.push(ResultsAreaContestCandidate {
                        id: Uuid::new_v4().into(),
                        tenant_id: tenant_id.into(),
                        election_event_id: election_event_id.into(),
                        election_id: election.election_id.clone(),
                        contest_id: contest.contest.id.clone(),
                        candidate_id: candidate.candidate.id.clone(),
                        results_event_id: results_event_id.into(),
                        area_id: area.id.clone(),
                        cast_votes: Some(candidate.total_count as i64),
                        cast_votes_percent: Some(cast_votes_percent.clamp(0.0, 1.0).try_into()?),
                        winning_position: candidate.winning_position.map(|val| val as i64),
                        points: None,
                        created_at: None,
                        last_updated_at: None,
                        labels: None,
                        annotations: None,
                        documents: None,
                    });
                }
            } else {
                results_contests.push(ResultsContest {
                    id: Uuid::new_v4().into(),
                    tenant_id: tenant_id.into(),
                    election_event_id: election_event_id.into(),
                    election_id: election.election_id.clone(),
                    contest_id: contest.contest.id.clone(),
                    results_event_id: results_event_id.into(),
                    elegible_census: Some(contest.contest_result.census as i64),
                    total_valid_votes: Some(contest.contest_result.total_valid_votes as i64),
                    explicit_invalid_votes: Some(
                        contest.contest_result.invalid_votes.explicit as i64,
                    ),
                    implicit_invalid_votes: Some(
                        contest.contest_result.invalid_votes.implicit as i64,
                    ),
                    blank_votes: Some(contest.contest_result.total_blank_votes as i64),
                    voting_type: contest.contest.voting_type.clone(),
                    counting_algorithm: contest.contest.counting_algorithm.clone(),
                    name: contest.contest.name.clone(),
                    created_at: None,
                    last_updated_at: None,
                    labels: None,
                    annotations: Some(annotations),
                    total_invalid_votes: Some(contest.contest_result.total_invalid_votes as i64),
                    total_invalid_votes_percent: Some(
                        total_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    total_valid_votes_percent: Some(
                        total_valid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    explicit_invalid_votes_percent: Some(
                        explicit_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    implicit_invalid_votes_percent: Some(
                        implicit_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    blank_votes_percent: Some(
                        total_blank_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    total_votes: Some(contest.contest_result.total_votes as i64),
                    total_votes_percent: Some(total_votes_percent.clamp(0.0, 1.0).try_into()?),
                    documents: None,
                    total_auditable_votes: Some(contest.contest_result.auditable_votes as i64),
                    total_auditable_votes_percent: Some(
                        auditable_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                });

                let votes_base: f64 = cmp::max(
                    contest.contest_result.total_votes
                        - contest.contest_result.total_invalid_votes
                        - contest.contest_result.total_blank_votes,
                    1,
                ) as f64;

                for candidate in &contest.candidate_result {
                    let cast_votes_percent: f64 = (candidate.total_count as f64) / votes_base;
                    results_contest_candidates.push(ResultsContestCandidate {
                        id: Uuid::new_v4().into(),
                        tenant_id: tenant_id.into(),
                        election_event_id: election_event_id.into(),
                        election_id: election.election_id.clone(),
                        contest_id: contest.contest.id.clone(),
                        candidate_id: candidate.candidate.id.clone(),
                        results_event_id: results_event_id.into(),
                        cast_votes: Some(candidate.total_count as i64),
                        winning_position: candidate.winning_position.map(|val| val as i64),
                        points: None,
                        created_at: None,
                        last_updated_at: None,
                        labels: None,
                        annotations: None,
                        cast_votes_percent: Some(cast_votes_percent.clamp(0.0, 1.0).try_into()?),
                        documents: None,
                    });
                }
            }
        }
    }

    create_results_contest_sqlite(sqlite_transaction, results_contests).await?;

    create_results_area_contests_sqlite(sqlite_transaction, results_area_contests).await?;

    create_results_election_sqlite(sqlite_transaction, results_elections).await?;

    create_results_contest_candidates_sqlite(sqlite_transaction, results_contest_candidates)
        .await?;

    create_results_area_contest_candidates_sqlite(
        sqlite_transaction,
        results_area_contest_candidates,
    )
    .await?;

    Ok(())
}
