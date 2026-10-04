// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! What the protected actions the existing routes start sign, and when they
//! wait: with the rule off every route runs as today; with it on the gate
//! refuses what can't happen now (a change the Post doesn't allow, reopening
//! voting closed under signatures, initializing without a publication, an
//! ambiguous Post)
//! and otherwise starts the request for what it will do. A new publication,
//! another registry match or a rejection cancels what waits. The effects
//! check what was signed before they act. Scheduled dates leave opening and
//! closing that need signatures to people.
//!
//! Each test commits its own tenant. Labels and numbers come from the two
//! configurations each test runs under.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;
#[path = "support/signing_actions.rs"]
mod signing_actions;

use async_trait::async_trait;
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::signing::{CancelReason, RequesterSigning, SigningAction, SigningRequestStatus};
use sequent_core::signing::{SigningRequirement, SigningRule};
use sequent_core::types::permissions::Permissions;
use serde_json::json;
use signing::*;
use signing_actions::*;
use std::sync::Mutex;
use uuid::Uuid;
use windmill::postgres::signing::{get_signing_rule, upsert_signing_rule};
use windmill::services::scheduled_outcome::rule_snapshot;
use windmill::services::signing::actions::configuration::{
    cancel_for_new_publication, gate_publication, publication_digest, publish, NO_CHANGES,
};
use windmill::services::signing::actions::initialize::{create_report_tally, gate_tally_creation};
use windmill::services::signing::actions::voter::{
    approve, cancel_for_rejection, gate_voter_approval, VoterApprover,
};
use windmill::services::signing::actions::voting::scheduled_change_needs_signatures;
use windmill::services::signing::actions::{EffectProgress, EffectRefused};
use windmill::services::signing::guard::GuardOutcome;
use windmill::services::signing::log::{stage, LogScope, LogStep, SystemOutcome};
use windmill::services::signing::{InvalidReason, SigningCaller, SigningError, SigningResult};

/// Two configurations: a Post label and how many sign.
const PRESETS: [(&str, u16); 2] = [("madrid-pe", 2), ("faculty-of-science", 3)];

fn invalid<T: std::fmt::Debug>(result: SigningResult<T>) -> InvalidReason {
    match result {
        Err(SigningError::Invalid { reason, .. }) => reason,
        other => panic!("expected an invalid input, got {other:?}"),
    }
}

fn refused_code(result: anyhow::Result<serde_json::Value>) -> String {
    match result {
        Err(error) => error
            .downcast_ref::<EffectRefused>()
            .unwrap_or_else(|| panic!("expected a refusal, got {error:#}"))
            .code
            .clone(),
        Ok(value) => panic!("expected a refusal, got {value}"),
    }
}

async fn request_count(w: &World) -> i64 {
    w.pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT count(*) FROM sequent_backend.signing_request
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&w.tenant, &w.event],
        )
        .await
        .unwrap()
        .get(0)
}

async fn tally_gate(
    w: &World,
    who: &SigningCaller,
    tally_type: &str,
    elections: Vec<String>,
    configuration: bool,
) -> SigningResult<GuardOutcome> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let outcome = gate_tally_creation(
        &tx,
        who,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &elections,
        tally_type,
        configuration,
        None,
    )
    .await;
    tx.commit().await.unwrap();
    outcome
}

async fn publication_gate(w: &World, who: &SigningCaller, id: &str) -> SigningResult<GuardOutcome> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let outcome = gate_publication(&tx, who, &w.tenant.to_string(), &w.event.to_string(), id).await;
    tx.commit().await.unwrap();
    outcome
}

#[tokio::test]
async fn with_the_rules_off_every_action_runs_as_today() {
    let w = world("rules-off").await;
    let who = starter(&w);
    for status in [VotingStatus::OPEN, VotingStatus::CLOSED] {
        let outcome = status_gate(&w, &who, status, Some(vec![VotingStatusChannel::ONLINE])).await;
        assert_eq!(outcome.unwrap(), GuardOutcome::Proceed);
    }
    let post = vec![w.post.to_string()];
    assert_eq!(
        tally_gate(&w, &who, "INITIALIZATION_REPORT", post, false)
            .await
            .unwrap(),
        GuardOutcome::Proceed
    );
    assert_eq!(
        publication_gate(&w, &who, &Uuid::new_v4().to_string())
            .await
            .unwrap(),
        GuardOutcome::Proceed
    );
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let voter = gate_voter_approval(
        &tx,
        &tx,
        &who,
        &w.tenant.to_string(),
        &w.event.to_string(),
        "not-a-uuid",
        "registry-user",
    )
    .await;
    assert_eq!(voter.unwrap(), GuardOutcome::Proceed);
    // Ids that are not UUIDs fail in the route as they always did.
    let malformed = gate_publication(&tx, &who, "tenant", "event", "publication").await;
    assert_eq!(malformed.unwrap(), GuardOutcome::Proceed);
    tx.commit().await.unwrap();
    assert_eq!(request_count(&w).await, 0);
}

