// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The statements windmill appends to a transparent event's `events` log
//! (step-integration.md, section 2.9; Vitrine's `specs/entry.md`, sections
//! 5.1 to 5.3): when the event's one voting window opens, a
//! `vitrine/config/v1` statement per election and the `voting-opened`
//! milestone that names them; once its last election closed, `voting-closed`
//! with each election's ballot count; and a milestone when a tally completes
//! and when results are published. Each is signed with the operator's
//! `authority` key and stored before it is submitted, so that every retry
//! submits the same bytes, and its receipt chain is kept once a checkpoint
//! that meets the policy's quorum contains it.

use super::client::{Answer, Service, SidecarClient, MAX_WAIT_MS};
use super::policy::{public_refusal, PolicyFacts};
use super::transparency::{check_policy_facts, transparency_mode};
use super::window::{grace_secs, timestamp};
use super::{canonical_id, LogName};
use crate::postgres::ballot_style::get_ballot_styles_by_elections;
use crate::postgres::election::get_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::postgres::vitrine::{
    get_vitrine_event, get_vitrine_log, get_vitrine_logs, get_vitrine_pending_events,
    get_vitrine_statement, get_vitrine_statements, insert_vitrine_statement, lock_election_event,
    set_vitrine_statement_chain, vitrine_voting_opened, StatementKind, VitrineLog,
    VitrineStatement,
};
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_status::get_election_status;
use crate::services::protocol_manager::get_event_store;
use crate::tasks::vitrine_statements::log_vitrine_statements;
use anyhow::{anyhow, bail, ensure, Context, Result};
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use base64::Engine;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::vitrine::proof::{BLOCK_POLICY_ACK, BLOCK_RECORD};
use electoral_log::vitrine::{
    election_key_hash, leaf_hash, receipt_chain, style_digest, style_list, styles_hash, ChainBlock,
    ConfigStatement, Demo, EventKind, EventsEntry, Hash, MilestoneKind, MilestoneStatement,
    NoteSigner, Release, Subject, Submitted, Visibility,
};
use sequent_core::ballot::{PublicKeyConfig, TransparencyMode, VotingStatus, VotingStatusChannel};
use sequent_core::types::hasura::core::{Election, ElectionEvent};
use sequent_core::types::scheduled_event::{generate_voting_period_dates, ScheduledEvent};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::Duration;
use strum::IntoEnumIterator;
use tracing::{info, warn};

/// The operator's `authority` key, a `PRIVATE+KEY+<name>+<key ID>+<key>`
/// file, which signs the statements.
pub const AUTHORITY_KEY_FILE_ENV: &str = "WINDMILL_VITRINE_AUTHORITY_KEY_FILE";
/// The voting portal's release, which the config statements list and the
/// booth checks against its own.
pub const PORTAL_VERSION_ENV: &str = "WINDMILL_VITRINE_PORTAL_VERSION";
pub const PORTAL_PACKAGE: &str = "sequent/voting-portal";
pub const VOTING_OPENED_KEY: &str = MilestoneKind::VOTING_OPENED;
pub const VOTING_CLOSED_KEY: &str = MilestoneKind::VOTING_CLOSED;
const CONFIG_KEY_PREFIX: &str = "config:";
const RELEASE_LINE: &str = "release ";
const AUTHORITY_LINE: &str = "authority";
/// How long after the last close, grace periods included, `voting-closed`
/// waits before it counts the ballots, so that the casts that passed their
/// status check before the close have committed.
pub const CLOSE_SETTLE_SECS: i64 = 60;
/// How long one run waits for receipts that meet the quorum before it
/// leaves the rest to the next.
const RECEIPT_WAIT: Duration = Duration::from_secs(20);
const RECEIPT_POLL: Duration = Duration::from_secs(1);
/// A run queued at a status change starts after the change's transaction
/// commits.
const QUEUE_DELAY_SECS: i64 = 3;
const QUEUE_TIMEOUT: Duration = Duration::from_secs(5);
/// Seconds after which a queued run is dropped, since the sweep queues
/// another.
pub const RUN_EXPIRES_SECS: u32 = 60;
const CHANNELS: [VotingStatusChannel; 4] = [
    VotingStatusChannel::ONLINE,
    VotingStatusChannel::KIOSK,
    VotingStatusChannel::EARLY_VOTING,
    VotingStatusChannel::TELEPHONE,
];

/// A milestone of the event after voting closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EventMilestone {
    TallyCompleted { tally_session_id: String },
    ResultsPublished { publication_id: String },
}

impl EventMilestone {
    /// Its `kind` line.
    pub fn token(&self) -> &'static str {
        match self {
            EventMilestone::TallyCompleted { .. } => "tally-completed",
            EventMilestone::ResultsPublished { .. } => "results-published",
        }
    }

    /// Its statement's key: one per tally session or results publication.
    pub fn key(&self) -> String {
        let id = match self {
            EventMilestone::TallyCompleted { tally_session_id } => tally_session_id,
            EventMilestone::ResultsPublished { publication_id } => publication_id,
        };
        format!("{}:{id}", self.token())
    }
}

/// The key of an election's config statement.
pub fn config_key(election_id: &str) -> Result<String> {
    Ok(format!("{CONFIG_KEY_PREFIX}{}", canonical_id(election_id)?))
}

/// `<package> <version>` of the voting portal's `release` line in a config
/// statement.
pub fn portal_release(statement: &str) -> Option<String> {
    let text = statement.split("\n\n").next()?;
    text.lines().find_map(|line| {
        let (package, version) = line.strip_prefix(RELEASE_LINE)?.split_once(' ')?;
        (package == PORTAL_PACKAGE).then(|| format!("{package} {version}"))
    })
}

