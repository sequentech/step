// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Approving a configuration version (A3): publishing a generated ballot
//! publication (`/publish-ballot`).
//!
//! The request signs the publication and the SHA-256 of what publishing it
//! writes (the event, its elections and every ballot style, as the voter
//! files hold them), recomputed and compared before it publishes. With them
//! it signs what changed since the last publication of the same target, as
//! codes the portal words: the signing rules saved since (each with what it
//! needed before and needs now), the scheduled events created since, and
//! whether the ballots and contests changed. Its configuration version is
//! the one publishing it makes. One request waits per target (the event, or
//! the Post of an election-level publication). Generating a new publication cancels the
//! one waiting for its target and the initializations waiting for its Post,
//! and an event-level one every one waiting (PayloadChanged). The HSM/KMS
//! signature of the configuration package stays with EMS-MANIFEST.
//! The approval also signs the lifecycle policies, Open/Close voting rules,
//! initialization report policies and target schedules with their fingerprints.

use super::{event_ids, gate, refuse, subject_of, EffectProgress};
use crate::adapters::publication_files::PgPublicationRows;
use crate::ports::publication_files::PublicationRows;
use crate::postgres::ballot_publication::get_ballot_publication_by_id;
use crate::postgres::signing::{
    list_signing_rules, list_waiting_signing_requests, lock_signing_event,
    lock_waiting_signing_request, SigningRequestRow,
};
use crate::postgres::signing_actions::{
    count_published_configuration_versions, count_scheduled_events_since, last_published_at,
    signing_rules_before,
};
use crate::services::ballot_styles::ballot_publication::{
    get_ballot_publication_diff, update_publish_ballot, BallotPublicationValidationError,
};
use crate::services::scheduled_outcome::{
    lifecycle_snapshot, post_channels_of, PublicationLifecycle,
};
use crate::services::signing::guard::{GuardOutcome, GuardRequest, RequestScope};
use crate::services::signing::requests::cancel_request;
use crate::services::signing::{SigningCaller, SigningError, SigningResult};
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use futures::TryStreamExt;
use sequent_core::ballot::{EInitializeReportPolicy, LifecyclePolicies};
use sequent_core::signing::{CancelReason, SigningAction, SigningRequirement, SigningRule};
use sequent_core::types::hasura::core::BallotPublication;
use sequent_core::types::scheduled_outcome::{
    LifecycleSnapshot, RuleSnapshot, ScheduledTransition,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Whether the ballots and contests changed since the last publication.
pub const FIRST_VERSION: &str = "first-version";
pub const NO_CHANGES: &str = "no-changes";
pub const CHANGED: &str = "changed";

/// What an approve-configuration request signs: the publication, the digest
/// of what publishing it writes, and the changes the panel shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationSubject {
    pub ballot_publication_id: String,
    /// SHA-256 of what publishing writes; see [`publication_digest`].
    pub digest: String,
    /// The signing rules saved since the last publication, each
    /// `ACTION=BEFORE>AFTER` ([`rule_change`]). Requests started before
    /// carry the action id alone.
    pub signing_rules: Vec<String>,
    /// The active scheduled events created since the last publication.
    pub scheduled_events: i64,
    /// [`FIRST_VERSION`], [`NO_CHANGES`] or [`CHANGED`].
    pub ballots_and_contests: String,
    /// The lifecycle the approval authorizes (VOTE-LIFECYCLE §5a–§5b): the
    /// policies, the Open/Close voting rules and the scheduled openings and
    /// closings of the target, as the publication's snapshot records them.
    /// A request from before these fields reads the defaults and an empty
    /// schedule, which covers nothing.
    #[serde(default)]
    pub policies: LifecyclePolicies,
    #[serde(default)]
    pub open_voting: RuleSnapshot,
    #[serde(default)]
    pub close_voting: RuleSnapshot,
    #[serde(default)]
    pub schedule: Vec<ScheduledTransition>,
    /// Each Post's enabled voting channels (the target's Post, or every Post
    /// of the event): a covered transition runs only while they are the same.
    #[serde(default)]
    pub post_channels: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub initialization_report_policies: BTreeMap<String, EInitializeReportPolicy>,
}

impl ConfigurationSubject {
    /// The lifecycle snapshot it signs.
    pub fn lifecycle(&self) -> LifecycleSnapshot {
        LifecycleSnapshot {
            policies: self.policies.clone(),
            open_voting: self.open_voting.clone(),
            close_voting: self.close_voting.clone(),
            schedule: self.schedule.clone(),
            initialization_report_policies: self.initialization_report_policies.clone(),
        }
    }
}

