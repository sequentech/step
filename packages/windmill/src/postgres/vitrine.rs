// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Vitrine tables: the logs the sidecar created for an election event, its
//! policy, and the statements windmill appended to its `events` log. Hasura
//! doesn't track them, because they hold the `events` log's submission token.

use crate::services::vitrine::LogName;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{TransparencyMode, VitrinePresentation};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use std::str::FromStr;
use strum_macros::{AsRefStr, Display, EnumString};
use tokio_postgres::Row;
use uuid::Uuid;

/// A log of an election event, as the sidecar created it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VitrineLog {
    pub name: LogName,
    pub sidecar_name: String,
    pub log_id: i64,
    pub instance_id: String,
    pub origin: String,
    pub url: String,
    pub vkey: String,
    pub log_time_vkey: String,
    pub visibility: TransparencyMode,
    /// The `events` log's only.
    pub submission_token: Option<String>,
    /// The log's `vitrine/policy-ack/v1` note for the uploaded policy.
    pub policy_ack: Option<String>,
    /// The log's `vitrine/witness-enroll/v1` note for the uploaded policy.
    pub enrolment: Option<String>,
    pub frozen: bool,
}

const LOG_COLUMNS: &str = "name, sidecar_name, log_id, instance_id, origin, url, vkey, \
     log_time_vkey, visibility, submission_token, policy_ack, enrolment, \
     frozen_at IS NOT NULL AS frozen";

impl TryFrom<&Row> for VitrineLog {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self> {
        let name: String = row.try_get("name")?;
        let visibility: String = row.try_get("visibility")?;
        Ok(VitrineLog {
            name: LogName::from_str(&name).map_err(|_| anyhow!("Unknown Vitrine log {name}"))?,
            sidecar_name: row.try_get("sidecar_name")?,
            log_id: row.try_get("log_id")?,
            instance_id: row.try_get("instance_id")?,
            origin: row.try_get("origin")?,
            url: row.try_get("url")?,
            vkey: row.try_get("vkey")?,
            log_time_vkey: row.try_get("log_time_vkey")?,
            visibility: TransparencyMode::from_str(&visibility)
                .map_err(|_| anyhow!("Unknown Vitrine log visibility {visibility}"))?,
            submission_token: row.try_get("submission_token")?,
            policy_ack: row.try_get("policy_ack")?,
            enrolment: row.try_get("enrolment")?,
            frozen: row.try_get("frozen")?,
        })
    }
}

fn event_ids(tenant_id: &str, election_event_id: &str) -> Result<(Uuid, Uuid)> {
    Ok((
        parse_uuid_v4(tenant_id).context("Error parsing tenant_id as UUID")?,
        parse_uuid_v4(election_event_id).context("Error parsing election_event_id as UUID")?,
    ))
}

/// The event's logs, `ballots` first.
pub async fn get_vitrine_logs(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<VitrineLog>> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    let rows = tx
        .query(
            &format!(
                "SELECT {LOG_COLUMNS} FROM sequent_backend.vitrine_log \
                 WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY name"
            ),
            &[&tenant, &event],
        )
        .await
        .context("Error reading the event's Vitrine logs")?;
    rows.iter().map(VitrineLog::try_from).collect()
}

pub async fn get_vitrine_log(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    name: LogName,
) -> Result<Option<VitrineLog>> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    let row = tx
        .query_opt(
            &format!(
                "SELECT {LOG_COLUMNS} FROM sequent_backend.vitrine_log \
                 WHERE tenant_id = $1 AND election_event_id = $2 AND name = $3"
            ),
            &[&tenant, &event, &name.as_ref()],
        )
        .await
        .with_context(|| format!("Error reading the event's Vitrine {name} log"))?;
    row.as_ref().map(VitrineLog::try_from).transpose()
}