/// The voting portal's release, from `WINDMILL_VITRINE_PORTAL_VERSION`.
pub fn deployment_release() -> Result<Release> {
    let version = std::env::var(PORTAL_VERSION_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("{PORTAL_VERSION_ENV} isn't set"))?;
    Ok(Release {
        package: PORTAL_PACKAGE.to_string(),
        version,
    })
}

/// The operator's authority key, from `WINDMILL_VITRINE_AUTHORITY_KEY_FILE`.
pub fn authority_signer() -> Result<NoteSigner> {
    let path = std::env::var(AUTHORITY_KEY_FILE_ENV)
        .map_err(|_| anyhow!("{AUTHORITY_KEY_FILE_ENV} isn't set"))?;
    let text = std::fs::read_to_string(&path).with_context(|| format!("Error reading {path}"))?;
    NoteSigner::from_private_key(&text).with_context(|| format!("{path} isn't a note signing key"))
}

/// Checks that the signer is one of the policy's authorities, so that the
/// logs' rules accept its statements.
pub fn check_authority(signer: &NoteSigner, policy: &str) -> Result<()> {
    let vkey = signer.vkey();
    let text = policy.split("\n\n").next().unwrap_or_default();
    let listed = text.lines().any(|line| {
        let mut words = line.split(' ');
        words.next() == Some(AUTHORITY_LINE) && words.nth(1) == Some(vkey.as_str())
    });
    ensure!(
        listed,
        "The key of {AUTHORITY_KEY_FILE_ENV}, {vkey}, isn't one of the policy's authorities"
    );
    Ok(())
}

fn visibility(mode: TransparencyMode) -> Result<Visibility> {
    match mode {
        TransparencyMode::PUBLIC => Ok(Visibility::Public),
        TransparencyMode::PRIVATE => Ok(Visibility::Private),
        TransparencyMode::DISABLED => bail!("The event's transparency is disabled"),
    }
}

/// What an election's config statement says of its ballot styles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectionConfig {
    pub election_id: String,
    /// SHA-256 over the public key's bytes, as the booth encrypts to it.
    pub key: Hash,
    /// The digests of its distinct styles, in ascending byte order.
    pub style_digests: Vec<Hash>,
    pub demo: Demo,
}

#[derive(Deserialize)]
struct StyleKey {
    public_key: Option<PublicKeyConfig>,
}

/// The config of an election from the exact texts of its ballot styles, as
/// the voting portal receives them.
pub fn election_config(election_id: &str, styles: &[&str]) -> Result<ElectionConfig> {
    let mut key: Option<Hash> = None;
    let mut demo = false;
    let mut digests = Vec::with_capacity(styles.len());
    for style in styles {
        let parsed: StyleKey = serde_json::from_str(style)
            .with_context(|| format!("A ballot style of election {election_id} isn't JSON"))?;
        let public_key = parsed
            .public_key
            .ok_or_else(|| anyhow!("A ballot style of election {election_id} has no public key"))?;
        let bytes = STANDARD_NO_PAD
            .decode(public_key.public_key.as_bytes())
            .with_context(|| format!("The public key of election {election_id} isn't base64"))?;
        let hash = election_key_hash(&bytes);
        ensure!(
            key.is_none_or(|k| k == hash),
            "The ballot styles of election {election_id} have different public keys"
        );
        key = Some(hash);
        demo |= public_key.is_demo;
        digests.push(style_digest(style));
    }
    let key =
        key.ok_or_else(|| anyhow!("Election {election_id} has no published ballot styles"))?;
    Ok(ElectionConfig {
        election_id: canonical_id(election_id)?,
        key,
        style_digests: style_list(&digests),
        demo: if demo { Demo::Yes } else { Demo::No },
    })
}

/// The configs of the event's elections, from their published ballot
/// styles.
pub async fn election_configs(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    elections: &[Election],
) -> Result<Vec<ElectionConfig>> {
    let ids: Vec<String> = elections.iter().map(|e| e.id.clone()).collect();
    let styles = get_ballot_styles_by_elections(tx, tenant_id, election_event_id, &ids).await?;
    elections
        .iter()
        .map(|election| {
            let texts: Vec<&str> = styles
                .iter()
                .filter(|s| s.election_id == election.id)
                .filter_map(|s| s.ballot_eml.as_deref())
                .collect();
            election_config(&election.id, &texts)
        })
        .collect()
}

fn events_row(
    signer: &NoteSigner,
    key: String,
    kind: StatementKind,
    entry_kind: EventKind,
    time: u64,
    text: &str,
) -> Result<VitrineStatement> {
    let note = signer.sign(text)?;
    let entry = EventsEntry {
        kind: entry_kind,
        time,
        subject: Subject::Nothing,
        statement: Some(note.clone()),
        details: None,
    }
    .encode()?;
    Ok(VitrineStatement {
        key,
        kind,
        election_id: None,
        statement: note,
        entry: STANDARD.encode(&entry),
        leaf_hash: hex::encode(leaf_hash(&entry)),
        style_hashes: None,
        chain: None,
    })
}

/// An election's signed config statement and its `events` entry, whose
/// `time` is the statement's (the `statement-matches` rules).
pub fn config_statement(
    signer: &NoteSigner,
    election_event_id: &str,
    config: &ElectionConfig,
    facts: &PolicyFacts,
    policy_id: &str,
    release: &Release,
    time: u64,
) -> Result<VitrineStatement> {
    let text = ConfigStatement {
        event_id: canonical_id(election_event_id)?,
        election_id: config.election_id.clone(),
        key: config.key,
        styles: styles_hash(&config.style_digests),
        opens: u64::try_from(facts.opens).context("The policy opens before 1970")?,
        closes: u64::try_from(facts.closes).context("The policy closes before 1970")?,
        policy_id: policy_id.to_string(),
        releases: vec![release.clone()],
        transparency: visibility(facts.visibility)?,
        demo: config.demo,
        time,
    }
    .text()?;
    let mut row = events_row(
        signer,
        config_key(&config.election_id)?,
        StatementKind::Config,
        EventKind::Config,
        time,
        &text,
    )?;
    row.election_id = Some(config.election_id.clone());
    row.style_hashes = Some(config.style_digests.iter().map(hex::encode).collect());
    Ok(row)
}