#[tokio::test]
async fn opening_and_closing_sign_their_channels_and_each_one_s_status_before() {
    for (label, required) in PRESETS {
        let w = world(label).await;
        let who = starter(&w);
        w.rule(
            SigningAction::OpenVoting,
            required,
            RequesterSigning::NotAllowed,
            Some(60),
        )
        .await;
        w.rule(
            SigningAction::CloseVoting,
            required,
            RequesterSigning::NotAllowed,
            Some(60),
        )
        .await;
        w.execute(
            "UPDATE sequent_backend.election_event
             SET voting_channels = '{\"online\": true, \"kiosk\": true}' WHERE id = $1",
            &[&w.event],
        )
        .await;
        set_post_status(
            &w,
            json!({"voting_status": "NOT_STARTED", "kiosk_voting_status": "PAUSED"}),
        )
        .await;

        // Without channels, the event's enabled channels open.
        let open = waiting_id(status_gate(&w, &who, VotingStatus::OPEN, None).await);
        let request = w.request(open).await;
        assert_eq!(
            request.subject,
            json!({"channels": ["KIOSK", "ONLINE"], "from": ["KIOSK=PAUSED", "ONLINE=NOT_STARTED"]})
        );
        assert_eq!(request.election_id, Some(w.post));
        assert_eq!(request.required, i32::from(required));
        assert_eq!(request.permission_label.as_deref(), Some(label));
        // Asking again answers the same request.
        assert_eq!(
            waiting_id(status_gate(&w, &who, VotingStatus::OPEN, None).await),
            open
        );
        // Closing a channel that never started can't happen.
        assert_eq!(
            invalid(
                status_gate(
                    &w,
                    &who,
                    VotingStatus::CLOSED,
                    Some(vec![VotingStatusChannel::ONLINE])
                )
                .await
            ),
            InvalidReason::Transition
        );
        let close = waiting_id(
            status_gate(
                &w,
                &who,
                VotingStatus::CLOSED,
                Some(vec![VotingStatusChannel::KIOSK]),
            )
            .await,
        );
        assert_eq!(
            w.request(close).await.subject,
            json!({"channels": ["KIOSK"], "from": ["KIOSK=PAUSED"]})
        );
        // Pausing is not protected.
        assert_eq!(
            status_gate(&w, &who, VotingStatus::PAUSED, None)
                .await
                .unwrap(),
            GuardOutcome::Proceed
        );
        // Someone outside the Post can't start it.
        let outsider = caller(
            "outsider",
            &[Permissions::ELECTION_STATE_WRITE],
            &["other-post"],
        );
        assert!(matches!(
            status_gate(
                &w,
                &outsider,
                VotingStatus::OPEN,
                Some(vec![VotingStatusChannel::ONLINE])
            )
            .await,
            Err(SigningError::Forbidden(_))
        ));
    }
}

#[tokio::test]
async fn opening_waits_for_a_required_initialization_report() {
    let w = world("init-required").await;
    w.rule(
        SigningAction::OpenVoting,
        1,
        RequesterSigning::NotAllowed,
        Some(60),
    )
    .await;
    w.execute(
        "UPDATE sequent_backend.election
         SET presentation = presentation || '{\"initialization_report_policy\": \"required\"}'
         WHERE id = $1",
        &[&w.post],
    )
    .await;
    assert_eq!(
        invalid(
            status_gate(
                &w,
                &starter(&w),
                VotingStatus::OPEN,
                Some(vec![VotingStatusChannel::ONLINE])
            )
            .await
        ),
        InvalidReason::Transition
    );
}

