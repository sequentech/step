// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a scheduled opening or closing of voting will do, and why
//! (VOTE-LIFECYCLE design §5a–§5c).
//!
//! One function, [`evaluate`], decides the outcome of a scheduled
//! START/END_VOTING_PERIOD from two copies of the configuration: the
//! **current** one (live rules and policies) and the **published** one (the
//! [`LifecycleSnapshot`] of the newest published ballot publication of the
//! Post or the event; the defaults before anything is published). The
//! transition runs only if both allow it:
//!
//! - it needs signatures when the Open/Close voting rule is Required in
//!   either copy;
//! - it is covered when the newest executed Approve configuration of the
//!   Post or the event signed this row with the same fingerprint;
//! - an uncovered close runs without signatures only when both copies say
//!   `RUN_AS_SYSTEM`; an uncovered opening never runs.
//!
//! The scheduler at fire time ([`crate::services::signing::actions::voting`]),
//! the Scheduled Events list ([`scheduled_outcomes`]) and the stored
//! predictions ([`recompute_predictions`]) all call it, so what the list
//! says is what the scheduler does. The answer is an [`Explanation`]: the
//! outcome, every check with both copies' values (i18n keys, see [`keys`]),
//! the deciding check and the next step.
//!
//! An event-wide row applies to every Post, so it has one outcome per Post:
//! a Post's own approval or publication covers the event-wide rows too.

