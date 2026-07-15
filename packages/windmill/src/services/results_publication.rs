// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::get_document;
use crate::postgres::election_event::{
    get_election_event_by_id, update_election_event_presentation,
};
use crate::postgres::tally_results_publication::{
    get_publication_by_id, list_active_public_publications, mark_publication_published,
    mark_publication_superseded, TallyResultsPublication,
};
use crate::postgres::tally_session_execution::get_tally_session_execution_documents;
use crate::services::documents::{
    get_document_as_temp_file, upload_and_return_document, upload_and_return_public_event_document,
};
use crate::types::results_publication::{
    ConfigureResultsWebsitePolicyInput, ConfigureResultsWebsitePolicyOutput, ResultsRouteScope,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use rusqlite::{params_from_iter, Connection, OptionalExtension, ToSql};
use sequent_core::ballot::{
    ElectionEventPresentation, ElectionPresentation, ResultsWebsiteAccess, ResultsWebsitePolicy,
    ResultsWebsiteStatus, ResultsWebsiteVisibilityScope,
};
use sequent_core::services::s3;
use sequent_core::temp_path::{generate_temp_file, get_file_size};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

fn placeholders(count: usize) -> String {
    (0..count).map(|_| "?").collect::<Vec<_>>().join(",")
}

fn selected_contest_ids(publication: &TallyResultsPublication) -> Result<Vec<String>> {
    Ok(publication.published_contest_ids.clone())
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
) -> Result<ConfigureResultsWebsitePolicyOutput> {
    input.validate()?;
    let election_event = get_election_event_by_id(tx, tenant_id, &input.election_event_id).await?;
    let mut presentation = election_event.get_presentation()?.unwrap_or_default();
    presentation.results_website = Some(serde_json::to_string(&input.policy())?);
    update_election_event_presentation(
        tx,
        tenant_id,
        &input.election_event_id,
        serde_json::to_value(presentation)?,
    )
    .await?;
    refresh_public_results_index(tx, tenant_id, &input.election_event_id).await?;

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

fn execute_delete_not_in(conn: &Connection, table: &str, contest_ids: &[String]) -> Result<()> {
    if contest_ids.is_empty() {
        conn.execute(&format!("DELETE FROM {table}"), [])?;
        return Ok(());
    }

    let sql = format!(
        "DELETE FROM {table} WHERE contest_id NOT IN ({})",
        placeholders(contest_ids.len())
    );
    let params = contest_ids.iter().map(|id| id as &dyn ToSql);
    conn.execute(&sql, params_from_iter(params))?;
    Ok(())
}

fn filter_result_tables(
    conn: &Connection,
    selected_contests: &[String],
    area_id: Option<&str>,
) -> Result<()> {
    for table in [
        "results_contest_candidate",
        "results_contest",
        "results_area_contest_candidate",
        "results_area_contest",
    ] {
        if table_exists(conn, table) {
            execute_delete_not_in(conn, table, selected_contests)?;
        }
    }

    if let Some(area_id) = area_id {
        for table in [
            "results_contest_candidate",
            "results_contest",
            "results_election",
        ] {
            if table_exists(conn, table) {
                conn.execute(&format!("DELETE FROM {table}"), [])?;
            }
        }

        for table in ["results_area_contest_candidate", "results_area_contest"] {
            if table_exists(conn, table) {
                conn.execute(
                    &format!("DELETE FROM {table} WHERE area_id <> ?"),
                    [area_id],
                )?;
            }
        }
        if table_exists(conn, "results_election_area") {
            conn.execute(
                "DELETE FROM results_election_area WHERE area_id <> ?",
                [area_id],
            )?;
        }
    }

    Ok(())
}

fn copy_filtered_sqlite(
    source_path: &Path,
    selected_contests: &[String],
    area_id: Option<&str>,
) -> Result<NamedTempFile> {
    let target = generate_temp_file("results-publication", ".sqlite")?;
    fs::copy(source_path, target.path())?;

    {
        let conn = Connection::open(target.path())?;
        filter_result_tables(&conn, selected_contests, area_id)?;
    }

    Ok(target)
}

fn query_manifest_contests(
    source_path: &Path,
    publication: &TallyResultsPublication,
    selected_contests: &[String],
) -> Result<Vec<Value>> {
    let conn = Connection::open(source_path)?;
    let selected: HashSet<&str> = selected_contests.iter().map(String::as_str).collect();
    let election_ids = &publication.election_ids;

    if election_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut contests = Vec::new();

    if table_exists(&conn, "contest") {
        let sql = format!(
            "SELECT id AS contest_id, election_id FROM contest WHERE election_id IN ({}) ORDER BY election_id, id",
            placeholders(election_ids.len())
        );
        let params = election_ids.iter().map(|id| id as &dyn ToSql);
        let mut statement = conn.prepare(&sql)?;
        let mut rows = statement.query(params_from_iter(params))?;

        while let Some(row) = rows.next()? {
            let contest_id: String = row.get("contest_id")?;
            let election_id: String = row.get("election_id")?;
            contests.push(json!({
                "election_id": election_id,
                "contest_id": contest_id,
                "area_id": null,
                "publication_state": if selected.contains(contest_id.as_str()) {
                    "published"
                } else {
                    "not_published"
                },
                "positions": null
            }));
        }
    }

    if contests.is_empty() && table_exists(&conn, "results_contest") {
        let sql = format!(
            "SELECT DISTINCT contest_id, election_id FROM results_contest WHERE election_id IN ({}) ORDER BY election_id, contest_id",
            placeholders(election_ids.len())
        );
        let params = election_ids.iter().map(|id| id as &dyn ToSql);
        let mut statement = conn.prepare(&sql)?;
        let mut rows = statement.query(params_from_iter(params))?;

        while let Some(row) = rows.next()? {
            let contest_id: String = row.get("contest_id")?;
            let election_id: String = row.get("election_id")?;
            contests.push(json!({
                "election_id": election_id,
                "contest_id": contest_id,
                "area_id": null,
                "publication_state": if selected.contains(contest_id.as_str()) {
                    "published"
                } else {
                    "not_published"
                },
                "positions": null
            }));
        }
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
) -> Result<Value> {
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

    let mut election_css = serde_json::Map::new();
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
                election_css.insert(election_id, Value::String(css));
            }
        }
    }

    Ok(json!({
        "election_event": election_event_css,
        "elections": election_css,
    }))
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

fn query_area_ids(source_path: &Path, selected_contests: &[String]) -> Result<Vec<String>> {
    if selected_contests.is_empty() {
        return Ok(vec![]);
    }

    let conn = Connection::open(source_path)?;
    if !table_exists(&conn, "results_area_contest") {
        return Ok(vec![]);
    }

    let sql = format!(
        "SELECT DISTINCT area_id FROM results_area_contest WHERE contest_id IN ({}) ORDER BY area_id",
        placeholders(selected_contests.len())
    );
    let params = selected_contests.iter().map(|id| id as &dyn ToSql);
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
    s3::delete_files_from_s3(s3::get_public_bucket()?, prefix, true).await
}

pub async fn delete_public_publication_route_artifacts(
    publication: &TallyResultsPublication,
) -> Result<()> {
    match publication.route_scope {
        ResultsRouteScope::Election => {
            let prefix = event_public_path(
                &publication.tenant_id,
                &publication.election_event_id,
                &publication_base_path(publication),
            );
            s3::delete_files_from_s3(s3::get_public_bucket()?, prefix, true).await
        }
        ResultsRouteScope::Event => {
            let full_prefix = event_public_path(
                &publication.tenant_id,
                &publication.election_event_id,
                "results/full-",
            );
            let manifest_prefix = event_public_path(
                &publication.tenant_id,
                &publication.election_event_id,
                "results/manifest-",
            );
            s3::delete_files_from_s3(s3::get_public_bucket()?, full_prefix, true).await?;
            s3::delete_files_from_s3(s3::get_public_bucket()?, manifest_prefix, true).await
        }
    }
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
    contests: Vec<Value>,
    artifacts: Value,
    custom_css: Value,
    language_config: &ManifestLanguageConfig,
) -> Value {
    json!({
        "schema_version": 1,
        "tenant_id": publication.tenant_id,
        "election_event_id": publication.election_event_id,
        "election_ids": publication.election_ids,
        "route_scope": publication.route_scope,
        "route_election_id": publication.route_election_id,
        "publication_id": publication.id,
        "tally_session_id": publication.tally_session_id,
        "tally_session_execution_id": publication.tally_session_execution_id,
        "results_event_id": publication.results_event_id,
        "version": publication.version,
        "access": publication.access,
        "visibility_scope": publication.visibility_scope,
        "default_locale": language_config.default_locale,
        "available_languages": language_config.available_languages,
        "title": {"en": "Election Results"},
        "custom_css": custom_css,
        "contests": contests,
        "artifacts": artifacts
    })
}

async fn publish_public_artifacts(
    tx: &Transaction<'_>,
    publication: &TallyResultsPublication,
    source_path: &Path,
    selected_contests: &[String],
    contests: Vec<Value>,
    custom_css: Value,
    language_config: &ManifestLanguageConfig,
) -> Result<(Value, Value)> {
    let base = publication_base_path(publication);
    let sqlite_name = format!("{base}/full-v{}.sqlite", publication.version);
    let manifest_name = format!("{base}/manifest-v{}.json", publication.version);
    let latest_manifest_name = format!("{base}/manifest-latest.json");
    let sqlite = copy_filtered_sqlite(source_path, selected_contests, None)?;
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

    let artifacts = json!({
        "full_sqlite": {
            "document_id": sqlite_document.id,
            "public_path": event_public_path(
                &publication.tenant_id,
                &publication.election_event_id,
                &sqlite_name
            )
        }
    });
    let manifest = build_manifest(
        publication,
        contests,
        artifacts.clone(),
        custom_css,
        language_config,
    );
    let (_manifest_file, manifest_path, manifest_size) =
        write_json_file("results-manifest", &manifest)?;

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
    upload_and_return_public_event_document(
        tx,
        &manifest_path,
        manifest_size,
        "application/json",
        &publication.tenant_id,
        &publication.election_event_id,
        &latest_manifest_name,
        None,
    )
    .await?;

    let documents = json!({
        "manifest": {
            "document_id": manifest_document.id,
            "public_path": event_public_path(
                &publication.tenant_id,
                &publication.election_event_id,
                &manifest_name
            ),
            "latest_public_path": event_public_path(
                &publication.tenant_id,
                &publication.election_event_id,
                &latest_manifest_name
            )
        },
        "full_sqlite": artifacts["full_sqlite"]
    });

    Ok((documents, manifest))
}

async fn publish_private_artifacts(
    tx: &Transaction<'_>,
    publication: &TallyResultsPublication,
    source_path: &Path,
    selected_contests: &[String],
    contests: Vec<Value>,
    custom_css: Value,
    language_config: &ManifestLanguageConfig,
) -> Result<(Value, Value)> {
    if publication.visibility_scope == ResultsWebsiteVisibilityScope::AreaBased {
        let mut area_documents = serde_json::Map::new();
        for area_id in query_area_ids(source_path, selected_contests)? {
            let sqlite = copy_filtered_sqlite(source_path, selected_contests, Some(&area_id))?;
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
            area_documents.insert(area_id, json!({"document_id": document.id}));
        }

        let artifacts = json!({"areas": area_documents});
        let manifest = build_manifest(
            publication,
            contests,
            artifacts.clone(),
            custom_css,
            language_config,
        );
        return Ok((json!({"area_sqlite": artifacts["areas"]}), manifest));
    }

    let sqlite = copy_filtered_sqlite(source_path, selected_contests, None)?;
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

    let artifacts = json!({"full_sqlite": {"document_id": document.id}});
    let manifest = build_manifest(
        publication,
        contests,
        artifacts.clone(),
        custom_css,
        language_config,
    );
    Ok((json!({"full_sqlite": artifacts["full_sqlite"]}), manifest))
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
                delete_public_publication_route_artifacts(&publication).await?;
                mark_publication_superseded(tx, &publication).await?;
            }
        }

        policy_matching_publications
    } else {
        delete_public_results_artifacts(tenant_id, election_event_id).await?;
        for publication in active_publications {
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

    mark_publication_published(tx, &publication, documents, manifest).await?;
    if publication.access != ResultsWebsiteAccess::Public {
        delete_public_publication_route_artifacts(&publication).await?;
    }
    refresh_public_results_index(tx, &publication.tenant_id, &publication.election_event_id)
        .await?;

    Ok(())
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

    #[test]
    fn area_filter_excludes_global_and_other_area_results() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            r#"
                CREATE TABLE results_contest_candidate (contest_id TEXT, candidate_id TEXT);
                CREATE TABLE results_contest (contest_id TEXT);
                CREATE TABLE results_election (election_id TEXT);
                CREATE TABLE results_area_contest_candidate (
                    contest_id TEXT,
                    candidate_id TEXT,
                    area_id TEXT
                );
                CREATE TABLE results_area_contest (contest_id TEXT, area_id TEXT);
                CREATE TABLE results_election_area (election_id TEXT, area_id TEXT);

                INSERT INTO results_contest_candidate VALUES ('contest-1', 'candidate-1');
                INSERT INTO results_contest VALUES ('contest-1');
                INSERT INTO results_election VALUES ('election-1');
                INSERT INTO results_area_contest_candidate VALUES
                    ('contest-1', 'candidate-1', 'area-1'),
                    ('contest-1', 'candidate-1', 'area-2'),
                    ('contest-2', 'candidate-2', 'area-1');
                INSERT INTO results_area_contest VALUES
                    ('contest-1', 'area-1'),
                    ('contest-1', 'area-2'),
                    ('contest-2', 'area-1');
                INSERT INTO results_election_area VALUES
                    ('election-1', 'area-1'),
                    ('election-1', 'area-2');
            "#,
        )?;

        filter_result_tables(&conn, &["contest-1".to_string()], Some("area-1"))?;

        assert_eq!(row_count(&conn, "results_contest_candidate")?, 0);
        assert_eq!(row_count(&conn, "results_contest")?, 0);
        assert_eq!(row_count(&conn, "results_election")?, 0);
        assert_eq!(row_count(&conn, "results_area_contest_candidate")?, 1);
        assert_eq!(row_count(&conn, "results_area_contest")?, 1);
        assert_eq!(row_count(&conn, "results_election_area")?, 1);

        let retained_area: String = conn.query_row(
            "SELECT area_id FROM results_area_contest LIMIT 1",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(retained_area, "area-1");

        let retained_contest: String = conn.query_row(
            "SELECT contest_id FROM results_area_contest LIMIT 1",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(retained_contest, "contest-1");

        Ok(())
    }
}