/// The key one approve-configuration request waits under: the Post of an
/// election-level publication, or the event.
pub fn target_key(election_id: Option<&str>) -> String {
    election_id.unwrap_or("event").to_owned()
}

/// The scope key of the approve-configuration requests of a target.
pub fn target_scope_key(election_id: Option<&str>) -> String {
    scope(Uuid::nil(), Uuid::nil(), election_id).scope_key()
}

fn scope(tenant_id: Uuid, election_event_id: Uuid, election_id: Option<&str>) -> RequestScope {
    RequestScope {
        tenant_id,
        election_event_id,
        election_id: None,
        area_id: None,
        trustee_id: None,
        subject_key: Some(target_key(election_id)),
    }
}

/// `value` as text with every object's keys in order. Arrays keep their
/// order, which can matter (candidates).
pub fn canonical_text(value: &Value) -> String {
    fn sorted(value: &Value) -> Value {
        match value {
            Value::Object(fields) => {
                let mut keys: Vec<&String> = fields.keys().collect();
                keys.sort();
                let mut map = Map::new();
                for key in keys {
                    map.insert(key.clone(), sorted(&fields[key]));
                }
                Value::Object(map)
            }
            Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
            other => other.clone(),
        }
    }
    sorted(value).to_string()
}

fn sha256_hex(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

/// The id of a publication row.
fn row_id(row: &Value) -> Result<String> {
    row.get("id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .context("A publication row has no id")
}

/// SHA-256 over what publishing `publication` writes: the event, then its
/// elections and every ballot style, each a row-set in order of id, each
/// row in [`canonical_text`]. The styles are hashed one at a time.
pub async fn publication_digest_of(
    rows: &impl PublicationRows,
    tenant_id: Uuid,
    election_event_id: Uuid,
    publication_id: Uuid,
) -> Result<String> {
    let event = rows.event(tenant_id, election_event_id).await?;
    let mut elections = rows
        .elections(tenant_id, election_event_id, publication_id)
        .await?
        .into_iter()
        .map(|row| Ok((row_id(&row)?, canonical_text(&row))))
        .collect::<Result<Vec<_>>>()?;
    elections.sort();
    let styles = rows
        .styles(tenant_id, election_event_id, publication_id)
        .await?;
    futures::pin_mut!(styles);
    let mut style_hashes = vec![];
    while let Some(style) = styles.try_next().await? {
        style_hashes.push((row_id(&style)?, sha256_hex(&canonical_text(&style))));
    }
    style_hashes.sort();
    let mut hasher = Sha256::new();
    hasher.update(b"event\n");
    hasher.update(canonical_text(&event).as_bytes());
    for (id, text) in elections {
        hasher.update(format!("\nelection {id}\n").as_bytes());
        hasher.update(text.as_bytes());
    }
    for (id, hash) in style_hashes {
        hasher.update(format!("\nstyle {id} {hash}").as_bytes());
    }
    Ok(hex::encode(hasher.finalize()))
}

/// [`publication_digest_of`] on the database.
pub async fn publication_digest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    publication_id: Uuid,
) -> Result<String> {
    publication_digest_of(
        &PgPublicationRows {
            transaction: hasura_transaction,
        },
        tenant_id,
        election_event_id,
        publication_id,
    )
    .await
}

/// What the ballots and contests did, from a publication diff's two sides.
pub fn ballots_change(diff: &Value) -> &'static str {
    let current = diff.pointer("/current/ballot_styles").map(canonical_text);
    match diff.get("previous") {
        None | Some(Value::Null) => FIRST_VERSION,
        Some(previous) if previous.get("ballot_styles").map(canonical_text) == current => {
            NO_CHANGES
        }
        Some(_) => CHANGED,
    }
}

/// A rule's value in a `signing_rules` entry: `off`, or the signatures it needs.
fn rule_value(rule: &SigningRule) -> String {
    match rule.requirement {
        SigningRequirement::NotRequired => "off".to_owned(),
        SigningRequirement::Required => rule.required().to_string(),
    }
}

/// A `signing_rules` entry, which the portal words: `ACTION=BEFORE>AFTER`,
/// or `ACTION=AFTER` when what it was is unknown (a rule saved without a
/// logged change, e.g. imported).
pub fn rule_change(rule: &SigningRule, before: Option<&SigningRule>) -> String {
    match before {
        Some(before) => format!(
            "{}={}>{}",
            rule.action,
            rule_value(before),
            rule_value(rule)
        ),
        None => format!("{}={}", rule.action, rule_value(rule)),
    }
}