use crate::postgres::election_initialization::list_scheduled_closes;
use crate::postgres::signing::{
    list_signing_posts, list_signing_rules, lock_signing_event, SigningPost,
};
use crate::services::election_event_status::TransitionRefusal;
use crate::services::initialization_schedule::{effective_close, WaitReason};
use crate::services::initialization_scope::{
    initialization_refusal, load_event_initialization, EventInitialization, ScopeCopies,
};
use crate::services::signing::actions::configuration::{
    canonical_text, target_scope_key, ConfigurationSubject,
};
use crate::services::signing::log::{stage, Actor, LogStep, SystemOutcome};
use crate::services::signing::{action_title, log_scope};
use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::ballot::{
    EInitializeReportPolicy, ElectionEventPresentation, InitializationScope, LifecyclePolicies,
    UnsignedScheduledClosePolicy, VotingStatusChannel,
};
use sequent_core::signing::{SigningAction, SigningRequirement, SigningRule};
use sequent_core::time_zones::{lifecycle_policies, restrictive_lifecycle_policies};
use sequent_core::types::hasura::core::VotingChannels;
use sequent_core::types::scheduled_event::{
    generate_manage_date_task_name, EventProcessors, ManageElectionDatePayload,
};
use sequent_core::types::scheduled_outcome::{
    AuthorizedBy, Check, CheckId, CheckValue, Explanation, LifecycleSnapshot, RuleSnapshot,
    ScheduledOutcomeKind, ScheduledTransition,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::str::FromStr;
use tracing::instrument;
use uuid::Uuid;

/// The i18n message keys of every check value, next step and save notice.
/// The texts are the Admin Portal's; parameters are named in each comment.
pub mod keys {
    /// `{signatures}`: the rule needs that many signatures.
    pub const NEEDS_SIGNATURES_YES: &str = "scheduledOutcome.check.needsSignatures.yes";
    pub const NEEDS_SIGNATURES_NO: &str = "scheduledOutcome.check.needsSignatures.no";
    /// `{code}`: the approval that signed this exact row.
    pub const COVERED_YES: &str = "scheduledOutcome.check.covered.yes";
    /// `{code}`: the row changed after that approval signed it.
    pub const COVERED_CHANGED: &str = "scheduledOutcome.check.covered.changed";
    /// `{code, edited_at, edited_by}`: as `changed`, with the last edit.
    pub const COVERED_CHANGED_BY: &str = "scheduledOutcome.check.covered.changedBy";
    /// `{code}`: the approval doesn't list the row (it came after it).
    pub const COVERED_NOT_IN_APPROVAL: &str = "scheduledOutcome.check.covered.notInApproval";
    pub const COVERED_OVERRIDDEN_BY_SIGNED_POST: &str =
        "scheduledOutcome.check.covered.overriddenBySignedPostRow";
    pub const COVERED_NO_APPROVAL: &str = "scheduledOutcome.check.covered.noApproval";
    /// `{code}`: the Post's voting channels changed since the approval
    /// signed them.
    pub const COVERED_CHANNELS_CHANGED: &str = "scheduledOutcome.check.covered.channelsChanged";
    /// `{code, fired_at}`: the signed transition already ran at this Post;
    /// running it again needs signatures.
    pub const COVERED_ALREADY_FIRED: &str = "scheduledOutcome.check.covered.alreadyFired";
    /// `{code, scheduled_date}`: more than [`super::LATE_FIRE_MINUTES`]
    /// minutes after its time; the signed schedule doesn't cover it any more.
    pub const COVERED_LATE: &str = "scheduledOutcome.check.covered.late";
    pub const UNSIGNED_CLOSE_REFUSE: &str = "scheduledOutcome.check.unsignedClose.refuse";
    pub const UNSIGNED_CLOSE_RUN_AS_SYSTEM: &str =
        "scheduledOutcome.check.unsignedClose.runAsSystem";
    pub const STRICTER_COPY_SAME: &str = "scheduledOutcome.check.stricterCopy.same";
    /// The current settings are stricter: applied now.
    pub const STRICTER_COPY_CURRENT_STRICTER: &str =
        "scheduledOutcome.check.stricterCopy.currentStricter";
    /// The current settings are looser: they apply after the next approved
    /// publication.
    pub const STRICTER_COPY_CURRENT_LOOSER: &str =
        "scheduledOutcome.check.stricterCopy.currentLooser";
    /// Each copy is stricter in one value: together they decide.
    pub const STRICTER_COPY_COMBINED: &str = "scheduledOutcome.check.stricterCopy.combined";
    /// `{publication_id, published_at}`.
    pub const DEFAULTS_PUBLISHED: &str = "scheduledOutcome.check.defaults.published";
    pub const DEFAULTS_NOTHING_PUBLISHED: &str = "scheduledOutcome.check.defaults.nothingPublished";
    /// `{publication_id, published_at}`: published before publications kept
    /// a lifecycle snapshot, so the defaults stand for it.
    pub const DEFAULTS_NO_SNAPSHOT: &str = "scheduledOutcome.check.defaults.noSnapshot";
    pub const INITIALIZATION_WAITING: &str = "scheduledOutcome.check.initialization.waiting";
    pub const VOTING_CLOSE_PASSED: &str = "scheduledOutcome.check.votingClose.passed";
    pub const NEXT_INITIALIZE: &str = "scheduledOutcome.nextStep.initialize";
    pub const NEXT_CLOSED: &str = "scheduledOutcome.nextStep.closed";
    pub const NEXT_NONE: &str = "scheduledOutcome.nextStep.none";
    pub const NEXT_PUBLISH_AND_APPROVE: &str = "scheduledOutcome.nextStep.publishAndApprove";
    /// Approve configuration needs no signatures, so no approval can cover
    /// the row.
    pub const NEXT_REQUIRE_CONFIGURATION_APPROVAL: &str =
        "scheduledOutcome.nextStep.requireConfigurationApproval";
    pub const NEXT_ASK_SIGNERS_TO_CLOSE: &str = "scheduledOutcome.nextStep.askSignersToClose";
    pub const NEXT_ASK_SIGNERS_TO_OPEN: &str = "scheduledOutcome.nextStep.askSignersToOpen";
    /// Rule and policy saves: how the change applies.
    pub const APPLIES_TIGHTENS: &str = "scheduledOutcome.applies.tightens";
    pub const APPLIES_LOOSENS: &str = "scheduledOutcome.applies.loosens";
    pub const APPLIES_TIGHTENS_AND_LOOSENS: &str = "scheduledOutcome.applies.tightensAndLoosens";

    /// Every key, for the translation checks.
    pub const ALL: [&str; 32] = [
        COVERED_OVERRIDDEN_BY_SIGNED_POST,
        INITIALIZATION_WAITING,
        VOTING_CLOSE_PASSED,
        NEXT_INITIALIZE,
        NEXT_CLOSED,
        NEEDS_SIGNATURES_YES,
        NEEDS_SIGNATURES_NO,
        COVERED_YES,
        COVERED_CHANGED,
        COVERED_CHANGED_BY,
        COVERED_NOT_IN_APPROVAL,
        COVERED_NO_APPROVAL,
        COVERED_CHANNELS_CHANGED,
        COVERED_ALREADY_FIRED,
        COVERED_LATE,
        UNSIGNED_CLOSE_REFUSE,
        UNSIGNED_CLOSE_RUN_AS_SYSTEM,
        STRICTER_COPY_SAME,
        STRICTER_COPY_CURRENT_STRICTER,
        STRICTER_COPY_CURRENT_LOOSER,
        STRICTER_COPY_COMBINED,
        DEFAULTS_PUBLISHED,
        DEFAULTS_NOTHING_PUBLISHED,
        DEFAULTS_NO_SNAPSHOT,
        NEXT_NONE,
        NEXT_PUBLISH_AND_APPROVE,
        NEXT_REQUIRE_CONFIGURATION_APPROVAL,
        NEXT_ASK_SIGNERS_TO_CLOSE,
        NEXT_ASK_SIGNERS_TO_OPEN,
        APPLIES_TIGHTENS,
        APPLIES_LOOSENS,
        APPLIES_TIGHTENS_AND_LOOSENS,
    ];
}

/// How long after its time a covered transition still runs authorized by
/// the signed schedule.
pub const LATE_FIRE_MINUTES: i64 = 15;

/// The annotation of a scheduled event that holds its last prediction.
pub const PREDICTED_OUTCOME: &str = "predicted_outcome";
/// The annotation of a scheduled event that holds what happened when it
/// fired.
pub const FIRED_OUTCOME: &str = "fired_outcome";

// ---------------------------------------------------------------------------
// Snapshots

fn sha256_hex(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

/// What a publication records of a signing rule. The digest covers what
/// the rule asks for, not its revision.
pub fn rule_snapshot(rule: &SigningRule) -> RuleSnapshot {
    let content = json!({
        "action": rule.action,
        "requirement": rule.requirement,
        "signatures": rule.signatures,
        "requester_signing": rule.requester_signing,
        "expires_minutes": rule.expires_minutes,
    });
    RuleSnapshot {
        required: rule.is_required(),
        signatures: rule.is_required().then(|| u32::from(rule.required())),
        digest: sha256_hex(&canonical_text(&content)),
    }
}

/// The signing action a scheduled opening or closing of voting is.
pub fn transition_action(processor: &str) -> Option<SigningAction> {
    match EventProcessors::from_str(processor).ok()? {
        EventProcessors::START_VOTING_PERIOD => Some(SigningAction::OpenVoting),
        EventProcessors::END_VOTING_PERIOD => Some(SigningAction::CloseVoting),
        _ => None,
    }
}

/// A scheduled opening or closing, as this module reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledRow {
    pub transition: ScheduledTransition,
    pub annotations: Value,
    /// Whether the transaction that reads it wrote it.
    pub written_now: bool,
}

impl ScheduledRow {
    pub fn action(&self) -> SigningAction {
        transition_action(&self.transition.event_processor).unwrap_or(SigningAction::OpenVoting)
    }

    /// The row's Post, `None` for an event-wide row.
    pub fn election(&self) -> Option<Uuid> {
        self.transition
            .election_id
            .as_deref()
            .and_then(|id| Uuid::parse_str(id).ok())
    }

    /// The payload as the scheduled task reads it.
    pub fn payload(&self) -> ManageElectionDatePayload {
        ManageElectionDatePayload {
            election_id: self.transition.election_id.clone(),
            voting_channels: self.transition.voting_channels.as_ref().map(|channels| {
                channels
                    .iter()
                    .filter_map(|channel| VotingStatusChannel::from_str(channel).ok())
                    .collect()
            }),
        }
    }

    /// The last stored prediction.
    pub fn prediction(&self) -> Option<Prediction> {
        self.annotations
            .get(PREDICTED_OUTCOME)
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
    }
}

/// The transition a scheduled event is, with its fingerprint; `None` for a
/// processor that doesn't open or close voting.
pub fn transition_of(
    id: &str,
    processor: &str,
    cron_config: Option<&Value>,
    event_payload: Option<&Value>,
) -> Option<ScheduledTransition> {
    transition_action(processor)?;
    let text = |field: &str| {
        cron_config
            .and_then(|config| config.get(field))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    let payload: ManageElectionDatePayload = event_payload
        .cloned()
        .and_then(|payload| serde_json::from_value(payload).ok())
        .unwrap_or_default();
    // The order of the channels doesn't change what the row does.
    let mut channels: Vec<String> = payload
        .voting_channels
        .unwrap_or_default()
        .iter()
        .map(|channel| channel.to_string())
        .collect();
    channels.sort();
    channels.dedup();
    let mut transition = ScheduledTransition {
        scheduled_event_id: id.to_owned(),
        event_processor: processor.to_owned(),
        election_id: payload.election_id.filter(|id| !id.is_empty()),
        scheduled_date: text("scheduled_date"),
        local: text("local"),
        timezone: text("timezone"),
        voting_channels: (!channels.is_empty()).then_some(channels),
        fingerprint: String::new(),
    };
    transition.fingerprint = fingerprint(&transition);
    Some(transition)
}

/// SHA-256 of the canonical text of every field of `transition` but the
/// fingerprint.
pub fn fingerprint(transition: &ScheduledTransition) -> String {
    let mut value = serde_json::to_value(transition).unwrap_or(Value::Null);
    if let Value::Object(fields) = &mut value {
        fields.remove("fingerprint");
    }
    sha256_hex(&canonical_text(&value))
}

/// The rows that apply to a publication of `target`: a Post's own rows and
/// the event-wide ones, or every row for the event. Sorted by id.
pub fn schedule_for(rows: &[ScheduledRow], target: Option<Uuid>) -> Vec<ScheduledTransition> {
    let mut schedule: Vec<ScheduledTransition> = rows
        .iter()
        .filter(|row| match (target, row.election()) {
            (None, _) | (_, None) => true,
            (Some(post), Some(election)) => post == election,
        })
        .map(|row| row.transition.clone())
        .collect();
    schedule.sort_by(|a, b| a.scheduled_event_id.cmp(&b.scheduled_event_id));
    schedule
}

/// The scheduled openings and closings that haven't fired, or the one row
/// `only` (fired or not).
#[instrument(skip(hasura_transaction), err)]
async fn read_rows(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    only: Option<&str>,
) -> Result<Vec<ScheduledRow>> {
    read_schedule_rows(
        hasura_transaction,
        tenant_id,
        election_event_id,
        only,
        false,
    )
    .await
    .map(|(rows, _)| rows)
}

async fn read_schedule_rows(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    only: Option<&str>,
    include_stopped: bool,
) -> Result<(Vec<ScheduledRow>, HashMap<String, ScheduleRowMetadata>)> {
    // `written_now`: the row was inserted or updated by this transaction
    // (the write a recompute follows).
    let rows = hasura_transaction
        .query(
            "SELECT id, event_processor, cron_config, event_payload, annotations, task_id, stopped_at IS NOT NULL AS stopped,
                 COALESCE(xmin::text::bigint = txid_current_if_assigned() % 4294967296, false)
                     AS written_now
             FROM sequent_backend.scheduled_event
             WHERE tenant_id = $1 AND election_event_id = $2
                 AND event_processor IN ('START_VOTING_PERIOD', 'END_VOTING_PERIOD')
                 AND archived_at IS NULL
                 AND (($3::text IS NULL AND ($4::boolean OR stopped_at IS NULL)) OR id::text = $3)
             ORDER BY id",
            &[&tenant_id, &election_event_id, &only, &include_stopped],
        )
        .await
        .context("Error reading the scheduled openings and closings")?;
    let mut scheduled = vec![];
    let mut metadata = HashMap::new();
    for row in rows {
        let id: Uuid = row.try_get("id")?;
        metadata.insert(
            id.to_string(),
            ScheduleRowMetadata {
                task_id: row.try_get("task_id")?,
                stopped: row.try_get("stopped")?,
            },
        );
        let processor: Option<String> = row.try_get("event_processor")?;
        let cron_config: Option<Value> = row.try_get("cron_config")?;
        let event_payload: Option<Value> = row.try_get("event_payload")?;
        let annotations: Option<Value> = row.try_get("annotations")?;
        if let Some(transition) = transition_of(
            &id.to_string(),
            processor.as_deref().unwrap_or_default(),
            cron_config.as_ref(),
            event_payload.as_ref(),
        ) {
            scheduled.push(ScheduledRow {
                transition,
                annotations: annotations
                    .filter(Value::is_object)
                    .unwrap_or_else(|| json!({})),
                written_now: row.try_get("written_now")?,
            });
        }
    }
    Ok((scheduled, metadata))
}

async fn read_presentation(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Option<ElectionEventPresentation>> {
    let presentation: Option<Value> = hasura_transaction
        .query_opt(
            "SELECT presentation FROM sequent_backend.election_event
             WHERE tenant_id = $1 AND id = $2",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the event's presentation")?
        .and_then(|row| row.get(0));
    Ok(presentation.and_then(|value| serde_json::from_value(value).ok()))
}

/// The snapshot a publication of `target` (a Post, or the event) records
/// now: the current policies and rules and the schedule that applies to it.
#[instrument(skip(hasura_transaction), err)]
pub async fn lifecycle_snapshot(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    target: Option<Uuid>,
) -> Result<LifecycleSnapshot> {
    let rules = Rules::read(hasura_transaction, tenant_id, election_event_id).await?;
    let presentation = read_presentation(hasura_transaction, tenant_id, election_event_id).await?;
    let rows = read_rows(hasura_transaction, tenant_id, election_event_id, None).await?;
    let initialization_report_policies = crate::postgres::election::get_elections(
        hasura_transaction,
        &tenant_id.to_string(),
        &election_event_id.to_string(),
    )
    .await?
    .into_iter()
    .filter(|post| {
        target
            .map(|target| post.id == target.to_string())
            .unwrap_or(true)
    })
    .map(|post| {
        let policy = post
            .get_presentation()
            .and_then(|presentation| presentation.initialization_report_policy)
            .unwrap_or_default();
        (post.id, policy)
    })
    .collect();
    Ok(LifecycleSnapshot {
        initialization_countries: None,
        initialization_report_policies,
        policies: lifecycle_policies(presentation.as_ref()),
        open_voting: rules.open_voting,
        close_voting: rules.close_voting,
        schedule: schedule_for(&rows, target),
    })
}

/// Country membership of the exact generated publication. Style material is
/// immutable after generation; live area/contest links cannot remove this evidence.
async fn publication_style_countries(
    transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
    target: Option<Uuid>,
) -> Result<BTreeMap<String, Vec<String>>> {
    let parent = transaction
        .query_opt(
            "SELECT COALESCE(is_generated, false) OR published_at IS NOT NULL AS generated,
             election_id, election_ids FROM sequent_backend.ballot_publication
         WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&tenant, &event, &publication],
        )
        .await?;
    let generated_parent = parent
        .as_ref()
        .map(|row| row.try_get::<_, bool>("generated"))
        .transpose()?
        .unwrap_or(false);
    // A recreated raw draft at an old publication UUID is not retained authority.
    // Missing parents cannot prove that their surviving rows are immutable either.
    if !generated_parent {
        return Ok(BTreeMap::new());
    }
    let covered_posts: BTreeSet<String> = if let Some(parent) = parent {
        if parent.try_get::<_, bool>("generated")? {
            if let Some(post) = parent.try_get::<_, Option<Uuid>>("election_id")? {
                [post.to_string()].into_iter().collect()
            } else {
                parent
                    .try_get::<_, Option<Vec<Uuid>>>("election_ids")?
                    .unwrap_or_default()
                    .into_iter()
                    .map(|post| post.to_string())
                    .collect()
            }
        } else {
            BTreeSet::new()
        }
    } else {
        BTreeSet::new()
    };
    let posts: BTreeSet<String> = crate::postgres::election::get_elections(
        transaction,
        &tenant.to_string(),
        &event.to_string(),
    )
    .await?
    .into_iter()
    .filter(|post| target.map(|id| post.id == id.to_string()).unwrap_or(true))
    .map(|post| post.id)
    .collect();
    let mut countries: BTreeMap<String, Vec<String>> = posts
        .intersection(&covered_posts)
        .map(|post| (post.clone(), Vec::new()))
        .collect();
    for row in transaction
        .query(
            "SELECT DISTINCT election_id::text, area_id::text FROM sequent_backend.ballot_style
         WHERE tenant_id = $1 AND election_event_id = $2 AND ballot_publication_id = $3
             ORDER BY election_id::text, area_id::text",
            &[&tenant, &event, &publication],
        )
        .await?
    {
        let post: String = row.try_get(0)?;
        if posts.contains(&post) {
            let areas = countries.entry(post).or_default();
            if let Some(area) = row.try_get::<_, Option<String>>(1)? {
                areas.push(area);
            }
        }
    }
    Ok(countries)
}

/// Captures membership under the generated publication's lock, as the
/// configuration subject and the ordinary publication snapshot both record it.
pub async fn capture_initialization_countries(
    transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
    target: Option<Uuid>,
) -> Result<BTreeMap<String, Vec<String>>> {
    let row = transaction
        .query_one(
            "SELECT COALESCE(is_generated, false) FROM sequent_backend.ballot_publication
         WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3 FOR UPDATE",
            &[&tenant, &event, &publication],
        )
        .await
        .context("Error locking the generated publication's country membership")?;
    anyhow::ensure!(
        row.try_get::<_, bool>(0)?,
        "Country membership needs a generated publication"
    );
    publication_style_countries(transaction, tenant, event, publication, target).await
}

/// Old signed payloads omit this map. Derive missing keys conservatively from
/// their exact frozen publication styles, never from mutable live topology.
async fn hydrate_initialization_countries(
    transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
    target: Option<Uuid>,
    snapshot: &mut LifecycleSnapshot,
) -> Result<()> {
    let frozen =
        publication_style_countries(transaction, tenant, event, publication, target).await?;
    if !frozen.is_empty() {
        let countries = snapshot
            .initialization_countries
            .get_or_insert_with(BTreeMap::new);
        for (post, areas) in frozen {
            countries.entry(post).or_insert(areas);
        }
    }
    Ok(())
}

/// Keeps the snapshot a publication of `election_id` (or the event)
/// records, in `sequent_backend.lifecycle_snapshot`: a table no Hasura
/// role can write, so the published copy is only what publishing (or a
/// signed approval, `approval_request_id`) recorded.
#[instrument(skip(hasura_transaction, snapshot), err)]
pub async fn write_publication_snapshot(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    publication_id: Uuid,
    election_id: Option<Uuid>,
    approval_request_id: Option<Uuid>,
    snapshot: &LifecycleSnapshot,
) -> Result<()> {
    hasura_transaction
        .execute(
            "INSERT INTO sequent_backend.lifecycle_snapshot
                 (tenant_id, election_event_id, ballot_publication_id, election_id,
                  approval_request_id, snapshot)
             VALUES ($1, $2, $3, $4, $5, $6)",
            &[
                &tenant_id,
                &election_event_id,
                &publication_id,
                &election_id,
                &approval_request_id,
                &serde_json::to_value(snapshot)?,
            ],
        )
        .await
        .context("Error keeping the publication's lifecycle snapshot")?;
    Ok(())
}

/// Keeps the [`lifecycle_snapshot`] of a publication's target, as
/// publishing it records it.
pub async fn snapshot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    publication_id: &str,
    target: Option<&str>,
) -> Result<()> {
    let tenant_id = Uuid::parse_str(tenant_id)?;
    let election_event_id = Uuid::parse_str(election_event_id)?;
    let target = target.map(Uuid::parse_str).transpose()?;
    let mut snapshot =
        lifecycle_snapshot(hasura_transaction, tenant_id, election_event_id, target).await?;
    snapshot.initialization_countries = Some(
        capture_initialization_countries(
            hasura_transaction,
            tenant_id,
            election_event_id,
            Uuid::parse_str(publication_id)?,
            target,
        )
        .await?,
    );
    write_publication_snapshot(
        hasura_transaction,
        tenant_id,
        election_event_id,
        Uuid::parse_str(publication_id)?,
        target,
        None,
        &snapshot,
    )
    .await
}

/// What publishing keeps as the publication's lifecycle snapshot.
#[derive(Debug, Clone, PartialEq)]
pub enum PublicationLifecycle {
    /// A publication without a configuration approval: the current values.
    Current,
    /// What a configuration approval signed.
    Signed {
        request_id: Uuid,
        snapshot: LifecycleSnapshot,
    },
    /// An approval from before lifecycle snapshots: nothing new is kept, so
    /// the previous signed copy stays the published one.
    KeepPrevious,
}

/// Keeps a publication's lifecycle snapshot as `lifecycle` says.
pub async fn keep_publication_lifecycle(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    publication_id: &str,
    target: Option<&str>,
    lifecycle: &PublicationLifecycle,
) -> Result<()> {
    match lifecycle {
        PublicationLifecycle::Current => {
            snapshot_publication(
                hasura_transaction,
                tenant_id,
                election_event_id,
                publication_id,
                target,
            )
            .await
        }
        PublicationLifecycle::Signed {
            request_id,
            snapshot,
        } => {
            write_publication_snapshot(
                hasura_transaction,
                Uuid::parse_str(tenant_id)?,
                Uuid::parse_str(election_event_id)?,
                Uuid::parse_str(publication_id)?,
                target.map(Uuid::parse_str).transpose()?,
                Some(*request_id),
                snapshot,
            )
            .await
        }
        PublicationLifecycle::KeepPrevious => Ok(()),
    }
}

/// The voting channels `channels` enables, sorted.
pub fn enabled_channels(channels: &VotingChannels) -> Vec<String> {
    let mut enabled: Vec<String> = [
        VotingStatusChannel::ONLINE,
        VotingStatusChannel::KIOSK,
        VotingStatusChannel::EARLY_VOTING,
        VotingStatusChannel::TELEPHONE,
    ]
    .into_iter()
    .filter(|channel| channel.channel_from(channels) == Some(true))
    .map(|channel| channel.to_string())
    .collect();
    enabled.sort();
    enabled
}

/// Each Post's enabled voting channels, for the Posts a publication of
/// `target` covers (the Post, or every Post of the event).
pub async fn post_channels_of(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    target: Option<Uuid>,
) -> Result<BTreeMap<String, Vec<String>>> {
    let mut channels = BTreeMap::new();
    for row in hasura_transaction
        .query(
            "SELECT id, voting_channels FROM sequent_backend.election
             WHERE tenant_id = $1 AND election_event_id = $2
                 AND ($3::uuid IS NULL OR id = $3)",
            &[&tenant_id, &election_event_id, &target],
        )
        .await
        .context("Error reading the Posts' voting channels")?
    {
        let id: Uuid = row.try_get("id")?;
        let configured: Option<Value> = row.try_get("voting_channels")?;
        channels.insert(
            id.to_string(),
            enabled_channels(
                &configured
                    .and_then(|configured| serde_json::from_value(configured).ok())
                    .unwrap_or_default(),
            ),
        );
    }
    Ok(channels)
}

/// The Post of a scheduled event that is that Post's own START/END row as
/// the event-wide dispatch tells them: not archived (checked by the
/// caller), its task id the Post's (`generate_manage_date_task_name`), and
/// its channels including ONLINE. An event-wide row doesn't change such a
/// Post.
pub fn own_row_post(
    tenant_id: &str,
    election_event_id: &str,
    processor: &EventProcessors,
    task_id: Option<&str>,
    payload: Option<&Value>,
) -> Option<Uuid> {
    let payload: ManageElectionDatePayload = serde_json::from_value(payload?.clone()).ok()?;
    let election_id = payload.election_id.clone()?;
    let expected =
        generate_manage_date_task_name(tenant_id, election_event_id, Some(&election_id), processor);
    (task_id == Some(expected.as_str())
        && payload.channels().contains(&VotingStatusChannel::ONLINE))
    .then(|| Uuid::parse_str(&election_id).ok())
    .flatten()
    // Only the Post's id exactly as stored (canonical text): the dispatch
    // compares the text, so another spelling isn't the Post's own row.
    .filter(|post| post.to_string() == election_id)
}

/// The Posts a scheduled opening or closing changes: its own Post, or for
/// an event-wide row every Post without its own row of the same kind
/// ([`own_row_post`]). The one definition the dispatch, the predictions
/// and the fire-time checks share.
pub async fn event_wide_targets(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    scheduled_event: &sequent_core::types::scheduled_event::ScheduledEvent,
) -> Result<Vec<String>> {
    let tenant = Uuid::parse_str(tenant_id)?;
    let event = Uuid::parse_str(election_event_id)?;
    let processor = scheduled_event
        .event_processor
        .as_ref()
        .map(|processor| processor.to_string())
        .unwrap_or_default();
    let Some(transition) = transition_of(
        &scheduled_event.id,
        &processor,
        scheduled_event
            .cron_config
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?
            .as_ref(),
        scheduled_event.event_payload.as_ref(),
    ) else {
        return Ok(vec![]);
    };
    let state = EventState::read(hasura_transaction, tenant, event).await?;
    Ok(state
        .posts_of(&ScheduledRow {
            transition,
            annotations: json!({}),
            written_now: false,
        })
        .into_iter()
        .map(|post| post.to_string())
        .collect())
}

/// Marks that a transition ran at a Post, once: a covered transition that
/// already ran there with the same fingerprint isn't covered again
/// (`sequent_backend.lifecycle_fired`, which no Hasura role can write).
pub async fn mark_fired(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    transition: &ScheduledTransition,
    election_id: Uuid,
    executed_channels: &[VotingStatusChannel],
) -> Result<()> {
    let executed_channels = serde_json::to_value(executed_channels)?;
    hasura_transaction
        .execute(
            "INSERT INTO sequent_backend.lifecycle_fired
                 (tenant_id, election_event_id, scheduled_event_id, election_id, fingerprint, executed_channels)
             VALUES ($1, $2, $3::text::uuid, $4, $5, $6)",
            &[
                &tenant_id,
                &election_event_id,
                &transition.scheduled_event_id,
                &election_id,
                &transition.fingerprint,
                &executed_channels,
            ],
        )
        .await
        .context("Error marking the transition as run")?;
    Ok(())
}

/// A kept snapshot as the Admin Portal reads it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotView {
    /// The Post of a Post-level publication; `None` for the event.
    pub election_id: Option<Uuid>,
    pub publication_id: Uuid,
    /// When it was kept (the database's clock).
    pub published_at: DateTime<Utc>,
    pub approval_request_id: Option<Uuid>,
    pub approval_code: Option<String>,
    /// Whether a configuration approval signed it.
    pub signed: bool,
    pub snapshot: LifecycleSnapshot,
}

