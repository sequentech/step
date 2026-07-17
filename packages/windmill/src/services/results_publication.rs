// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::{delete_documents, get_document};
use crate::postgres::election_event::{
    get_election_event_by_id, update_election_event_presentation,
};
use crate::postgres::tally_results_publication::{
    get_active_publication_for_route, get_publication_by_id, insert_publishing_publication,
    list_active_public_publications, list_superseded_publications, mark_publication_failed,
    mark_publication_published, mark_publication_superseded, revoke_publication,
    set_publication_finalization_error, validate_new_publication_source,
    NewTallyResultsPublication, TallyResultsPublication,
};
use crate::postgres::tally_session_execution::get_tally_session_execution_documents;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::documents::{
    get_document_as_temp_file, get_document_url, upload_and_return_document,
    upload_and_return_public_event_document,
};
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::tasks_execution::post;
use crate::types::results_publication::{
    ConfigureResultsWebsitePolicyInput, ConfigureResultsWebsitePolicyOutput,
    ContestPublicationState, FetchResultsArtifactInput, FetchResultsArtifactOutput,
    PublishResultsWebsiteInput, PublishResultsWebsiteOutput, RefreshResultsPublicationIndexInput,
    RefreshResultsPublicationIndexOutput, ResolveResultsPublicationInput,
    ResolveResultsPublicationOutput, ResultsManifestArtifact, ResultsManifestArtifacts,
    ResultsManifestContest, ResultsManifestCustomCss, ResultsPublicationDocuments,
    ResultsPublicationManifest, ResultsPublicationManifestDocument, ResultsPublicationStatus,
    ResultsRouteScope, RevokeResultsPublicationInput, RevokeResultsPublicationOutput,
};
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::{
    ContestIdString, ElectionIdString, ResultsPublicationAccessString, ResultsPublicationAction,
    ResultsPublicationDetails, ResultsPublicationIdString, ResultsPublicationRouteScopeString,
    ResultsPublicationVisibilityScopeString,
};
use rusqlite::{params_from_iter, Connection, OptionalExtension, ToSql};
use sequent_core::ballot::{
    ElectionEventPresentation, ElectionPresentation, ResultsWebsiteAccess, ResultsWebsitePolicy,
    ResultsWebsiteStatus, ResultsWebsiteVisibilityScope,
};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::s3;
use sequent_core::temp_path::{generate_temp_file, get_file_size};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use thiserror::Error;

const RESULTS_PORTAL_CLIENT_ID: &str = "results-portal";
const LEGACY_RESULTS_PORTAL_CLIENT_ID: &str = "voting-portal";

#[derive(Debug, Error)]
pub enum ResultsPublicationServiceError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Internal(#[from] anyhow::Error),
}

pub type ResultsPublicationServiceResult<T> =
    std::result::Result<T, ResultsPublicationServiceError>;

fn is_results_portal_client(client_id: &str) -> bool {
    matches!(
        client_id,
        RESULTS_PORTAL_CLIENT_ID | LEGACY_RESULTS_PORTAL_CLIENT_ID
    )
}

fn placeholders(count: usize) -> String {
    (0..count).map(|_| "?").collect::<Vec<_>>().join(",")
}

fn selected_contest_ids(publication: &TallyResultsPublication) -> Result<Vec<String>> {
    Ok(publication.published_contest_ids.clone())
}

fn results_publication_log_details(
    publication: &TallyResultsPublication,
    action: ResultsPublicationAction,
) -> ResultsPublicationDetails {
    ResultsPublicationDetails {
        publication_id: ResultsPublicationIdString(publication.id.clone()),
        action,
        route_scope: ResultsPublicationRouteScopeString(publication.route_scope.to_string()),
        route_election_id: ElectionIdString(publication.route_election_id.clone()),
        access: ResultsPublicationAccessString(publication.access.to_string()),
        visibility_scope: ResultsPublicationVisibilityScopeString(
            publication.visibility_scope.to_string(),
        ),
        contest_ids: publication
            .published_contest_ids
            .iter()
            .cloned()
            .map(ContestIdString)
            .collect(),
    }
}

pub async fn post_results_publication_action(
    tx: &Transaction<'_>,
    publication: &TallyResultsPublication,
    action: ResultsPublicationAction,
    user_id: &str,
    username: Option<String>,
) -> Result<()> {
    let election_event =
        get_election_event_by_id(tx, &publication.tenant_id, &publication.election_event_id)
            .await?;
    let board_name = get_election_event_board(election_event.bulletin_board_reference)
        .context("Missing electoral log board for results publication")?;
    let electoral_log = ElectoralLog::for_admin_user(
        tx,
        &board_name,
        &publication.tenant_id,
        &publication.election_event_id,
        user_id,
        username.clone(),
        Some(publication.election_ids.clone()),
        None,
    )
    .await?;

    electoral_log
        .post_results_publication_action(
            publication.election_event_id.clone(),
            results_publication_log_details(publication, action),
            Some(user_id.to_string()),
            username,
        )
        .await
        .context("Failed to post results publication action to the electoral log")
}

pub fn results_website_policy(
    presentation: &ElectionEventPresentation,
) -> Result<Option<ResultsWebsitePolicy>> {
    presentation
        .results_website
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .context("Invalid results website policy")
}

pub fn is_results_website_enabled(presentation: &ElectionEventPresentation) -> Result<bool> {
    Ok(results_website_policy(presentation)?
        .is_some_and(|policy| policy.status == ResultsWebsiteStatus::Enabled))
}

pub fn publication_matches_results_website_policy(
    presentation: &ElectionEventPresentation,
    publication: &TallyResultsPublication,
) -> Result<bool> {
    let Some(policy) = results_website_policy(presentation)? else {
        return Ok(false);
    };

    Ok(policy.status == ResultsWebsiteStatus::Enabled
        && policy.access == publication.access
        && policy.visibility_scope == publication.visibility_scope)
}

pub fn validate_results_website_policy(
    presentation: &ElectionEventPresentation,
    access: ResultsWebsiteAccess,
    visibility_scope: ResultsWebsiteVisibilityScope,
) -> Result<()> {
    let policy = results_website_policy(presentation)?
        .ok_or_else(|| anyhow!("Results website policy is not configured"))?;

    if policy.status != ResultsWebsiteStatus::Enabled {
        return Err(anyhow!(
            "Results website publishing is disabled for this election event"
        ));
    }
    if policy.access != access {
        return Err(anyhow!(
            "Results access does not match the election event results website policy"
        ));
    }
    if policy.visibility_scope != visibility_scope {
        return Err(anyhow!(
            "Results visibility does not match the election event results website policy"
        ));
    }

    Ok(())
}

pub async fn configure_results_website_policy(
    tx: &Transaction<'_>,
    tenant_id: &str,
    input: &ConfigureResultsWebsitePolicyInput,
) -> ResultsPublicationServiceResult<ConfigureResultsWebsitePolicyOutput> {
    input
        .validate()
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?;
    let election_event = get_election_event_by_id(tx, tenant_id, &input.election_event_id)
        .await
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?;
    let mut presentation = election_event
        .get_presentation()
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?
        .unwrap_or_default();
    presentation.results_website = Some(
        serde_json::to_string(&input.policy())
            .context("Failed to serialize results website policy")?,
    );
    update_election_event_presentation(
        tx,
        tenant_id,
        &input.election_event_id,
        serde_json::to_value(presentation)
            .context("Failed to serialize election event presentation")?,
    )
    .await?;
    Ok(ConfigureResultsWebsitePolicyOutput {
        election_event_id: input.election_event_id.clone(),
        status: input.status,
        access: input.access,
        visibility_scope: input.visibility_scope,
    })
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)",
        [table],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        == 1
}

const ELECTION_EVENT_PUBLIC_COLUMNS: &[&str] =
    &["id", "labels", "description", "presentation", "external_id"];
