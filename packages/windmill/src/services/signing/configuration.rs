// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! An election event's signing configuration as it travels in a bundle: its
//! rules and certificate checks on export and import, the mapping of the
//! Posts' `miru:area-threshold` to the transmit-results rule, and the
//! threshold the transmission send enforces.

use crate::postgres::signing::get_signing_checks;
use crate::postgres::signing::list_signing_rules;
use crate::postgres::signing::{get_signing_rule, lock_signing_event, upsert_signing_rule};
use crate::services::consolidation::eml_generator::{
    parse_area_threshold, prepend_miru_annotation, MIRU_AREA_THRESHOLD, MIRU_AREA_TRUSTEE_USERS,
};
use crate::services::electoral_log::ElectoralLogAdminContext;
use crate::services::signing::issuers::update_signing_checks;
use crate::services::signing::log::{stage, Actor, LogScope, LogStep, SystemOutcome};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::election_config::ImportElectionEventSchema;
use sequent_core::signing::{
    RequesterSigning, SigningAction, SigningChecks, SigningRequirement, SigningRule,
    MAX_EXPIRES_MINUTES, MAX_SIGNATURES,
};
use sequent_core::types::hasura::core::Area;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use tracing::{instrument, warn};
use uuid::Uuid;

/// Who an import is logged for when the task doesn't say who started it.
pub const SYSTEM_ACTOR_ID: &str = "system";

/// Where an imported rule came from, in its log entry's details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum RuleSource {
    /// The bundle's `signing_rules`.
    Import,
    /// The Posts' `miru:area-threshold`.
    AreaThreshold,
}

/// The person an import's log entries name: the administrator who started
/// it, else the system.
pub fn importer_actor(importer: Option<&ElectoralLogAdminContext>) -> Actor {
    match importer {
        Some(importer) => Actor {
            user_id: importer.user_id.clone(),
            username: importer
                .username
                .clone()
                .unwrap_or_else(|| importer.user_id.clone()),
        },
        None => Actor {
            user_id: SYSTEM_ACTOR_ID.to_owned(),
            username: SYSTEM_ACTOR_ID.to_owned(),
        },
    }
}

/// Imports the staff issuers carried by an election-event bundle, with
/// the same checks and electoral log entries as the Certificates settings.
pub async fn import_bundle_staff_issuers(
    transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    certificates: &[openssl::x509::X509],
    importer: Option<&ElectoralLogAdminContext>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<crate::services::signing::issuers::IssuerImport> {
    crate::services::signing::issuers::import_staff_issuers(
        transaction,
        tenant_id,
        election_event_id,
        certificates,
        &importer_actor(importer),
        super::Allowance::ElectionEventImport,
        now,
    )
    .await
}

/// A Post's `miru:area-threshold` annotation as written.
fn threshold_annotation(area: &Area) -> Option<&Value> {
    area.annotations
        .as_ref()?
        .get(prepend_miru_annotation(MIRU_AREA_THRESHOLD))
}

/// A Post's `miru:area-threshold`, read as its transmission package reads
/// it. `None` when absent or unreadable (annotations are text).
pub fn area_threshold(area: &Area) -> Option<i64> {
    match threshold_annotation(area)? {
        Value::String(text) => parse_area_threshold(text).ok(),
        _ => None,
    }
}

/// How warnings name a Post.
fn post_name(area: &Area) -> String {
    area.name.clone().unwrap_or_else(|| area.id.clone())
}

/// What the Posts' `miru:area-threshold` say about the transmit-results rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AreaThresholdMapping {
    /// The bundle has its own transmit-results rule, or no Post asks for
    /// signatures.
    Nothing,
    /// Every Post that asks for signatures asks for the same count.
    Rule(SigningRule),
    /// Posts ask for different counts (Post, threshold): each keeps its own,
    /// as in 2025, and no event-wide rule is made.
    Differing(Vec<(String, i64)>),
    /// Posts ask for more than [`MAX_SIGNATURES`] (Post, threshold): nothing
    /// is made from them.
    TooLarge(Vec<(String, i64)>),
}