/// Signed policy authority is the immutable subject, never the redundant kept copy.
/// A completed request is visible during its validated publication executor;
/// schedule coverage still separately requires an executed approval.
fn decode_stored_snapshot(
    snapshot: Value,
    approval_request_id: Option<Uuid>,
    subject: Option<Value>,
    scope_key: Option<String>,
    publication_id: Uuid,
    election_id: Option<Uuid>,
) -> Result<LifecycleSnapshot> {
    if approval_request_id.is_none() {
        return serde_json::from_value(snapshot).context("A kept lifecycle snapshot doesn't read");
    }
    let subject: ConfigurationSubject = serde_json::from_value(
        subject
            .context("A signed lifecycle snapshot has no valid completed configuration approval")?,
    )
    .context("A signed lifecycle configuration subject doesn't read")?;
    let target = election_id.map(|id| id.to_string());
    anyhow::ensure!(
        scope_key.as_deref() == Some(target_scope_key(target.as_deref()).as_str())
            && Uuid::parse_str(&subject.ballot_publication_id).ok() == Some(publication_id),
        "A signed lifecycle snapshot references another publication or target"
    );
    Ok(subject.lifecycle())
}

/// Per target (each Post with publications of its own, and the event): the
/// newest kept snapshot, and the newest signed one when that is another.
#[instrument(skip(hasura_transaction), err)]
pub async fn lifecycle_snapshots(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<SnapshotView>> {
    let rows = hasura_transaction
        .query(
            "SELECT s.id, s.election_id, s.ballot_publication_id, s.created_at,
                 s.approval_request_id, r.code, s.snapshot, r.subject, r.scope_key
             FROM sequent_backend.lifecycle_snapshot s
             LEFT JOIN sequent_backend.signing_request r
                 ON r.id = s.approval_request_id AND r.tenant_id = s.tenant_id
                     AND r.election_event_id = s.election_event_id
                     AND r.action = 'approve-configuration'
                     AND r.status IN ('completed', 'executed') AND r.completed_at IS NOT NULL
             WHERE s.tenant_id = $1 AND s.election_event_id = $2
             ORDER BY s.created_at DESC, s.id DESC",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the lifecycle snapshots")?;
    let mut newest: Vec<(Uuid, SnapshotView)> = vec![];
    let mut signed: Vec<(Uuid, SnapshotView)> = vec![];
    for row in rows {
        let snapshot: Value = row.try_get("snapshot")?;
        let approval_request_id: Option<Uuid> = row.try_get("approval_request_id")?;
        let mut view = SnapshotView {
            election_id: row.try_get("election_id")?,
            publication_id: row.try_get("ballot_publication_id")?,
            published_at: row.try_get("created_at")?,
            approval_request_id,
            approval_code: row.try_get("code")?,
            signed: approval_request_id.is_some(),
            snapshot: decode_stored_snapshot(
                snapshot,
                approval_request_id,
                row.try_get("subject")?,
                row.try_get("scope_key")?,
                row.try_get("ballot_publication_id")?,
                row.try_get("election_id")?,
            )?,
        };
        hydrate_initialization_countries(
            hasura_transaction,
            tenant_id,
            election_event_id,
            view.publication_id,
            view.election_id,
            &mut view.snapshot,
        )
        .await?;
        let id: Uuid = row.try_get("id")?;
        if !newest
            .iter()
            .any(|(_, kept)| kept.election_id == view.election_id)
        {
            newest.push((id, view.clone()));
        }
        if view.signed
            && !signed
                .iter()
                .any(|(_, kept)| kept.election_id == view.election_id)
        {
            signed.push((id, view));
        }
    }
    let mut views: Vec<SnapshotView> = vec![];
    for (id, view) in newest {
        let target = view.election_id;
        views.push(view);
        if let Some((_, other)) = signed
            .iter()
            .find(|(signed_id, kept)| kept.election_id == target && *signed_id != id)
        {
            views.push(other.clone());
        }
    }
    Ok(views)
}

// ---------------------------------------------------------------------------
// The two copies and the coverage

/// The Open/Close voting and Approve configuration rules, now.
#[derive(Debug, Clone, PartialEq)]
pub struct Rules {
    pub open_voting: RuleSnapshot,
    pub close_voting: RuleSnapshot,
    pub approve_configuration: bool,
}

impl Rules {
    pub async fn read(
        hasura_transaction: &Transaction<'_>,
        tenant_id: Uuid,
        election_event_id: Uuid,
    ) -> Result<Self> {
        let saved: HashMap<SigningAction, SigningRule> =
            list_signing_rules(hasura_transaction, tenant_id, election_event_id)
                .await?
                .into_iter()
                .map(|row| (row.rule.action, row.rule))
                .collect();
        let rule = |action| {
            saved
                .get(&action)
                .cloned()
                .unwrap_or_else(|| SigningRule::default_for(action))
        };
        Ok(Rules {
            open_voting: rule_snapshot(&rule(SigningAction::OpenVoting)),
            close_voting: rule_snapshot(&rule(SigningAction::CloseVoting)),
            approve_configuration: rule(SigningAction::ApproveConfiguration).is_required(),
        })
    }

    pub fn of(&self, action: SigningAction) -> &RuleSnapshot {
        if action == SigningAction::CloseVoting {
            &self.close_voting
        } else {
            &self.open_voting
        }
    }
}

/// A kept lifecycle snapshot of a publication.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredSnapshot {
    pub publication_id: Uuid,
    /// The Post of a Post-level publication.
    pub election_id: Option<Uuid>,
    /// The approval whose signed snapshot it is, if one published it.
    pub approval_request_id: Option<Uuid>,
    /// When it was kept (the database's clock).
    pub created_at: DateTime<Utc>,
    pub snapshot: LifecycleSnapshot,
}

/// A published ballot publication (only to tell "nothing published" from
/// "published before snapshots"; its own columns are writable, so they
/// never decide a value).
#[derive(Debug, Clone, PartialEq)]
pub struct Publication {
    pub id: Uuid,
    pub election_id: Option<Uuid>,
    pub published_at: DateTime<Utc>,
}

/// An executed Approve configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Approval {
    pub request_id: Uuid,
    pub code: String,
    /// The Post of a Post-level approval.
    pub target: Option<Uuid>,
    pub executed_at: DateTime<Utc>,
    /// The schedule it signed (`[]` for a request from before the field).
    pub schedule: Vec<ScheduledTransition>,
    /// Who signed it, in the order they signed.
    pub signers: Vec<String>,
    /// Each Post's enabled voting channels as it signed them.
    pub post_channels: BTreeMap<String, Vec<String>>,
    /// Read from the executed subject, never mutable publication annotations.
    pub initialization_scope: InitializationScope,
    pub initialization_report_policies: BTreeMap<String, EInitializeReportPolicy>,
}

impl Approval {
    pub fn authorized_by(&self) -> AuthorizedBy {
        AuthorizedBy {
            request_id: self.request_id.to_string(),
            code: self.code.clone(),
            signers: self.signers.clone(),
        }
    }
}

/// Whether a publication or an approval of `target` applies at `post`
/// (`None`: the event as a whole, which only the event's own apply to).
fn applies_at(target: Option<Uuid>, post: Option<Uuid>) -> bool {
    target.is_none() || (post.is_some() && target == post)
}

/// Dispatcher metadata that a pending schedule preview must preserve.
#[derive(Debug, Clone)]
pub struct ScheduleRowMetadata {
    pub task_id: Option<String>,
    pub stopped: bool,
}

/// The channels an executed transition actually changed in its transaction.
#[derive(Debug, Clone)]
pub struct FiredEffect {
    pub fired_at: DateTime<Utc>,
    pub channels: Vec<VotingStatusChannel>,
}

/// Everything an outcome of the event depends on, read once.
#[derive(Debug, Clone)]
pub struct EventState {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    /// The current policies.
    pub policies: LifecyclePolicies,
    /// The current rules.
    pub rules: Rules,
    /// Kept snapshots, newest first.
    pub snapshots: Vec<StoredSnapshot>,
    /// Published publications, newest first.
    pub publications: Vec<Publication>,
    /// Executed approvals, newest first.
    pub approvals: Vec<Approval>,
    pub posts: Vec<SigningPost>,
    /// Each Post's configured voting channels.
    pub post_channels: HashMap<Uuid, VotingChannels>,
    /// The (Post, processor) pairs with a scheduled event of the Post's own,
    /// as the event-wide dispatch tells them ([`own_row_post`]).
    pub own_rows: HashSet<(Uuid, String)>,
    /// The transitions that ran: (scheduled event, Post, fingerprint) and
    /// when.
    pub fired: HashMap<(String, Uuid, String), DateTime<Utc>>,
    /// Actual channel effects, separate from replay marks for no-op/legacy rows.
    pub fired_effects: HashMap<(String, Uuid, String), Vec<FiredEffect>>,
    /// The database's time when the state was read.
    pub now: DateTime<Utc>,
    /// Raw initialization rows; loading them never evaluates scopes.
    pub initialization: EventInitialization,
    pub live_closes: Vec<(Option<String>, String)>,
    /// Current rows including completed ones, for close boundaries and accurate previews.
    pub live_rows: Vec<ScheduledRow>,
    pub live_row_metadata: HashMap<String, ScheduleRowMetadata>,
    /// SQL NOW() used by schedule rearming (distinct from the fire-time clock).
    pub transaction_now: DateTime<Utc>,
}