const ELECTION_PUBLIC_COLUMNS: &[&str] = &[
    "id",
    "election_event_id",
    "labels",
    "description",
    "presentation",
    "external_id",
];
const CONTEST_PUBLIC_COLUMNS: &[&str] = &[
    "id",
    "election_event_id",
    "election_id",
    "labels",
    "is_acclaimed",
    "is_active",
    "description",
    "presentation",
    "min_votes",
    "max_votes",
    "voting_type",
    "counting_algorithm",
    "winning_candidates_num",
    "external_id",
];
const CANDIDATE_PUBLIC_COLUMNS: &[&str] = &[
    "id",
    "election_event_id",
    "contest_id",
    "labels",
    "description",
    "type",
    "presentation",
    "is_public",
    "external_id",
];
const AREA_PUBLIC_COLUMNS: &[&str] = &[
    "id",
    "election_event_id",
    "labels",
    "name",
    "description",
    "type",
    "parent_id",
    "presentation",
];
const RESULTS_EVENT_PUBLIC_COLUMNS: &[&str] = &["id", "election_event_id", "name", "labels"];
const RESULTS_ELECTION_PUBLIC_COLUMNS: &[&str] = &[
    "id",
    "election_event_id",
    "election_id",
    "results_event_id",
    "name",
    "elegible_census",
    "total_voters",
    "labels",
    "total_voters_percent",
];
const RESULTS_ELECTION_AREA_PUBLIC_COLUMNS: &[&str] = &[
    "id",
    "election_event_id",
    "election_id",
    "area_id",
    "results_event_id",
    "name",
];
const RESULTS_CONTEST_PUBLIC_COLUMNS: &[&str] = &[
    "id",
    "election_event_id",
    "election_id",
    "contest_id",
    "results_event_id",
    "elegible_census",
    "total_valid_votes",
    "explicit_invalid_votes",
    "implicit_invalid_votes",
    "total_blank_votes",
    "voting_type",
    "counting_algorithm",
    "name",
    "labels",
    "annotations",
    "total_invalid_votes",
    "total_invalid_votes_percent",
    "total_valid_votes_percent",
    "explicit_invalid_votes_percent",
    "implicit_invalid_votes_percent",
    "total_blank_votes_percent",
    "total_votes",
    "total_votes_percent",
    "total_auditable_votes",
    "total_auditable_votes_percent",
    "explicit_blank_votes",
    "implicit_blank_votes",
    "explicit_blank_votes_percent",
    "implicit_blank_votes_percent",
];
const RESULTS_CONTEST_CANDIDATE_PUBLIC_COLUMNS: &[&str] = &[
    "id",
    "election_event_id",
    "election_id",
    "contest_id",
    "candidate_id",
    "results_event_id",
    "cast_votes",
    "winning_position",
    "points",
    "labels",
    "cast_votes_percent",
];
const RESULTS_AREA_CONTEST_PUBLIC_COLUMNS: &[&str] = &[
    "id",
    "election_event_id",
    "election_id",
    "contest_id",
    "area_id",
    "results_event_id",
    "elegible_census",
    "total_valid_votes",
    "explicit_invalid_votes",
    "implicit_invalid_votes",
    "total_blank_votes",
    "labels",
    "annotations",
    "total_valid_votes_percent",
    "total_invalid_votes",
    "total_invalid_votes_percent",
    "explicit_invalid_votes_percent",
    "total_blank_votes_percent",
    "implicit_invalid_votes_percent",
    "total_votes",
    "total_votes_percent",
    "total_auditable_votes",
    "total_auditable_votes_percent",
    "explicit_blank_votes",
    "implicit_blank_votes",
    "explicit_blank_votes_percent",
    "implicit_blank_votes_percent",
];
const RESULTS_AREA_CONTEST_CANDIDATE_PUBLIC_COLUMNS: &[&str] = &[
    "id",
    "election_event_id",
    "election_id",
    "contest_id",
    "area_id",
    "candidate_id",
    "results_event_id",
    "cast_votes",
    "winning_position",
    "points",
    "labels",
    "cast_votes_percent",
];

fn publication_columns(table: &str) -> Option<&'static [&'static str]> {
    match table {
        "election_event" => Some(ELECTION_EVENT_PUBLIC_COLUMNS),
        "election" => Some(ELECTION_PUBLIC_COLUMNS),
        "contest" => Some(CONTEST_PUBLIC_COLUMNS),
        "candidate" => Some(CANDIDATE_PUBLIC_COLUMNS),
        "area" => Some(AREA_PUBLIC_COLUMNS),
        "results_event" => Some(RESULTS_EVENT_PUBLIC_COLUMNS),
        "results_election" => Some(RESULTS_ELECTION_PUBLIC_COLUMNS),
        "results_election_area" => Some(RESULTS_ELECTION_AREA_PUBLIC_COLUMNS),
        "results_contest" => Some(RESULTS_CONTEST_PUBLIC_COLUMNS),
        "results_contest_candidate" => Some(RESULTS_CONTEST_CANDIDATE_PUBLIC_COLUMNS),
        "results_area_contest" => Some(RESULTS_AREA_CONTEST_PUBLIC_COLUMNS),
        "results_area_contest_candidate" => Some(RESULTS_AREA_CONTEST_CANDIDATE_PUBLIC_COLUMNS),
        _ => None,
    }
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn publication_source_columns(conn: &Connection, table: &str) -> Result<Option<HashSet<String>>> {
    let exists = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM publication_source.sqlite_master \
         WHERE type = 'table' AND name = ?)",
        [table],
        |row| row.get::<_, i64>(0),
    )? == 1;
    if !exists {
        return Ok(None);
    }

    let mut statement = conn.prepare(&format!(
        "PRAGMA publication_source.table_info({})",
        quote_identifier(table)
    ))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<HashSet<_>>>()?;
    Ok(Some(columns))
}

fn copy_table_rows(
    conn: &Connection,
    table: &str,
    where_clause: &str,
    params: Vec<&dyn ToSql>,
) -> Result<()> {
    let allowed_columns = publication_columns(table)
        .ok_or_else(|| anyhow!("Table {table} is not allowed in published results"))?;
    let Some(source_columns) = publication_source_columns(conn, table)? else {
        return Ok(());
    };
    let selected_columns = allowed_columns
        .iter()
        .copied()
        .filter(|column| source_columns.contains(*column))
        .map(quote_identifier)
        .collect::<Vec<_>>();
    if selected_columns.is_empty() {
        return Err(anyhow!(
            "Published results table {table} has no allowed columns"
        ));
    }

    let table = quote_identifier(table);
    let columns = selected_columns.join(", ");
    conn.execute_batch(&format!("CREATE TABLE main.{table} ({columns})"))?;
    let sql = format!(
        "INSERT INTO main.{table} ({columns}) SELECT {columns} FROM publication_source.{table} WHERE {where_clause}"
    );
    conn.execute(&sql, params_from_iter(params))?;
    Ok(())
}

fn in_filter(column: &str, values: &[String]) -> String {
    if values.is_empty() {
        "0".to_string()
    } else {
        format!("{column} IN ({})", placeholders(values.len()))
    }
}

fn as_sql_params(values: &[String]) -> Vec<&dyn ToSql> {
    values.iter().map(|value| value as &dyn ToSql).collect()
}

fn copy_filtered_sqlite(
    source_path: &Path,
    publication: &TallyResultsPublication,
    selected_contests: &[String],
    area_id: Option<&str>,
) -> Result<NamedTempFile> {
    let target = generate_temp_file("results-publication", ".sqlite")?;
    let conn = Connection::open(target.path())?;
    let source_path_string = source_path.to_string_lossy();
    conn.execute(
        "ATTACH DATABASE ? AS publication_source",
        [source_path_string.as_ref()],
    )?;

    copy_table_rows(
        &conn,
        "election_event",
        "id = ?",
        vec![&publication.election_event_id],
    )?;
    copy_table_rows(
        &conn,
        "election",
        &in_filter("id", &publication.election_ids),
        as_sql_params(&publication.election_ids),
    )?;
    copy_table_rows(
        &conn,
        "contest",
        &in_filter("id", selected_contests),
        as_sql_params(selected_contests),
    )?;
    copy_table_rows(
        &conn,
        "candidate",
        &in_filter("contest_id", selected_contests),
        as_sql_params(selected_contests),
    )?;

    let area_id_param = area_id.map(str::to_string);
    let area_ids = match area_id_param.as_ref() {
        Some(area_id) => vec![area_id.clone()],
        None => query_area_ids(
            source_path,
            selected_contests,
            &publication.results_event_id,
        )?,
    };
    copy_table_rows(
        &conn,
        "area",
        &in_filter("id", &area_ids),
        as_sql_params(&area_ids),
    )?;
    copy_table_rows(
        &conn,
        "results_event",
        "id = ?",
        vec![&publication.results_event_id],
    )?;

    let election_filter = in_filter("election_id", &publication.election_ids);
    let results_election_filter = format!("{election_filter} AND results_event_id = ?");
    let mut results_election_params = as_sql_params(&publication.election_ids);
    results_election_params.push(&publication.results_event_id);
    if area_id.is_none() {
        copy_table_rows(
            &conn,
            "results_election",
            &results_election_filter,
            results_election_params,
        )?;
    } else {
        copy_table_rows(&conn, "results_election", "0", vec![])?;
    }

    let area_election_filter = if area_id.is_some() {
        format!("{results_election_filter} AND area_id = ?")
    } else {
        results_election_filter
    };
    let mut area_election_params = as_sql_params(&publication.election_ids);
    area_election_params.push(&publication.results_event_id);
    if let Some(area_id) = area_id_param.as_ref() {
        area_election_params.push(area_id as &dyn ToSql);
    }
    copy_table_rows(
        &conn,
        "results_election_area",
        &area_election_filter,
        area_election_params,
    )?;

    for table in ["results_contest", "results_contest_candidate"] {
        let mut filter = if area_id.is_some() {
            "0".to_string()
        } else {
            format!(
                "{} AND results_event_id = ?",
                in_filter("contest_id", selected_contests)
            )
        };
        let mut params = if area_id.is_some() {
            vec![]
        } else {
            as_sql_params(selected_contests)
        };
        if area_id.is_none() {
            params.push(&publication.results_event_id);
        }
        if table == "results_contest_candidate" && area_id.is_none() {
            filter.push_str(" AND candidate_id IN (SELECT id FROM main.candidate)");
        }
        copy_table_rows(&conn, table, &filter, params)?;
    }

    for table in ["results_area_contest", "results_area_contest_candidate"] {
        let mut filter = in_filter("contest_id", selected_contests);
        let mut params = as_sql_params(selected_contests);
        filter.push_str(" AND results_event_id = ?");
        params.push(&publication.results_event_id);
        if table == "results_area_contest_candidate" {
            filter.push_str(" AND candidate_id IN (SELECT id FROM main.candidate)");
        }
        if let Some(area_id) = area_id_param.as_ref() {
            filter.push_str(" AND area_id = ?");
            params.push(area_id as &dyn ToSql);
        }
        copy_table_rows(&conn, table, &filter, params)?;
    }

    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(anyhow!(
            "Published results SQLite failed integrity check: {integrity}"
        ));
    }
    conn.execute_batch("DETACH DATABASE publication_source; PRAGMA optimize;")?;

    Ok(target)
}