pub async fn insert_vitrine_log(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    log: &VitrineLog,
) -> Result<()> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    tx.execute(
        "INSERT INTO sequent_backend.vitrine_log \
         (tenant_id, election_event_id, name, sidecar_name, log_id, instance_id, origin, url, \
          vkey, log_time_vkey, visibility, submission_token) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        &[
            &tenant,
            &event,
            &log.name.as_ref(),
            &log.sidecar_name,
            &log.log_id,
            &log.instance_id,
            &log.origin,
            &log.url,
            &log.vkey,
            &log.log_time_vkey,
            &log.visibility.to_string(),
            &log.submission_token,
        ],
    )
    .await
    .with_context(|| format!("Error storing the event's Vitrine {} log", log.name))?;
    Ok(())
}

/// Stores a log's acknowledgement of the uploaded policy and its witness
/// enrolment note for it.
pub async fn set_vitrine_log_policy(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    name: LogName,
    policy_ack: &str,
    enrolment: Option<&str>,
) -> Result<()> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    tx.execute(
        "UPDATE sequent_backend.vitrine_log SET policy_ack = $4, enrolment = $5 \
         WHERE tenant_id = $1 AND election_event_id = $2 AND name = $3",
        &[&tenant, &event, &name.as_ref(), &policy_ack, &enrolment],
    )
    .await
    .with_context(|| format!("Error storing the Vitrine {name} log's policy"))?;
    Ok(())
}

pub async fn set_vitrine_log_frozen(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    name: LogName,
) -> Result<()> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    tx.execute(
        "UPDATE sequent_backend.vitrine_log SET frozen_at = coalesce(frozen_at, now()) \
         WHERE tenant_id = $1 AND election_event_id = $2 AND name = $3",
        &[&tenant, &event, &name.as_ref()],
    )
    .await
    .with_context(|| format!("Error marking the Vitrine {name} log frozen"))?;
    Ok(())
}

/// The event's trust policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VitrineEvent {
    pub policy_salt: Option<String>,
    pub policy: Option<String>,
    /// `sha256:` and the hex digits, as the sidecar returned it.
    pub policy_id: Option<String>,
}

pub async fn get_vitrine_event(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<VitrineEvent>> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    let row = tx
        .query_opt(
            "SELECT policy_salt, policy, policy_id FROM sequent_backend.vitrine_event \
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&tenant, &event],
        )
        .await
        .context("Error reading the event's Vitrine policy")?;
    row.map(|row| {
        Ok(VitrineEvent {
            policy_salt: row.try_get("policy_salt")?,
            policy: row.try_get("policy")?,
            policy_id: row
                .try_get::<_, Option<String>>("policy_id")?
                .map(|id| electoral_log::vitrine::normalize_policy_id(&id))
                .transpose()?,
        })
    })
    .transpose()
}

/// The event's row, created with `policy_salt` if it has none; a salt it
/// has already is kept, so that the drafts of a private event don't change.
pub async fn ensure_vitrine_event(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    policy_salt: Option<&str>,
) -> Result<VitrineEvent> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    tx.execute(
        "INSERT INTO sequent_backend.vitrine_event (tenant_id, election_event_id, policy_salt) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (tenant_id, election_event_id) DO UPDATE \
         SET policy_salt = coalesce(vitrine_event.policy_salt, EXCLUDED.policy_salt)",
        &[&tenant, &event, &policy_salt],
    )
    .await
    .context("Error storing the event's Vitrine policy salt")?;
    get_vitrine_event(tx, tenant_id, election_event_id)
        .await?
        .ok_or_else(|| anyhow!("The event's Vitrine row is missing"))
}

pub async fn set_vitrine_event_policy(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    policy: &str,
    policy_id: &str,
) -> Result<()> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    tx.execute(
        "INSERT INTO sequent_backend.vitrine_event \
         (tenant_id, election_event_id, policy, policy_id, policy_uploaded_at) \
         VALUES ($1, $2, $3, $4, now()) \
         ON CONFLICT (tenant_id, election_event_id) DO UPDATE \
         SET policy = EXCLUDED.policy, policy_id = EXCLUDED.policy_id, \
             policy_uploaded_at = EXCLUDED.policy_uploaded_at",
        &[&tenant, &event, &policy, &policy_id],
    )
    .await
    .context("Error storing the event's Vitrine policy")?;
    Ok(())
}

