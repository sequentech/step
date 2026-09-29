// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The monitoring configuration tables: an event's mode and generation, the
//! head of each document and its revisions. The order the writes must come
//! in, and what a stored revision means, are the migration's; see
//! `services::monitoring::config_store` for the protocol that follows it.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::{GenericClient, Transaction};
use sequent_core::monitoring::config::ConfigKind;
use sequent_core::monitoring::revision::{DashboardMode, DocumentChange, RevisionOrigin};
use std::str::FromStr;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

/// The election event whose monitoring configuration is meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventRef {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
}

/// Who saved a revision or reset the event to a preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author {
    pub id: String,
    pub name: Option<String>,
}

/// A preset at a version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetVersion {
    pub id: String,
    pub version: i32,
}

/// An event's monitoring row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitoringEvent {
    pub mode: DashboardMode,
    /// The preset the configuration was last reset to.
    pub preset: Option<PresetVersion>,
    pub generation: i64,
}

/// One stored revision of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRevision {
    pub kind: ConfigKind,
    pub key: String,
    pub revision: i32,
    pub change: DocumentChange,
    /// The document's text; absent for a removal.
    pub yaml: Option<String>,
    pub sha256: Option<String>,
    pub origin: RevisionOrigin,
    pub preset: Option<PresetVersion>,
    /// The event's generation once the change that wrote it was saved.
    pub config_generation: i64,
    pub author: Author,
    pub created_at: DateTime<Utc>,
}

/// A revision to write, at the revision its head was moved to.
#[derive(Debug, Clone, Copy)]
pub struct NewRevision<'a> {
    pub kind: ConfigKind,
    pub key: &'a str,
    pub revision: i32,
    pub change: DocumentChange,
    pub yaml: Option<&'a str>,
    pub sha256: Option<&'a str>,
    pub origin: RevisionOrigin,
    pub preset: Option<&'a PresetVersion>,
    pub config_generation: i64,
    pub author: &'a Author,
}

fn parsed<T: FromStr>(row: &Row, column: &str) -> Result<T> {
    let text: String = row.try_get(column)?;
    T::from_str(&text)
        .map_err(|_| anyhow::anyhow!("unknown {column} '{text}' in the monitoring tables"))
}

fn preset_in(row: &Row, id: &str, version: &str) -> Result<Option<PresetVersion>> {
    let id: Option<String> = row.try_get(id)?;
    let version: Option<i32> = row.try_get(version)?;
    Ok(id
        .zip(version)
        .map(|(id, version)| PresetVersion { id, version }))
}

fn preset(row: &Row) -> Result<Option<PresetVersion>> {
    preset_in(row, "preset_id", "preset_version")
}

impl TryFrom<&Row> for MonitoringEvent {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self> {
        Ok(MonitoringEvent {
            mode: parsed(row, "dashboard_mode")?,
            preset: preset(row)?,
            generation: row.try_get("config_generation")?,
        })
    }
}

/// The event's columns next to a revision's, which share some of their
/// names.
const EVENT_COLUMNS: &str = "e.dashboard_mode AS event_dashboard_mode,
     e.preset_id AS event_preset_id, e.preset_version AS event_preset_version,
     e.config_generation AS event_config_generation";

fn event_in(row: &Row) -> Result<MonitoringEvent> {
    Ok(MonitoringEvent {
        mode: parsed(row, "event_dashboard_mode")?,
        preset: preset_in(row, "event_preset_id", "event_preset_version")?,
        generation: row.try_get("event_config_generation")?,
    })
}

/// The event and the revisions next to it, from rows that repeat the event
/// once per revision, or name it once with no revision.
fn event_and_revisions(rows: Vec<Row>) -> Result<Option<(MonitoringEvent, Vec<StoredRevision>)>> {
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let event = event_in(first)?;
    let mut documents = Vec::with_capacity(rows.len());
    for row in &rows {
        if row.try_get::<_, Option<String>>("kind")?.is_some() {
            documents.push(StoredRevision::try_from(row)?);
        }
    }
    Ok(Some((event, documents)))
}

impl TryFrom<&Row> for StoredRevision {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self> {
        Ok(StoredRevision {
            kind: parsed(row, "kind")?,
            key: row.try_get("key")?,
            revision: row.try_get("revision")?,
            change: parsed(row, "change")?,
            yaml: row.try_get("yaml")?,
            sha256: row.try_get("sha256")?,
            origin: parsed(row, "origin")?,
            preset: preset(row)?,
            config_generation: row.try_get("config_generation")?,
            author: Author {
                id: row.try_get("author_id")?,
                name: row.try_get("author_name")?,
            },
            created_at: row.try_get("created_at")?,
        })
    }
}