/// A signed milestone statement and its `events` entry.
pub fn milestone_statement(
    signer: &NoteSigner,
    key: String,
    milestone: &MilestoneStatement,
) -> Result<VitrineStatement> {
    let entry_kind = match milestone.kind {
        MilestoneKind::VotingOpened => EventKind::VotingOpened,
        MilestoneKind::VotingClosed => EventKind::VotingClosed,
        MilestoneKind::Other(_) => EventKind::Milestone,
    };
    events_row(
        signer,
        key,
        StatementKind::Milestone,
        entry_kind,
        milestone.time,
        &milestone.text()?,
    )
}

/// The receipt chain of a statement: the `events` log's acknowledgement of
/// the policy and the receipt against the checkpoint that meets the quorum.
pub fn statement_chain(policy_id: &str, policy_ack: Option<&str>, receipt: &str) -> Result<String> {
    let mut blocks = Vec::new();
    if let Some(ack) = policy_ack {
        blocks.push(ChainBlock {
            kind: BLOCK_POLICY_ACK,
            args: vec![],
            data: ack,
        });
    }
    blocks.push(ChainBlock {
        kind: BLOCK_RECORD,
        args: vec![],
        data: receipt,
    });
    receipt_chain(policy_id, LogName::Events.as_ref(), &blocks)
}

fn check_receipt(receipt: &str, entry: &[u8]) -> Result<()> {
    match Submitted::parse(receipt)? {
        Submitted::Receipt { entry: logged } if logged == entry => Ok(()),
        _ => bail!("The sidecar's receipt isn't for the entry"),
    }
}

fn leaf_of(statement: &VitrineStatement) -> Result<Hash> {
    hex::decode(&statement.leaf_hash)?
        .try_into()
        .map_err(|_| anyhow!("The leaf hash of {} isn't 32 bytes", statement.key))
}

fn now_secs() -> i64 {
    Utc::now().timestamp()
}

fn date(time: i64) -> String {
    DateTime::from_timestamp(time, 0)
        .map(|t| t.to_rfc3339())
        .unwrap_or_else(|| time.to_string())
}

/// When the last ballot of the event can have been accepted, once no channel
/// of any election is open or paused and every election closed or the
/// policy's window ended: the latest close, or the end of an online grace
/// period after it, whichever is later. `None` while voting can go on.
pub fn voting_end(
    elections: &[Election],
    scheduled: &[ScheduledEvent],
    tenant_id: &str,
    election_event_id: &str,
    policy_closes: i64,
    now: i64,
) -> Result<Option<i64>> {
    if elections.is_empty() {
        return Ok(None);
    }
    let mut end = 0_i64;
    for election in elections {
        let status = get_election_status(election.status.clone()).unwrap_or_default();
        let mut closed = false;
        for channel in CHANNELS {
            match status.status_by_channel(channel) {
                VotingStatus::OPEN | VotingStatus::PAUSED => return Ok(None),
                VotingStatus::CLOSED => closed = true,
                VotingStatus::NOT_STARTED => {}
            }
            if let Some(stopped) = status.dates_by_channel(channel).last_stopped_at {
                end = end.max(stopped.timestamp());
            }
        }
        if !closed && now < policy_closes {
            return Ok(None);
        }
        let grace = grace_secs(election)?;
        if grace > 0 {
            let dates = generate_voting_period_dates(
                scheduled.to_vec(),
                tenant_id,
                election_event_id,
                Some(&election.id),
            )?;
            let online_close = timestamp(dates.end_date)?.or(status
                .voting_period_dates
                .last_stopped_at
                .map(|t| t.timestamp()));
            if let Some(close) = online_close {
                end = end.max(close.saturating_add(grace));
            }
        }
    }
    Ok(Some(end))
}

/// Whether the `ballots` log sequenced its whole queue into a checkpoint
/// that meets the quorum, from its internal `status`: then the entries that
/// `voting-closed` counts are sequenced before it.
pub fn ballots_settled(status: &Value) -> bool {
    let rows = status
        .pointer("/internal/queue/rows")
        .and_then(Value::as_u64);
    let committed = status
        .pointer("/internal/committed_size")
        .and_then(Value::as_u64);
    let served = status.pointer("/checkpoint/size").and_then(Value::as_u64);
    let quorum = status
        .pointer("/checkpoint/time")
        .is_some_and(|t| !t.is_null());
    match (rows, committed) {
        (Some(0), Some(0)) => true,
        (Some(0), Some(committed)) => quorum && served.is_some_and(|s| s >= committed),
        _ => false,
    }
}

/// The `count` lines of `voting-closed`: every election of the event, with
/// the ballots the ballot box accepted for it.
pub fn closing_counts(
    elections: &[Election],
    accepted: &[(String, i64)],
) -> Result<Vec<(String, u64)>> {
    let accepted: BTreeMap<String, i64> = accepted
        .iter()
        .map(|(id, n)| Ok((canonical_id(id)?, *n)))
        .collect::<Result<_>>()?;
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for election in elections {
        let id = canonical_id(&election.id)?;
        let n = accepted.get(&id).copied().unwrap_or(0);
        counts.insert(id, u64::try_from(n).context("A negative ballot count")?);
    }
    Ok(counts.into_iter().collect())
}