#[tokio::test]
async fn while_closing_needs_signatures_voting_closed_under_signatures_is_not_reopened() {
    let w = world("closed-post").await;
    let who = starter(&w);
    set_post_status(&w, json!({"voting_status": "CLOSED"})).await;
    let reopen = || {
        status_gate(
            &w,
            &who,
            VotingStatus::OPEN,
            Some(vec![VotingStatusChannel::ONLINE]),
        )
    };
    // Opening needs no signatures and closing neither: reopening runs.
    assert_eq!(reopen().await.unwrap(), GuardOutcome::Proceed);
    w.rule(
        SigningAction::CloseVoting,
        2,
        RequesterSigning::NotAllowed,
        Some(60),
    )
    .await;
    match reopen().await {
        Err(SigningError::Invalid { reason, message }) => {
            assert_eq!(reason, InvalidReason::ClosedUnderSignatures);
            // The wire value the portal reads in `extensions.reason`.
            assert_eq!(
                serde_json::to_value(reason).unwrap(),
                json!("closed-under-signatures")
            );
            assert_eq!(
                message,
                "The ONLINE channel was closed under signatures: it can't be reopened through signing."
            );
        }
        other => panic!("expected an invalid input, got {other:?}"),
    }
    assert_eq!(request_count(&w).await, 0);
}

/// Inserts a ballot publication of the world's event with one style.
async fn publication(w: &World, published: bool, generated: bool, election: Option<Uuid>) -> Uuid {
    let id = Uuid::new_v4();
    publication_fixture_write(
        w,
        "INSERT INTO sequent_backend.ballot_publication
             (id, tenant_id, election_event_id, is_generated, election_ids, election_id,
              published_at, created_at)
         VALUES ($1, $2, $3, $4, ARRAY[$5::uuid], $6,
                 CASE WHEN $7 THEN now() - interval '1 hour' END,
                 now() - CASE WHEN $7 THEN interval '2 hours' ELSE interval '0' END)",
        &[
            &id, &w.tenant, &w.event, &generated, &w.post, &election, &published,
        ],
    )
    .await;
    publication_fixture_write(
        w,
        "INSERT INTO sequent_backend.ballot_style
             (id, tenant_id, election_event_id, election_id, area_id, ballot_publication_id, status)
         VALUES ($1, $2, $3, $4, $5, $6, 'generated')",
        &[&Uuid::new_v4(), &w.tenant, &w.event, &w.post, &w.area, &id],
    )
    .await;
    id
}

#[tokio::test]
async fn initializing_a_post_signs_its_published_publication() {
    let w = world("init-post").await;
    let who = caller("sbei-chair", &[Permissions::ADMIN_CEREMONY], &[&w.label]);
    w.rule(
        SigningAction::InitializeVoting,
        2,
        RequesterSigning::Allowed,
        Some(60),
    )
    .await;
    let post = vec![w.post.to_string()];
    assert_eq!(
        invalid(tally_gate(&w, &who, "INITIALIZATION_REPORT", post.clone(), false).await),
        InvalidReason::NoPublication
    );
    let published = publication(&w, true, true, None).await;
    let id = waiting_id(tally_gate(&w, &who, "INITIALIZATION_REPORT", post.clone(), false).await);
    assert_eq!(
        w.request(id).await.subject,
        json!({ "publication_id": published.to_string() })
    );
    assert_eq!(
        tally_gate(&w, &who, "ELECTORAL_RESULTS", post.clone(), false)
            .await
            .unwrap(),
        GuardOutcome::Proceed
    );
    assert_eq!(
        invalid(
            tally_gate(
                &w,
                &who,
                "INITIALIZATION_REPORT",
                vec![w.post.to_string(), Uuid::new_v4().to_string()],
                false
            )
            .await
        ),
        InvalidReason::Input
    );
    assert_eq!(
        invalid(tally_gate(&w, &who, "INITIALIZATION_REPORT", post, true).await),
        InvalidReason::Input
    );
}