/// The subject of publishing `publication`.
pub async fn configuration_subject(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    publication: &BallotPublication,
) -> Result<ConfigurationSubject> {
    let publication_id = Uuid::parse_str(&publication.id)?;
    let diff = serde_json::to_value(
        get_ballot_publication_diff(
            hasura_transaction,
            tenant_id.to_string(),
            election_event_id.to_string(),
            publication.id.clone(),
            None,
        )
        .await?,
    )?;
    let target = publication
        .election_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()?;
    let since = last_published_at(hasura_transaction, tenant_id, election_event_id, target).await?;
    let before =
        signing_rules_before(hasura_transaction, tenant_id, election_event_id, since).await?;
    let signing_rules = list_signing_rules(hasura_transaction, tenant_id, election_event_id)
        .await?
        .into_iter()
        .filter(|row| since.map_or(true, |since| row.updated_at > since))
        .map(|row| {
            let was = before
                .get(&row.rule.action.to_string())
                .and_then(|old| serde_json::from_value::<SigningRule>(old.clone()).ok());
            rule_change(&row.rule, was.as_ref())
        })
        .collect();
    let scheduled_events =
        count_scheduled_events_since(hasura_transaction, tenant_id, election_event_id, since)
            .await?;
    let lifecycle =
        lifecycle_snapshot(hasura_transaction, tenant_id, election_event_id, target).await?;
    Ok(ConfigurationSubject {
        ballot_publication_id: publication.id.clone(),
        digest: publication_digest(
            hasura_transaction,
            tenant_id,
            election_event_id,
            publication_id,
        )
        .await?,
        signing_rules,
        scheduled_events,
        ballots_and_contests: ballots_change(&diff).to_owned(),
        policies: lifecycle.policies,
        open_voting: lifecycle.open_voting,
        close_voting: lifecycle.close_voting,
        schedule: lifecycle.schedule,
        initialization_report_policies: lifecycle.initialization_report_policies,
        post_channels: post_channels_of(hasura_transaction, tenant_id, election_event_id, target)
            .await?,
    })
}

/// Whether publishing a ballot publication runs now; see [`super::gate`].
pub async fn gate_publication(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
) -> SigningResult<GuardOutcome> {
    let Some((tenant_id, election_event_id)) = event_ids(tenant_id, election_event_id) else {
        return Ok(GuardOutcome::Proceed);
    };
    let Ok(publication_uuid) = Uuid::parse_str(ballot_publication_id) else {
        return Ok(GuardOutcome::Proceed);
    };
    // The signing lock, then the publication row: what is read below stays
    // as read until the route publishes in this transaction.
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    hasura_transaction
        .query_opt(
            "SELECT id FROM sequent_backend.ballot_publication
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3 FOR UPDATE",
            &[&tenant_id, &election_event_id, &publication_uuid],
        )
        .await
        .map_err(anyhow::Error::from)?;
    // An unknown or published publication fails or publishes again as today.
    let Some(publication) = get_ballot_publication_by_id(
        hasura_transaction,
        &tenant_id.to_string(),
        &election_event_id.to_string(),
        ballot_publication_id,
    )
    .await?
    .filter(|publication| publication.published_at.is_none()) else {
        return Ok(GuardOutcome::Proceed);
    };
    gate(
        hasura_transaction,
        caller,
        SigningAction::ApproveConfiguration,
        tenant_id,
        election_event_id,
        || async move {
            if !publication.is_generated.unwrap_or(false) || publication.deleted_at.is_some() {
                return Err(SigningError::invalid(
                    crate::services::signing::InvalidReason::Transition,
                    "Only a generated, current ballot publication can be published.",
                ));
            }
            let subject = configuration_subject(
                hasura_transaction,
                tenant_id,
                election_event_id,
                &publication,
            )
            .await?;
            // The version publishing it makes: an event-level publication
            // is the next one; a Post's joins the one in force.
            let published = count_published_configuration_versions(
                hasura_transaction,
                tenant_id,
                election_event_id,
            )
            .await?;
            let version = match publication.election_id {
                None => published + 1,
                Some(_) => published,
            };
            Ok(GuardRequest {
                action: SigningAction::ApproveConfiguration,
                scope: scope(
                    tenant_id,
                    election_event_id,
                    publication.election_id.as_deref(),
                ),
                subject: serde_json::to_value(subject).map_err(anyhow::Error::from)?,
                document: None,
                config_revision: Some(version.to_string()),
            })
        },
    )
    .await
}