async fn has_statement(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    key: &str,
) -> Result<bool> {
    Ok(get_vitrine_statement(tx, tenant_id, election_event_id, key)
        .await?
        .is_some())
}

async fn voting_end_of(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    facts: &PolicyFacts,
) -> Result<Option<i64>> {
    let elections = get_elections(tx, tenant_id, election_event_id).await?;
    let scheduled =
        find_scheduled_event_by_election_event_id(tx, tenant_id, election_event_id).await?;
    voting_end(
        &elections,
        &scheduled,
        tenant_id,
        election_event_id,
        facts.closes,
        now_secs(),
    )
}

/// Refuses to open voting in a transparent event that isn't ready for its
/// one voting window, or whose window already closed: with `public` while an
/// election limits revotes, without both logs and the sealed policy, outside
/// the policy's window, for other elections than the policy's, without
/// published ballot styles, or without the authority key and the portal
/// release that the config statements need.
///
/// It locks the event first, as every status change does when it writes the
/// event's status, so that enabling transparency or logging `voting-closed`
/// can't interleave with the change.
pub async fn check_voting_status_change(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    new_status: &VotingStatus,
) -> Result<()> {
    lock_election_event(tx, tenant_id, election_event_id).await?;
    if *new_status != VotingStatus::OPEN {
        return Ok(());
    }
    let event = get_election_event_by_id(tx, tenant_id, election_event_id).await?;
    let mode = transparency_mode(&event)?;
    if !mode.is_enabled() {
        return Ok(());
    }
    ensure!(
        !has_statement(tx, tenant_id, election_event_id, VOTING_CLOSED_KEY).await?,
        "Voting closed in this transparent election event: it can't open again"
    );
    if vitrine_voting_opened(tx, tenant_id, election_event_id).await? {
        return Ok(());
    }
    let elections = get_elections(tx, tenant_id, election_event_id).await?;
    if mode == TransparencyMode::PUBLIC {
        if let Some(refusal) = public_refusal(&elections) {
            bail!("A public transparent event needs unlimited revotes ({refusal})");
        }
    }
    let logs = get_vitrine_logs(tx, tenant_id, election_event_id).await?;
    ensure!(
        logs.len() == LogName::iter().count(),
        "The event's Vitrine logs aren't created yet: enable its transparency again"
    );
    let policy = get_vitrine_event(tx, tenant_id, election_event_id)
        .await?
        .and_then(|v| v.policy)
        .ok_or_else(|| anyhow!("Upload the event's sealed Vitrine policy before voting opens"))?;
    let facts = PolicyFacts::read(&policy)?;
    let now = now_secs();
    ensure!(
        now >= facts.opens,
        "The event's Vitrine policy opens at {}: voting can't open before",
        date(facts.opens)
    );
    ensure!(
        now < facts.closes,
        "The event's Vitrine policy closed at {}",
        date(facts.closes)
    );
    let election_ids: Vec<String> = elections.iter().map(|e| e.id.clone()).collect();
    check_policy_facts(&facts, mode, election_event_id, &election_ids)
        .map_err(|e| anyhow!("{e}"))?;
    election_configs(tx, tenant_id, election_event_id, &elections).await?;
    check_authority(&authority_signer()?, &policy)?;
    deployment_release()?;
    Ok(())
}

/// Refuses to publish ballots again in a transparent event once its voting
/// window opened: the config statements in force name its ballot styles.
pub async fn check_configuration_change(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let event = get_election_event_by_id(tx, tenant_id, election_event_id).await?;
    if !transparency_mode(&event)?.is_enabled() {
        return Ok(());
    }
    lock_election_event(tx, tenant_id, election_event_id).await?;
    ensure!(
        !vitrine_voting_opened(tx, tenant_id, election_event_id).await?,
        "Voting opened in this transparent election event: its ballots can't be published again"
    );
    Ok(())
}

/// Stores the config statements and `voting-opened` once the voting window
/// opened.
async fn log_opening(
    db: &mut DbClient,
    signer: &NoteSigner,
    tenant_id: &str,
    election_event_id: &str,
    facts: &PolicyFacts,
    policy_id: &str,
) -> Result<()> {
    let tx = db.transaction().await?;
    if has_statement(&tx, tenant_id, election_event_id, VOTING_OPENED_KEY).await?
        || !vitrine_voting_opened(&tx, tenant_id, election_event_id).await?
    {
        return Ok(());
    }
    lock_election_event(&tx, tenant_id, election_event_id).await?;
    if has_statement(&tx, tenant_id, election_event_id, VOTING_OPENED_KEY).await? {
        return Ok(());
    }
    let elections = get_elections(&tx, tenant_id, election_event_id).await?;
    let configs = election_configs(&tx, tenant_id, election_event_id, &elections).await?;
    let release = deployment_release()?;
    let time = u64::try_from(now_secs())?;
    let mut lines = Vec::with_capacity(configs.len());
    for config in &configs {
        let statement = config_statement(
            signer,
            election_event_id,
            config,
            facts,
            policy_id,
            &release,
            time,
        )?;
        let stored =
            insert_vitrine_statement(&tx, tenant_id, election_event_id, &statement).await?;
        lines.push((config.election_id.clone(), leaf_of(&stored)?));
    }
    let opened = milestone_statement(
        signer,
        VOTING_OPENED_KEY.to_string(),
        &MilestoneStatement {
            event_id: canonical_id(election_event_id)?,
            kind: MilestoneKind::VotingOpened,
            time,
            configs: lines,
            counts: vec![],
        },
    )?;
    insert_vitrine_statement(&tx, tenant_id, election_event_id, &opened).await?;
    tx.commit().await?;
    info!("Stored the Vitrine opening statements of event {election_event_id}");
    Ok(())
}