#[tokio::test]
async fn a_configuration_version_signs_what_publishing_writes() {
    for (label, required) in PRESETS {
        let w = world(label).await;
        let who = caller("configuration-manager", &[Permissions::PUBLISH_WRITE], &[]);
        w.rule(
            SigningAction::ApproveConfiguration,
            required,
            RequesterSigning::Allowed,
            None,
        )
        .await;
        publication(&w, true, true, None).await;
        // A published publication soft-deleted later still counts as a version.
        let deleted = publication(&w, true, true, None).await;
        w.execute(
            "UPDATE sequent_backend.ballot_publication SET deleted_at = now() WHERE id = $1",
            &[&deleted],
        )
        .await;
        let current = publication(&w, false, true, None).await;
        // The event is locked down: that doesn't change what publishing needs.
        w.execute(
            "UPDATE sequent_backend.election_event
             SET presentation = '{\"locked_down\": \"LOCKED_DOWN\"}' WHERE id = $1 AND set_config('sequent.trusted_write', 'on', true) = 'on'",
            &[&w.event],
        )
        .await;

        let id = waiting_id(publication_gate(&w, &who, &current.to_string()).await);
        let request = w.request(id).await;
        assert_eq!(request.election_id, None);
        // Two versions were published (one deleted later): this publishes the third.
        assert_eq!(request.config_revision.as_deref(), Some("3"));
        let mut client = w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let digest = publication_digest(&tx, w.tenant, w.event, current)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(
            request.subject,
            json!({
                "ballot_publication_id": current.to_string(),
                "digest": digest,
                // Saved without a change log: what it needs now only.
                "signing_rules": [format!("approve-configuration={required}")],
                "scheduled_events": 0,
                "ballots_and_contests": NO_CHANGES,
                // Nothing configured: the default policies and rules, no schedule.
                "policies": {"initialization_scope": "post", "unsigned_scheduled_close": "refuse"},
                "open_voting": rule_snapshot(&SigningRule::default_for(SigningAction::OpenVoting)),
                "close_voting": rule_snapshot(&SigningRule::default_for(SigningAction::CloseVoting)),
                "schedule": [],
                // The Post enables the default channel.
                "post_channels": { w.post.to_string(): ["ONLINE"] },
                "initialization_report_policies": { w.post.to_string(): "not-required" },
                // The fixture publication contains one country for this Post.
                "initialization_countries": { w.post.to_string(): [w.area.to_string()] },
            })
        );

        // What publishing writes changes: the effect refuses before acting.
        w.execute(
            "UPDATE sequent_backend.ballot_style SET status = 'regenerated'
             WHERE ballot_publication_id = $1",
            &[&current],
        )
        .await;
        let tx = client.transaction().await.unwrap();
        let progress = EffectProgress::default();
        assert_eq!(
            refused_code(publish(&tx, &request, &progress).await),
            "payload-changed"
        );
        assert!(!progress.reached_outside());
        tx.rollback().await.unwrap();

        // An unpublishable publication can't wait for signatures.
        let draft = publication(&w, false, false, None).await;
        assert_eq!(
            invalid(publication_gate(&w, &who, &draft.to_string()).await),
            InvalidReason::Transition
        );
    }
}