const REVISION_COLUMNS: &str = "c.kind, c.key, c.revision, c.change, c.yaml, c.sha256, c.origin,
     c.preset_id, c.preset_version, c.config_generation, c.author_id, c.author_name,
     c.created_at";

/// Settings, themes, widgets, dashboards; each by key.
const DOCUMENT_ORDER: &str =
    "array_position(ARRAY['settings', 'theme', 'widget', 'dashboard'], c.kind), c.key";

fn revisions(rows: Vec<Row>) -> Result<Vec<StoredRevision>> {
    rows.iter().map(StoredRevision::try_from).collect()
}

#[instrument(err, skip(transaction))]
pub async fn get_monitoring_event(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> Result<Option<MonitoringEvent>> {
    transaction
        .query_opt(
            "SELECT dashboard_mode, preset_id, preset_version, config_generation
             FROM sequent_backend.monitoring_event
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to read the monitoring event")?
        .as_ref()
        .map(MonitoringEvent::try_from)
        .transpose()
}

/// Gives the event a monitoring row, on the standard dashboard, unless it
/// has one.
#[instrument(err, skip(transaction))]
pub async fn create_monitoring_event(transaction: &Transaction<'_>, event: EventRef) -> Result<()> {
    transaction
        .execute(
            "INSERT INTO sequent_backend.monitoring_event (tenant_id, election_event_id)
             VALUES ($1, $2)
             ON CONFLICT (tenant_id, election_event_id) DO NOTHING",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to create the monitoring event")?;
    Ok(())
}

/// Raises the event's generation by one, which holds its row until the
/// transaction ends: every change of the event's configuration starts here,
/// so changes queue behind one another. The event as it is then; `None`
/// without a monitoring row.
#[instrument(err, skip(transaction))]
pub async fn raise_generation(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> Result<Option<MonitoringEvent>> {
    transaction
        .query_opt(
            "UPDATE sequent_backend.monitoring_event
             SET config_generation = config_generation + 1, updated_at = now()
             WHERE tenant_id = $1 AND election_event_id = $2
             RETURNING dashboard_mode, preset_id, preset_version, config_generation",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to raise the monitoring configuration generation")?
        .as_ref()
        .map(MonitoringEvent::try_from)
        .transpose()
}

/// Sets the mode and preset, after [`raise_generation`] in the same
/// transaction.
#[instrument(err, skip(transaction))]
pub async fn update_mode_and_preset(
    transaction: &Transaction<'_>,
    event: EventRef,
    mode: DashboardMode,
    preset: Option<&PresetVersion>,
) -> Result<()> {
    transaction
        .execute(
            "UPDATE sequent_backend.monitoring_event
             SET dashboard_mode = $3, preset_id = $4, preset_version = $5
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[
                &event.tenant_id,
                &event.election_event_id,
                &mode.to_string(),
                &preset.map(|preset| preset.id.as_str()),
                &preset.map(|preset| preset.version),
            ],
        )
        .await
        .context("Failed to set the monitoring dashboard mode")?;
    Ok(())
}

/// The revision each document's head names, removals included.
#[instrument(err, skip(transaction))]
pub async fn get_heads(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> Result<Vec<StoredRevision>> {
    let rows = transaction
        .query(
            &format!(
                "SELECT {REVISION_COLUMNS}
                 FROM sequent_backend.monitoring_config_head h
                 JOIN sequent_backend.monitoring_config c USING
                     (tenant_id, election_event_id, kind, key, revision)
                 WHERE h.tenant_id = $1 AND h.election_event_id = $2
                 ORDER BY {DOCUMENT_ORDER}"
            ),
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to read the monitoring configuration heads")?;
    revisions(rows)
}

/// The event and the revision each document's head names, removals included,
/// read by one statement, so they are of one moment whatever the isolation
/// level; `None` without a monitoring row.
#[instrument(err, skip(transaction))]
pub async fn get_event_and_heads(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> Result<Option<(MonitoringEvent, Vec<StoredRevision>)>> {
    let rows = transaction
        .query(
            &format!(
                "SELECT {EVENT_COLUMNS}, {REVISION_COLUMNS}
                 FROM sequent_backend.monitoring_event e
                 LEFT JOIN sequent_backend.monitoring_config_head h
                     ON h.tenant_id = e.tenant_id AND h.election_event_id = e.election_event_id
                 LEFT JOIN sequent_backend.monitoring_config c
                     ON c.tenant_id = h.tenant_id AND c.election_event_id = h.election_event_id
                    AND c.kind = h.kind AND c.key = h.key AND c.revision = h.revision
                 WHERE e.tenant_id = $1 AND e.election_event_id = $2
                 ORDER BY {DOCUMENT_ORDER}"
            ),
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to read the monitoring event and its heads")?;
    event_and_revisions(rows)
}

/// The event as it is now, and the last revision of each document written
/// by the change that raised the generation to `generation` or by one
/// before it, removals included; read by one statement. `None` without a
/// monitoring row.
#[instrument(err, skip(transaction))]
pub async fn get_event_and_revisions_at(
    transaction: &Transaction<'_>,
    event: EventRef,
    generation: i64,
) -> Result<Option<(MonitoringEvent, Vec<StoredRevision>)>> {
    let rows = transaction
        .query(
            &format!(
                "SELECT {EVENT_COLUMNS}, {REVISION_COLUMNS}
                 FROM sequent_backend.monitoring_event e
                 LEFT JOIN LATERAL (
                     SELECT DISTINCT ON (r.kind, r.key) r.*
                     FROM sequent_backend.monitoring_config r
                     WHERE r.tenant_id = e.tenant_id
                       AND r.election_event_id = e.election_event_id
                       AND r.config_generation <= $3
                     ORDER BY r.kind, r.key, r.revision DESC
                 ) c ON true
                 WHERE e.tenant_id = $1 AND e.election_event_id = $2
                 ORDER BY {DOCUMENT_ORDER}"
            ),
            &[&event.tenant_id, &event.election_event_id, &generation],
        )
        .await
        .context("Failed to read the monitoring configuration at a generation")?;
    event_and_revisions(rows)
}

/// Whether the election event exists.
#[instrument(err, skip(client))]
pub async fn election_event_exists(client: &impl GenericClient, event: EventRef) -> Result<bool> {
    Ok(client
        .query_opt(
            "SELECT 1 FROM sequent_backend.election_event WHERE tenant_id = $1 AND id = $2",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to read the election event")?
        .is_some())
}

/// Whether the election event exists, holding it so it cannot be deleted
/// before the transaction ends.
#[instrument(err, skip(transaction))]
pub async fn lock_election_event(transaction: &Transaction<'_>, event: EventRef) -> Result<bool> {
    Ok(transaction
        .query_opt(
            "SELECT 1 FROM sequent_backend.election_event
             WHERE tenant_id = $1 AND id = $2
             FOR KEY SHARE",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to read the election event")?
        .is_some())
}

/// Makes now the checks the tables defer to commit, so that a change the
/// electoral log is about to record is one that will commit.
#[instrument(err, skip(transaction))]
pub async fn check_deferred_constraints(transaction: &Transaction<'_>) -> Result<()> {
    transaction
        .batch_execute("SET CONSTRAINTS ALL IMMEDIATE")
        .await
        .context("The monitoring configuration change does not pass the tables' checks")
}

/// The revision a document's head names; `None` for a document never saved.
#[instrument(err, skip(transaction))]
pub async fn get_head(
    transaction: &Transaction<'_>,
    event: EventRef,
    kind: ConfigKind,
    key: &str,
) -> Result<Option<StoredRevision>> {
    let rows = transaction
        .query(
            &format!(
                "SELECT {REVISION_COLUMNS}
                 FROM sequent_backend.monitoring_config_head h
                 JOIN sequent_backend.monitoring_config c USING
                     (tenant_id, election_event_id, kind, key, revision)
                 WHERE h.tenant_id = $1 AND h.election_event_id = $2
                   AND h.kind = $3 AND h.key = $4"
            ),
            &[
                &event.tenant_id,
                &event.election_event_id,
                &kind.to_string(),
                &key,
            ],
        )
        .await
        .context("Failed to read the monitoring configuration head")?;
    Ok(revisions(rows)?.pop())
}

/// Moves a document's head from `from` to the next revision. False when the
/// head is not at `from`: the conflict to report.
#[instrument(err, skip(transaction))]
pub async fn advance_head(
    transaction: &Transaction<'_>,
    event: EventRef,
    kind: ConfigKind,
    key: &str,
    from: i32,
) -> Result<bool> {
    let moved = transaction
        .execute(
            "UPDATE sequent_backend.monitoring_config_head
             SET revision = revision + 1, updated_at = now()
             WHERE tenant_id = $1 AND election_event_id = $2 AND kind = $3 AND key = $4
               AND revision = $5",
            &[
                &event.tenant_id,
                &event.election_event_id,
                &kind.to_string(),
                &key,
                &from,
            ],
        )
        .await
        .context("Failed to move the monitoring configuration head")?;
    Ok(moved == 1)
}

/// Starts a new document's head at revision 1. False when it has one.
#[instrument(err, skip(transaction))]
pub async fn create_head(
    transaction: &Transaction<'_>,
    event: EventRef,
    kind: ConfigKind,
    key: &str,
) -> Result<bool> {
    let created = transaction
        .execute(
            "INSERT INTO sequent_backend.monitoring_config_head
                 (tenant_id, election_event_id, kind, key, revision)
             VALUES ($1, $2, $3, $4, 1)
             ON CONFLICT (tenant_id, election_event_id, kind, key) DO NOTHING",
            &[
                &event.tenant_id,
                &event.election_event_id,
                &kind.to_string(),
                &key,
            ],
        )
        .await
        .context("Failed to create the monitoring configuration head")?;
    Ok(created == 1)
}

#[instrument(err, skip(transaction, revision), fields(kind = %revision.kind, key = revision.key, revision = revision.revision))]
pub async fn insert_revision(
    transaction: &Transaction<'_>,
    event: EventRef,
    revision: NewRevision<'_>,
) -> Result<StoredRevision> {
    let row = transaction
        .query_one(
            &format!(
                "INSERT INTO sequent_backend.monitoring_config AS c
                     (tenant_id, election_event_id, kind, key, revision, change, yaml, sha256,
                      origin, preset_id, preset_version, config_generation, author_id,
                      author_name)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                 RETURNING {REVISION_COLUMNS}"
            ),
            &[
                &event.tenant_id,
                &event.election_event_id,
                &revision.kind.to_string(),
                &revision.key,
                &revision.revision,
                &revision.change.to_string(),
                &revision.yaml,
                &revision.sha256,
                &revision.origin.to_string(),
                &revision.preset.map(|preset| preset.id.as_str()),
                &revision.preset.map(|preset| preset.version),
                &revision.config_generation,
                &revision.author.id,
                &revision.author.name,
            ],
        )
        .await
        .context("Failed to store the monitoring configuration revision")?;
    StoredRevision::try_from(&row)
}

/// Every revision of a document, newest first.
#[instrument(err, skip(transaction))]
pub async fn get_revisions(
    transaction: &Transaction<'_>,
    event: EventRef,
    kind: ConfigKind,
    key: &str,
) -> Result<Vec<StoredRevision>> {
    let rows = transaction
        .query(
            &format!(
                "SELECT {REVISION_COLUMNS}
                 FROM sequent_backend.monitoring_config c
                 WHERE c.tenant_id = $1 AND c.election_event_id = $2
                   AND c.kind = $3 AND c.key = $4
                 ORDER BY c.revision DESC"
            ),
            &[
                &event.tenant_id,
                &event.election_event_id,
                &kind.to_string(),
                &key,
            ],
        )
        .await
        .context("Failed to read the monitoring configuration history")?;
    revisions(rows)
}

#[instrument(err, skip(transaction))]
pub async fn get_revision(
    transaction: &Transaction<'_>,
    event: EventRef,
    kind: ConfigKind,
    key: &str,
    revision: i32,
) -> Result<Option<StoredRevision>> {
    let rows = transaction
        .query(
            &format!(
                "SELECT {REVISION_COLUMNS}
                 FROM sequent_backend.monitoring_config c
                 WHERE c.tenant_id = $1 AND c.election_event_id = $2
                   AND c.kind = $3 AND c.key = $4 AND c.revision = $5"
            ),
            &[
                &event.tenant_id,
                &event.election_event_id,
                &kind.to_string(),
                &key,
                &revision,
            ],
        )
        .await
        .context("Failed to read the monitoring configuration revision")?;
    Ok(revisions(rows)?.pop())
}