async fn log_milestone(
    db: &mut DbClient,
    signer: &NoteSigner,
    tenant_id: &str,
    election_event_id: &str,
    milestone: &EventMilestone,
) -> Result<()> {
    let tx = db.transaction().await?;
    if !has_statement(&tx, tenant_id, election_event_id, VOTING_OPENED_KEY).await? {
        info!(
            "Event {election_event_id} has no voting-opened statement: no {} milestone",
            milestone.token()
        );
        return Ok(());
    }
    let key = milestone.key();
    lock_election_event(&tx, tenant_id, election_event_id).await?;
    if has_statement(&tx, tenant_id, election_event_id, &key).await? {
        return Ok(());
    }
    let statement = milestone_statement(
        signer,
        key,
        &MilestoneStatement {
            event_id: canonical_id(election_event_id)?,
            kind: MilestoneKind::Other(milestone.token().to_string()),
            time: u64::try_from(now_secs())?,
            configs: vec![],
            counts: vec![],
        },
    )?;
    insert_vitrine_statement(&tx, tenant_id, election_event_id, &statement).await?;
    tx.commit().await?;
    Ok(())
}

/// Stores `voting-closed` once voting ended and the `ballots` log sequenced
/// every ballot.
async fn log_closing(
    db: &mut DbClient,
    signer: &NoteSigner,
    sidecar: &SidecarClient,
    tenant_id: &str,
    election_event_id: &str,
    ballots: &VitrineLog,
    facts: &PolicyFacts,
) -> Result<()> {
    let tx = db.transaction().await?;
    let due = has_statement(&tx, tenant_id, election_event_id, VOTING_OPENED_KEY).await?
        && !has_statement(&tx, tenant_id, election_event_id, VOTING_CLOSED_KEY).await?;
    let end = match due {
        true => voting_end_of(&tx, tenant_id, election_event_id, facts).await?,
        false => None,
    };
    tx.commit().await?;
    let Some(end) = end.filter(|end| now_secs() >= end.saturating_add(CLOSE_SETTLE_SECS)) else {
        return Ok(());
    };
    if !ballots_settled(&sidecar.status(&ballots.sidecar_name).await?) {
        info!("Waiting for the ballots log of event {election_event_id} before voting-closed");
        return Ok(());
    }
    let accepted = get_event_store(election_event_id)
        .await?
        .accepted_counts(election_event_id)
        .await?;

    let tx = db.transaction().await?;
    lock_election_event(&tx, tenant_id, election_event_id).await?;
    if has_statement(&tx, tenant_id, election_event_id, VOTING_CLOSED_KEY).await?
        || voting_end_of(&tx, tenant_id, election_event_id, facts)
            .await?
            .is_none_or(|again| again != end)
    {
        return Ok(());
    }
    let elections = get_elections(&tx, tenant_id, election_event_id).await?;
    let statement = milestone_statement(
        signer,
        VOTING_CLOSED_KEY.to_string(),
        &MilestoneStatement {
            event_id: canonical_id(election_event_id)?,
            kind: MilestoneKind::VotingClosed,
            time: u64::try_from(now_secs())?,
            configs: vec![],
            counts: closing_counts(&elections, &accepted)?,
        },
    )?;
    insert_vitrine_statement(&tx, tenant_id, election_event_id, &statement).await?;
    tx.commit().await?;
    info!("Stored the Vitrine voting-closed statement of event {election_event_id}");
    Ok(())
}

/// Submits the statements without a receipt chain, in the order they were
/// made, stopping at the first the log hasn't sequenced, and keeps the
/// chains of those a checkpoint that meets the quorum contains.
async fn submit_statements(
    db: &mut DbClient,
    sidecar: &SidecarClient,
    tenant_id: &str,
    election_event_id: &str,
    log: &VitrineLog,
    policy_id: &str,
) -> Result<()> {
    let token = log
        .submission_token
        .as_deref()
        .ok_or_else(|| anyhow!("The events log of event {election_event_id} has no token"))?;
    let tx = db.transaction().await?;
    let pending: Vec<VitrineStatement> = get_vitrine_statements(&tx, tenant_id, election_event_id)
        .await?
        .into_iter()
        .filter(|s| s.chain.is_none())
        .collect();
    tx.commit().await?;

    let mut sequenced = Vec::new();
    for statement in pending {
        let entry = STANDARD
            .decode(&statement.entry)
            .with_context(|| format!("The entry of {} isn't base64", statement.key))?;
        match sidecar
            .add(&log.sidecar_name, token, &entry, MAX_WAIT_MS)
            .await?
        {
            Answer::Receipt(receipt) => {
                check_receipt(&receipt, &entry)?;
                sequenced.push((statement, entry));
            }
            Answer::Queued => {
                info!("The events log hasn't sequenced {} yet", statement.key);
                break;
            }
        }
    }

    let deadline = tokio::time::Instant::now() + RECEIPT_WAIT;
    while !sequenced.is_empty() {
        let mut waiting = Vec::new();
        for (statement, entry) in sequenced {
            match sidecar
                .receipt(&log.sidecar_name, &statement.leaf_hash)
                .await?
            {
                Some(receipt) => {
                    check_receipt(&receipt, &entry)?;
                    let chain = statement_chain(policy_id, log.policy_ack.as_deref(), &receipt)?;
                    let tx = db.transaction().await?;
                    set_vitrine_statement_chain(
                        &tx,
                        tenant_id,
                        election_event_id,
                        &statement.key,
                        &chain,
                    )
                    .await?;
                    tx.commit().await?;
                    info!("Stored the receipt chain of {}", statement.key);
                }
                None => waiting.push((statement, entry)),
            }
        }
        if waiting.is_empty() || tokio::time::Instant::now() >= deadline {
            break;
        }
        sequenced = waiting;
        tokio::time::sleep(RECEIPT_POLL).await;
    }
    Ok(())
}