impl EventState {
    #[instrument(skip(hasura_transaction), err)]
    pub async fn read(
        hasura_transaction: &Transaction<'_>,
        tenant_id: Uuid,
        election_event_id: Uuid,
    ) -> Result<Self> {
        Self::read_for_action(
            hasura_transaction,
            tenant_id,
            election_event_id,
            SigningAction::OpenVoting,
        )
        .await
    }

    /// Closing is independent of initialization topology and must remain available on bad area data.
    pub async fn read_for_closes(
        hasura_transaction: &Transaction<'_>,
        tenant_id: Uuid,
        election_event_id: Uuid,
    ) -> Result<Self> {
        Self::read_for_action(
            hasura_transaction,
            tenant_id,
            election_event_id,
            SigningAction::CloseVoting,
        )
        .await
    }

    async fn read_for_action(
        hasura_transaction: &Transaction<'_>,
        tenant_id: Uuid,
        election_event_id: Uuid,
        action: SigningAction,
    ) -> Result<Self> {
        let presentation =
            read_presentation(hasura_transaction, tenant_id, election_event_id).await?;
        let rules = Rules::read(hasura_transaction, tenant_id, election_event_id).await?;
        let mut snapshots = hasura_transaction
            .query(
                "SELECT s.ballot_publication_id, s.election_id, s.approval_request_id, s.created_at,
                     s.snapshot, r.subject, r.scope_key
                 FROM sequent_backend.lifecycle_snapshot s
                 LEFT JOIN sequent_backend.signing_request r
                     ON r.id = s.approval_request_id AND r.tenant_id = s.tenant_id
                         AND r.election_event_id = s.election_event_id
                         AND r.action = 'approve-configuration'
                         AND r.status IN ('completed', 'executed') AND r.completed_at IS NOT NULL
                 WHERE s.tenant_id = $1 AND s.election_event_id = $2
                 ORDER BY s.created_at DESC, s.id DESC",
                &[&tenant_id, &election_event_id],
            )
            .await
            .context("Error reading the lifecycle snapshots")?
            .into_iter()
            .map(|row| -> Result<StoredSnapshot> {
                let snapshot: Value = row.try_get("snapshot")?;
                Ok(StoredSnapshot {
                    publication_id: row.try_get("ballot_publication_id")?,
                    election_id: row.try_get("election_id")?,
                    approval_request_id: row.try_get("approval_request_id")?,
                    created_at: row.try_get("created_at")?,
                    snapshot: decode_stored_snapshot(
                        snapshot,
                        row.try_get("approval_request_id")?,
                        row.try_get("subject")?,
                        row.try_get("scope_key")?,
                        row.try_get("ballot_publication_id")?,
                        row.try_get("election_id")?,
                    )?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        for kept in &mut snapshots {
            hydrate_initialization_countries(
                hasura_transaction,
                tenant_id,
                election_event_id,
                kept.publication_id,
                kept.election_id,
                &mut kept.snapshot,
            )
            .await?;
        }
        let publications = hasura_transaction
            .query(
                "SELECT id, election_id, published_at
                 FROM sequent_backend.ballot_publication
                 WHERE tenant_id = $1 AND election_event_id = $2 AND published_at IS NOT NULL
                 ORDER BY published_at DESC, id",
                &[&tenant_id, &election_event_id],
            )
            .await
            .context("Error reading the published publications")?
            .into_iter()
            .map(|row| -> Result<Publication> {
                Ok(Publication {
                    id: row.try_get("id")?,
                    election_id: row.try_get("election_id")?,
                    published_at: row.try_get("published_at")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        // Who signed each executed configuration approval, in signing order.
        let mut signers: HashMap<Uuid, Vec<String>> = HashMap::new();
        for row in hasura_transaction
            .query(
                "SELECT a.request_id, COALESCE(a.display_name, a.username)
                 FROM sequent_backend.signing_approval a
                 JOIN sequent_backend.signing_request r
                     ON r.id = a.request_id AND r.tenant_id = a.tenant_id
                         AND r.election_event_id = a.election_event_id
                 WHERE a.tenant_id = $1 AND a.election_event_id = $2
                     AND r.action = 'approve-configuration' AND r.status = 'executed'
                 ORDER BY a.signed_at, a.id",
                &[&tenant_id, &election_event_id],
            )
            .await
            .context("Error reading who signed the configuration approvals")?
        {
            signers
                .entry(row.try_get(0)?)
                .or_default()
                .push(row.try_get(1)?);
        }
        let event_key = target_scope_key(None);
        let approvals = hasura_transaction
            .query(
                "SELECT id, code, scope_key, subject, executed_at
                 FROM sequent_backend.signing_request
                 WHERE tenant_id = $1 AND election_event_id = $2
                     AND action = 'approve-configuration' AND status = 'executed'
                 ORDER BY executed_at DESC, id",
                &[&tenant_id, &election_event_id],
            )
            .await
            .context("Error reading the executed configuration approvals")?
            .into_iter()
            .map(|row| -> Result<Approval> {
                let id: Uuid = row.try_get("id")?;
                let scope_key: String = row.try_get("scope_key")?;
                let subject: Value = row.try_get("subject")?;
                Ok(Approval {
                    request_id: id,
                    code: row.try_get("code")?,
                    target: if scope_key == event_key {
                        None
                    } else {
                        scope_key
                            .rsplit('|')
                            .next()
                            .and_then(|key| Uuid::parse_str(key).ok())
                    },
                    executed_at: row.try_get("executed_at")?,
                    schedule: subject
                        .get("schedule")
                        .cloned()
                        .and_then(|schedule| serde_json::from_value(schedule).ok())
                        .unwrap_or_default(),
                    initialization_scope: subject
                        .get("policies")
                        .cloned()
                        .map(serde_json::from_value::<LifecyclePolicies>)
                        .transpose()
                        .context("Executed configuration initialization policies don't read")?
                        .unwrap_or_default()
                        .initialization_scope,
                    initialization_report_policies: subject
                        .get("initialization_report_policies")
                        .cloned()
                        .map(serde_json::from_value)
                        .transpose()
                        .context("Executed report policies don't read")?
                        .unwrap_or_default(),
                    signers: signers.remove(&id).unwrap_or_default(),
                    post_channels: subject
                        .get("post_channels")
                        .cloned()
                        .and_then(|channels| serde_json::from_value(channels).ok())
                        .unwrap_or_default(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut post_channels = HashMap::new();
        for row in hasura_transaction
            .query(
                "SELECT id, voting_channels FROM sequent_backend.election
                 WHERE tenant_id = $1 AND election_event_id = $2",
                &[&tenant_id, &election_event_id],
            )
            .await
            .context("Error reading the Posts' voting channels")?
        {
            let channels: Option<Value> = row.try_get("voting_channels")?;
            post_channels.insert(
                row.try_get::<_, Uuid>("id")?,
                channels
                    .and_then(|channels| serde_json::from_value(channels).ok())
                    .unwrap_or_default(),
            );
        }
        let tenant_text = tenant_id.to_string();
        let event_text = election_event_id.to_string();
        let own_rows = hasura_transaction
            .query(
                "SELECT task_id, event_payload, event_processor
                 FROM sequent_backend.scheduled_event
                 WHERE tenant_id = $1 AND election_event_id = $2 AND archived_at IS NULL
                     AND event_processor IN ('START_VOTING_PERIOD', 'END_VOTING_PERIOD')",
                &[&tenant_id, &election_event_id],
            )
            .await
            .context("Error reading the Posts' own schedules")?
            .into_iter()
            .filter_map(|row| {
                let task_id: Option<String> = row.get(0);
                let payload: Option<Value> = row.get(1);
                let processor: String = row.get::<_, Option<String>>(2)?;
                let post = own_row_post(
                    &tenant_text,
                    &event_text,
                    &EventProcessors::from_str(&processor).ok()?,
                    task_id.as_deref(),
                    payload.as_ref(),
                )?;
                Some((post, processor))
            })
            .collect();
        let fired_rows = hasura_transaction
            .query(
                "SELECT scheduled_event_id, election_id, fingerprint, fired_at, executed_channels
                 FROM sequent_backend.lifecycle_fired
                 WHERE tenant_id = $1 AND election_event_id = $2",
                &[&tenant_id, &election_event_id],
            )
            .await
            .context("Error reading the transitions that ran")?;
        let mut fired: HashMap<(String, Uuid, String), DateTime<Utc>> = HashMap::new();
        let mut fired_effects: HashMap<(String, Uuid, String), Vec<FiredEffect>> = HashMap::new();
        for row in fired_rows {
            let id: Uuid = row.try_get(0)?;
            let key = (id.to_string(), row.try_get(1)?, row.try_get(2)?);
            let fired_at: DateTime<Utc> = row.try_get(3)?;
            fired
                .entry(key.clone())
                .and_modify(|previous| *previous = (*previous).min(fired_at))
                .or_insert(fired_at);
            if let Some(channels) = row.try_get::<_, Option<Value>>(4)? {
                fired_effects.entry(key).or_default().push(FiredEffect {
                    fired_at,
                    channels: serde_json::from_value(channels)
                        .context("Executed transition channels don't read")?,
                });
            }
        }
        let clocks = hasura_transaction
            .query_one("SELECT clock_timestamp(), transaction_timestamp()", &[])
            .await?;
        let now: DateTime<Utc> = clocks.try_get(0)?;
        let transaction_now: DateTime<Utc> = clocks.try_get(1)?;
        let mut initialization = if action == SigningAction::CloseVoting {
            EventInitialization::default()
        } else {
            load_event_initialization(hasura_transaction, &tenant_text, &event_text).await?
        };
        for (post_id, post) in &mut initialization.posts {
            let id = Uuid::parse_str(post_id)?;
            if let Some(kept) = snapshots
                .iter()
                .find(|kept| applies_at(kept.election_id, Some(id)))
            {
                if let Some(countries) = kept
                    .snapshot
                    .initialization_countries
                    .as_ref()
                    .and_then(|map| map.get(post_id))
                {
                    post.areas.extend(countries.iter().cloned());
                } else {
                    post.unresolved_published_countries = true;
                }
            }
        }
        let live_closes =
            list_scheduled_closes(hasura_transaction, tenant_id, election_event_id).await?;
        let (live_rows, live_row_metadata) =
            read_schedule_rows(hasura_transaction, tenant_id, election_event_id, None, true)
                .await?;
        Ok(EventState {
            initialization,
            live_closes,
            live_rows,
            live_row_metadata,
            transaction_now,
            tenant_id,
            election_event_id,
            policies: lifecycle_policies(presentation.as_ref()),
            rules,
            snapshots,
            publications,
            approvals,
            posts: list_signing_posts(hasura_transaction, tenant_id, election_event_id).await?,
            post_channels,
            own_rows,
            fired,
            fired_effects,
            now,
        })
    }

    /// The published copy at a Post (its own publication's or the event's),
    /// or of the event: the newest kept snapshot; the defaults when there is
    /// none.
    pub fn published_copy(&self, post: Option<Uuid>, action: SigningAction) -> PublishedCopy {
        if let Some(kept) = self
            .snapshots
            .iter()
            .find(|kept| applies_at(kept.election_id, post))
        {
            return PublishedCopy::Snapshot {
                publication_id: kept.publication_id,
                published_at: kept.created_at,
                values: CopyValues {
                    rule: if action == SigningAction::CloseVoting {
                        kept.snapshot.close_voting.clone()
                    } else {
                        kept.snapshot.open_voting.clone()
                    },
                    close_policy: kept.snapshot.policies.unsigned_scheduled_close.clone(),
                },
            };
        }
        match self
            .publications
            .iter()
            .find(|publication| applies_at(publication.election_id, post))
        {
            Some(publication) => PublishedCopy::NoSnapshot {
                publication_id: publication.id,
                published_at: publication.published_at,
            },
            None => PublishedCopy::Nothing,
        }
    }

    fn own_row_key(&self, row: &ScheduledRow) -> Option<(Uuid, String)> {
        let metadata = self
            .live_row_metadata
            .get(&row.transition.scheduled_event_id)?;
        let processor = EventProcessors::from_str(&row.transition.event_processor).ok()?;
        let payload = serde_json::to_value(row.payload()).ok()?;
        own_row_post(
            &self.tenant_id.to_string(),
            &self.election_event_id.to_string(),
            &processor,
            metadata.task_id.as_deref(),
            Some(&payload),
        )
        .map(|post| (post, row.transition.event_processor.clone()))
    }

    /// Current and immutable published initialization scope at this target.
    pub fn initialization_scopes(&self, post: Option<Uuid>) -> ScopeCopies {
        let published = self
            .snapshots
            .iter()
            .find(|kept| applies_at(kept.election_id, post))
            .map(|kept| {
                kept.approval_request_id
                    .and_then(|id| {
                        self.approvals
                            .iter()
                            .find(|approval| approval.request_id == id)
                    })
                    .map(|approval| approval.initialization_scope.clone())
                    .unwrap_or_else(|| kept.snapshot.policies.initialization_scope.clone())
            });
        ScopeCopies {
            current: self.policies.initialization_scope.clone(),
            published,
        }
    }

    pub fn published_report_policy(&self, post: Uuid) -> Option<EInitializeReportPolicy> {
        let kept = self
            .snapshots
            .iter()
            .find(|kept| applies_at(kept.election_id, Some(post)))?;
        let policies = kept
            .approval_request_id
            .and_then(|id| {
                self.approvals
                    .iter()
                    .find(|approval| approval.request_id == id)
            })
            .map(|approval| &approval.initialization_report_policies)
            .unwrap_or(&kept.snapshot.initialization_report_policies);
        policies.get(&post.to_string()).cloned()
    }

    pub fn initialization_refusal_at(&self, post: Uuid) -> Option<TransitionRefusal> {
        // EVENT requirements also retain every other Post's published REQUIRED switch.
        let mut initialization = self.initialization.clone();
        for (id, state) in &mut initialization.posts {
            if let Ok(id) = Uuid::parse_str(id) {
                state.requires_report |=
                    self.published_report_policy(id) == Some(EInitializeReportPolicy::REQUIRED);
            }
        }
        initialization_refusal(
            &self.initialization_scopes(Some(post)),
            &initialization,
            &post.to_string(),
        )
    }

    /// An immutable signed close still bounds a waiting opening after its live row disappears.
    pub fn opening_wait_reason(&self, row: &ScheduledRow, post: Uuid) -> Option<WaitReason> {
        let opening = row
            .transition
            .scheduled_date
            .as_deref()
            .and_then(|date| DateTime::parse_from_rfc3339(date).ok())
            .map(|date| date.with_timezone(&Utc));
        let channels = row.payload().channels();
        // An arbitrary live opening moved after a signed close cannot create a new period.
        let signed_opening = self.approval(Some(post)).is_some_and(|approval| {
            approval.schedule.iter().any(|entry| {
                entry.event_processor == "START_VOTING_PERIOD"
                    && entry.scheduled_event_id == row.transition.scheduled_event_id
                    && entry.fingerprint == row.transition.fingerprint
            })
        });
        let signed_close = self.approval(Some(post)).and_then(|approval| {
            let own: Vec<_> = approval
                .schedule
                .iter()
                .filter(|entry| {
                    entry.event_processor == "END_VOTING_PERIOD"
                        && entry.election_id.as_deref() == Some(post.to_string().as_str())
                })
                .collect();
            let overrides_event = own.iter().any(|entry| {
                ScheduledRow {
                    transition: (**entry).clone(),
                    annotations: json!({}),
                    written_now: false,
                }
                .payload()
                .channels()
                .contains(&VotingStatusChannel::ONLINE)
            });
            approval
                .schedule
                .iter()
                .filter(|entry| entry.event_processor == "END_VOTING_PERIOD")
                .filter(|entry| {
                    entry.election_id.as_deref() == Some(post.to_string().as_str())
                        || (!overrides_event && entry.election_id.is_none())
                })
                .filter(|entry| {
                    ScheduledRow {
                        transition: (*entry).clone(),
                        annotations: json!({}),
                        written_now: false,
                    }
                    .payload()
                    .channels()
                    .iter()
                    .any(|channel| channels.contains(channel))
                })
                .filter_map(|entry| {
                    entry
                        .scheduled_date
                        .as_deref()
                        .and_then(|date| DateTime::parse_from_rfc3339(date).ok())
                        .map(|date| date.with_timezone(&Utc))
                })
                .filter(|close| {
                    !signed_opening || opening.map(|opening| *close >= opening).unwrap_or(true)
                })
                .min()
        });
        let live_close = if self.live_rows.is_empty() {
            effective_close(&self.live_closes, &post.to_string())
        } else {
            let own_overrides = self.live_rows.iter().any(|candidate| {
                self.own_row_key(candidate) == Some((post, "END_VOTING_PERIOD".into()))
            });
            self.live_rows
                .iter()
                .filter(|candidate| candidate.action() == SigningAction::CloseVoting)
                .filter(|candidate| match candidate.election() {
                    None => !own_overrides,
                    Some(target) if target == post => {
                        let expected = generate_manage_date_task_name(
                            &self.tenant_id.to_string(),
                            &self.election_event_id.to_string(),
                            Some(&post.to_string()),
                            &EventProcessors::END_VOTING_PERIOD,
                        );
                        self.live_row_metadata
                            .get(&candidate.transition.scheduled_event_id)
                            .and_then(|metadata| metadata.task_id.as_deref())
                            == Some(expected.as_str())
                    }
                    _ => false,
                })
                .filter(|candidate| {
                    candidate
                        .payload()
                        .channels()
                        .iter()
                        .any(|channel| channels.contains(channel))
                })
                .filter_map(|candidate| {
                    candidate
                        .transition
                        .scheduled_date
                        .as_deref()
                        .and_then(|date| DateTime::parse_from_rfc3339(date).ok())
                        .map(|date| date.with_timezone(&Utc))
                })
                .filter(|close| {
                    !signed_opening || opening.map(|opening| *close >= opening).unwrap_or(true)
                })
                .min()
        }
        .filter(|close| {
            !signed_opening || opening.map(|opening| *close >= opening).unwrap_or(true)
        });
        if let Some(close) = signed_close
            .into_iter()
            .chain(live_close)
            .min()
            .filter(|close| *close <= self.now)
        {
            return Some(WaitReason::AfterClose(close));
        }
        self.initialization_refusal_at(post)
            .map(WaitReason::Initialization)
    }

    /// Whether `action` needs signatures at a Post in either copy.
    pub fn needs_signatures(&self, action: SigningAction, post: Option<Uuid>) -> bool {
        self.rules.of(action).required || self.published_copy(post, action).values().rule.required
    }

    /// The newest executed approval at a Post (its own or the event's), or
    /// of the event.
    pub fn approval(&self, post: Option<Uuid>) -> Option<&Approval> {
        self.approvals
            .iter()
            .find(|approval| applies_at(approval.target, post))
    }

    pub fn post_name(&self, post: Option<Uuid>) -> String {
        match post {
            None => "every Post".to_owned(),
            Some(id) => self
                .posts
                .iter()
                .find(|candidate| candidate.id == id)
                .map(|candidate| candidate.name.clone())
                .unwrap_or_else(|| id.to_string()),
        }
    }

    /// The Posts a row changes: its own; for an event-wide row, every Post
    /// without its own row of the same kind ([`own_row_post`]), as the
    /// event-wide dispatch changes them. See [`event_wide_targets`].
    pub fn posts_of(&self, row: &ScheduledRow) -> Vec<Uuid> {
        if let Some(post) = row.election() {
            return vec![post];
        }
        self.posts
            .iter()
            .map(|post| post.id)
            .filter(|post| {
                !self
                    .own_rows
                    .contains(&(*post, row.transition.event_processor.clone()))
            })
            .collect()
    }

    /// A Post's enabled voting channels, sorted.
    pub fn enabled_channels(&self, post: Uuid) -> Vec<String> {
        enabled_channels(&self.post_channels.get(&post).cloned().unwrap_or_default())
    }

    /// What decides a row at a Post.
    pub fn inputs(&self, row: &ScheduledRow, post: Option<Uuid>, moment: Moment) -> Inputs {
        let action = row.action();
        let coverage = match self.approval(post) {
            None => Coverage::NoApproval,
            Some(approval) => {
                let signed = approval
                    .schedule
                    .iter()
                    .find(|signed| signed.scheduled_event_id == row.transition.scheduled_event_id);
                match signed {
                    None => Coverage::NotInApproval(approval.authorized_by()),
                    Some(signed) if signed.fingerprint == row.transition.fingerprint => {
                        self.covered_now(row, post, approval.authorized_by(), approval)
                    }
                    Some(_) => Coverage::Changed {
                        by: approval.authorized_by(),
                        edit: row
                            .prediction()
                            .filter(|prediction| {
                                prediction.fingerprint == row.transition.fingerprint
                            })
                            .and_then(|prediction| {
                                Some((prediction.edited_at?, prediction.edited_by?))
                            }),
                    },
                }
            }
        };
        Inputs {
            action,
            current: CopyValues {
                rule: self.rules.of(action).clone(),
                close_policy: self.policies.unsigned_scheduled_close.clone(),
            },
            published: self.published_copy(post, action),
            coverage,
            approval_required: self.rules.approve_configuration,
            moment,
        }
    }

    /// A signed, unchanged row: covered unless the Post's channels changed
    /// since signing, it already ran at the Post, or it is late.
    fn covered_now(
        &self,
        row: &ScheduledRow,
        post: Option<Uuid>,
        by: AuthorizedBy,
        approval: &Approval,
    ) -> Coverage {
        let Some(post) = post else {
            return Coverage::Covered(by);
        };
        if approval.post_channels.get(&post.to_string()) != Some(&self.enabled_channels(post)) {
            return Coverage::ChannelsChanged(by);
        }
        let key = (
            row.transition.scheduled_event_id.clone(),
            post,
            row.transition.fingerprint.clone(),
        );
        if let Some(fired_at) = self.fired.get(&key) {
            return Coverage::AlreadyFired {
                by,
                fired_at: instant(fired_at),
            };
        }
        let due = row
            .transition
            .scheduled_date
            .as_deref()
            .and_then(|date| DateTime::parse_from_rfc3339(date).ok())
            .map(|date| date.with_timezone(&Utc));
        match due {
            Some(due) if self.now <= due + chrono::Duration::minutes(LATE_FIRE_MINUTES) => {
                Coverage::Covered(by)
            }
            _ => Coverage::Late {
                by,
                scheduled_date: row.transition.scheduled_date.clone(),
            },
        }
    }

    /// The explanation of a row at a Post.
    pub fn explain(&self, row: &ScheduledRow, post: Option<Uuid>, moment: Moment) -> Explanation {
        let mut explanation = evaluate(&self.inputs(row, post, moment));
        if row.action() == SigningAction::OpenVoting && row.election().is_none() {
            if let Some((approval, own)) = post.and_then(|post| {
                self.approval(Some(post)).and_then(|approval| {
                    approval
                        .schedule
                        .iter()
                        .find(|entry| {
                            entry.event_processor == "START_VOTING_PERIOD"
                                && entry.election_id.as_deref() == Some(post.to_string().as_str())
                                && ScheduledRow {
                                    transition: (*entry).clone(),
                                    annotations: json!({}),
                                    written_now: false,
                                }
                                .payload()
                                .channels()
                                .contains(&VotingStatusChannel::ONLINE)
                        })
                        .map(|own| (approval, own))
                })
            }) {
                let check = Check {
                    id: CheckId::Covered,
                    current: value(
                        keys::COVERED_OVERRIDDEN_BY_SIGNED_POST,
                        json!({"code": approval.code, "scheduled_event_id": own.scheduled_event_id}),
                    ),
                    published: None,
                    allows: false,
                };
                if explanation.outcome != ScheduledOutcomeKind::Refused {
                    explanation
                        .checks
                        .retain(|check| check.id != CheckId::Covered);
                    explanation.deciding = CheckId::Covered;
                    explanation.outcome = ScheduledOutcomeKind::Refused;
                    explanation.authorized_by = None;
                    explanation.next_step = value(keys::NEXT_PUBLISH_AND_APPROVE, json!({}));
                    explanation.checks.push(check);
                }
            }
        }
        if row.action() == SigningAction::OpenVoting {
            if let Some(post) = post {
                if let Some(reason) = self.opening_wait_reason(row, post) {
                    let (id, key, params) = match &reason {
                        WaitReason::Initialization(refusal) => (
                            CheckId::Initialization,
                            keys::INITIALIZATION_WAITING,
                            json!({"reason": refusal.code(), "message": refusal.message(&self.post_name(Some(post)), &sequent_core::ballot::VotingStatus::OPEN, &sequent_core::ballot::VotingStatus::NOT_STARTED), "current_scope": self.initialization_scopes(Some(post)).current, "published_scope": self.initialization_scopes(Some(post)).published}),
                        ),
                        WaitReason::AfterClose(close) => (
                            CheckId::VotingClose,
                            keys::VOTING_CLOSE_PASSED,
                            json!({"closes_at": instant(close)}),
                        ),
                    };
                    explanation.checks.push(Check {
                        id: id.clone(),
                        current: value(key, params),
                        published: None,
                        allows: false,
                    });
                    // Preserve signing refusals (including replay and late-fire).
                    if explanation.outcome != ScheduledOutcomeKind::Refused {
                        explanation.outcome = match reason {
                            WaitReason::Initialization(_) => {
                                ScheduledOutcomeKind::WaitingForInitialization
                            }
                            WaitReason::AfterClose(_) => ScheduledOutcomeKind::Refused,
                        };
                        explanation.deciding = id;
                        explanation.next_step = value(
                            match reason {
                                WaitReason::Initialization(_) => keys::NEXT_INITIALIZE,
                                WaitReason::AfterClose(_) => keys::NEXT_CLOSED,
                            },
                            json!({}),
                        );
                    }
                }
            }
        }
        explanation
    }

    /// The predicted outcomes of a row: one per Post it applies to.
    pub fn explain_row(&self, row: &ScheduledRow) -> Vec<PostOutcome> {
        self.posts_of(row)
            .into_iter()
            .map(|post| PostOutcome {
                election_id: Some(post),
                explanation: self.explain(row, Some(post), Moment::Prediction),
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Evaluation (no database)

/// When an outcome is computed: ahead of time, or as the schedule fires.
/// Only the next step differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moment {
    Prediction,
    FireTime,
}

/// The values of one copy of the configuration that an outcome reads.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyValues {
    /// The action's rule.
    pub rule: RuleSnapshot,
    pub close_policy: UnsignedScheduledClosePolicy,
}

impl CopyValues {
    /// The values that stand for the published copy before anything (with
    /// a snapshot) is published: rules not required, the restrictive
    /// policies.
    pub fn defaults() -> Self {
        CopyValues {
            rule: RuleSnapshot::default(),
            close_policy: restrictive_lifecycle_policies().unsigned_scheduled_close,
        }
    }
}

/// The published copy.
#[derive(Debug, Clone, PartialEq)]
pub enum PublishedCopy {
    /// Nothing is published yet: the defaults apply.
    Nothing,
    /// Published before publications kept a snapshot: the defaults apply.
    NoSnapshot {
        publication_id: Uuid,
        published_at: DateTime<Utc>,
    },
    Snapshot {
        publication_id: Uuid,
        published_at: DateTime<Utc>,
        values: CopyValues,
    },
}

impl PublishedCopy {
    pub fn values(&self) -> CopyValues {
        match self {
            PublishedCopy::Snapshot { values, .. } => values.clone(),
            _ => CopyValues::defaults(),
        }
    }

    fn is_defaults(&self) -> bool {
        !matches!(self, PublishedCopy::Snapshot { .. })
    }
}

/// Whether the newest executed approval signed the row as it is now.
#[derive(Debug, Clone, PartialEq)]
pub enum Coverage {
    NoApproval,
    /// The approval doesn't list the row.
    NotInApproval(AuthorizedBy),
    /// The row changed since; the last edit (time, user) when known.
    Changed {
        by: AuthorizedBy,
        edit: Option<(String, String)>,
    },
    Covered(AuthorizedBy),
    /// Signed, but the Post's voting channels changed since.
    ChannelsChanged(AuthorizedBy),
    /// Signed, and it already ran at this Post (`fired_at`).
    AlreadyFired {
        by: AuthorizedBy,
        fired_at: String,
    },
    /// Signed, but it is more than [`LATE_FIRE_MINUTES`] past its time.
    Late {
        by: AuthorizedBy,
        scheduled_date: Option<String>,
    },
}

/// Everything [`evaluate`] reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Inputs {
    pub action: SigningAction,
    pub current: CopyValues,
    pub published: PublishedCopy,
    pub coverage: Coverage,
    /// Whether Approve configuration needs signatures now.
    pub approval_required: bool,
    pub moment: Moment,
}

fn value(key: &str, params: Value) -> CheckValue {
    CheckValue {
        message_key: key.to_owned(),
        params: match params {
            Value::Object(map) => map,
            _ => Map::new(),
        },
    }
}

fn rule_value(rule: &RuleSnapshot) -> CheckValue {
    match rule.signatures.filter(|_| rule.required) {
        Some(signatures) => value(
            keys::NEEDS_SIGNATURES_YES,
            json!({ "signatures": signatures }),
        ),
        None => value(keys::NEEDS_SIGNATURES_NO, json!({})),
    }
}

fn policy_value(policy: &UnsignedScheduledClosePolicy) -> CheckValue {
    match policy {
        UnsignedScheduledClosePolicy::REFUSE => value(keys::UNSIGNED_CLOSE_REFUSE, json!({})),
        UnsignedScheduledClosePolicy::RUN_AS_SYSTEM => {
            value(keys::UNSIGNED_CLOSE_RUN_AS_SYSTEM, json!({}))
        }
    }
}

fn instant(at: &DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// How much an outcome lets happen, to take the stricter of two.
fn rank(outcome: &ScheduledOutcomeKind) -> u8 {
    match outcome {
        ScheduledOutcomeKind::Refused | ScheduledOutcomeKind::WaitingForInitialization => 0,
        ScheduledOutcomeKind::RunsUnsigned => 1,
        ScheduledOutcomeKind::Runs => 2,
    }
}

fn outcome_of(
    action: SigningAction,
    needs: bool,
    covered: bool,
    policy: &UnsignedScheduledClosePolicy,
) -> ScheduledOutcomeKind {
    if !needs || covered {
        ScheduledOutcomeKind::Runs
    } else if action != SigningAction::CloseVoting {
        ScheduledOutcomeKind::Refused
    } else if *policy == UnsignedScheduledClosePolicy::RUN_AS_SYSTEM {
        ScheduledOutcomeKind::RunsUnsigned
    } else {
        ScheduledOutcomeKind::Refused
    }
}

/// The outcome of a scheduled opening or closing and its reasons; see the
/// module documentation.
///
/// Checks, in order: `needs_signatures` (both rules); `covered` (when
/// signatures are needed); `unsigned_close` (an uncovered close: both
/// policies); `stricter_copy` (whether the published copy holds back what
/// the current settings alone allow); `defaults` (what stands for the
/// published copy). The deciding check is the one that held the transition
/// back: the published copy (`stricter_copy`, or `defaults` before anything
/// is published) when the current settings alone would allow more,
/// otherwise `covered` (openings) or `unsigned_close` (closes).
pub fn evaluate(inputs: &Inputs) -> Explanation {
    let current = &inputs.current;
    let published = inputs.published.values();
    let close = inputs.action == SigningAction::CloseVoting;
    let covered_by = match &inputs.coverage {
        Coverage::Covered(by) => Some(by.clone()),
        _ => None,
    };
    let covered = covered_by.is_some();
    let needs = current.rule.required || published.rule.required;
    let policy = if current.close_policy == UnsignedScheduledClosePolicy::RUN_AS_SYSTEM
        && published.close_policy == UnsignedScheduledClosePolicy::RUN_AS_SYSTEM
    {
        UnsignedScheduledClosePolicy::RUN_AS_SYSTEM
    } else {
        UnsignedScheduledClosePolicy::REFUSE
    };
    let outcome = outcome_of(inputs.action, needs, covered, &policy);
    let current_only = outcome_of(
        inputs.action,
        current.rule.required,
        covered,
        &current.close_policy,
    );
    let published_only = outcome_of(
        inputs.action,
        published.rule.required,
        covered,
        &published.close_policy,
    );
    // The published copy (alone or with the current one) holds back what
    // the current settings alone would allow.
    let held_back = rank(&outcome) < rank(&current_only);

    let mut checks = vec![Check {
        id: CheckId::NeedsSignatures,
        current: rule_value(&current.rule),
        published: Some(rule_value(&published.rule)),
        allows: !needs,
    }];
    if needs {
        let coverage = match &inputs.coverage {
            Coverage::NoApproval => value(keys::COVERED_NO_APPROVAL, json!({})),
            Coverage::NotInApproval(by) => {
                value(keys::COVERED_NOT_IN_APPROVAL, json!({ "code": by.code }))
            }
            Coverage::Changed { by, edit: None } => {
                value(keys::COVERED_CHANGED, json!({ "code": by.code }))
            }
            Coverage::Changed {
                by,
                edit: Some((at, user)),
            } => value(
                keys::COVERED_CHANGED_BY,
                json!({ "code": by.code, "edited_at": at, "edited_by": user }),
            ),
            Coverage::Covered(by) => value(keys::COVERED_YES, json!({ "code": by.code })),
            Coverage::ChannelsChanged(by) => {
                value(keys::COVERED_CHANNELS_CHANGED, json!({ "code": by.code }))
            }
            Coverage::AlreadyFired { by, fired_at } => value(
                keys::COVERED_ALREADY_FIRED,
                json!({ "code": by.code, "fired_at": fired_at }),
            ),
            Coverage::Late { by, scheduled_date } => value(
                keys::COVERED_LATE,
                json!({ "code": by.code, "scheduled_date": scheduled_date }),
            ),
        };
        checks.push(Check {
            id: CheckId::Covered,
            current: coverage,
            published: None,
            allows: covered,
        });
        if close && !covered {
            checks.push(Check {
                id: CheckId::UnsignedClose,
                current: policy_value(&current.close_policy),
                published: Some(policy_value(&published.close_policy)),
                allows: policy == UnsignedScheduledClosePolicy::RUN_AS_SYSTEM,
            });
        }
    }
    let stricter = if current_only == published_only {
        keys::STRICTER_COPY_SAME
    } else if outcome == current_only {
        keys::STRICTER_COPY_CURRENT_STRICTER
    } else if outcome == published_only {
        keys::STRICTER_COPY_CURRENT_LOOSER
    } else {
        keys::STRICTER_COPY_COMBINED
    };
    checks.push(Check {
        id: CheckId::StricterCopy,
        current: value(stricter, json!({})),
        published: None,
        allows: !held_back,
    });
    let defaults = match &inputs.published {
        PublishedCopy::Nothing => value(keys::DEFAULTS_NOTHING_PUBLISHED, json!({})),
        PublishedCopy::NoSnapshot {
            publication_id,
            published_at,
        } => value(
            keys::DEFAULTS_NO_SNAPSHOT,
            json!({ "publication_id": publication_id, "published_at": instant(published_at) }),
        ),
        PublishedCopy::Snapshot {
            publication_id,
            published_at,
            ..
        } => value(
            keys::DEFAULTS_PUBLISHED,
            json!({ "publication_id": publication_id, "published_at": instant(published_at) }),
        ),
    };
    checks.push(Check {
        id: CheckId::Defaults,
        current: defaults,
        published: None,
        allows: !(held_back && inputs.published.is_defaults()),
    });

    let deciding = match outcome {
        ScheduledOutcomeKind::Runs if !needs => CheckId::NeedsSignatures,
        ScheduledOutcomeKind::Runs => CheckId::Covered,
        _ if held_back && inputs.published.is_defaults() => CheckId::Defaults,
        _ if held_back => CheckId::StricterCopy,
        _ if close => CheckId::UnsignedClose,
        _ => CheckId::Covered,
    };
    let next_step = match (&outcome, inputs.moment) {
        (ScheduledOutcomeKind::Runs, _) => keys::NEXT_NONE,
        (ScheduledOutcomeKind::WaitingForInitialization, _) => keys::NEXT_INITIALIZE,
        // Closing with signatures before the deadline avoids the unsigned close.
        (ScheduledOutcomeKind::RunsUnsigned, Moment::Prediction) => keys::NEXT_ASK_SIGNERS_TO_CLOSE,
        (ScheduledOutcomeKind::RunsUnsigned, Moment::FireTime) => keys::NEXT_NONE,
        (ScheduledOutcomeKind::Refused, Moment::FireTime) if close => {
            keys::NEXT_ASK_SIGNERS_TO_CLOSE
        }
        (ScheduledOutcomeKind::Refused, Moment::FireTime) => keys::NEXT_ASK_SIGNERS_TO_OPEN,
        (ScheduledOutcomeKind::Refused, Moment::Prediction) if !inputs.approval_required => {
            keys::NEXT_REQUIRE_CONFIGURATION_APPROVAL
        }
        (ScheduledOutcomeKind::Refused, Moment::Prediction) => keys::NEXT_PUBLISH_AND_APPROVE,
    };
    Explanation {
        authorized_by: if outcome == ScheduledOutcomeKind::Runs && needs {
            covered_by
        } else {
            None
        },
        outcome,
        checks,
        deciding,
        next_step: value(next_step, json!({})),
    }
}

// ---------------------------------------------------------------------------
// Words for the electoral log (English, like every signing log description)

fn outcome_words(explanation: &Explanation) -> String {
    match explanation.outcome {
        ScheduledOutcomeKind::Runs => match &explanation.authorized_by {
            Some(by) => format!("runs, authorized by configuration {}", by.code),
            None => "runs, no signatures needed".to_owned(),
        },
        ScheduledOutcomeKind::RunsUnsigned => "runs without signatures".to_owned(),
        ScheduledOutcomeKind::Refused => "refused".to_owned(),
        ScheduledOutcomeKind::WaitingForInitialization => "waiting for initialization".to_owned(),
    }
}

/// The deciding check, in words.
pub fn check_words(explanation: &Explanation) -> String {
    let check = explanation
        .checks
        .iter()
        .find(|check| check.id == explanation.deciding);
    let param = |name: &str| {
        check
            .and_then(|check| check.current.params.get(name))
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_owned()
    };
    match check.map(|check| check.current.message_key.as_str()) {
        Some(keys::INITIALIZATION_WAITING) => param("message"),
        Some(keys::VOTING_CLOSE_PASSED) => format!("voting closed at {}", param("closes_at")),
        Some(keys::NEEDS_SIGNATURES_NO) => "no signatures needed".to_owned(),
        Some(keys::COVERED_OVERRIDDEN_BY_SIGNED_POST) => format!("the signed configuration {} uses the Post's own opening {}; the event-wide opening does not apply", param("code"), param("scheduled_event_id")),
        Some(keys::COVERED_YES) => format!("in the signed configuration {}", param("code")),
        Some(keys::COVERED_CHANGED) => format!(
            "not in the signed configuration (changed since approval {})",
            param("code")
        ),
        Some(keys::COVERED_CHANGED_BY) => format!(
            "not in the signed configuration (edited on {} by {})",
            param("edited_at"),
            param("edited_by")
        ),
        Some(keys::COVERED_NOT_IN_APPROVAL) => format!(
            "not in the signed configuration (approval {} doesn't include it)",
            param("code")
        ),
        Some(keys::COVERED_CHANNELS_CHANGED) => format!(
            "not in the signed configuration (the Post's voting channels changed since approval {})",
            param("code")
        ),
        Some(keys::COVERED_ALREADY_FIRED) => format!(
            "the signed transition already ran on {}; running it again needs signatures",
            param("fired_at")
        ),
        Some(keys::COVERED_LATE) => format!(
            "more than {LATE_FIRE_MINUTES} minutes after its time ({}); the signed schedule no longer covers it",
            param("scheduled_date")
        ),
        Some(keys::COVERED_NO_APPROVAL) => {
            "not in a signed configuration: no configuration approval yet".to_owned()
        }
        Some(keys::UNSIGNED_CLOSE_REFUSE) => {
            "it needs signatures and isn't in the signed configuration; a scheduled close without signatures is refused"
                .to_owned()
        }
        Some(keys::UNSIGNED_CLOSE_RUN_AS_SYSTEM) => {
            "it isn't in the signed configuration; a scheduled close without signatures runs"
                .to_owned()
        }
        Some(keys::STRICTER_COPY_CURRENT_LOOSER) | Some(keys::STRICTER_COPY_COMBINED) => {
            "changed since the published configuration: takes effect after the next approved publication"
                .to_owned()
        }
        Some(keys::DEFAULTS_NOTHING_PUBLISHED) => "nothing published: defaults apply".to_owned(),
        Some(keys::DEFAULTS_NO_SNAPSHOT) => {
            "published before lifecycle snapshots: defaults apply".to_owned()
        }
        _ => explanation.deciding.to_string(),
    }
}

fn next_step_words(explanation: &Explanation) -> &'static str {
    match explanation.next_step.message_key.as_str() {
        keys::NEXT_PUBLISH_AND_APPROVE => "publish and approve the configuration",
        keys::NEXT_REQUIRE_CONFIGURATION_APPROVAL => {
            "make Approve configuration need signatures, then publish and approve the configuration"
        }
        keys::NEXT_ASK_SIGNERS_TO_CLOSE => "ask the Post's signers to close voting",
        keys::NEXT_ASK_SIGNERS_TO_OPEN => "ask the Post's signers to open voting",
        _ => "no action needed",
    }
}

/// "Scheduled close of {Post} at {time}".
pub fn transition_words(transition: &ScheduledTransition, post_name: &str) -> String {
    let what = match transition_action(&transition.event_processor) {
        Some(SigningAction::CloseVoting) => "close",
        _ => "opening",
    };
    let when = match (&transition.local, &transition.timezone) {
        (Some(local), Some(zone)) => format!("{} {zone}", local.replacen('T', " ", 1)),
        _ => transition
            .scheduled_date
            .clone()
            .unwrap_or_else(|| "an unset time".to_owned()),
    };
    format!("Scheduled {what} of {post_name} at {when}")
}

/// The description of a changed prediction.
pub fn change_description(
    transition: &ScheduledTransition,
    post_name: &str,
    before: Option<&Explanation>,
    after: &Explanation,
) -> String {
    let was = before
        .map(|before| format!(" (was: {})", outcome_words(before)))
        .unwrap_or_default();
    format!(
        "{}: now {}{was}. Deciding check: {}. Next step: {}.",
        transition_words(transition, post_name),
        outcome_words(after),
        check_words(after),
        next_step_words(after),
    )
}

/// The description of a scheduled change at fire time.
pub fn fired_words(action: SigningAction, explanation: &Explanation, post_name: &str) -> String {
    let verb = match action {
        SigningAction::CloseVoting => "Closed voting",
        _ => "Opened voting",
    };
    match (&explanation.outcome, &explanation.authorized_by) {
        (ScheduledOutcomeKind::Runs, Some(by)) => format!(
            "{verb} at {post_name} on schedule, authorized by the signed configuration {}",
            by.code
        ),
        (ScheduledOutcomeKind::Runs, None) => format!("{verb} at {post_name} on schedule"),
        (ScheduledOutcomeKind::RunsUnsigned, _) => {
            format!("{verb} at {post_name} on schedule without signatures")
        }
        (ScheduledOutcomeKind::WaitingForInitialization, _) => format!(
            "Scheduled opening at {post_name} waits for initialization: {}",
            check_words(explanation)
        ),
        (ScheduledOutcomeKind::Refused, _) => format!(
            "Did not {} at {post_name} on schedule: {}",
            action_title(action).to_lowercase(),
            check_words(explanation)
        ),
    }
}

// ---------------------------------------------------------------------------
// Predictions

/// A row's outcome at one Post.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostOutcome {
    pub election_id: Option<Uuid>,
    pub explanation: Explanation,
}

/// What `annotations.predicted_outcome` keeps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prediction {
    /// The row as it was predicted.
    pub fingerprint: String,
    /// When and by whom the row last changed, as a recompute saw it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_by: Option<String>,
    /// One per Post the row applies to.
    pub outcomes: Vec<PostOutcome>,
}

impl Prediction {
    fn outcome_at(&self, post: Option<Uuid>) -> Option<&Explanation> {
        self.outcomes
            .iter()
            .find(|outcome| outcome.election_id == post)
            .map(|outcome| &outcome.explanation)
    }
}

/// One row's outcome at one Post, as `get_scheduled_outcomes` answers it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledOutcome {
    pub scheduled_event_id: String,
    pub election_id: Option<Uuid>,
    pub explanation: Explanation,
}

/// A row and Post whose outcome a change changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeChange {
    pub scheduled_event_id: String,
    pub election_id: Option<Uuid>,
    /// `None` for a row that doesn't exist yet.
    pub before: Option<Explanation>,
    pub after: Explanation,
}

/// The outcome of every future scheduled opening and closing of the event,
/// one per row and Post.
pub async fn scheduled_outcomes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<ScheduledOutcome>> {
    let state = EventState::read(hasura_transaction, tenant_id, election_event_id).await?;
    let rows = read_rows(hasura_transaction, tenant_id, election_event_id, None).await?;
    Ok(outcomes_of(&state, &rows))
}

pub fn outcomes_of(state: &EventState, rows: &[ScheduledRow]) -> Vec<ScheduledOutcome> {
    rows.iter()
        .flat_map(|row| {
            state
                .explain_row(row)
                .into_iter()
                .map(|outcome| ScheduledOutcome {
                    scheduled_event_id: row.transition.scheduled_event_id.clone(),
                    election_id: outcome.election_id,
                    explanation: outcome.explanation,
                })
        })
        .collect()
}

/// The rows and Posts whose outcome differs between two states.
pub fn changes_between(
    before: (&EventState, &[ScheduledRow]),
    after: (&EventState, &[ScheduledRow]),
) -> Vec<OutcomeChange> {
    let old: HashMap<(String, Option<Uuid>), Explanation> = outcomes_of(before.0, before.1)
        .into_iter()
        .map(|outcome| {
            (
                (outcome.scheduled_event_id, outcome.election_id),
                outcome.explanation,
            )
        })
        .collect();
    outcomes_of(after.0, after.1)
        .into_iter()
        .filter_map(|outcome| {
            let before = old
                .get(&(outcome.scheduled_event_id.clone(), outcome.election_id))
                .cloned();
            let same = before
                .as_ref()
                .is_some_and(|before| before.outcome == outcome.explanation.outcome);
            (!same).then_some(OutcomeChange {
                scheduled_event_id: outcome.scheduled_event_id,
                election_id: outcome.election_id,
                before,
                after: outcome.explanation,
            })
        })
        .collect()
}

async fn write_annotation(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    scheduled_event_id: &str,
    key: &str,
    value: &Value,
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.scheduled_event
             SET annotations = COALESCE(annotations, '{}'::jsonb)
                 || jsonb_build_object($4::text, $5::jsonb)
             WHERE tenant_id = $1 AND election_event_id = $2 AND id::text = $3",
            &[
                &tenant_id,
                &election_event_id,
                &scheduled_event_id,
                &key,
                value,
            ],
        )
        .await
        .with_context(|| format!("Error keeping the scheduled event's {key}"))?;
    Ok(())
}

/// Recomputes the predicted outcome of every future scheduled opening and
/// closing of the event, after a write that can change one, and logs each
/// row and Post whose outcome changed (or, for a row this transaction
/// created or changed, that had no prediction yet) as one
/// `ScheduledOutcomeChanged` step for `actor`, who made the write
/// (details `{scheduled_event_id, election_id, action, fingerprint, before,
/// after}`). Outcomes that stay the same log nothing. Call it in the
/// write's transaction, after the write; its log steps take the event's
/// signing lock, which comes before any other lock of the transaction.
#[instrument(skip(hasura_transaction), err)]
pub async fn recompute_predictions(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    actor: &Actor,
) -> Result<()> {
    let (Ok(tenant_id), Ok(election_event_id)) = (
        Uuid::parse_str(tenant_id),
        Uuid::parse_str(election_event_id),
    ) else {
        return Ok(());
    };
    // The signing lock first: the log steps take it, and it comes before
    // any row lock.
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    let rows = read_rows(hasura_transaction, tenant_id, election_event_id, None).await?;
    if rows.is_empty() {
        return Ok(());
    }
    let state = EventState::read(hasura_transaction, tenant_id, election_event_id).await?;
    let now: DateTime<Utc> = hasura_transaction
        .query_one("SELECT clock_timestamp()", &[])
        .await?
        .try_get(0)?;
    for row in &rows {
        let stored = row.prediction();
        // A row this write changed is attributed to it; a row changed by
        // another path (a direct edit) is marked changed, by nobody known.
        let (edited_at, edited_by) = match &stored {
            Some(stored) if stored.fingerprint == row.transition.fingerprint => {
                (stored.edited_at.clone(), stored.edited_by.clone())
            }
            Some(_) if row.written_now => (Some(instant(&now)), Some(actor.username.clone())),
            _ => (None, None),
        };
        let mut prediction = Prediction {
            fingerprint: row.transition.fingerprint.clone(),
            edited_at,
            edited_by,
            outcomes: vec![],
        };
        let mut seen = row.clone();
        if let Value::Object(fields) = &mut seen.annotations {
            fields.insert(
                PREDICTED_OUTCOME.to_owned(),
                serde_json::to_value(&prediction)?,
            );
        }
        prediction.outcomes = state.explain_row(&seen);
        if stored.as_ref() == Some(&prediction) {
            continue;
        }
        for outcome in &prediction.outcomes {
            let before = stored
                .as_ref()
                .and_then(|stored| stored.outcome_at(outcome.election_id));
            if before.is_some_and(|before| before.outcome == outcome.explanation.outcome) {
                continue;
            }
            // A first prediction is logged only for a row this write created
            // or changed; others are seeded silently.
            if before.is_none() && !row.written_now {
                continue;
            }
            stage(
                hasura_transaction,
                &LogStep {
                    kind: SigningStatementKind::ScheduledOutcomeChanged,
                    user: actor.clone(),
                    system: SystemOutcome::Info,
                    scope: log_scope(tenant_id, election_event_id, outcome.election_id, None),
                    description: change_description(
                        &row.transition,
                        &state.post_name(outcome.election_id),
                        before,
                        &outcome.explanation,
                    ),
                    details: json!({
                        "scheduled_event_id": row.transition.scheduled_event_id,
                        "election_id": outcome.election_id,
                        "action": row.action(),
                        "fingerprint": row.transition.fingerprint,
                        "before": before,
                        "after": outcome.explanation,
                    }),
                },
            )
            .await?;
        }
        crate::postgres::trusted_write(hasura_transaction).await?;
        write_annotation(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &row.transition.scheduled_event_id,
            PREDICTED_OUTCOME,
            &serde_json::to_value(&prediction)?,
        )
        .await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Fire time

/// The row the scheduler fires, with the event's state; `None` when it is
/// not a scheduled opening or closing.
pub async fn fire_time_state(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    scheduled_event_id: &str,
) -> Result<Option<(EventState, ScheduledRow)>> {
    let Some(row) = read_rows(
        hasura_transaction,
        tenant_id,
        election_event_id,
        Some(scheduled_event_id),
    )
    .await?
    .pop() else {
        return Ok(None);
    };
    let state = if row.action() == SigningAction::CloseVoting {
        EventState::read_for_closes(hasura_transaction, tenant_id, election_event_id).await?
    } else {
        EventState::read(hasura_transaction, tenant_id, election_event_id).await?
    };
    Ok(Some((state, row)))
}

/// Keep each Post's latest actual result and its own timestamp across partial event openings.
fn merge_fired_outcomes(previous: Option<&Value>, current: &Value) -> Value {
    let mut posts = BTreeMap::new();
    for outcome in previous.into_iter().chain(std::iter::once(current)) {
        for post in outcome
            .get("posts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if post.get("outcome").and_then(Value::as_str) == Some("waiting-for-initialization") {
                continue;
            }
            let Some(id) = post.get("election_id").and_then(Value::as_str) else {
                continue;
            };
            let mut post = post.clone();
            if post.get("fired_at").is_none() {
                post["fired_at"] = outcome.get("at").cloned().unwrap_or(Value::Null);
            }
            posts.insert(id.to_owned(), post);
        }
    }
    json!({"at": current.get("at").cloned().or_else(|| previous.and_then(|value| value.get("at").cloned())), "posts": posts.into_values().collect::<Vec<_>>()})
}

/// Keeps what happened when the row fired (annotation [`FIRED_OUTCOME`]),
/// for the Publish tab's card.
pub async fn record_fired_outcome(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    scheduled_event_id: &str,
    fired: &Value,
) -> Result<()> {
    let previous = hasura_transaction.query_opt(
        "SELECT annotations -> $4::text FROM sequent_backend.scheduled_event WHERE tenant_id = $1 AND election_event_id = $2 AND id::text = $3 FOR UPDATE",
        &[&tenant_id, &election_event_id, &scheduled_event_id, &FIRED_OUTCOME],
    ).await?.and_then(|row| row.get::<_, Option<Value>>(0));
    let merged = merge_fired_outcomes(previous.as_ref(), fired);
    crate::postgres::trusted_write(hasura_transaction).await?;
    write_annotation(
        hasura_transaction,
        tenant_id,
        election_event_id,
        scheduled_event_id,
        FIRED_OUTCOME,
        &merged,
    )
    .await
}

// ---------------------------------------------------------------------------
// Rule and policy saves

/// How a rule or policy change applies to scheduled openings and closings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeApplies {
    /// Stricter: applies now to manual and scheduled actions.
    Tightens,
    /// Looser: applies now to manual actions; to scheduled openings and
    /// closings after the next approved publication.
    Loosens,
    /// Stricter in one value and looser in another.
    TightensAndLoosens,
}

impl ChangeApplies {
    pub fn message_key(self) -> &'static str {
        match self {
            ChangeApplies::Tightens => keys::APPLIES_TIGHTENS,
            ChangeApplies::Loosens => keys::APPLIES_LOOSENS,
            ChangeApplies::TightensAndLoosens => keys::APPLIES_TIGHTENS_AND_LOOSENS,
        }
    }

    pub fn sentence(self) -> &'static str {
        match self {
            ChangeApplies::Tightens => "Applies now to manual and scheduled actions.",
            ChangeApplies::Loosens => {
                "Applies now to manual actions; to scheduled openings and closings after the next approved publication."
            }
            ChangeApplies::TightensAndLoosens => {
                "The stricter part applies now; the looser part applies to scheduled openings and closings after the next approved publication."
            }
        }
    }

    fn of(tightens: bool, loosens: bool) -> Option<Self> {
        match (tightens, loosens) {
            (true, true) => Some(ChangeApplies::TightensAndLoosens),
            (true, false) => Some(ChangeApplies::Tightens),
            (false, true) => Some(ChangeApplies::Loosens),
            (false, false) => None,
        }
    }
}

/// How changing a rule from `old` to `new` applies: for Open/Close voting
/// and Approve configuration, needing signatures (or more of them) tightens
/// and needing none (or fewer) loosens; `None` for another action or a
/// change of neither.
pub fn rule_change_applies(old: &SigningRule, new: &SigningRule) -> Option<ChangeApplies> {
    if !matches!(
        new.action,
        SigningAction::OpenVoting
            | SigningAction::CloseVoting
            | SigningAction::ApproveConfiguration
    ) {
        return None;
    }
    let both = old.is_required() && new.is_required();
    ChangeApplies::of(
        (!old.is_required() && new.is_required()) || (both && new.required() > old.required()),
        (old.is_required() && !new.is_required()) || (both && new.required() < old.required()),
    )
}

/// How changing the lifecycle policies from `old` to `new` applies.
pub fn policy_change_applies(
    old: &LifecyclePolicies,
    new: &LifecyclePolicies,
) -> Option<ChangeApplies> {
    use sequent_core::ballot::InitializationScope::POST;
    use UnsignedScheduledClosePolicy::{REFUSE, RUN_AS_SYSTEM};
    let (close_tightens, close_loosens) =
        match (&old.unsigned_scheduled_close, &new.unsigned_scheduled_close) {
            (RUN_AS_SYSTEM, REFUSE) => (true, false),
            (REFUSE, RUN_AS_SYSTEM) => (false, true),
            _ => (false, false),
        };
    // POST asks least; EVENT and POST_AND_COUNTRY each ask something the
    // other doesn't.
    let (scope_tightens, scope_loosens) =
        match (&old.initialization_scope, &new.initialization_scope) {
            (old, new) if old == new => (false, false),
            (POST, _) => (true, false),
            (_, POST) => (false, true),
            _ => (true, true),
        };
    ChangeApplies::of(
        close_tightens || scope_tightens,
        close_loosens || scope_loosens,
    )
}

/// A change, as it would be saved, whose effect on the outcomes a preview
/// shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingChange {
    /// A scheduled event as it would be saved (a new one has no id).
    ScheduledEvent {
        #[serde(default)]
        id: Option<String>,
        event_processor: String,
        #[serde(default)]
        cron_config: Option<Value>,
        #[serde(default)]
        event_payload: Option<Value>,
    },
    /// The event's lifecycle policies.
    Policies(LifecyclePolicies),
    /// A signing rule: only whether it is required (and how many sign)
    /// matters to the outcomes.
    Rule {
        action: SigningAction,
        required: bool,
        #[serde(default)]
        signatures: Option<u16>,
    },
}

/// What saving a change would do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangePreview {
    /// For a rule or policy change: how it applies.
    pub applies: Option<ChangeApplies>,
    pub changes: Vec<OutcomeChange>,
}

/// The id a preview gives a scheduled event that doesn't exist yet.
pub const NEW_SCHEDULED_EVENT: &str = "new";

/// The state and rows after `change`, and how a rule or policy change
/// applies.
pub fn apply_change(
    state: &EventState,
    rows: &[ScheduledRow],
    change: &PendingChange,
) -> (EventState, Vec<ScheduledRow>, Option<ChangeApplies>) {
    let mut after_state = state.clone();
    let mut after_rows = rows.to_vec();
    let mut applies = None;
    match change {
        PendingChange::ScheduledEvent {
            id,
            event_processor,
            cron_config,
            event_payload,
        } => {
            let id = id.clone().unwrap_or_else(|| NEW_SCHEDULED_EVENT.to_owned());
            let existing = after_state
                .live_rows
                .iter()
                .find(|row| row.transition.scheduled_event_id == id)
                .cloned();
            let annotations = existing
                .as_ref()
                .map(|row| row.annotations.clone())
                .unwrap_or_else(|| json!({}));
            after_rows.retain(|row| row.transition.scheduled_event_id != id);
            after_state
                .live_rows
                .retain(|row| row.transition.scheduled_event_id != id);
            if let Some(transition) = transition_of(
                &id,
                event_processor,
                cron_config.as_ref(),
                event_payload.as_ref(),
            ) {
                let mut metadata = after_state
                    .live_row_metadata
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| ScheduleRowMetadata {
                        task_id: EventProcessors::from_str(event_processor)
                            .ok()
                            .map(|processor| {
                                generate_manage_date_task_name(
                                    &state.tenant_id.to_string(),
                                    &state.election_event_id.to_string(),
                                    transition.election_id.as_deref(),
                                    &processor,
                                )
                            }),
                        stopped: false,
                    });
                if transition
                    .scheduled_date
                    .as_deref()
                    .and_then(|date| DateTime::parse_from_rfc3339(date).ok())
                    .is_some_and(|date| date.with_timezone(&Utc) > state.transaction_now)
                {
                    metadata.stopped = false;
                }
                let row = ScheduledRow {
                    transition,
                    annotations,
                    written_now: true,
                };
                if !metadata.stopped {
                    after_rows.push(row.clone());
                }
                after_state.live_rows.push(row);
                after_state.live_row_metadata.insert(id, metadata);
            } else {
                after_state.live_row_metadata.remove(&id);
            }
        }
        PendingChange::Policies(policies) => {
            applies = policy_change_applies(&state.policies, policies);
            after_state.policies = policies.clone();
        }
        PendingChange::Rule {
            action,
            required,
            signatures,
        } => {
            let rule_of = |required: bool, signatures: Option<u32>| {
                let mut rule = SigningRule::default_for(*action);
                if required {
                    rule.requirement = SigningRequirement::Required;
                }
                rule.signatures = signatures
                    .and_then(|signatures| u16::try_from(signatures).ok())
                    .unwrap_or(1);
                rule
            };
            match action {
                SigningAction::OpenVoting | SigningAction::CloseVoting => {
                    let old = state.rules.of(*action);
                    let new_rule = rule_of(*required, signatures.map(u32::from).or(old.signatures));
                    applies =
                        rule_change_applies(&rule_of(old.required, old.signatures), &new_rule);
                    let snapshot = rule_snapshot(&new_rule);
                    if *action == SigningAction::OpenVoting {
                        after_state.rules.open_voting = snapshot;
                    } else {
                        after_state.rules.close_voting = snapshot;
                    }
                }
                SigningAction::ApproveConfiguration => {
                    applies = rule_change_applies(
                        &rule_of(state.rules.approve_configuration, None),
                        &rule_of(*required, None),
                    );
                    after_state.rules.approve_configuration = *required;
                }
                _ => {}
            }
        }
    }
    if matches!(change, PendingChange::ScheduledEvent { .. }) {
        after_state.own_rows = after_state
            .live_rows
            .iter()
            .filter_map(|row| after_state.own_row_key(row))
            .collect();
        after_state.live_closes = after_state
            .live_rows
            .iter()
            .filter(|row| row.action() == SigningAction::CloseVoting)
            .filter_map(|row| {
                row.transition
                    .scheduled_date
                    .clone()
                    .map(|date| (row.transition.election_id.clone(), date))
            })
            .collect();
    }
    (after_state, after_rows, applies)
}

/// Preview a batch atomically: later changes observe earlier rows and ownership overrides.
pub async fn preview_changes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    changes: &[PendingChange],
) -> Result<Vec<OutcomeChange>> {
    let before = EventState::read(hasura_transaction, tenant_id, election_event_id).await?;
    let rows = read_rows(hasura_transaction, tenant_id, election_event_id, None).await?;
    let mut after = before.clone();
    let mut after_rows = rows.clone();
    for (index, change) in changes.iter().enumerate() {
        let mut change = change.clone();
        if let PendingChange::ScheduledEvent { id, .. } = &mut change {
            if id.is_none() {
                *id = Some(format!("{NEW_SCHEDULED_EVENT}-{index}"));
            }
        }
        (after, after_rows, _) = apply_change(&after, &after_rows, &change);
    }
    Ok(changes_between((&before, &rows), (&after, &after_rows)))
}

/// What saving `change` would do to the outcomes, without saving it.
pub async fn preview_change(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    change: &PendingChange,
) -> Result<ChangePreview> {
    let state = EventState::read(hasura_transaction, tenant_id, election_event_id).await?;
    let rows = read_rows(hasura_transaction, tenant_id, election_event_id, None).await?;
    let (after_state, after_rows, applies) = apply_change(&state, &rows, change);
    Ok(ChangePreview {
        applies,
        changes: changes_between((&state, &rows), (&after_state, &after_rows)),
    })
}

/// Saving the lifecycle policies was refused: the event is locked down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockedDown;

impl std::fmt::Display for LockedDown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The election event is locked down; its lifecycle policies can't change."
        )
    }
}

