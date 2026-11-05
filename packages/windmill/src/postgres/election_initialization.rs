// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The dated record of each initialization: one row per country (area) of a
//! Post that an initialization report covered (`election_initialization`).

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use tokio_postgres::Row;
use tracing::instrument;
use uuid::Uuid;

/// One initialization of a country of a Post (a Post without countries has
/// one with `area_id: None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectionInitialization {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub area_id: Option<Uuid>,
    pub tally_session_id: Uuid,
    pub results_event_id: Option<Uuid>,
    pub report_hash: Option<String>,
    pub document_id: Option<Uuid>,
    pub created_by: Option<String>,
    pub created_by_username: Option<String>,
    pub election_name: Option<String>,
    pub area_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub log_staged_at: Option<DateTime<Utc>>,
}

/// What the tally records for one country.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewElectionInitialization {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub area_id: Option<Uuid>,
    pub tally_session_id: Uuid,
    pub results_event_id: Option<Uuid>,
    pub report_hash: Option<String>,
    pub document_id: Option<Uuid>,
    pub created_by: Option<String>,
    pub created_by_username: Option<String>,
    pub election_name: Option<String>,
    pub area_name: Option<String>,
}

impl TryFrom<Row> for ElectionInitialization {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(ElectionInitialization {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            election_id: row.try_get("election_id")?,
            area_id: row.try_get("area_id")?,
            tally_session_id: row.try_get("tally_session_id")?,
            results_event_id: row.try_get("results_event_id")?,
            report_hash: row.try_get("report_hash")?,
            document_id: row.try_get("document_id")?,
            created_by: row.try_get("created_by")?,
            created_by_username: row.try_get("created_by_username")?,
            election_name: row.try_get("election_name")?,
            area_name: row.try_get("area_name")?,
            created_at: row.try_get("created_at")?,
            log_staged_at: row.try_get("log_staged_at")?,
        })
    }
}

const COLUMNS: &str = "id, tenant_id, election_event_id, election_id, area_id, tally_session_id,
    results_event_id, report_hash, document_id, created_by, created_by_username, election_name,
    area_name, created_at, log_staged_at";

/// Records one initialization and returns it, dated by the database.
#[instrument(skip(hasura_transaction), err)]
pub async fn insert_election_initialization(
    hasura_transaction: &Transaction<'_>,
    initialization: &NewElectionInitialization,
) -> Result<ElectionInitialization> {
    let row = hasura_transaction
        .query_one(
            &format!(
                "INSERT INTO sequent_backend.election_initialization
                     (tenant_id, election_event_id, election_id, area_id, tally_session_id,
                      results_event_id, report_hash, document_id, created_by, created_by_username,
                      election_name, area_name)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                 RETURNING {COLUMNS}"
            ),
            &[
                &initialization.tenant_id,
                &initialization.election_event_id,
                &initialization.election_id,
                &initialization.area_id,
                &initialization.tally_session_id,
                &initialization.results_event_id,
                &initialization.report_hash,
                &initialization.document_id,
                &initialization.created_by,
                &initialization.created_by_username,
                &initialization.election_name,
                &initialization.area_name,
            ],
        )
        .await
        .context("Error recording the initialization")?;
    row.try_into()
}

/// Every initialization of the event, oldest first.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_election_initializations(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<ElectionInitialization>> {
    hasura_transaction
        .query(
            &format!(
                "SELECT {COLUMNS} FROM sequent_backend.election_initialization
                 WHERE tenant_id = $1 AND election_event_id = $2
                 ORDER BY created_at, id"
            ),
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the initializations")?
        .into_iter()
        .map(ElectionInitialization::try_from)
        .collect()
}

/// Locks a Post's row until the transaction ends, so that two reports of
/// the Post completing at once record its countries one after the other.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_post_for_initialization(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
) -> Result<()> {
    hasura_transaction
        .query_opt(
            "SELECT id FROM sequent_backend.election
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3
             FOR UPDATE",
            &[&tenant_id, &election_event_id, &election_id],
        )
        .await
        .context("Error locking the Post for its initialization")?
        .ok_or_else(|| anyhow::anyhow!("Post {election_id} not found"))?;
    Ok(())
}

/// The initializations of the event whose electoral log entry isn't
/// staged yet, oldest first, locked until the transaction ends.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_unstaged_initializations(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<ElectionInitialization>> {
    hasura_transaction
        .query(
            &format!(
                "SELECT {COLUMNS} FROM sequent_backend.election_initialization
                 WHERE tenant_id = $1 AND election_event_id = $2 AND log_staged_at IS NULL
                 ORDER BY created_at, id
                 FOR UPDATE SKIP LOCKED"
            ),
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the initializations to log")?
        .into_iter()
        .map(ElectionInitialization::try_from)
        .collect()
}