/// The transmit-results rule a 2025-style bundle implies, so it keeps its
/// minimum: when the bundle has no transmit-results rule and every Post with
/// a positive threshold has the same one, a rule that needs it. It is started
/// and signed as the 2025 transmission was: whoever starts it may sign (a
/// Post's members all signed), and it never expires.
pub fn map_area_thresholds(rules: Option<&[SigningRule]>, areas: &[Area]) -> AreaThresholdMapping {
    let has_transmit_rule = rules
        .unwrap_or_default()
        .iter()
        .any(|rule| rule.action == SigningAction::TransmitResults);
    if has_transmit_rule {
        return AreaThresholdMapping::Nothing;
    }
    let mut asking: Vec<(String, i64)> = Vec::new();
    for area in areas {
        let Some(raw) = threshold_annotation(area) else {
            continue;
        };
        match area_threshold(area) {
            Some(threshold) if threshold > 0 => asking.push((post_name(area), threshold)),
            Some(_) => {}
            None => warn!(area_id = %area.id, %raw, "ignoring an unreadable area threshold"),
        }
    }
    let Some(&(_, first)) = asking.first() else {
        return AreaThresholdMapping::Nothing;
    };
    let too_large: Vec<(String, i64)> = asking
        .iter()
        .filter(|(_, threshold)| *threshold > i64::from(MAX_SIGNATURES))
        .cloned()
        .collect();
    if !too_large.is_empty() {
        return AreaThresholdMapping::TooLarge(too_large);
    }
    if asking.iter().any(|(_, threshold)| *threshold != first) {
        return AreaThresholdMapping::Differing(asking);
    }
    let Ok(signatures) = u16::try_from(first) else {
        return AreaThresholdMapping::TooLarge(asking);
    };
    AreaThresholdMapping::Rule(SigningRule {
        action: SigningAction::TransmitResults,
        requirement: SigningRequirement::Required,
        signatures,
        requester_signing: RequesterSigning::Allowed,
        expires_minutes: None,
        revision: 0,
    })
}

/// The Posts with fewer SBEIs (`miru:area-trustee-users`) than `required`,
/// which could not complete a transmission on their own.
pub fn posts_short_of_sbeis(areas: &[Area], required: u16) -> Vec<String> {
    areas
        .iter()
        .filter(|area| {
            let sbeis = area
                .annotations
                .as_ref()
                .and_then(|annotations| {
                    annotations.get(prepend_miru_annotation(MIRU_AREA_TRUSTEE_USERS))
                })
                .and_then(Value::as_str)
                .and_then(|list| serde_json::from_str::<Vec<String>>(list).ok());
            sbeis.is_some_and(|sbeis| sbeis.len() < usize::from(required))
        })
        .map(post_name)
        .collect()
}

/// The signatures a transmission needs before it is sent: the
/// transmit-results rule's count when the rule needs signatures, otherwise
/// the Post's annotation, as before rules existed.
pub fn transmission_threshold(rule: Option<&SigningRule>, annotation_threshold: i64) -> i64 {
    match rule {
        Some(rule) if rule.is_required() => i64::from(rule.required()),
        _ => annotation_threshold,
    }
}

/// [`transmission_threshold`] with the event's saved rule.
#[instrument(skip(hasura_transaction), err)]
pub async fn event_transmission_threshold(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    annotation_threshold: i64,
) -> Result<i64> {
    let rule = get_signing_rule(
        hasura_transaction,
        tenant_id,
        election_event_id,
        SigningAction::TransmitResults,
    )
    .await?
    .map(|row| row.rule);
    Ok(transmission_threshold(rule.as_ref(), annotation_threshold))
}

/// An event's signing configuration as an export writes it. A part the
/// event never saved is `None`, so it is left out of the bundle.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExportedSigningConfiguration {
    pub rules: Option<Vec<SigningRule>>,
    pub checks: Option<SigningChecks>,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn export_signing_configuration(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<ExportedSigningConfiguration> {
    let rules: Vec<SigningRule> =
        list_signing_rules(hasura_transaction, tenant_id, election_event_id)
            .await?
            .into_iter()
            .map(|row| row.rule)
            .collect();
    let checks = get_signing_checks(hasura_transaction, tenant_id, election_event_id)
        .await?
        .map(|row| row.checks);
    Ok(ExportedSigningConfiguration {
        rules: (!rules.is_empty()).then_some(rules),
        checks,
    })
}

/// Refuses a rule set that would not save as written: an action twice, or a
/// rule its own checks refuse.
fn check_rules(rules: &[SigningRule]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for rule in rules {
        if !seen.insert(rule.action.to_string()) {
            return Err(anyhow!(
                "the signing rules name {} more than once",
                rule.action
            ));
        }
        rule.validate()
            .map_err(|err| anyhow!("the signing rule of {}: {err}", rule.action))?;
        // The bundle validation refuses these first; kept for other callers.
        if rule.signatures > MAX_SIGNATURES
            || rule
                .expires_minutes
                .is_some_and(|minutes| minutes > MAX_EXPIRES_MINUTES)
        {
            return Err(anyhow!(
                "the signing rule of {} is out of range",
                rule.action
            ));
        }
    }
    Ok(())
}

/// Saves one imported rule on a new event and queues its log entries.
async fn save_imported_rule(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    rule: &SigningRule,
    source: RuleSource,
    actor: &Actor,
) -> Result<SigningRule> {
    let saved = upsert_signing_rule(
        hasura_transaction,
        tenant_id,
        election_event_id,
        rule,
        0,
        &actor.user_id,
        Some(&actor.username),
    )
    .await?
    .ok_or_else(|| {
        anyhow!(
            "the election event already has a signing rule for {}",
            rule.action
        )
    })?
    .rule;
    let needs = if saved.is_required() {
        format!("{} signatures needed", saved.required())
    } else {
        "no signatures needed".to_owned()
    };
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::SigningRuleChanged,
            user: actor.clone(),
            system: SystemOutcome::Info,
            scope: LogScope {
                tenant_id,
                election_event_id,
                election_id: None,
                area_id: None,
            },
            description: match source {
                RuleSource::Import => {
                    format!("Imported the signing rule of {}: {needs}", saved.action)
                }
                RuleSource::AreaThreshold => format!(
                    "Set the signing rule of {} from the Posts' area thresholds: {needs}",
                    saved.action
                ),
            },
            details: json!({
                "action": saved.action.to_string(),
                "old": SigningRule::default_for(saved.action),
                "new": saved,
                "cancelled": [],
                "source": source.to_string(),
            }),
        },
    )
    .await
    .context("Error logging the imported signing rule")?;
    Ok(saved)
}