/// Whether the event's one voting window started: a channel of the event or
/// of one of its elections opened, or windmill logged `voting-opened`.
pub async fn vitrine_voting_opened(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<bool> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    let row = tx
        .query_one(
            "SELECT sequent_backend.vitrine_voting_opened($1, $2)",
            &[&tenant, &event],
        )
        .await
        .context("Error checking whether voting opened")?;
    Ok(row.try_get(0)?)
}

/// Waits for, and holds until the transaction ends, the lock of the event's
/// row, which orders the opening and closing checks with the statements
/// windmill logs.
pub async fn lock_election_event(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    tx.query_opt(
        "SELECT 1 FROM sequent_backend.election_event \
         WHERE tenant_id = $1 AND id = $2 FOR NO KEY UPDATE",
        &[&tenant, &event],
    )
    .await
    .context("Error locking the election event")?
    .ok_or_else(|| anyhow!("Election event {election_event_id} not found"))?;
    Ok(())
}

/// The statement types of the `events` log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum StatementKind {
    Config,
    Milestone,
}

/// A statement windmill appended to the event's `events` log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VitrineStatement {
    /// What it is, once per event: `config:<election id>`, `voting-opened`,
    /// `voting-closed`, `tally-completed:<tally session id>` or
    /// `results-published:<publication id>`.
    pub key: String,
    pub kind: StatementKind,
    pub election_id: Option<String>,
    /// The signed note.
    pub statement: String,
    /// The `events` entry, in base64.
    pub entry: String,
    pub leaf_hash: String,
    /// Config statements: the hex digests of the election's ballot styles.
    pub style_hashes: Option<Vec<String>>,
    /// Its receipt chain, once a checkpoint that meets the quorum covers it.
    pub chain: Option<String>,
}

impl TryFrom<&Row> for VitrineStatement {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self> {
        let kind: String = row.try_get("kind")?;
        Ok(VitrineStatement {
            key: row.try_get("key")?,
            kind: StatementKind::from_str(&kind)
                .map_err(|_| anyhow!("Unknown Vitrine statement kind {kind}"))?,
            election_id: row
                .try_get::<_, Option<Uuid>>("election_id")?
                .map(|id| id.to_string()),
            statement: row.try_get("statement")?,
            entry: row.try_get("entry")?,
            leaf_hash: row.try_get("leaf_hash")?,
            style_hashes: row.try_get("style_hashes")?,
            chain: row.try_get("chain")?,
        })
    }
}

const STATEMENT_COLUMNS: &str =
    "key, kind, election_id, statement, entry, leaf_hash, style_hashes, chain";

/// Stores a statement before windmill submits it, unless the event has one
/// with its key, and returns the one stored: a retry submits the same bytes.
pub async fn insert_vitrine_statement(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    statement: &VitrineStatement,
) -> Result<VitrineStatement> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    let election = statement
        .election_id
        .as_deref()
        .map(parse_uuid_v4)
        .transpose()
        .context("Error parsing election_id as UUID")?;
    tx.execute(
        "INSERT INTO sequent_backend.vitrine_statement \
         (tenant_id, election_event_id, key, kind, election_id, statement, entry, leaf_hash, \
          style_hashes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT (tenant_id, election_event_id, key) DO NOTHING",
        &[
            &tenant,
            &event,
            &statement.key,
            &statement.kind.as_ref(),
            &election,
            &statement.statement,
            &statement.entry,
            &statement.leaf_hash,
            &statement.style_hashes,
        ],
    )
    .await
    .with_context(|| format!("Error storing the Vitrine statement {}", statement.key))?;
    get_vitrine_statement(tx, tenant_id, election_event_id, &statement.key)
        .await?
        .ok_or_else(|| anyhow!("The Vitrine statement {} is missing", statement.key))
}