impl std::error::Error for LockedDown {}

/// What saving the lifecycle policies answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedPolicies {
    pub policies: LifecyclePolicies,
    /// How the change applies; `None` when nothing changed.
    pub applies: Option<ChangeApplies>,
    /// The scheduled openings and closings (per Post) whose outcome changed.
    pub changes: Vec<OutcomeChange>,
}

/// Saves the event's lifecycle policies (`presentation.lifecycle_policies`),
/// then recomputes the predictions for `actor`, in the caller's
/// transaction.
#[instrument(skip(hasura_transaction), err)]
pub async fn save_lifecycle_policies(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    policies: &LifecyclePolicies,
    actor: &Actor,
) -> Result<SavedPolicies> {
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    // Like the signing rules: after lockdown, a change goes through a
    // configuration approval.
    if crate::services::election::is_election_event_locked_down_in(
        hasura_transaction,
        &tenant_id.to_string(),
        &election_event_id.to_string(),
    )
    .await?
    {
        return Err(anyhow::Error::new(LockedDown));
    }
    let preview = preview_change(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &PendingChange::Policies(policies.clone()),
    )
    .await?;
    let updated = hasura_transaction
        .execute(
            "UPDATE sequent_backend.election_event
             SET presentation = jsonb_set(
                 COALESCE(presentation, '{}'::jsonb), '{lifecycle_policies}', $3::jsonb, true)
             WHERE tenant_id = $1 AND id = $2",
            &[
                &tenant_id,
                &election_event_id,
                &serde_json::to_value(policies)?,
            ],
        )
        .await
        .context("Error saving the lifecycle policies")?;
    anyhow::ensure!(updated == 1, "There is no such election event.");
    recompute_predictions(
        hasura_transaction,
        &tenant_id.to_string(),
        &election_event_id.to_string(),
        actor,
    )
    .await?;
    Ok(SavedPolicies {
        policies: policies.clone(),
        applies: preview.applies,
        changes: preview.changes,
    })
}

#[cfg(test)]
#[path = "scheduled_outcome_tests.rs"]
mod scheduled_outcome_tests;