#[derive(Clone, Copy)]
enum ManifestContestSource {
    Contest,
    ResultsContest,
}

impl ManifestContestSource {
    fn table(self) -> &'static str {
        match self {
            Self::Contest => "contest",
            Self::ResultsContest => "results_contest",
        }
    }

    fn contest_id_column(self) -> &'static str {
        match self {
            Self::Contest => "id",
            Self::ResultsContest => "contest_id",
        }
    }
}

fn query_manifest_contests_from_source(
    conn: &Connection,
    source: ManifestContestSource,
    election_ids: &[String],
    selected_contests: &[String],
) -> Result<Vec<ResultsManifestContest>> {
    let table = source.table();
    let contest_id_column = source.contest_id_column();
    if !table_exists(conn, table) {
        return Ok(vec![]);
    }

    let sql = format!(
        "SELECT DISTINCT {contest_id_column} AS contest_id, election_id FROM {table} WHERE election_id IN ({}) AND {contest_id_column} IN ({}) ORDER BY election_id, contest_id",
        placeholders(election_ids.len()),
        placeholders(selected_contests.len())
    );
    let params = election_ids
        .iter()
        .chain(selected_contests.iter())
        .map(|id| id as &dyn ToSql);
    let mut statement = conn.prepare(&sql)?;
    let mut rows = statement.query(params_from_iter(params))?;
    let mut contests = Vec::new();

    while let Some(row) = rows.next()? {
        contests.push(ResultsManifestContest {
            election_id: row.get("election_id")?,
            contest_id: row.get("contest_id")?,
            area_id: None,
            publication_state: ContestPublicationState::Published,
            positions: None,
        });
    }

    Ok(contests)
}

fn query_manifest_contests(
    source_path: &Path,
    publication: &TallyResultsPublication,
    selected_contests: &[String],
) -> Result<Vec<ResultsManifestContest>> {
    let conn = Connection::open(source_path)?;
    let election_ids = &publication.election_ids;

    if election_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut contests = query_manifest_contests_from_source(
        &conn,
        ManifestContestSource::Contest,
        election_ids,
        selected_contests,
    )?;
    if contests.is_empty() {
        contests = query_manifest_contests_from_source(
            &conn,
            ManifestContestSource::ResultsContest,
            election_ids,
            selected_contests,
        )?;
    }

    Ok(contests)
}

fn normalize_css(css: Option<String>) -> Option<String> {
    css.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn election_event_css_from_presentation(presentation: Option<String>) -> Result<Option<String>> {
    presentation
        .map(|value| serde_json::from_str::<ElectionEventPresentation>(&value))
        .transpose()
        .context("Invalid election event presentation in results database")
        .map(|presentation| presentation.and_then(|presentation| normalize_css(presentation.css)))
}

fn election_css_from_presentation(presentation: Option<String>) -> Result<Option<String>> {
    presentation
        .map(|value| serde_json::from_str::<ElectionPresentation>(&value))
        .transpose()
        .context("Invalid election presentation in results database")
        .map(|presentation| presentation.and_then(|presentation| normalize_css(presentation.css)))
}

fn query_manifest_custom_css(
    source_path: &Path,
    publication: &TallyResultsPublication,
) -> Result<ResultsManifestCustomCss> {
    let conn = Connection::open(source_path)?;

    let election_event_css = if table_exists(&conn, "election_event") {
        let presentation = conn
            .query_row(
                "SELECT presentation FROM election_event WHERE id = ? LIMIT 1",
                [&publication.election_event_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();

        election_event_css_from_presentation(presentation)?
    } else {
        None
    };

    let mut election_css = HashMap::new();
    if table_exists(&conn, "election") && !publication.election_ids.is_empty() {
        let sql = format!(
            "SELECT id, presentation FROM election WHERE id IN ({})",
            placeholders(publication.election_ids.len())
        );
        let params = publication.election_ids.iter().map(|id| id as &dyn ToSql);
        let mut statement = conn.prepare(&sql)?;
        let mut rows = statement.query(params_from_iter(params))?;

        while let Some(row) = rows.next()? {
            let election_id: String = row.get("id")?;
            let presentation: Option<String> = row.get("presentation")?;

            if let Some(css) = election_css_from_presentation(presentation)? {
                election_css.insert(election_id, Some(css));
            }
        }
    }

    Ok(ResultsManifestCustomCss {
        election_event: election_event_css,
        elections: election_css,
    })
}

#[derive(Clone)]
struct ManifestLanguageConfig {
    default_locale: String,
    available_languages: Vec<String>,
}

fn normalize_language_config(
    default_locale: Option<&str>,
    available_languages: Option<&Vec<String>>,
) -> ManifestLanguageConfig {
    let default_locale = default_locale
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("en")
        .to_string();

    let mut languages = Vec::new();
    if let Some(values) = available_languages {
        for value in values {
            let language = value.trim();
            if language.is_empty() {
                continue;
            }
            if !languages.iter().any(|existing| existing == language) {
                languages.push(language.to_string());
            }
        }
    }

    if languages.is_empty() {
        languages.push(default_locale.clone());
    }

    ManifestLanguageConfig {
        default_locale,
        available_languages: languages,
    }
}

fn language_config_from_presentation(
    presentation: Option<String>,
) -> Result<ManifestLanguageConfig> {
    let Some(presentation) = presentation else {
        return Ok(normalize_language_config(None, None));
    };
    let parsed = serde_json::from_str::<ElectionEventPresentation>(&presentation)
        .context("Invalid election event presentation in results database")?;
    let default_locale = parsed
        .language_conf
        .as_ref()
        .and_then(|config| config.default_language_code.as_deref());
    let available_languages = parsed
        .language_conf
        .as_ref()
        .and_then(|config| config.enabled_language_codes.as_ref());

    Ok(normalize_language_config(
        default_locale,
        available_languages,
    ))
}

fn query_manifest_language_config(
    source_path: &Path,
    publication: &TallyResultsPublication,
) -> Result<ManifestLanguageConfig> {
    let conn = Connection::open(source_path)?;

    if !table_exists(&conn, "election_event") {
        return Ok(normalize_language_config(None, None));
    }

    let presentation = conn
        .query_row(
            "SELECT presentation FROM election_event WHERE id = ? LIMIT 1",
            [&publication.election_event_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();

    language_config_from_presentation(presentation)
}

fn query_area_ids(
    source_path: &Path,
    selected_contests: &[String],
    results_event_id: &str,
) -> Result<Vec<String>> {
    if selected_contests.is_empty() {
        return Ok(vec![]);
    }

    let conn = Connection::open(source_path)?;
    if !table_exists(&conn, "results_area_contest") {
        return Ok(vec![]);
    }

    let sql = format!(
        "SELECT DISTINCT area_id FROM results_area_contest WHERE contest_id IN ({}) AND results_event_id = ? ORDER BY area_id",
        placeholders(selected_contests.len())
    );
    let params = selected_contests
        .iter()
        .map(|id| id as &dyn ToSql)
        .chain(std::iter::once(&results_event_id as &dyn ToSql));
    let mut statement = conn.prepare(&sql)?;
    let mut rows = statement.query(params_from_iter(params))?;
    let mut area_ids = Vec::new();

    while let Some(row) = rows.next()? {
        area_ids.push(row.get("area_id")?);
    }

    Ok(area_ids)
}

fn write_json_file(prefix: &str, value: &Value) -> Result<(NamedTempFile, String, u64)> {
    let file = generate_temp_file(prefix, ".json")?;
    fs::write(file.path(), serde_json::to_vec_pretty(value)?)?;
    let path = file.path().to_string_lossy().to_string();
    let size = get_file_size(&path)?;
    Ok((file, path, size))
}

fn event_public_path(tenant_id: &str, election_event_id: &str, name: &str) -> String {
    s3::get_public_election_event_document_name_key(tenant_id, election_event_id, name)
}

async fn upload_public_json_key(key: &str, value: &Value) -> Result<()> {
    let (_file, path, _size) = write_json_file("results-public-index", value)?;
    s3::upload_file_to_s3(
        key.to_string(),
        false,
        s3::get_public_bucket()?,
        "application/json".to_string(),
        path,
        Some("no-store, max-age=0".to_string()),
        Some("results-index.json".to_string()),
    )
    .await?;
    Ok(())
}

async fn delete_public_results_artifacts(tenant_id: &str, election_event_id: &str) -> Result<()> {
    let prefix = event_public_path(tenant_id, election_event_id, "results");
    s3::delete_files_from_s3(s3::get_public_bucket()?, prefix, s3::S3Endpoint::Server).await
}

pub async fn delete_public_publication_route_artifacts(
    publication: &TallyResultsPublication,
) -> Result<()> {
    delete_public_publication_artifacts(publication, true).await
}

fn collect_publication_document_ids(value: &Value, ids: &mut HashSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(document_id) = object.get("document_id").and_then(Value::as_str) {
                ids.insert(document_id.to_string());
            }
            for value in object.values() {
                collect_publication_document_ids(value, ids);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_publication_document_ids(value, ids);
            }
        }
        _ => {}
    }
}

fn collect_publication_public_paths(
    value: &Value,
    include_latest: bool,
    paths: &mut HashSet<String>,
) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if (key == "public_path" || (include_latest && key == "latest_public_path"))
                    && value.is_string()
                {
                    paths.insert(value.as_str().unwrap_or_default().to_string());
                }
                collect_publication_public_paths(value, include_latest, paths);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_publication_public_paths(value, include_latest, paths);
            }
        }
        _ => {}
    }
}