/// Saves an imported bundle's signing configuration on its new event.
///
/// Absent rules and checks save nothing (every action keeps its default),
/// except that Posts with a `miru:area-threshold` and no transmit-results rule
/// get one (see [`map_area_thresholds`]). The revisions in
/// the bundle are not kept: the new event's configuration starts at 1. Each
/// saved rule and the checks are logged for `actor`.
#[instrument(skip(hasura_transaction, rules, checks, areas), err)]
pub async fn import_signing_configuration(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    rules: Option<&[SigningRule]>,
    checks: Option<&SigningChecks>,
    areas: &[Area],
    actor: &Actor,
) -> Result<()> {
    let mapped = match map_area_thresholds(rules, areas) {
        AreaThresholdMapping::Rule(rule) => {
            let short = posts_short_of_sbeis(areas, rule.signatures);
            if !short.is_empty() {
                warn!(
                    posts = ?short,
                    signatures = rule.signatures,
                    "these Posts have fewer SBEIs than the transmission threshold"
                );
            }
            Some(rule)
        }
        AreaThresholdMapping::Differing(posts) => {
            warn!(
                ?posts,
                "the Posts ask for different transmission thresholds; each keeps its own \
                 and no transmit-results rule is made"
            );
            None
        }
        AreaThresholdMapping::TooLarge(posts) => {
            warn!(
                ?posts,
                max = MAX_SIGNATURES,
                "transmission thresholds above the most a rule may need are not mapped"
            );
            None
        }
        AreaThresholdMapping::Nothing => None,
    };
    if rules.is_none() && checks.is_none() && mapped.is_none() {
        return Ok(());
    }
    if let Some(rules) = rules {
        check_rules(rules)?;
    }
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    for rule in rules.unwrap_or_default() {
        save_imported_rule(
            hasura_transaction,
            tenant_id,
            election_event_id,
            rule,
            RuleSource::Import,
            actor,
        )
        .await?;
    }
    if let Some(rule) = mapped {
        save_imported_rule(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &rule,
            RuleSource::AreaThreshold,
            actor,
        )
        .await?;
    }
    if let Some(checks) = checks {
        update_signing_checks(
            hasura_transaction,
            tenant_id,
            election_event_id,
            checks,
            0,
            actor,
            Some(&actor.username),
        )
        .await?
        .ok_or_else(|| anyhow!("the election event already has certificate checks"))?;
    }
    Ok(())
}

/// Fills an exported bundle's signing configuration from its event.
#[instrument(skip(hasura_transaction, bundle), err)]
pub async fn export_bundle_signing(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    bundle: &mut ImportElectionEventSchema,
) -> Result<()> {
    let exported =
        export_signing_configuration(hasura_transaction, tenant_id, election_event_id).await?;
    bundle.signing_rules = exported.rules;
    bundle.signing_checks = exported.checks;
    Ok(())
}

/// Saves an imported bundle's signing configuration on its new event, after
/// its areas (whose thresholds may imply the transmit-results rule), logged
/// for `importer` (else the system).
#[instrument(skip(hasura_transaction, bundle, importer), err)]
pub async fn import_bundle_signing(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    bundle: &ImportElectionEventSchema,
    importer: Option<&ElectoralLogAdminContext>,
) -> Result<()> {
    import_signing_configuration(
        hasura_transaction,
        tenant_id,
        election_event_id,
        bundle.signing_rules.as_deref(),
        bundle.signing_checks.as_ref(),
        &bundle.areas,
        &importer_actor(importer),
    )
    .await
}
