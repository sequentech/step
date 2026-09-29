// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The monitoring configuration tables: an event's mode and generation, the
//! head of each document and its revisions. The order the writes must come
//! in, and what a stored revision means, are the migration's; see
//! `services::monitoring::config_store` for the protocol that follows it.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
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

fn preset(row: &Row) -> Result<Option<PresetVersion>> {
    let id: Option<String> = row.try_get("preset_id")?;
    let version: Option<i32> = row.try_get("preset_version")?;
    Ok(id
        .zip(version)
        .map(|(id, version)| PresetVersion { id, version }))
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