pub async fn get_vitrine_statement(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    key: &str,
) -> Result<Option<VitrineStatement>> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    let row = tx
        .query_opt(
            &format!(
                "SELECT {STATEMENT_COLUMNS} FROM sequent_backend.vitrine_statement \
                 WHERE tenant_id = $1 AND election_event_id = $2 AND key = $3"
            ),
            &[&tenant, &event, &key],
        )
        .await
        .with_context(|| format!("Error reading the Vitrine statement {key}"))?;
    row.as_ref().map(VitrineStatement::try_from).transpose()
}

/// The event's statements, in the order windmill made them.
pub async fn get_vitrine_statements(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<VitrineStatement>> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    let rows = tx
        .query(
            &format!(
                "SELECT {STATEMENT_COLUMNS} FROM sequent_backend.vitrine_statement \
                 WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY created_at, key"
            ),
            &[&tenant, &event],
        )
        .await
        .context("Error reading the event's Vitrine statements")?;
    rows.iter().map(VitrineStatement::try_from).collect()
}

pub async fn set_vitrine_statement_chain(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    key: &str,
    chain: &str,
) -> Result<()> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    tx.execute(
        "UPDATE sequent_backend.vitrine_statement SET chain = $4, receipted_at = now() \
         WHERE tenant_id = $1 AND election_event_id = $2 AND key = $3 AND chain IS NULL",
        &[&tenant, &event, &key, &chain],
    )
    .await
    .with_context(|| format!("Error storing the receipt chain of {key}"))?;
    Ok(())
}

/// The events with a policy and an unfrozen `events` log that have a
/// statement still to receipt, or whose voting window opened and has no
/// `voting-closed` statement yet, as `(tenant ID, election event ID)`.
pub async fn get_vitrine_pending_events(tx: &Transaction<'_>) -> Result<Vec<(String, String)>> {
    let rows = tx
        .query(
            "SELECT l.tenant_id::text, l.election_event_id::text \
             FROM sequent_backend.vitrine_log l \
             JOIN sequent_backend.vitrine_event e \
               ON e.tenant_id = l.tenant_id AND e.election_event_id = l.election_event_id \
             WHERE l.name = 'events' AND l.frozen_at IS NULL AND e.policy_id IS NOT NULL AND ( \
                 EXISTS (SELECT 1 FROM sequent_backend.vitrine_statement s \
                         WHERE s.tenant_id = l.tenant_id \
                           AND s.election_event_id = l.election_event_id \
                           AND s.chain IS NULL) \
                 OR (sequent_backend.vitrine_voting_opened(l.tenant_id, l.election_event_id) \
                     AND NOT EXISTS (SELECT 1 FROM sequent_backend.vitrine_statement s \
                                     WHERE s.tenant_id = l.tenant_id \
                                       AND s.election_event_id = l.election_event_id \
                                       AND s.key = 'voting-closed')))",
            &[],
        )
        .await
        .context("Error listing the events with Vitrine statements to log")?;
    rows.iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
        .collect()
}

/// Sets the event's `presentation.transparency` and `presentation.vitrine`
/// (removed when `None`) in one statement, leaving the other keys as they
/// are, so that an edit of the event's presentation made meanwhile stays.
pub async fn update_presentation_transparency(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    mode: TransparencyMode,
    vitrine: Option<&VitrinePresentation>,
) -> Result<()> {
    let (tenant, event) = event_ids(tenant_id, election_event_id)?;
    let vitrine = vitrine.map(serde_json::to_value).transpose()?;
    tx.execute(
        "UPDATE sequent_backend.election_event \
         SET presentation = (coalesce(presentation, '{}'::jsonb) - 'transparency' - 'vitrine') \
             || jsonb_build_object('transparency', $3::text) \
             || CASE WHEN $4::jsonb IS NULL THEN '{}'::jsonb \
                     ELSE jsonb_build_object('vitrine', $4::jsonb) END \
         WHERE tenant_id = $1 AND id = $2",
        &[&tenant, &event, &mode.to_string(), &vitrine],
    )
    .await
    .context("Error updating the event's transparency")?;
    Ok(())
}