/// Marks an initialization's electoral log entry as staged.
#[instrument(skip(hasura_transaction), err)]
pub async fn mark_initialization_log_staged(
    hasura_transaction: &Transaction<'_>,
    id: Uuid,
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.election_initialization
             SET log_staged_at = clock_timestamp() WHERE id = $1",
            &[&id],
        )
        .await
        .context("Error marking the initialization as logged")?;
    Ok(())
}

/// The (Post, area) pairs of the event that have a ballot style: where
/// voters of a Post vote.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_post_ballot_style_areas(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<(String, String)>> {
    Ok(hasura_transaction
        .query(
            "SELECT DISTINCT election_id::text, area_id::text FROM sequent_backend.ballot_style
             WHERE tenant_id = $1 AND election_event_id = $2
               AND deleted_at IS NULL AND area_id IS NOT NULL",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the Posts' ballot styles")?
        .into_iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect())
}

/// The event's scheduled closes: (the Post, or `None` for the event-wide
/// one, and the instant), from active rows of either state.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_scheduled_closes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<(Option<String>, String)>> {
    Ok(hasura_transaction
        .query(
            "SELECT event_payload->>'election_id', cron_config->>'scheduled_date'
             FROM sequent_backend.scheduled_event
             WHERE tenant_id = $1 AND election_event_id = $2
               AND event_processor = 'END_VOTING_PERIOD' AND archived_at IS NULL
               AND cron_config->>'scheduled_date' IS NOT NULL",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the scheduled closes")?
        .into_iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect())
}

/// The hash and documents of a Post's report in a results event, if the
/// results have that Post.
#[instrument(skip(hasura_transaction), err)]
pub async fn find_post_report(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_id: Uuid,
    results_event_id: Uuid,
) -> Result<Option<(Option<serde_json::Value>, Option<serde_json::Value>)>> {
    Ok(hasura_transaction
        .query_opt(
            "SELECT annotations, documents FROM sequent_backend.results_election
             WHERE tenant_id = $1 AND election_id = $2 AND results_event_id = $3
             ORDER BY created_at DESC LIMIT 1",
            &[&tenant_id, &election_id, &results_event_id],
        )
        .await
        .context("Error reading the Post's initialization report")?
        .map(|row| (row.get(0), row.get(1))))
}

/// Sets one key of a scheduled event's annotations.
#[instrument(skip(hasura_transaction, value), err)]
pub async fn set_scheduled_event_annotation(
    hasura_transaction: &Transaction<'_>,
    scheduled_event_id: Uuid,
    key: &str,
    value: &serde_json::Value,
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.scheduled_event
             SET annotations = jsonb_set(COALESCE(annotations, '{}'::jsonb), ARRAY[$2::text], $3)
             WHERE id = $1",
            &[&scheduled_event_id, &key, value],
        )
        .await
        .context("Error annotating the scheduled event")?;
    Ok(())
}

/// Keeps actual per-Post country coverage once, in the session creation transaction.
/// Hasura cannot write this private evidence and later session edits cannot replace it.
pub async fn insert_initialization_report_coverage(
    transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    session: Uuid,
    coverage: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<()> {
    for (post, countries) in coverage {
        Uuid::parse_str(post)?;
        for country in countries {
            Uuid::parse_str(country)?;
        }
    }
    crate::postgres::trusted_write(transaction).await?;
    transaction.execute(
        "INSERT INTO sequent_backend.initialization_report_coverage (tenant_id, election_event_id, tally_session_id, coverage)
         VALUES ($1, $2, $3, $4)",
        &[&tenant, &event, &session, &serde_json::to_value(coverage)?],
    ).await.context("Error keeping initialization report coverage")?;
    Ok(())
}

/// Actual creation-time coverage; absence on an unfinished legacy session
/// requires creating a fresh initialization report rather than guessing.
pub async fn initialization_report_coverage(
    transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    session: Uuid,
) -> Result<Option<std::collections::BTreeMap<String, Vec<String>>>> {
    transaction
        .query_opt(
            "SELECT coverage FROM sequent_backend.initialization_report_coverage
         WHERE tenant_id = $1 AND election_event_id = $2 AND tally_session_id = $3",
            &[&tenant, &event, &session],
        )
        .await
        .context("Error reading initialization report coverage")?
        .map(|row| {
            serde_json::from_value(row.try_get::<_, serde_json::Value>(0)?)
                .map_err(anyhow::Error::from)
        })
        .transpose()
}