fn expected_publication_public_paths(
    publication: &TallyResultsPublication,
    include_latest: bool,
) -> HashSet<String> {
    let base = publication_base_path(publication);
    let mut paths = HashSet::from([
        event_public_path(
            &publication.tenant_id,
            &publication.election_event_id,
            &format!("{base}/full-v{}.sqlite", publication.version),
        ),
        event_public_path(
            &publication.tenant_id,
            &publication.election_event_id,
            &format!("{base}/manifest-v{}.json", publication.version),
        ),
    ]);
    if include_latest {
        paths.insert(event_public_path(
            &publication.tenant_id,
            &publication.election_event_id,
            &format!("{base}/manifest-latest.json"),
        ));
    }
    paths
}

async fn delete_public_publication_artifacts(
    publication: &TallyResultsPublication,
    include_latest: bool,
) -> Result<()> {
    let mut paths = HashSet::new();
    collect_publication_public_paths(&publication.documents, include_latest, &mut paths);
    paths.extend(expected_publication_public_paths(
        publication,
        include_latest,
    ));

    for path in paths {
        s3::delete_files_from_s3(s3::get_public_bucket()?, path, s3::S3Endpoint::Server).await?;
    }
    Ok(())
}

async fn delete_publication_artifacts_internal(
    tx: &Transaction<'_>,
    publication: &TallyResultsPublication,
    include_latest: bool,
) -> Result<()> {
    let mut document_ids = HashSet::new();
    collect_publication_document_ids(&publication.documents, &mut document_ids);

    if publication.access == ResultsWebsiteAccess::Public {
        delete_public_publication_artifacts(publication, include_latest).await?;
    } else {
        for document_id in &document_ids {
            let Some(document) = get_document(
                tx,
                &publication.tenant_id,
                Some(publication.election_event_id.clone()),
                document_id,
            )
            .await?
            else {
                continue;
            };
            let key = s3::get_document_key(
                &publication.tenant_id,
                Some(&publication.election_event_id),
                document_id,
                document.name.as_deref().unwrap_or_default(),
            );
            s3::delete_files_from_s3(s3::get_private_bucket()?, key, s3::S3Endpoint::Server)
                .await?;
        }
    }

    let document_ids = document_ids.into_iter().collect::<Vec<_>>();
    delete_documents(
        tx,
        &publication.tenant_id,
        &publication.election_event_id,
        &document_ids,
    )
    .await?;
    Ok(())
}

pub async fn delete_publication_artifacts(
    tx: &Transaction<'_>,
    publication: &TallyResultsPublication,
) -> Result<()> {
    delete_publication_artifacts_internal(tx, publication, true).await
}

async fn delete_superseded_publication_artifacts(
    tx: &Transaction<'_>,
    publication: &TallyResultsPublication,
) -> Result<()> {
    delete_publication_artifacts_internal(tx, publication, false).await
}

async fn source_sqlite_file(
    tx: &Transaction<'_>,
    publication: &TallyResultsPublication,
) -> Result<NamedTempFile> {
    let documents = get_tally_session_execution_documents(
        tx,
        &publication.tenant_id,
        &publication.election_event_id,
        &publication.tally_session_execution_id,
    )
    .await?
    .ok_or_else(|| anyhow!("No tally execution documents found"))?;
    let sqlite_document_id = documents
        .sqlite
        .ok_or_else(|| anyhow!("No SQLite document found on tally execution"))?;
    let document = get_document(
        tx,
        &publication.tenant_id,
        Some(publication.election_event_id.clone()),
        &sqlite_document_id,
    )
    .await?
    .ok_or_else(|| anyhow!("SQLite document not found"))?;

    get_document_as_temp_file(&publication.tenant_id, &document).await
}

fn publication_base_path(publication: &TallyResultsPublication) -> String {
    match publication.route_scope {
        ResultsRouteScope::Election => format!(
            "results/elections/{}",
            publication.route_election_id.clone().unwrap_or_default()
        ),
        ResultsRouteScope::Event => "results".to_string(),
    }
}

fn build_manifest(
    publication: &TallyResultsPublication,
    contests: Vec<ResultsManifestContest>,
    artifacts: ResultsManifestArtifacts,
    custom_css: ResultsManifestCustomCss,
    language_config: &ManifestLanguageConfig,
) -> ResultsPublicationManifest {
    ResultsPublicationManifest {
        schema_version: 1,
        tenant_id: publication.tenant_id.clone(),
        election_event_id: publication.election_event_id.clone(),
        election_ids: publication.election_ids.clone(),
        route_scope: publication.route_scope,
        route_election_id: publication.route_election_id.clone(),
        publication_id: publication.id.clone(),
        tally_session_id: publication.tally_session_id.clone(),
        tally_session_execution_id: publication.tally_session_execution_id.clone(),
        results_event_id: publication.results_event_id.clone(),
        version: publication.version,
        access: publication.access,
        visibility_scope: publication.visibility_scope,
        default_locale: Some(language_config.default_locale.clone()),
        available_languages: language_config.available_languages.clone(),
        title: HashMap::from([("en".to_string(), "Election Results".to_string())]),
        custom_css,
        contests,
        artifacts,
    }
}

async fn publish_public_artifacts(
    tx: &Transaction<'_>,
    publication: &TallyResultsPublication,
    source_path: &Path,
    selected_contests: &[String],
    contests: Vec<ResultsManifestContest>,
    custom_css: ResultsManifestCustomCss,
    language_config: &ManifestLanguageConfig,
) -> Result<(Value, ResultsPublicationManifest)> {
    let base = publication_base_path(publication);
    let sqlite_name = format!("{base}/full-v{}.sqlite", publication.version);
    let manifest_name = format!("{base}/manifest-v{}.json", publication.version);
    let latest_manifest_name = format!("{base}/manifest-latest.json");
    let sqlite = copy_filtered_sqlite(source_path, publication, selected_contests, None)?;
    let sqlite_path = sqlite.path().to_string_lossy().to_string();
    let sqlite_size = get_file_size(&sqlite_path)?;
    let sqlite_document = upload_and_return_public_event_document(
        tx,
        &sqlite_path,
        sqlite_size,
        "application/x-sqlite3",
        &publication.tenant_id,
        &publication.election_event_id,
        &sqlite_name,
        None,
    )
    .await?;

    let full_sqlite = ResultsManifestArtifact {
        document_id: Some(sqlite_document.id),
        public_path: Some(event_public_path(
            &publication.tenant_id,
            &publication.election_event_id,
            &sqlite_name,
        )),
    };
    let artifacts = ResultsManifestArtifacts {
        full_sqlite: Some(full_sqlite.clone()),
        areas: None,
    };
    let manifest = build_manifest(
        publication,
        contests,
        artifacts.clone(),
        custom_css,
        language_config,
    );
    let manifest_value = serde_json::to_value(&manifest)?;
    let (_manifest_file, manifest_path, manifest_size) =
        write_json_file("results-manifest", &manifest_value)?;

    let manifest_document = upload_and_return_public_event_document(
        tx,
        &manifest_path,
        manifest_size,
        "application/json",
        &publication.tenant_id,
        &publication.election_event_id,
        &manifest_name,
        None,
    )
    .await?;
    let documents = ResultsPublicationDocuments {
        manifest: Some(ResultsPublicationManifestDocument {
            document_id: Some(manifest_document.id),
            public_path: Some(event_public_path(
                &publication.tenant_id,
                &publication.election_event_id,
                &manifest_name,
            )),
            latest_public_path: Some(event_public_path(
                &publication.tenant_id,
                &publication.election_event_id,
                &latest_manifest_name,
            )),
        }),
        full_sqlite: Some(full_sqlite),
        area_sqlite: None,
    };

    Ok((serde_json::to_value(documents)?, manifest))
}