/// Saves `action`'s rule (`None`: off) and logs the change as the rule
/// drawer's save does: the rule before and after.
async fn save_rule(w: &World, action: SigningAction, signatures: Option<u16>) {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let current = get_signing_rule(&tx, w.tenant, w.event, action)
        .await
        .unwrap();
    let old = current
        .as_ref()
        .map(|row| row.rule.clone())
        .unwrap_or_else(|| SigningRule::default_for(action));
    let saved = upsert_signing_rule(
        &tx,
        w.tenant,
        w.event,
        &SigningRule {
            action,
            requirement: match signatures {
                Some(_) => SigningRequirement::Required,
                None => SigningRequirement::NotRequired,
            },
            signatures: signatures.unwrap_or(1),
            requester_signing: RequesterSigning::Allowed,
            expires_minutes: None,
            revision: 0,
        },
        current.map_or(0, |row| row.rule.revision),
        "configuration-manager",
        Some("Configuration Manager"),
    )
    .await
    .unwrap()
    .unwrap();
    stage(
        &tx,
        &LogStep {
            kind: SigningStatementKind::SigningRuleChanged,
            user: caller("configuration-manager", &[], &[]).actor(),
            system: SystemOutcome::Info,
            scope: LogScope {
                tenant_id: w.tenant,
                election_event_id: w.event,
                election_id: None,
                area_id: None,
            },
            description: format!("Changed the signing rule of {action}"),
            details: json!({"action": action.to_string(), "old": old, "new": saved.rule}),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

/// A configuration version signs each signing rule saved since the last
/// publication with what it needs now and what it needed then, from the
/// first change logged since (`ACTION=BEFORE>AFTER`, `off` or the
/// signatures); the version it publishes is the next one, or, for a Post's
/// publication, the one in force.
#[tokio::test]
async fn a_configuration_version_signs_each_rule_before_and_after() {
    for (label, required) in PRESETS {
        let w = world(label).await;
        let who = caller("configuration-manager", &[Permissions::PUBLISH_WRITE], &[]);
        // Changed before the last publication: not part of this version.
        save_rule(&w, SigningAction::OpenVoting, Some(5)).await;
        w.execute(
            "UPDATE sequent_backend.signing_log_outbox SET occurred_at = now() - interval '2 hours'
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&w.tenant, &w.event],
        )
        .await;
        publication(&w, true, true, None).await;
        // Since then.
        save_rule(&w, SigningAction::ApproveConfiguration, Some(required)).await;
        save_rule(&w, SigningAction::CloseVoting, Some(1)).await;
        save_rule(&w, SigningAction::CloseVoting, Some(required)).await;
        save_rule(&w, SigningAction::OpenVoting, Some(2)).await;
        save_rule(&w, SigningAction::OpenVoting, None).await;
        // Saved without a change log (e.g. imported).
        w.rule(
            SigningAction::TransmitResults,
            2,
            RequesterSigning::Allowed,
            None,
        )
        .await;

        let current = publication(&w, false, true, None).await;
        let request = w
            .request(waiting_id(
                publication_gate(&w, &who, &current.to_string()).await,
            ))
            .await;
        assert_eq!(request.config_revision.as_deref(), Some("2"));
        let mut rules: Vec<String> =
            serde_json::from_value(request.subject["signing_rules"].clone()).unwrap();
        rules.sort();
        assert_eq!(
            rules,
            [
                format!("approve-configuration=off>{required}"),
                format!("close-voting=off>{required}"),
                "open-voting=5>off".to_string(),
                "transmit-results=2".to_string(),
            ]
        );

        let post_level = publication(&w, false, true, Some(w.post)).await;
        let request = w
            .request(waiting_id(
                publication_gate(&w, &who, &post_level.to_string()).await,
            ))
            .await;
        assert_eq!(request.config_revision.as_deref(), Some("1"));
    }
}

#[tokio::test]
async fn a_new_publication_cancels_what_it_supersedes() {
    let w = world("supersede-post").await;
    let who = caller("configuration-manager", &[Permissions::PUBLISH_WRITE], &[]);
    w.rule(
        SigningAction::ApproveConfiguration,
        2,
        RequesterSigning::Allowed,
        None,
    )
    .await;
    let event_level = publication(&w, false, true, None).await;
    let post_level = publication(&w, false, true, Some(w.post)).await;
    let event_request = waiting_id(publication_gate(&w, &who, &event_level.to_string()).await);
    let post_request = waiting_id(publication_gate(&w, &who, &post_level.to_string()).await);

    let cancel = |election: Option<String>| {
        let (w, who) = (w.clone(), who.clone());
        async move {
            let mut client = w.pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            let cancelled = cancel_for_new_publication(
                &tx,
                &who,
                &w.tenant.to_string(),
                &w.event.to_string(),
                election.as_deref(),
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            cancelled
        }
    };
    // A Post's new publication cancels only its own.
    assert_eq!(cancel(Some(Uuid::new_v4().to_string())).await, 0);
    assert_eq!(cancel(Some(w.post.to_string())).await, 1);
    assert_eq!(
        w.request(post_request).await.cancel_reason,
        Some(CancelReason::PayloadChanged)
    );
    assert_eq!(
        w.request(event_request).await.status,
        SigningRequestStatus::Waiting
    );
    // An event-level one supersedes every one.
    let again = waiting_id(publication_gate(&w, &who, &post_level.to_string()).await);
    assert_eq!(cancel(None).await, 2);
    for id in [event_request, again] {
        assert_eq!(
            w.request(id).await.cancel_reason,
            Some(CancelReason::PayloadChanged)
        );
    }
}

/// Generating a new publication (of `election`, or of the event), as the
/// route does before it prepares it: how many waiting requests it cancelled.
async fn new_publication(w: &World, who: &SigningCaller, election: Option<Uuid>) -> usize {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let cancelled = cancel_for_new_publication(
        &tx,
        who,
        &w.tenant.to_string(),
        &w.event.to_string(),
        election.map(|id| id.to_string()).as_deref(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    cancelled
}

/// A new configuration version changes what initializing a Post would
/// create its report from, so the Post's waiting initialization is
/// cancelled (PayloadChanged) as it is generated, not refused later.
#[tokio::test]
async fn a_new_configuration_version_cancels_the_posts_waiting_initialization() {
    for (label, required) in PRESETS {
        let w = world(label).await;
        let who = caller("sbei-chair", &[Permissions::ADMIN_CEREMONY], &[&w.label]);
        w.rule(
            SigningAction::InitializeVoting,
            required,
            RequesterSigning::Allowed,
            Some(60),
        )
        .await;
        publication(&w, true, true, None).await;
        let post = vec![w.post.to_string()];
        let first =
            waiting_id(tally_gate(&w, &who, "INITIALIZATION_REPORT", post.clone(), false).await);

        // Another Post's new publication leaves it waiting.
        assert_eq!(new_publication(&w, &who, Some(Uuid::new_v4())).await, 0);
        assert_eq!(w.request(first).await.status, SigningRequestStatus::Waiting);

        // Its own Post's cancels it.
        assert_eq!(new_publication(&w, &who, Some(w.post)).await, 1);
        let cancelled = w.request(first).await;
        assert_eq!(cancelled.status, SigningRequestStatus::Cancelled);
        assert_eq!(cancelled.cancel_reason, Some(CancelReason::PayloadChanged));

        // An event-level one cancels every Post's.
        let again =
            waiting_id(tally_gate(&w, &who, "INITIALIZATION_REPORT", post.clone(), false).await);
        assert_ne!(again, first);
        assert_eq!(new_publication(&w, &who, None).await, 1);
        assert_eq!(
            w.request(again).await.cancel_reason,
            Some(CancelReason::PayloadChanged)
        );
        let cancels = w.entries("SigningRequestCancelled").await;
        assert_eq!(cancels.len(), 2, "{cancels:?}");
    }
}

/// The initialization runs only for the publication it signed: once the
/// Post's published publication is another one, or none, it refuses
/// before it acts.
#[tokio::test]
async fn initializing_refuses_once_the_posts_publication_changed() {
    let w = world("init-changed").await;
    let who = caller("sbei-chair", &[Permissions::ADMIN_CEREMONY], &[&w.label]);
    w.rule(
        SigningAction::InitializeVoting,
        2,
        RequesterSigning::Allowed,
        Some(60),
    )
    .await;
    let signed = publication(&w, true, true, None).await;
    publication_fixture_write(
        &w,
        "UPDATE sequent_backend.ballot_publication
         SET published_at = now() - interval '3 hours' WHERE id = $1",
        &[&signed],
    )
    .await;
    let id = waiting_id(
        tally_gate(
            &w,
            &who,
            "INITIALIZATION_REPORT",
            vec![w.post.to_string()],
            false,
        )
        .await,
    );
    let request = w.request(id).await;
    let run = |request: windmill::postgres::signing::SigningRequestRow| {
        let w = w.clone();
        async move {
            let mut client = w.pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            let progress = EffectProgress::default();
            let outcome = create_report_tally(&tx, &request, &progress).await;
            tx.rollback().await.unwrap();
            (outcome, progress.reached_outside())
        }
    };

    // Control: with the signed publication it goes on to create the tally
    // (which this world can't), past the publication check.
    let (outcome, _) = run(request.clone()).await;
    if let Err(error) = &outcome {
        assert!(
            error
                .downcast_ref::<EffectRefused>()
                .map_or(true, |refused| refused.code != "publication-changed"),
            "{error:#}"
        );
    }

    // A newer publication of the Post was published meanwhile.
    let newer = publication(&w, true, true, None).await;
    let (outcome, reached) = run(request.clone()).await;
    assert_eq!(refused_code(outcome), "publication-changed");
    assert!(!reached);

    // Or the Post has none any more.
    w.execute(
        "UPDATE sequent_backend.ballot_publication SET deleted_at = now()
         WHERE id = ANY($1)",
        &[&vec![signed, newer]],
    )
    .await;
    let (outcome, reached) = run(request).await;
    assert_eq!(refused_code(outcome), "publication-changed");
    assert!(!reached);
}

/// An application of the world's area, which takes part in the Post
/// through a contest, and the registry record's Keycloak rows.
async fn application(w: &World, tx: &Transaction<'_>) -> Uuid {
    let contest = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
         VALUES ($1, $2, $3, $4)",
        &[&contest, &w.tenant, &w.event, &w.post],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.area_contest (id, tenant_id, election_event_id, area_id, contest_id)
         VALUES ($1, $2, $3, $4, $5)",
        &[&Uuid::new_v4(), &w.tenant, &w.event, &w.area, &contest],
    )
    .await
    .unwrap();
    let id = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.applications
             (id, tenant_id, election_event_id, area_id, applicant_id, status, verification_type,
              applicant_data, annotations, created_at, permission_label)
         VALUES ($1, $2, $3, $4, 'applicant', 'PENDING', 'MANUAL', '{}',
                 '{\"manual_verify_reason\": \"Passport renewed after the registry record\"}',
                 '2028-03-02T10:00:00Z', $5)",
        &[&id, &w.tenant, &w.event, &w.area, &w.label],
    )
    .await
    .unwrap();
    keycloak_tables(tx).await;
    let realm = format!("tenant-{}-event-{}", w.tenant, w.event);
    tx.execute(
        "INSERT INTO realm (id, name) VALUES ('realm', $1)",
        &[&realm],
    )
    .await
    .unwrap();
    for (user, first, last) in [("ramon", "Ramon", "Garcia"), ("other", "Rosa", "Garcia")] {
        tx.execute(
            "INSERT INTO user_entity (id, username, first_name, last_name, enabled, realm_id)
             VALUES ($1, $2, $3, $4, true, 'realm')",
            &[&user, &format!("{user}-name"), &first, &last],
        )
        .await
        .unwrap();
    }
    id
}

async fn voter_gate(
    w: &World,
    tx: &Transaction<'_>,
    who: &SigningCaller,
    application: Uuid,
    registry: &str,
) -> SigningResult<GuardOutcome> {
    gate_voter_approval(
        tx,
        tx,
        who,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &application.to_string(),
        registry,
    )
    .await
}

#[tokio::test]
async fn approving_a_voter_signs_the_application_and_rejecting_it_cancels() {
    for (label, required) in PRESETS {
        let w = world(label).await;
        let who = caller("ofov", &[Permissions::APPLICATION_WRITE], &[&w.label]);
        w.rule(
            SigningAction::ApproveVoter,
            required,
            RequesterSigning::NotAllowed,
            Some(60),
        )
        .await;
        let mut client = w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let application = application(&w, &tx).await;
        let first = waiting_id(voter_gate(&w, &tx, &who, application, "ramon").await);
        // Matching the application to another registry record replaces it.
        let second = waiting_id(voter_gate(&w, &tx, &who, application, "other").await);
        tx.commit().await.unwrap();

        assert_ne!(first, second);
        let first = w.request(first).await;
        assert_eq!(first.cancel_reason, Some(CancelReason::PayloadChanged));
        assert_eq!(
            first.subject,
            json!({
                "application_id": application.to_string(),
                "applicant_registry_id": "ramon",
                "decision": "approve",
                "status": "PENDING",
                "submitted_at": "2028-03-02",
                "reason": "Passport renewed after the registry record",
                "registry_record": "Garcia, Ramon (ramon-name)",
            })
        );
        let second_row = w.request(second).await;
        assert_eq!(second_row.election_id, Some(w.post));
        assert_eq!(second_row.area_id, Some(w.area));
        assert_eq!(second_row.required, i32::from(required));

        // Rejecting the application cancels the approval waiting for it.
        let tx = client.transaction().await.unwrap();
        let cancelled = cancel_for_rejection(
            &tx,
            &who,
            &w.tenant.to_string(),
            &w.event.to_string(),
            &application.to_string(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(cancelled, 1);
        assert_eq!(
            w.request(second).await.cancel_reason,
            Some(CancelReason::PayloadChanged)
        );
    }
}

#[tokio::test]
async fn an_area_voting_in_two_posts_needs_the_application_s_label() {
    let w = world("ambiguous-post").await;
    let who = caller("ofov", &[Permissions::APPLICATION_WRITE], &[&w.label]);
    w.rule(
        SigningAction::ApproveVoter,
        1,
        RequesterSigning::NotAllowed,
        Some(60),
    )
    .await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let application = application(&w, &tx).await;
    let other_post = Uuid::new_v4();
    let contest = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, permission_label)
         VALUES ($1, $2, $3, 'another-label')",
        &[&other_post, &w.tenant, &w.event],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
         VALUES ($1, $2, $3, $4)",
        &[&contest, &w.tenant, &w.event, &other_post],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.area_contest (id, tenant_id, election_event_id, area_id, contest_id)
         VALUES ($1, $2, $3, $4, $5)",
        &[&Uuid::new_v4(), &w.tenant, &w.event, &w.area, &contest],
    )
    .await
    .unwrap();
    // The application's label names one of the two.
    let id = waiting_id(voter_gate(&w, &tx, &who, application, "ramon").await);
    assert_eq!(w_request_post(&tx, id).await, Some(w.post));
    tx.execute(
        "UPDATE sequent_backend.applications SET permission_label = NULL WHERE id = $1",
        &[&application],
    )
    .await
    .unwrap();
    assert_eq!(
        invalid(voter_gate(&w, &tx, &who, application, "ramon").await),
        InvalidReason::AmbiguousPost
    );
    tx.rollback().await.unwrap();
}

async fn w_request_post(tx: &Transaction<'_>, id: Uuid) -> Option<Uuid> {
    tx.query_one(
        "SELECT election_id FROM sequent_backend.signing_request WHERE id = $1",
        &[&id],
    )
    .await
    .unwrap()
    .get(0)
}

/// Keycloak and the messages, as a fake: whether the registry record is
/// verified, and the confirmations it made.
struct FakeVoters {
    verified: bool,
    confirmed: Mutex<Vec<(String, String, String)>>,
}

#[async_trait]
impl VoterApprover for FakeVoters {
    async fn is_verified(&self, _realm: &str, _user_id: &str) -> anyhow::Result<bool> {
        Ok(self.verified)
    }

    async fn group_names(
        &self,
        _tenant_realm: &str,
        _user_id: &str,
    ) -> anyhow::Result<Vec<String>> {
        Ok(vec!["ofov".into()])
    }

    async fn confirm(
        &self,
        _tx: &Transaction<'_>,
        application_id: &str,
        _tenant_id: &str,
        _election_event_id: &str,
        registry_user_id: &str,
        _admin_id: &str,
        admin_name: &str,
        _group_names: &[String],
    ) -> anyhow::Result<()> {
        self.confirmed.lock().unwrap().push((
            application_id.to_owned(),
            registry_user_id.to_owned(),
            admin_name.to_owned(),
        ));
        Ok(())
    }
}

#[tokio::test]
async fn the_approval_checks_the_application_before_it_confirms_it() {
    let w = world("approval-effect").await;
    let who = caller("ofov", &[Permissions::APPLICATION_WRITE], &[&w.label]);
    w.rule(
        SigningAction::ApproveVoter,
        1,
        RequesterSigning::NotAllowed,
        Some(60),
    )
    .await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let application = application(&w, &tx).await;
    let id = waiting_id(voter_gate(&w, &tx, &who, application, "ramon").await);
    tx.commit().await.unwrap();
    let request = w.request(id).await;
    let voters = |verified| FakeVoters {
        verified,
        confirmed: Mutex::new(vec![]),
    };

    // Already a verified voter: refused before acting.
    let tx = client.transaction().await.unwrap();
    let verified = voters(true);
    let progress = EffectProgress::default();
    assert_eq!(
        refused_code(approve(&tx, &request, &[], &verified, &progress).await),
        "already-verified"
    );
    assert!(verified.confirmed.lock().unwrap().is_empty());
    tx.rollback().await.unwrap();

    // Confirmed by the person who started it; the signers on the application.
    let tx = client.transaction().await.unwrap();
    let fresh = voters(false);
    let progress = EffectProgress::default();
    approve(&tx, &request, &[], &fresh, &progress)
        .await
        .unwrap();
    assert!(progress.reached_outside());
    assert_eq!(
        *fresh.confirmed.lock().unwrap(),
        vec![(
            application.to_string(),
            "ramon".to_owned(),
            "ofov display".to_owned()
        )]
    );
    let annotations: serde_json::Value = tx
        .query_one(
            "SELECT annotations FROM sequent_backend.applications WHERE id = $1",
            &[&application],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(annotations["signing_request_id"], json!(id));
    assert_eq!(annotations["signed_by"], json!([]));

    // The application changed meanwhile: refused.
    tx.execute(
        "UPDATE sequent_backend.applications SET status = 'REJECTED' WHERE id = $1",
        &[&application],
    )
    .await
    .unwrap();
    let progress = EffectProgress::default();
    assert_eq!(
        refused_code(approve(&tx, &request, &[], &voters(false), &progress).await),
        "application-changed"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn scheduled_dates_leave_protected_changes_to_people() {
    let w = world("scheduled-post").await;
    let scheduled = |status: VotingStatus| {
        let w = w.clone();
        async move {
            let mut client = w.pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            let skipped = scheduled_change_needs_signatures(
                &tx,
                &w.tenant.to_string(),
                &w.event.to_string(),
                Some(&w.post.to_string()),
                &status,
                "scheduled-1",
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            skipped
        }
    };
    assert!(!scheduled(VotingStatus::OPEN).await);
    w.rule(
        SigningAction::OpenVoting,
        2,
        RequesterSigning::NotAllowed,
        Some(60),
    )
    .await;
    assert!(scheduled(VotingStatus::OPEN).await);
    assert!(!scheduled(VotingStatus::CLOSED).await);
    assert!(!scheduled(VotingStatus::PAUSED).await);
    let rows = w.outbox().await;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].4.as_deref(), Some("scheduled-event"));
    let (_, kind, event_type, log_type, _, description) = &rows[1];
    assert_eq!(kind, "SigningActionExecuted");
    assert_eq!(event_type, "SYSTEM");
    assert_eq!(log_type, "ERROR");
    assert!(description.contains("open voting"), "{description}");
}