/// Logs the event's due statements and keeps their receipts: what the
/// per-event task runs, at status changes, after tallies and results
/// publications, and from the sweep.
pub async fn log_event_statements(
    tenant_id: &str,
    election_event_id: &str,
    milestone: Option<&EventMilestone>,
) -> Result<()> {
    let mut db = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting a database client")?;
    let tx = db.transaction().await?;
    let event = get_election_event_by_id(&tx, tenant_id, election_event_id).await?;
    let mode = transparency_mode(&event)?;
    let events = get_vitrine_log(&tx, tenant_id, election_event_id, LogName::Events).await?;
    let ballots = get_vitrine_log(&tx, tenant_id, election_event_id, LogName::Ballots).await?;
    let vitrine = get_vitrine_event(&tx, tenant_id, election_event_id)
        .await?
        .unwrap_or_default();
    tx.commit().await?;
    let (Some(events), Some(ballots), Some(policy), Some(policy_id)) =
        (events, ballots, vitrine.policy, vitrine.policy_id)
    else {
        return Ok(());
    };
    if !mode.is_enabled() || events.frozen {
        return Ok(());
    }
    let signer = authority_signer()?;
    check_authority(&signer, &policy)?;
    let facts = PolicyFacts::read(&policy)?;
    let sidecar = SidecarClient::from_env(Service::Windmill)?;

    log_opening(
        &mut db,
        &signer,
        tenant_id,
        election_event_id,
        &facts,
        &policy_id,
    )
    .await?;
    if let Some(milestone) = milestone {
        log_milestone(&mut db, &signer, tenant_id, election_event_id, milestone).await?;
    }
    log_closing(
        &mut db,
        &signer,
        &sidecar,
        tenant_id,
        election_event_id,
        &ballots,
        &facts,
    )
    .await?;
    submit_statements(
        &mut db,
        &sidecar,
        tenant_id,
        election_event_id,
        &events,
        &policy_id,
    )
    .await
}

/// The events the sweep runs [`log_event_statements`] for.
pub async fn pending_events() -> Result<Vec<(String, String)>> {
    let mut db = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting a database client")?;
    let tx = db.transaction().await?;
    let events = get_vitrine_pending_events(&tx).await?;
    tx.commit().await?;
    Ok(events)
}

/// Queues a run of [`log_event_statements`] once a status change that opened
/// or closed voting in a transparent event commits.
pub async fn queue_after_status_change(event: &ElectionEvent, new_status: &VotingStatus) {
    if !matches!(new_status, VotingStatus::OPEN | VotingStatus::CLOSED) {
        return;
    }
    match transparency_mode(event) {
        Ok(mode) if mode.is_enabled() => {
            queue_event_statements(&event.tenant_id, &event.id, None).await
        }
        Ok(_) => {}
        Err(e) => warn!(
            "Error reading the transparency of event {}: {e:?}",
            event.id
        ),
    }
}