async fn publish_private_artifacts(
    tx: &Transaction<'_>,
    publication: &TallyResultsPublication,
    source_path: &Path,
    selected_contests: &[String],
    contests: Vec<ResultsManifestContest>,
    custom_css: ResultsManifestCustomCss,
    language_config: &ManifestLanguageConfig,
) -> Result<(Value, ResultsPublicationManifest)> {
    if publication.visibility_scope == ResultsWebsiteVisibilityScope::AreaBased {
        let mut area_documents = HashMap::new();
        for area_id in query_area_ids(
            source_path,
            selected_contests,
            &publication.results_event_id,
        )? {
            let sqlite =
                copy_filtered_sqlite(source_path, publication, selected_contests, Some(&area_id))?;
            let sqlite_path = sqlite.path().to_string_lossy().to_string();
            let sqlite_size = get_file_size(&sqlite_path)?;
            let document = upload_and_return_document(
                tx,
                &sqlite_path,
                sqlite_size,
                "application/x-sqlite3",
                &publication.tenant_id,
                Some(publication.election_event_id.clone()),
                &format!("results-area-{area_id}-v{}.sqlite", publication.version),
                None,
                false,
            )
            .await?;
            area_documents.insert(
                area_id,
                ResultsManifestArtifact {
                    document_id: Some(document.id),
                    public_path: None,
                },
            );
        }

        let artifacts = ResultsManifestArtifacts {
            full_sqlite: None,
            areas: Some(area_documents.clone()),
        };
        let manifest = build_manifest(
            publication,
            contests,
            artifacts,
            custom_css,
            language_config,
        );
        let documents = ResultsPublicationDocuments {
            area_sqlite: Some(area_documents),
            ..Default::default()
        };
        return Ok((serde_json::to_value(documents)?, manifest));
    }

    let sqlite = copy_filtered_sqlite(source_path, publication, selected_contests, None)?;
    let sqlite_path = sqlite.path().to_string_lossy().to_string();
    let sqlite_size = get_file_size(&sqlite_path)?;
    let document = upload_and_return_document(
        tx,
        &sqlite_path,
        sqlite_size,
        "application/x-sqlite3",
        &publication.tenant_id,
        Some(publication.election_event_id.clone()),
        &format!("results-full-v{}.sqlite", publication.version),
        None,
        false,
    )
    .await?;

    let full_sqlite = ResultsManifestArtifact {
        document_id: Some(document.id),
        public_path: None,
    };
    let artifacts = ResultsManifestArtifacts {
        full_sqlite: Some(full_sqlite.clone()),
        areas: None,
    };
    let manifest = build_manifest(
        publication,
        contests,
        artifacts,
        custom_css,
        language_config,
    );
    let documents = ResultsPublicationDocuments {
        full_sqlite: Some(full_sqlite),
        ..Default::default()
    };
    Ok((serde_json::to_value(documents)?, manifest))
}