/// Cancels the requests a new publication supersedes (PayloadChanged): the
/// configuration version waiting for its target and the initializations
/// waiting for its Post, or, for an event-level publication, every one of
/// them waiting. A new configuration version changes what initializing a
/// Post would start from, so its initialization starts again. It takes the
/// event's signing lock first, so call it before the route takes the
/// publication lock. How many it cancelled.
pub async fn cancel_for_new_publication(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
) -> SigningResult<usize> {
    let Some((tenant_id, election_event_id)) = event_ids(tenant_id, election_event_id) else {
        return Ok(0);
    };
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    let own_key = scope(tenant_id, election_event_id, election_id).scope_key();
    let waiting = list_waiting_signing_requests(
        hasura_transaction,
        tenant_id,
        election_event_id,
        SigningAction::ApproveConfiguration,
    )
    .await?;
    let post = election_id.map(Uuid::parse_str).transpose().ok().flatten();
    let initializations = list_waiting_signing_requests(
        hasura_transaction,
        tenant_id,
        election_event_id,
        SigningAction::InitializeVoting,
    )
    .await?;
    let superseded = waiting
        .iter()
        .filter(|candidate| election_id.is_none() || candidate.scope_key == own_key)
        .map(|candidate| (SigningAction::ApproveConfiguration, candidate))
        .chain(
            initializations
                .iter()
                .filter(|candidate| election_id.is_none() || candidate.election_id == post)
                .map(|candidate| (SigningAction::InitializeVoting, candidate)),
        );
    let mut cancelled = 0;
    for (action, candidate) in superseded {
        if let Some(request) = lock_waiting_signing_request(
            hasura_transaction,
            tenant_id,
            election_event_id,
            action,
            &candidate.scope_key,
        )
        .await?
        {
            cancel_request(
                hasura_transaction,
                &request,
                CancelReason::PayloadChanged,
                caller.actor(),
                Some("A new ballot publication was generated."),
            )
            .await?;
            cancelled += 1;
        }
    }
    Ok(cancelled)
}

/// The effect of a signed configuration version: the publication is
/// published, once what it writes is still what was signed.
pub async fn publish(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    progress: &EffectProgress,
) -> Result<Value> {
    let signed: ConfigurationSubject = subject_of(request)?;
    let tenant_id = request.tenant_id.to_string();
    let election_event_id = request.election_event_id.to_string();
    let publication = get_ballot_publication_by_id(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &signed.ballot_publication_id,
    )
    .await?
    .ok_or_else(|| {
        refuse(
            "publication-missing",
            "The ballot publication no longer exists.",
        )
    })?;
    if publication.published_at.is_some() {
        // Published meanwhile, not by this request: the request would count
        // as executed without keeping what it signed. It fails instead.
        return Err(refuse(
            "payload-changed",
            format!(
                "The ballot publication was published before signing request {} ran.",
                request.code
            ),
        ));
    }
    if publication.deleted_at.is_some() || !publication.is_generated.unwrap_or(false) {
        return Err(refuse(
            "publication-superseded",
            "The ballot publication is no longer the one to publish.",
        ));
    }
    let digest = publication_digest(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        Uuid::parse_str(&publication.id)?,
    )
    .await?;
    if digest != signed.digest {
        return Err(refuse(
            "payload-changed",
            format!(
                "What publishing writes changed since signing request {} started.",
                request.code
            ),
        ));
    }
    // Publishing writes the voter files and posts to the bulletin board.
    progress.reach_outside();
    // The published copy is what the signers authorized, even if a rule or
    // the schedule changed since the request started. A request from before
    // these fields signed no lifecycle: the previous signed copy stays.
    let lifecycle = if request.subject.get("policies").is_some() {
        PublicationLifecycle::Signed {
            request_id: request.id,
            snapshot: signed.lifecycle(),
        }
    } else {
        PublicationLifecycle::KeepPrevious
    };
    update_publish_ballot(
        hasura_transaction,
        request.requested_by.clone(),
        request.requested_by_username.clone(),
        tenant_id,
        election_event_id,
        signed.ballot_publication_id.clone(),
        &lifecycle,
    )
    .await
    .map_err(
        |error| match error.downcast_ref::<BallotPublicationValidationError>() {
            Some(invalid) => refuse("publication-invalid", invalid.to_string()),
            None => error,
        },
    )?;
    Ok(json!({
        "ballot_publication_id": signed.ballot_publication_id,
        "digest": signed.digest,
    }))
}