/// Queues a run of [`log_event_statements`] for a transparent event. A
/// failure only warns: the sweep runs it again.
pub async fn queue_event_statements(
    tenant_id: &str,
    election_event_id: &str,
    milestone: Option<EventMilestone>,
) {
    let task = log_vitrine_statements::new(
        tenant_id.to_string(),
        election_event_id.to_string(),
        milestone,
    )
    .with_eta(Utc::now() + chrono::Duration::seconds(QUEUE_DELAY_SECS))
    .with_expires_in(RUN_EXPIRES_SECS);
    let queued = tokio::time::timeout(QUEUE_TIMEOUT, async {
        get_celery_app().await.send_task(task).await
    })
    .await;
    match queued {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => warn!("Error queueing the Vitrine statements of {election_event_id}: {e:?}"),
        Err(_) => warn!("Timed out queueing the Vitrine statements of {election_event_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use electoral_log::vitrine::EventsEntry;
    use serde_json::json;

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    const EVENT: &str = "e7a2b3c4-0000-4000-8000-000000000001";
    const MAYOR: &str = "e7a2b3c4-0000-4000-8000-000000000002";
    const COUNCIL: &str = "e7a2b3c4-0000-4000-8000-000000000003";
    const POLICY_ID: &str =
        "sha256:93a933ed8cc8eede70ddcaee9e7b585ba37dda55abd9df2c74d4c09ae671d01e";
    const KEY_BYTES: [u8; 32] = [7; 32];

    fn signer() -> NoteSigner {
        NoteSigner::from_seed("sequent.localhost/authority", &[5; 32]).unwrap()
    }

    fn style(area: &str, is_demo: bool) -> String {
        json!({
            "id": area,
            "public_key": {
                "public_key": STANDARD_NO_PAD.encode(KEY_BYTES),
                "is_demo": is_demo,
            },
        })
        .to_string()
    }

    fn facts() -> PolicyFacts {
        PolicyFacts {
            visibility: TransparencyMode::PUBLIC,
            opens: 1_793_520_000,
            closes: 1_793_566_800,
            event_id: Some(EVENT.into()),
            election_ids: [MAYOR.to_string(), COUNCIL.to_string()].into(),
        }
    }

    fn release() -> Release {
        Release {
            package: PORTAL_PACKAGE.into(),
            version: "10.1.0".into(),
        }
    }

    #[test]
    fn the_config_hashes_the_key_and_the_distinct_styles() {
        let north = style("north", false);
        let south = style("south", false);
        let config = election_config(MAYOR, &[&north, &south, &north]).unwrap();
        assert_eq!(config.key, election_key_hash(&KEY_BYTES));
        let mut digests = vec![style_digest(&north), style_digest(&south)];
        digests.sort();
        assert_eq!(config.style_digests, digests);
        assert_eq!(config.demo, Demo::No);
        let demo = style("demo", true);
        assert_eq!(
            election_config(MAYOR, &[&north, &demo]).unwrap().demo,
            Demo::Yes
        );
    }

    #[test]
    fn an_election_needs_published_styles_with_one_key() {
        assert!(election_config(MAYOR, &[]).is_err());
        assert!(election_config(MAYOR, &["{}"]).is_err());
        assert!(election_config(MAYOR, &["not json"]).is_err());
        let other = json!({
            "public_key": { "public_key": STANDARD_NO_PAD.encode([8; 32]), "is_demo": false },
        })
        .to_string();
        assert!(election_config(MAYOR, &[&style("north", false), &other]).is_err());
    }

    #[test]
    fn a_config_statement_is_signed_and_its_entry_states_its_time() {
        let config = election_config(MAYOR, &[&style("north", false)]).unwrap();
        let row = config_statement(
            &signer(),
            EVENT,
            &config,
            &facts(),
            POLICY_ID,
            &release(),
            1_793_520_005,
        )
        .unwrap();
        let text = row.statement.split("\n\n").next().unwrap();
        assert_eq!(
            text,
            format!(
                "vitrine/config/v1\nevent {EVENT}\nelection {MAYOR}\nkey {}\nstyles {}\n\
                 opens 1793520000\ncloses 1793566800\npolicy {POLICY_ID}\n\
                 release sequent/voting-portal 10.1.0\ntransparency public\ndemo no\n\
                 time 1793520005",
                STANDARD.encode(election_key_hash(&KEY_BYTES)),
                STANDARD.encode(styles_hash(&config.style_digests)),
            )
        );
        assert!(row
            .statement
            .contains("\n\u{2014} sequent.localhost/authority "));
        assert_eq!(row.key, format!("config:{MAYOR}"));
        assert_eq!(row.kind, StatementKind::Config);
        assert_eq!(row.election_id.as_deref(), Some(MAYOR));
        assert_eq!(
            row.style_hashes,
            Some(config.style_digests.iter().map(hex::encode).collect())
        );
        let entry = STANDARD.decode(&row.entry).unwrap();
        assert_eq!(row.leaf_hash, hex::encode(leaf_hash(&entry)));
        let decoded = EventsEntry::decode(&entry).unwrap();
        assert_eq!(decoded.kind, EventKind::Config);
        assert_eq!(decoded.time, 1_793_520_005);
        assert_eq!(decoded.subject, Subject::Nothing);
        assert_eq!(decoded.statement.as_deref(), Some(row.statement.as_str()));
        assert_eq!(
            portal_release(&row.statement).as_deref(),
            Some("sequent/voting-portal 10.1.0")
        );
    }

    #[test]
    fn voting_opened_names_the_config_entries() {
        let config = election_config(MAYOR, &[&style("north", false)]).unwrap();
        let row = config_statement(
            &signer(),
            EVENT,
            &config,
            &facts(),
            POLICY_ID,
            &release(),
            9,
        )
        .unwrap();
        let opened = milestone_statement(
            &signer(),
            VOTING_OPENED_KEY.into(),
            &MilestoneStatement {
                event_id: EVENT.into(),
                kind: MilestoneKind::VotingOpened,
                time: 9,
                configs: vec![(MAYOR.into(), leaf_of(&row).unwrap())],
                counts: vec![],
            },
        )
        .unwrap();
        let config_line = format!(
            "\nconfig {MAYOR} {}\n",
            STANDARD.encode(leaf_hash(&STANDARD.decode(&row.entry).unwrap()))
        );
        assert!(
            opened.statement.contains(&config_line),
            "{}",
            opened.statement
        );
        let entry = EventsEntry::decode(&STANDARD.decode(&opened.entry).unwrap()).unwrap();
        assert_eq!(entry.kind, EventKind::VotingOpened);
        assert_eq!(entry.time, 9);
        let milestone = EventMilestone::TallyCompleted {
            tally_session_id: "s".into(),
        };
        let other = milestone_statement(
            &signer(),
            milestone.key(),
            &MilestoneStatement {
                event_id: EVENT.into(),
                kind: MilestoneKind::Other(milestone.token().into()),
                time: 12,
                configs: vec![],
                counts: vec![],
            },
        )
        .unwrap();
        assert_eq!(other.key, "tally-completed:s");
        let entry = EventsEntry::decode(&STANDARD.decode(&other.entry).unwrap()).unwrap();
        assert_eq!(entry.kind, EventKind::Milestone);
        assert!(other
            .statement
            .contains("\nkind tally-completed\ntime 12\n"));
    }

    #[test]
    fn the_chain_holds_the_ack_and_the_record() {
        let chain = statement_chain(POLICY_ID, Some("ack\n"), "receipt\n").unwrap();
        assert_eq!(
            chain,
            format!(
                "vitrine/receipt-chain/v1\npolicy {POLICY_ID}\nlog events\n\
                 policy-ack 4\nack\n\nrecord 8\nreceipt\n\n"
            )
        );
    }

    #[test]
    fn only_a_receipt_for_the_entry_is_kept() {
        let entry = b"entry".to_vec();
        let receipt = format!(
            "c2sp.org/tlog-proof@v1\nextra {}\n",
            STANDARD.encode(&entry)
        );
        assert!(check_receipt(&receipt, &entry).is_ok());
        assert!(check_receipt(&receipt, b"other").is_err());
        assert!(check_receipt("queued\n", &entry).is_err());
    }

    #[test]
    fn the_authority_must_be_the_policys() {
        let signer = signer();
        let policy = format!(
            "vitrine/policy/v1\nauthority sequent {}\n\n\u{2014} x y\n",
            signer.vkey()
        );
        assert!(check_authority(&signer, &policy).is_ok());
        let other = NoteSigner::from_seed("other.localhost/authority", &[6; 32]).unwrap();
        assert!(check_authority(&other, &policy).is_err());
        assert!(
            check_authority(&signer, &format!("vitrine/policy/v1\n\n{}", signer.vkey())).is_err()
        );
    }

    fn election(id: &str, status: Option<Value>, presentation: Option<Value>) -> Election {
        serde_json::from_value(json!({
            "id": id,
            "tenant_id": TENANT,
            "election_event_id": EVENT,
            "status": status,
            "presentation": presentation,
        }))
        .unwrap()
    }

    fn status(online: &str, stopped: Option<&str>) -> Value {
        json!({
            "voting_status": online,
            "voting_period_dates": { "last_stopped_at": stopped },
        })
    }

    #[test]
    fn voting_ends_when_every_election_closed_and_its_grace_ended() {
        let closes = facts().closes;
        let closed = |id: &str, at: &str| election(id, Some(status("CLOSED", Some(at))), None);
        let end = voting_end(
            &[
                closed(MAYOR, "2026-11-01T20:00:00Z"),
                closed(COUNCIL, "2026-11-01T21:00:00Z"),
            ],
            &[],
            TENANT,
            EVENT,
            closes,
            closes,
        )
        .unwrap();
        assert_eq!(end, Some(1_793_566_800));

        let grace = json!({ "grace_period_policy": "grace-period-without-alert", "grace_period_secs": 600 });
        let end = voting_end(
            &[election(
                MAYOR,
                Some(status("CLOSED", Some("2026-11-01T20:00:00Z"))),
                Some(grace),
            )],
            &[],
            TENANT,
            EVENT,
            closes,
            closes,
        )
        .unwrap();
        assert_eq!(end, Some(1_793_563_200 + 600));
    }

    #[test]
    fn voting_goes_on_while_a_channel_is_open_or_paused_or_an_election_may_open() {
        let closes = facts().closes;
        for online in ["OPEN", "PAUSED"] {
            assert_eq!(
                voting_end(
                    &[
                        election(
                            MAYOR,
                            Some(status("CLOSED", Some("2026-11-01T20:00:00Z"))),
                            None
                        ),
                        election(COUNCIL, Some(status(online, None)), None),
                    ],
                    &[],
                    TENANT,
                    EVENT,
                    closes,
                    closes,
                )
                .unwrap(),
                None
            );
        }
        let unopened = [
            election(
                MAYOR,
                Some(status("CLOSED", Some("2026-11-01T20:00:00Z"))),
                None,
            ),
            election(COUNCIL, None, None),
        ];
        assert_eq!(
            voting_end(&unopened, &[], TENANT, EVENT, closes, closes - 1).unwrap(),
            None
        );
        assert_eq!(
            voting_end(&unopened, &[], TENANT, EVENT, closes, closes).unwrap(),
            Some(1_793_563_200)
        );
        assert_eq!(
            voting_end(&[], &[], TENANT, EVENT, closes, closes).unwrap(),
            None
        );
    }

    #[test]
    fn voting_closed_waits_for_the_ballots_log() {
        let status = |rows: u64, committed: u64, served: Option<u64>, time: Value| {
            json!({
                "checkpoint": served.map(|size| json!({ "size": size, "time": time })),
                "internal": { "committed_size": committed, "queue": { "rows": rows } },
            })
        };
        assert!(ballots_settled(&status(0, 0, None, Value::Null)));
        assert!(ballots_settled(&status(
            0,
            12,
            Some(12),
            json!(1_793_566_900)
        )));
        assert!(!ballots_settled(&status(
            1,
            12,
            Some(12),
            json!(1_793_566_900)
        )));
        assert!(!ballots_settled(&status(
            0,
            12,
            Some(11),
            json!(1_793_566_900)
        )));
        assert!(!ballots_settled(&status(0, 12, Some(12), Value::Null)));
        assert!(!ballots_settled(&json!({ "size": 12 })));
    }

    #[test]
    fn every_election_has_a_count() {
        let elections = [election(COUNCIL, None, None), election(MAYOR, None, None)];
        assert_eq!(
            closing_counts(&elections, &[(MAYOR.to_uppercase(), 5)]).unwrap(),
            vec![(MAYOR.to_string(), 5), (COUNCIL.to_string(), 0)]
        );
        assert!(closing_counts(&elections, &[(MAYOR.into(), -1)]).is_err());
    }

    #[test]
    fn milestones_travel_as_tagged_json() {
        let milestone = EventMilestone::ResultsPublished {
            publication_id: "p".into(),
        };
        let value = serde_json::to_value(&milestone).unwrap();
        assert_eq!(
            value,
            json!({ "kind": "results-published", "publication_id": "p" })
        );
        assert_eq!(
            serde_json::from_value::<EventMilestone>(value).unwrap(),
            milestone
        );
        assert_eq!(milestone.key(), "results-published:p");
    }

    #[test]
    fn config_keys_are_canonical() {
        assert_eq!(
            config_key(&MAYOR.to_uppercase()).unwrap(),
            format!("config:{MAYOR}")
        );
        assert!(config_key("mayor").is_err());
        assert_eq!(
            portal_release("vitrine/config/v1\nrelease other/app 1\n"),
            None
        );
    }
}