pub async fn refresh_public_results_index(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let election_event = get_election_event_by_id(tx, tenant_id, election_event_id).await?;
    let presentation = election_event.get_presentation()?.unwrap_or_default();
    let active_publications =
        list_active_public_publications(tx, tenant_id, election_event_id).await?;
    let active_publications = if is_results_website_enabled(&presentation)? {
        let mut policy_matching_publications = Vec::new();

        for publication in active_publications {
            if publication_matches_results_website_policy(&presentation, &publication)? {
                policy_matching_publications.push(publication);
            } else {
                delete_publication_artifacts(tx, &publication).await?;
                mark_publication_superseded(tx, &publication).await?;
            }
        }

        policy_matching_publications
    } else {
        delete_public_results_artifacts(tenant_id, election_event_id).await?;
        for publication in active_publications {
            delete_publication_artifacts(tx, &publication).await?;
            mark_publication_superseded(tx, &publication).await?;
        }
        Vec::new()
    };
    let publications = active_publications
        .into_iter()
        .map(|active| {
            let base = publication_base_path(&active);
            let route_scope = active.route_scope;
            let route_election_id = active.route_election_id.clone();
            let route = if route_scope == ResultsRouteScope::Election {
                format!(
                    "/{}/elections/{}",
                    active.election_event_id,
                    route_election_id.clone().unwrap_or_default()
                )
            } else {
                format!("/{}", active.election_event_id)
            };
            let manifest_public_path = event_public_path(
                &active.tenant_id,
                &active.election_event_id,
                &format!("{base}/manifest-latest.json"),
            );
            json!({
                "publication_id": active.id,
                "route_scope": route_scope,
                "route": route,
                "route_election_id": route_election_id,
                "election_ids": active.election_ids,
                "access": active.access,
                "visibility_scope": active.visibility_scope,
                "manifest_public_path": if active.access == ResultsWebsiteAccess::Public {
                    Some(manifest_public_path)
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();

    let index = json!({
        "schema_version": 1,
        "tenant_id": tenant_id,
        "election_event_id": election_event_id,
        "publications": publications
    });
    let key = format!("results-index/{election_event_id}.json");
    upload_public_json_key(&key, &index).await
}

pub async fn publish_results_website_artifacts(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    publication_id: &str,
) -> Result<()> {
    let publication =
        get_publication_by_id(tx, tenant_id, election_event_id, publication_id).await?;
    match publication.publication_status {
        ResultsPublicationStatus::Published => {
            return Ok(());
        }
        ResultsPublicationStatus::Publishing | ResultsPublicationStatus::Failed => {}
        ResultsPublicationStatus::Revoked | ResultsPublicationStatus::Superseded => {
            return Err(anyhow!(
                "Cannot publish a {} results publication",
                publication.publication_status
            ));
        }
    }
    let selected_contests = selected_contest_ids(&publication)?;
    let source_sqlite = source_sqlite_file(tx, &publication).await?;
    let source_path: PathBuf = source_sqlite.path().to_path_buf();
    let contests = query_manifest_contests(&source_path, &publication, &selected_contests)?;
    let custom_css = query_manifest_custom_css(&source_path, &publication)?;
    let language_config = query_manifest_language_config(&source_path, &publication)?;

    let (documents, manifest) = if publication.access == ResultsWebsiteAccess::Public {
        publish_public_artifacts(
            tx,
            &publication,
            &source_path,
            &selected_contests,
            contests,
            custom_css,
            &language_config,
        )
        .await?
    } else {
        publish_private_artifacts(
            tx,
            &publication,
            &source_path,
            &selected_contests,
            contests,
            custom_css,
            &language_config,
        )
        .await?
    };

    mark_publication_published(
        tx,
        &publication,
        documents,
        serde_json::to_value(&manifest)?,
    )
    .await?;
    Ok(())
}

async fn upload_latest_public_manifest(publication: &TallyResultsPublication) -> Result<()> {
    if publication.access != ResultsWebsiteAccess::Public {
        return Ok(());
    }

    let manifest = publication
        .manifest
        .as_ref()
        .ok_or_else(|| anyhow!("Published results publication has no manifest"))?;
    let (_manifest_file, manifest_path, _manifest_size) =
        write_json_file("results-manifest-latest", manifest)?;
    let latest_manifest_name = format!(
        "{}/manifest-latest.json",
        publication_base_path(publication)
    );
    s3::upload_file_to_s3(
        event_public_path(
            &publication.tenant_id,
            &publication.election_event_id,
            &latest_manifest_name,
        ),
        false,
        s3::get_public_bucket()?,
        "application/json".to_string(),
        manifest_path,
        Some("no-store, max-age=0".to_string()),
        Some("manifest-latest.json".to_string()),
    )
    .await
}

pub async fn finalize_results_website_publication(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    publication_id: &str,
    user_id: &str,
    username: Option<String>,
) -> Result<()> {
    let publication =
        get_publication_by_id(tx, tenant_id, election_event_id, publication_id).await?;
    if publication.publication_status != ResultsPublicationStatus::Published {
        return Err(anyhow!("Results publication is not published"));
    }

    upload_latest_public_manifest(&publication).await?;
    if publication.access != ResultsWebsiteAccess::Public {
        delete_public_publication_route_artifacts(&publication).await?;
    }
    refresh_public_results_index(tx, tenant_id, election_event_id).await?;

    for superseded in list_superseded_publications(tx, tenant_id, election_event_id).await? {
        delete_superseded_publication_artifacts(tx, &superseded).await?;
    }

    post_results_publication_action(
        tx,
        &publication,
        ResultsPublicationAction::Publish,
        user_id,
        username,
    )
    .await?;
    set_publication_finalization_error(tx, tenant_id, election_event_id, publication_id, None)
        .await?;

    Ok(())
}

fn publication_documents(
    publication: &TallyResultsPublication,
) -> Result<ResultsPublicationDocuments> {
    serde_json::from_value(publication.documents.clone())
        .context("Invalid stored results publication documents")
}

fn manifest_public_path(publication: &TallyResultsPublication) -> Result<Option<String>> {
    Ok(publication_documents(publication)?
        .manifest
        .and_then(|manifest| manifest.latest_public_path.or(manifest.public_path)))
}

fn publication_matches_requested_route(
    publication: &TallyResultsPublication,
    election_id: Option<&str>,
) -> bool {
    match publication.route_scope {
        ResultsRouteScope::Election => {
            publication.route_election_id.as_deref().is_some()
                && publication.route_election_id.as_deref() == election_id
        }
        ResultsRouteScope::Event => election_id
            .map(|id| {
                publication
                    .election_ids
                    .iter()
                    .any(|election| election == id)
            })
            .unwrap_or(true),
    }
}

fn authorize_results_reader(
    claims: &JwtClaims,
    election_event_id: &str,
    publication: &TallyResultsPublication,
    requested_election_id: Option<&str>,
) -> ResultsPublicationServiceResult<()> {
    if !is_results_portal_client(&claims.azp) {
        let can_read = claims.hasura_claims.allowed_roles.iter().any(|role| {
            role == &Permissions::PUBLISH_RESULTS_READ.to_string()
                || role == &Permissions::PUBLISH_RESULTS_WRITE.to_string()
        });
        return can_read.then_some(()).ok_or_else(|| {
            ResultsPublicationServiceError::Unauthorized(
                "Missing results publication permission".to_string(),
            )
        });
    }

    let expected_realm = get_event_realm(&claims.hasura_claims.tenant_id, election_event_id);
    let issuer_realm = claims.iss.trim_end_matches('/').rsplit('/').next();
    if issuer_realm != Some(expected_realm.as_str()) {
        return Err(ResultsPublicationServiceError::Forbidden(
            "Token is not valid for this election event".to_string(),
        ));
    }

    let authorized_election_ids = claims
        .hasura_claims
        .authorized_election_ids
        .as_ref()
        .ok_or_else(|| {
            ResultsPublicationServiceError::Forbidden(
                "No authorized elections are available".to_string(),
            )
        })?;
    let is_authorized = match requested_election_id {
        Some(election_id) => {
            authorized_election_ids.iter().any(|id| id == election_id)
                && publication.election_ids.iter().any(|id| id == election_id)
        }
        None => publication
            .election_ids
            .iter()
            .any(|election_id| authorized_election_ids.iter().any(|id| id == election_id)),
    };

    is_authorized.then_some(()).ok_or_else(|| {
        ResultsPublicationServiceError::Forbidden(
            "Not authorized to view these election results".to_string(),
        )
    })
}

fn manifest_for_reader(
    publication: &TallyResultsPublication,
    claims: &JwtClaims,
) -> ResultsPublicationServiceResult<Option<ResultsPublicationManifest>> {
    let mut manifest = publication
        .manifest
        .clone()
        .map(serde_json::from_value)
        .transpose()
        .context("Invalid stored results publication manifest")?;
    if publication.visibility_scope != ResultsWebsiteVisibilityScope::AreaBased
        || !is_results_portal_client(&claims.azp)
    {
        return Ok(manifest);
    }

    let area_id = claims.hasura_claims.area_id.as_ref().ok_or_else(|| {
        ResultsPublicationServiceError::Forbidden("No voter area is available".to_string())
    })?;
    let areas = manifest
        .as_mut()
        .and_then(|manifest| manifest.artifacts.areas.as_mut())
        .ok_or_else(|| {
            ResultsPublicationServiceError::NotFound(
                "No area results artifacts are available".to_string(),
            )
        })?;
    let area_document = areas.remove(area_id).ok_or_else(|| {
        ResultsPublicationServiceError::Forbidden(
            "No results artifact is available for this voter area".to_string(),
        )
    })?;
    areas.clear();
    areas.insert(area_id.clone(), area_document);
    Ok(manifest)
}

pub async fn configure_results_website_policy_request(
    tenant_id: &str,
    input: &ConfigureResultsWebsitePolicyInput,
) -> ResultsPublicationServiceResult<ConfigureResultsWebsitePolicyOutput> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to acquire Hasura database connection")?;
    let tx = client
        .transaction()
        .await
        .context("Failed to start results website policy transaction")?;
    let output = configure_results_website_policy(&tx, tenant_id, input).await?;
    tx.commit()
        .await
        .context("Failed to commit results website policy")?;

    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to acquire Hasura database connection")?;
    let tx = client
        .transaction()
        .await
        .context("Failed to start results index refresh transaction")?;
    refresh_public_results_index(&tx, tenant_id, &input.election_event_id).await?;
    tx.commit()
        .await
        .context("Failed to commit results index refresh")?;

    Ok(output)
}

pub async fn request_results_website_publication(
    tenant_id: &str,
    user_id: &str,
    username: Option<String>,
    executed_by_user: &str,
    input: &PublishResultsWebsiteInput,
) -> ResultsPublicationServiceResult<PublishResultsWebsiteOutput> {
    input.validate().map_err(|err| {
        ResultsPublicationServiceError::BadRequest(format!("Invalid publication request: {err}"))
    })?;

    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to acquire Hasura database connection")?;
    let tx = client
        .transaction()
        .await
        .context("Failed to start publication validation transaction")?;
    let election_event = get_election_event_by_id(&tx, tenant_id, &input.election_event_id)
        .await
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?;
    let presentation = election_event
        .get_presentation()
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?
        .unwrap_or_default();
    validate_results_website_policy(&presentation, input.access, input.visibility_scope).map_err(
        |err| {
            ResultsPublicationServiceError::BadRequest(format!(
                "Invalid publication request: {err}"
            ))
        },
    )?;
    validate_new_publication_source(
        &tx,
        tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
        &input.tally_session_execution_id,
        &input.results_event_id,
        &input.election_ids,
        &input.contest_ids,
        input.route_election_id.as_deref(),
    )
    .await
    .map_err(|err| {
        ResultsPublicationServiceError::BadRequest(format!("Invalid publication source: {err}"))
    })?;
    tx.commit()
        .await
        .context("Failed to commit publication validation transaction")?;

    let task_execution = post(
        tenant_id,
        Some(&input.election_event_id),
        ETasksExecution::PUBLISH_RESULTS_WEBSITE,
        executed_by_user,
    )
    .await?;

    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to acquire Hasura database connection")?;
    let tx = client
        .transaction()
        .await
        .context("Failed to start publication transaction")?;
    let publication = insert_publishing_publication(
        &tx,
        NewTallyResultsPublication {
            tenant_id,
            election_event_id: &input.election_event_id,
            tally_session_id: &input.tally_session_id,
            tally_session_execution_id: &input.tally_session_execution_id,
            results_event_id: &input.results_event_id,
            task_execution_id: &task_execution.id,
            route_scope: input.route_scope,
            route_election_id: input.route_election_id.as_deref(),
            election_ids: &input.election_ids,
            access: input.access,
            visibility_scope: input.visibility_scope,
            contest_ids: &input.contest_ids,
            published_by_user_id: Some(user_id),
        },
    )
    .await?;
    tx.commit()
        .await
        .context("Failed to commit publication transaction")?;

    let publication_id = publication.id;
    let celery_app = get_celery_app().await;
    let error_msg = match celery_app
        .send_task(
            crate::tasks::publish_results_website::publish_results_website_task::new(
                tenant_id.to_string(),
                input.election_event_id.clone(),
                publication_id.clone(),
                user_id.to_string(),
                username,
                task_execution.clone(),
            ),
        )
        .await
    {
        Ok(_) => None,
        Err(err) => {
            let message = format!("Failed to send PUBLISH_RESULTS_WEBSITE task: {err:?}");
            let mut client = get_hasura_pool()
                .await
                .get()
                .await
                .context("Failed to acquire Hasura database connection")?;
            let tx = client
                .transaction()
                .await
                .context("Failed to start publication failure transaction")?;
            mark_publication_failed(
                &tx,
                tenant_id,
                &input.election_event_id,
                &publication_id,
                &message,
            )
            .await?;
            tx.commit()
                .await
                .context("Failed to commit publication failure")?;
            Some(message)
        }
    };

    Ok(PublishResultsWebsiteOutput {
        publication_id,
        task_execution_id: task_execution.id.clone(),
        publication_status: if error_msg.is_some() {
            ResultsPublicationStatus::Failed
        } else {
            ResultsPublicationStatus::Publishing
        },
        task_execution,
        error_msg,
    })
}

pub async fn resolve_results_publication_request(
    claims: &JwtClaims,
    input: &ResolveResultsPublicationInput,
) -> ResultsPublicationServiceResult<Option<ResolveResultsPublicationOutput>> {
    let tenant_id = &claims.hasura_claims.tenant_id;
    let route_scope = if input.election_id.is_some() {
        ResultsRouteScope::Election
    } else {
        ResultsRouteScope::Event
    };
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to acquire Hasura database connection")?;
    let tx = client
        .transaction()
        .await
        .context("Failed to start results publication resolution transaction")?;
    let election_event = get_election_event_by_id(&tx, tenant_id, &input.ee_id)
        .await
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?;
    let presentation = election_event
        .get_presentation()
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?
        .unwrap_or_default();
    if !is_results_website_enabled(&presentation)
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?
    {
        tx.commit()
            .await
            .context("Failed to commit results publication resolution")?;
        return Ok(None);
    }

    let mut publication = get_active_publication_for_route(
        &tx,
        tenant_id,
        &input.ee_id,
        route_scope,
        input.election_id.as_deref(),
    )
    .await?;
    if publication.is_none() && input.election_id.is_some() {
        publication = get_active_publication_for_route(
            &tx,
            tenant_id,
            &input.ee_id,
            ResultsRouteScope::Event,
            None,
        )
        .await?;
    }
    if let Some(candidate) = publication.as_ref() {
        let matches_policy =
            publication_matches_results_website_policy(&presentation, candidate)
                .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?;
        if !publication_matches_requested_route(candidate, input.election_id.as_deref())
            || !matches_policy
        {
            publication = None;
        }
    }
    let Some(publication) = publication else {
        tx.commit()
            .await
            .context("Failed to commit results publication resolution")?;
        return Ok(None);
    };
    authorize_results_reader(
        claims,
        &input.ee_id,
        &publication,
        input.election_id.as_deref(),
    )?;
    let manifest_public_path = manifest_public_path(&publication)?;
    let manifest = manifest_for_reader(&publication, claims)?;
    tx.commit()
        .await
        .context("Failed to commit results publication resolution")?;

    Ok(Some(ResolveResultsPublicationOutput {
        tenant_id: publication.tenant_id,
        election_event_id: publication.election_event_id,
        access: publication.access,
        route_scope: publication.route_scope,
        election_ids: publication.election_ids,
        publication_id: publication.id,
        manifest_public_path,
        manifest_url: None,
        manifest,
    }))
}

pub async fn fetch_results_artifact_request(
    claims: &JwtClaims,
    input: &FetchResultsArtifactInput,
) -> ResultsPublicationServiceResult<FetchResultsArtifactOutput> {
    let tenant_id = &claims.hasura_claims.tenant_id;
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to acquire Hasura database connection")?;
    let tx = client
        .transaction()
        .await
        .context("Failed to start results artifact transaction")?;
    let election_event = get_election_event_by_id(&tx, tenant_id, &input.election_event_id)
        .await
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?;
    let presentation = election_event
        .get_presentation()
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?
        .unwrap_or_default();
    if !is_results_website_enabled(&presentation)
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?
    {
        return Err(ResultsPublicationServiceError::NotFound(
            "Results publication is not available".to_string(),
        ));
    }

    let publication = get_publication_by_id(
        &tx,
        tenant_id,
        &input.election_event_id,
        &input.publication_id,
    )
    .await?;
    if publication.publication_status != ResultsPublicationStatus::Published
        || !publication_matches_requested_route(&publication, input.election_id.as_deref())
        || !publication_matches_results_website_policy(&presentation, &publication)
            .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?
    {
        return Err(ResultsPublicationServiceError::NotFound(
            "Results publication is not available for this route".to_string(),
        ));
    }
    authorize_results_reader(
        claims,
        &input.election_event_id,
        &publication,
        input.election_id.as_deref(),
    )?;

    let documents = publication_documents(&publication)?;
    let document_ids = if publication.visibility_scope == ResultsWebsiteVisibilityScope::AreaBased {
        let area_id = claims.hasura_claims.area_id.as_ref().ok_or_else(|| {
            ResultsPublicationServiceError::Forbidden("No voter area is available".to_string())
        })?;
        documents
            .area_sqlite
            .as_ref()
            .and_then(|areas| areas.get(area_id))
            .and_then(|artifact| artifact.document_id.clone())
            .map(|document_id| vec![document_id])
            .ok_or_else(|| {
                ResultsPublicationServiceError::Forbidden(
                    "No results artifact is available for this voter area".to_string(),
                )
            })?
    } else {
        documents
            .full_sqlite
            .and_then(|artifact| artifact.document_id)
            .map(|document_id| vec![document_id])
            .ok_or_else(|| {
                ResultsPublicationServiceError::NotFound(
                    "No results artifact is available".to_string(),
                )
            })?
    };

    let mut urls = Vec::with_capacity(document_ids.len());
    for document_id in document_ids {
        let url = get_document_url(&tx, tenant_id, Some(&input.election_event_id), &document_id)
            .await?
            .ok_or_else(|| {
                ResultsPublicationServiceError::NotFound("Document not found".to_string())
            })?;
        urls.push(url);
    }
    tx.commit()
        .await
        .context("Failed to commit results artifact transaction")?;

    Ok(FetchResultsArtifactOutput { urls })
}

pub async fn revoke_results_publication_request(
    tenant_id: &str,
    user_id: &str,
    username: Option<String>,
    input: &RevokeResultsPublicationInput,
) -> ResultsPublicationServiceResult<RevokeResultsPublicationOutput> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to acquire Hasura database connection")?;
    let tx = client
        .transaction()
        .await
        .context("Failed to start results revocation transaction")?;
    let publication = get_publication_by_id(
        &tx,
        tenant_id,
        &input.election_event_id,
        &input.publication_id,
    )
    .await?;
    match publication.publication_status {
        ResultsPublicationStatus::Published => {
            revoke_publication(
                &tx,
                tenant_id,
                &input.election_event_id,
                &input.publication_id,
            )
            .await?;
            post_results_publication_action(
                &tx,
                &publication,
                ResultsPublicationAction::Revoke,
                user_id,
                username,
            )
            .await?;
        }
        ResultsPublicationStatus::Revoked => {}
        _ => {
            return Err(ResultsPublicationServiceError::Conflict(
                "Publication is not currently published".to_string(),
            ));
        }
    };
    tx.commit()
        .await
        .context("Failed to commit results revocation")?;

    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to acquire Hasura database connection")?;
    let tx = client
        .transaction()
        .await
        .context("Failed to start results revocation cleanup transaction")?;
    refresh_public_results_index(&tx, tenant_id, &input.election_event_id).await?;
    delete_publication_artifacts(&tx, &publication).await?;
    tx.commit()
        .await
        .context("Failed to commit results revocation cleanup")?;

    Ok(RevokeResultsPublicationOutput {
        publication_id: input.publication_id.clone(),
        publication_status: ResultsPublicationStatus::Revoked,
    })
}

pub async fn refresh_results_publication_index_request(
    tenant_id: &str,
    input: &RefreshResultsPublicationIndexInput,
) -> ResultsPublicationServiceResult<RefreshResultsPublicationIndexOutput> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to acquire Hasura database connection")?;
    let tx = client
        .transaction()
        .await
        .context("Failed to start results index refresh transaction")?;
    let election_event = get_election_event_by_id(&tx, tenant_id, &input.election_event_id)
        .await
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?;
    let presentation = election_event
        .get_presentation()
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?
        .unwrap_or_default();
    let results_enabled = is_results_website_enabled(&presentation)
        .map_err(|err| ResultsPublicationServiceError::BadRequest(err.to_string()))?;
    refresh_public_results_index(&tx, tenant_id, &input.election_event_id).await?;
    tx.commit()
        .await
        .context("Failed to commit results index refresh")?;

    Ok(RefreshResultsPublicationIndexOutput {
        election_event_id: input.election_event_id.clone(),
        results_enabled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_count(conn: &Connection, table: &str) -> Result<i64> {
        Ok(
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })?,
        )
    }

    fn table_columns(conn: &Connection, table: &str) -> Result<HashSet<String>> {
        let mut statement =
            conn.prepare(&format!("PRAGMA table_info({})", quote_identifier(table)))?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<HashSet<_>>>()?;
        Ok(columns)
    }

    #[test]
    fn results_website_policy_is_read_from_the_typed_presentation() -> Result<()> {
        let policy = ResultsWebsitePolicy {
            status: ResultsWebsiteStatus::Enabled,
            access: ResultsWebsiteAccess::Authenticated,
            visibility_scope: ResultsWebsiteVisibilityScope::AreaBased,
        };
        let presentation = ElectionEventPresentation {
            results_website: Some(serde_json::to_string(&policy)?),
            ..Default::default()
        };

        assert_eq!(results_website_policy(&presentation)?, Some(policy));
        assert!(is_results_website_enabled(&presentation)?);
        Ok(())
    }

    #[test]
    fn legacy_object_policy_is_migrated_to_a_string_when_deserialized() -> Result<()> {
        let presentation: ElectionEventPresentation = serde_json::from_value(json!({
            "results_website": {
                "status": "enabled",
                "access": "public",
                "visibility_scope": "full_event"
            }
        }))?;

        assert!(presentation
            .results_website
            .as_deref()
            .is_some_and(|value| value.starts_with('{')));
        assert!(serde_json::to_value(&presentation)?["results_website"].is_string());
        Ok(())
    }

    fn test_publication() -> TallyResultsPublication {
        TallyResultsPublication {
            id: "publication-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            election_event_id: "event-1".to_string(),
            tally_session_id: "session-1".to_string(),
            tally_session_execution_id: "execution-1".to_string(),
            results_event_id: "results-event-1".to_string(),
            task_execution_id: None,
            route_scope: ResultsRouteScope::Event,
            route_election_id: None,
            election_ids: vec!["election-1".to_string()],
            access: ResultsWebsiteAccess::Public,
            visibility_scope: ResultsWebsiteVisibilityScope::FullEvent,
            published_contest_ids: vec!["contest-1".to_string()],
            contest_publication_state: Default::default(),
            documents: json!({}),
            manifest: None,
            publication_status:
                crate::types::results_publication::ResultsPublicationStatus::Publishing,
            version: 1,
            error_message: None,
            published_by_user_id: None,
        }
    }

    #[test]
    fn filtered_sqlite_is_fresh_allow_listed_and_contains_only_selected_results() -> Result<()> {
        const BALLOT_SENTINEL: &str = "BALLOT-SECRET-MUST-NEVER-BE-PUBLISHED";
        const OTHER_EVENT_SENTINEL: &str = "OTHER-EVENT-MUST-NEVER-BE-PUBLISHED";
        const OTHER_RESULTS_EVENT_SENTINEL: &str = "OTHER-RESULTS-EVENT-MUST-NEVER-BE-PUBLISHED";
        const OTHER_AREA_SENTINEL: &str = "OTHER-AREA-MUST-NEVER-BE-PUBLISHED";
        const INTERNAL_COLUMN_SENTINEL: &str = "INTERNAL-EVENT-STATISTICS-MUST-NEVER-BE-PUBLISHED";
        let source = generate_temp_file("results-publication-source", ".sqlite")?;
        let conn = Connection::open(source.path())?;
        conn.execute_batch(
            r#"
                CREATE TABLE election_event (id TEXT, presentation TEXT, statistics TEXT);
                CREATE TABLE election (id TEXT, election_event_id TEXT, presentation TEXT);
                CREATE TABLE contest (id TEXT, election_id TEXT);
                CREATE TABLE candidate (
                    id TEXT,
                    contest_id TEXT,
                    is_public INTEGER,
                    labels TEXT
                );
                CREATE TABLE area (id TEXT, election_event_id TEXT);
                CREATE TABLE results_event (id TEXT, election_event_id TEXT);
                CREATE TABLE results_election (
                    election_id TEXT,
                    results_event_id TEXT,
                    annotations TEXT
                );
                CREATE TABLE results_contest (
                    contest_id TEXT,
                    election_id TEXT,
                    results_event_id TEXT,
                    annotations TEXT
                );
                CREATE TABLE results_contest_candidate (
                    contest_id TEXT,
                    candidate_id TEXT,
                    results_event_id TEXT,
                    annotations TEXT
                );
                CREATE TABLE results_election_area (
                    election_id TEXT,
                    area_id TEXT,
                    results_event_id TEXT
                );
                CREATE TABLE results_area_contest_candidate (
                    contest_id TEXT,
                    candidate_id TEXT,
                    area_id TEXT,
                    results_event_id TEXT,
                    annotations TEXT
                );
                CREATE TABLE results_area_contest (
                    contest_id TEXT,
                    election_id TEXT,
                    area_id TEXT,
                    results_event_id TEXT,
                    annotations TEXT
                );
                CREATE TABLE ballot (id TEXT, secret_payload TEXT);
                CREATE TABLE unrelated_config (id TEXT, secret_payload TEXT);

                INSERT INTO election_event VALUES
                    ('event-1', '{}', 'INTERNAL-EVENT-STATISTICS-MUST-NEVER-BE-PUBLISHED'),
                    ('event-2', 'OTHER-EVENT-MUST-NEVER-BE-PUBLISHED', NULL);
                INSERT INTO election VALUES
                    ('election-1', 'event-1', '{}'),
                    ('election-2', 'event-2', 'OTHER-EVENT-MUST-NEVER-BE-PUBLISHED');
                INSERT INTO contest VALUES
                    ('contest-1', 'election-1'),
                    ('contest-2', 'election-2');
                INSERT INTO candidate VALUES
                    ('candidate-1', 'contest-1', 0, NULL),
                    ('candidate-2', 'contest-2', 1, NULL);
                INSERT INTO area VALUES
                    ('area-1', 'event-1'),
                    ('area-private', 'event-1'),
                    ('area-2', 'event-2');
                INSERT INTO results_event VALUES
                    ('results-event-1', 'event-1'),
                    ('results-event-2', 'event-2'),
                    ('results-event-3', 'event-1');
                INSERT INTO results_election VALUES
                    ('election-1', 'results-event-1', NULL),
                    ('election-2', 'results-event-2', NULL),
                    ('election-1', 'results-event-3', 'OTHER-RESULTS-EVENT-MUST-NEVER-BE-PUBLISHED');
                INSERT INTO results_contest VALUES
                    ('contest-1', 'election-1', 'results-event-1', NULL),
                    ('contest-2', 'election-2', 'results-event-2', NULL),
                    ('contest-1', 'election-1', 'results-event-3', 'OTHER-RESULTS-EVENT-MUST-NEVER-BE-PUBLISHED');
                INSERT INTO results_contest_candidate VALUES
                    ('contest-1', 'candidate-1', 'results-event-1', NULL),
                    ('contest-2', 'candidate-2', 'results-event-2', NULL),
                    ('contest-1', 'candidate-1', 'results-event-3', 'OTHER-RESULTS-EVENT-MUST-NEVER-BE-PUBLISHED');
                INSERT INTO results_election_area VALUES
                    ('election-1', 'area-1', 'results-event-1'),
                    ('election-1', 'area-private', 'results-event-1'),
                    ('election-2', 'area-2', 'results-event-2'),
                    ('election-1', 'area-1', 'results-event-3');
                INSERT INTO results_area_contest_candidate VALUES
                    ('contest-1', 'candidate-1', 'area-1', 'results-event-1', NULL),
                    ('contest-1', 'candidate-1', 'area-private', 'results-event-1', 'OTHER-AREA-MUST-NEVER-BE-PUBLISHED'),
                    ('contest-2', 'candidate-2', 'area-2', 'results-event-2', NULL),
                    ('contest-1', 'candidate-1', 'area-1', 'results-event-3', 'OTHER-RESULTS-EVENT-MUST-NEVER-BE-PUBLISHED');
                INSERT INTO results_area_contest VALUES
                    ('contest-1', 'election-1', 'area-1', 'results-event-1', NULL),
                    ('contest-1', 'election-1', 'area-private', 'results-event-1', 'OTHER-AREA-MUST-NEVER-BE-PUBLISHED'),
                    ('contest-2', 'election-2', 'area-2', 'results-event-2', NULL),
                    ('contest-1', 'election-1', 'area-1', 'results-event-3', 'OTHER-RESULTS-EVENT-MUST-NEVER-BE-PUBLISHED');
                INSERT INTO ballot VALUES
                    ('ballot-1', 'BALLOT-SECRET-MUST-NEVER-BE-PUBLISHED');
                INSERT INTO unrelated_config VALUES
                    ('config-1', 'OTHER-EVENT-MUST-NEVER-BE-PUBLISHED');
            "#,
        )?;
        drop(conn);

        let publication = test_publication();
        let target = copy_filtered_sqlite(
            source.path(),
            &publication,
            &["contest-1".to_string()],
            None,
        )?;
        let target_conn = Connection::open(target.path())?;

        assert!(!table_exists(&target_conn, "ballot"));
        assert!(!table_exists(&target_conn, "unrelated_config"));
        assert!(!table_columns(&target_conn, "election_event")?.contains("statistics"));
        assert_eq!(row_count(&target_conn, "election_event")?, 1);
        assert_eq!(row_count(&target_conn, "election")?, 1);
        assert_eq!(row_count(&target_conn, "contest")?, 1);
        assert_eq!(row_count(&target_conn, "candidate")?, 1);
        assert_eq!(row_count(&target_conn, "area")?, 2);
        assert_eq!(row_count(&target_conn, "results_event")?, 1);
        assert_eq!(row_count(&target_conn, "results_election")?, 1);
        assert_eq!(row_count(&target_conn, "results_contest")?, 1);
        assert_eq!(row_count(&target_conn, "results_contest_candidate")?, 1);
        assert_eq!(row_count(&target_conn, "results_election_area")?, 2);
        assert_eq!(row_count(&target_conn, "results_area_contest")?, 2);
        assert_eq!(
            row_count(&target_conn, "results_area_contest_candidate")?,
            2
        );

        drop(target_conn);
        let target_bytes = fs::read(target.path())?;
        assert!(!target_bytes
            .windows(BALLOT_SENTINEL.len())
            .any(|bytes| bytes == BALLOT_SENTINEL.as_bytes()));
        assert!(!target_bytes
            .windows(OTHER_EVENT_SENTINEL.len())
            .any(|bytes| bytes == OTHER_EVENT_SENTINEL.as_bytes()));
        assert!(!target_bytes
            .windows(OTHER_RESULTS_EVENT_SENTINEL.len())
            .any(|bytes| bytes == OTHER_RESULTS_EVENT_SENTINEL.as_bytes()));
        assert!(!target_bytes
            .windows(INTERNAL_COLUMN_SENTINEL.len())
            .any(|bytes| bytes == INTERNAL_COLUMN_SENTINEL.as_bytes()));

        let area_target = copy_filtered_sqlite(
            source.path(),
            &publication,
            &["contest-1".to_string()],
            Some("area-1"),
        )?;
        let area_target_conn = Connection::open(area_target.path())?;
        assert_eq!(row_count(&area_target_conn, "area")?, 1);
        assert_eq!(row_count(&area_target_conn, "results_election")?, 0);
        assert_eq!(row_count(&area_target_conn, "results_contest")?, 0);
        assert_eq!(
            row_count(&area_target_conn, "results_contest_candidate")?,
            0
        );
        assert_eq!(row_count(&area_target_conn, "results_election_area")?, 1);
        assert_eq!(row_count(&area_target_conn, "results_area_contest")?, 1);
        assert_eq!(
            row_count(&area_target_conn, "results_area_contest_candidate")?,
            1
        );
        drop(area_target_conn);
        let area_target_bytes = fs::read(area_target.path())?;
        assert!(!area_target_bytes
            .windows(OTHER_AREA_SENTINEL.len())
            .any(|bytes| bytes == OTHER_AREA_SENTINEL.as_bytes()));

        Ok(())
    }
}
