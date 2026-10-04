// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Scheduled openings and closings under signing (VOTE-LIFECYCLE §5a–§5c):
//! a row that a signed configuration covers runs, authorized by it;
//! editing it drops the coverage; an uncovered opening is refused and an
//! uncovered close follows the policy of both copies. Policies and rules
//! are signed configuration: a loosening waits for the next approved
//! publication, a tightening applies at once, and before anything is
//! published the defaults apply. What the list predicts is what the
//! scheduler does, and a write that changes a prediction logs it once.
//!
//! Each test commits its own tenant, under the two configurations the
//! close policy has: the product default (REFUSE, the Madrid association)
//! and the preset that closes at the common deadline (RUN_AS_SYSTEM).

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;
#[path = "support/signing_actions.rs"]
mod signing_actions;

use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{
    InitializationScope, LifecyclePolicies, UnsignedScheduledClosePolicy, VotingStatus,
    VotingStatusChannel,
};
use sequent_core::signing::{
    CancelReason, RequesterSigning, SigningAction, SigningRequestStatus, SigningRequirement,
};
use sequent_core::types::permissions::Permissions;
use sequent_core::types::scheduled_outcome::{CheckId, ScheduledOutcomeKind};
use serde_json::{json, Value};
use signing::*;
use signing_actions::*;
use std::sync::Arc;
use uuid::Uuid;
use windmill::postgres::signing::{SigningApprovalRow, SigningRequestRow};
use windmill::services::scheduled_outcome::{
    event_wide_targets, keep_publication_lifecycle, lifecycle_snapshots, recompute_predictions,
    save_lifecycle_policies, scheduled_outcomes, snapshot_publication, write_publication_snapshot,
    ChangeApplies, LockedDown, PublicationLifecycle, ScheduledOutcome, FIRED_OUTCOME,
};
use windmill::services::signing::actions::configuration::{gate_publication, ConfigurationSubject};
use windmill::services::signing::actions::voting::{
    enforce_signed_closes, event_wide_decision, scheduled_change_for_posts,
    scheduled_change_needs_signatures, PostCloser,
};
use windmill::services::signing::actions::{
    executors, run_dispatched, subject_of, EffectProgress, NoSeal, RunOutcome, SignedActionEffects,
};
use windmill::services::signing::guard::GuardOutcome;
use windmill::services::signing::log::Actor;
use windmill::services::signing::rules::{save_rule, SaveRuleInput};

use UnsignedScheduledClosePolicy::{REFUSE, RUN_AS_SYSTEM};

/// The two configurations of the close policy.
const POLICIES: [UnsignedScheduledClosePolicy; 2] = [REFUSE, RUN_AS_SYSTEM];

const OPENS: &str = "START_VOTING_PERIOD";
const CLOSES: &str = "END_VOTING_PERIOD";

fn actor(user: &str) -> Actor {
    Actor {
        user_id: user.to_owned(),
        username: user.to_owned(),
    }
}

/// A rule of `action` that needs `signatures`, committed.
async fn require(w: &World, action: SigningAction, signatures: u16) {
    w.rule(action, signatures, RequesterSigning::Allowed, None)
        .await;
}

/// Sets the event's current lifecycle policies.
async fn set_policy(w: &World, close: UnsignedScheduledClosePolicy) {
    let policies = LifecyclePolicies {
        initialization_scope: InitializationScope::POST,
        unsigned_scheduled_close: close,
    };
    w.execute(
        "UPDATE sequent_backend.election_event
         SET presentation = jsonb_set(COALESCE(presentation, '{}'::jsonb),
             '{lifecycle_policies}', $2::jsonb, true)
         WHERE id = $1",
        &[&w.event, &serde_json::to_value(policies).unwrap()],
    )
    .await;
}

fn cron(local: &str) -> Value {
    json!({
        "scheduled_date": format!("{local}:00+04:00"),
        "local": local,
        "timezone": "Asia/Dubai",
    })
}

/// A scheduled opening or closing of the Post (or event-wide), committed.
async fn add_row(w: &World, processor: &str, election: Option<Uuid>, local: &str) -> Uuid {
    let id = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.scheduled_event
             (id, tenant_id, election_event_id, event_processor, cron_config, event_payload,
              created_by)
         VALUES ($1, $2, $3, $4, $5, $6, 'ofov.admin')",
        &[
            &id,
            &w.tenant,
            &w.event,
            &processor,
            &cron(local),
            &json!({ "election_id": election.map(|id| id.to_string()) }),
        ],
    )
    .await;
    id
}

async fn edit_row(w: &World, id: Uuid, local: &str) {
    w.execute(
        "UPDATE sequent_backend.scheduled_event SET cron_config = $2 WHERE id = $1",
        &[&id, &cron(local)],
    )
    .await;
}

/// Edits a row as the save path does: the edit and its recompute in one
/// transaction, by `user`.
async fn save_row(w: &World, id: Uuid, local: &str, user: &str) {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.scheduled_event SET cron_config = $2 WHERE id = $1",
        &[&id, &cron(local)],
    )
    .await
    .unwrap();
    recompute_predictions(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &actor(user),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

/// Creates a row as the save path does, with its recompute.
async fn create_row(
    w: &World,
    processor: &str,
    election: Option<Uuid>,
    local: &str,
    user: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.scheduled_event
             (id, tenant_id, election_event_id, event_processor, cron_config, event_payload,
              created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        &[
            &id,
            &w.tenant,
            &w.event,
            &processor,
            &cron(local),
            &json!({ "election_id": election.map(|id| id.to_string()) }),
            &user,
        ],
    )
    .await
    .unwrap();
    recompute_predictions(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &actor(user),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    id
}

/// Recomputes the predictions as `user`, committed.
async fn recompute(w: &World, user: &str) {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    recompute_predictions(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &actor(user),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

async fn publication_attack_refused(
    world: &World,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) {
    let mut client = world.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let error = tx
        .execute(sql, params)
        .await
        .expect_err("raw publication authority change must be refused");
    assert_eq!(
        error.code(),
        Some(&tokio_postgres::error::SqlState::INSUFFICIENT_PRIVILEGE)
    );
    tx.rollback().await.unwrap();
}

/// A generated publication of the event (or the Post) with one style.
async fn publication(w: &World, election: Option<Uuid>) -> Uuid {
    let id = Uuid::new_v4();
    publication_fixture_write(
        w,
        "INSERT INTO sequent_backend.ballot_publication
             (id, tenant_id, election_event_id, is_generated, election_ids, election_id, created_at)
         VALUES ($1, $2, $3, true, ARRAY[$4::uuid], $5, now())",
        &[&id, &w.tenant, &w.event, &w.post, &election],
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

/// Publishes without signatures as the publish route does: the publication
/// is published with its snapshot, then the predictions are recomputed.
async fn publish(w: &World, election: Option<Uuid>) -> Uuid {
    let id = publication(w, election).await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    windmill::postgres::trusted_write(&tx).await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.ballot_publication SET published_at = clock_timestamp()
         WHERE id = $1",
        &[&id],
    )
    .await
    .unwrap();
    snapshot_publication(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &id.to_string(),
        election.map(|id| id.to_string()).as_deref(),
    )
    .await
    .unwrap();
    recompute_predictions(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &actor("configuration-manager"),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    id
}

/// Publishes what an approval signed, as the configuration effect does:
/// the publication, with the signed lifecycle snapshot.
struct PublishingEffects;

#[async_trait]
impl SignedActionEffects for PublishingEffects {
    async fn run(
        &self,
        tx: &Transaction<'_>,
        request: &SigningRequestRow,
        _approvals: &[SigningApprovalRow],
        _progress: &EffectProgress,
    ) -> anyhow::Result<Value> {
        let signed: ConfigurationSubject = subject_of(request)?;
        let publication = Uuid::parse_str(&signed.ballot_publication_id)?;
        windmill::postgres::trusted_write(tx).await?;
        tx.execute(
            "UPDATE sequent_backend.ballot_publication SET published_at = clock_timestamp()
             WHERE id = $1",
            &[&publication],
        )
        .await?;
        let election: Option<Uuid> = tx
            .query_one(
                "SELECT election_id FROM sequent_backend.ballot_publication WHERE id = $1",
                &[&publication],
            )
            .await?
            .get(0);
        write_publication_snapshot(
            tx,
            request.tenant_id,
            request.election_event_id,
            publication,
            election,
            Some(request.id),
            &signed.lifecycle(),
        )
        .await?;
        Ok(json!({ "ballot_publication_id": signed.ballot_publication_id }))
    }
}

/// Publishes the event (or the Post) with a signed configuration approval
/// of `signers` people named `prefix-i`: its request id and code.
async fn approve(
    w: &World,
    election: Option<Uuid>,
    signers: usize,
    prefix: &str,
) -> (Uuid, String) {
    let publication = publication(w, election).await;
    let who = caller("configuration-manager", &[Permissions::PUBLISH_WRITE], &[]);
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let outcome = gate_publication(
        &tx,
        &who,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &publication.to_string(),
    )
    .await;
    tx.commit().await.unwrap();
    let id = match outcome.unwrap() {
        GuardOutcome::SigningRequired(summary) => summary.id,
        GuardOutcome::Proceed => panic!("approving the configuration needs signatures"),
    };
    let dispatched = Dispatched::default();
    sign_all(
        w,
        SigningAction::ApproveConfiguration,
        id,
        signers,
        prefix,
        &dispatched,
    )
    .await;
    let tasks = dispatched.tasks();
    assert_eq!(tasks.len(), 1);
    match run_dispatched(&mut client, &PublishingEffects, &NoSeal, &tasks[0])
        .await
        .unwrap()
    {
        RunOutcome::Executed(_) => {}
        other => panic!("expected the approval to run, got {other:?}"),
    }
    let request = w.request(id).await;
    assert_eq!(request.status, SigningRequestStatus::Executed);
    (id, request.code)
}

/// Fires a row at the Post as the scheduled task does: whether it is left
/// to people.
async fn fire(w: &World, row: Uuid, status: VotingStatus) -> bool {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let refused = scheduled_change_needs_signatures(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&w.post.to_string()),
        &status,
        &row.to_string(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    refused
}

async fn outcomes(w: &World) -> Vec<ScheduledOutcome> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let outcomes = scheduled_outcomes(&tx, w.tenant, w.event).await.unwrap();
    tx.rollback().await.unwrap();
    outcomes
}

async fn outcome_of(w: &World, row: Uuid) -> ScheduledOutcome {
    outcomes(w)
        .await
        .into_iter()
        .find(|outcome| outcome.scheduled_event_id == row.to_string())
        .expect("an outcome of the row")
}

/// The row's fire-time explanation, as it keeps it.
async fn fired_outcome(w: &World, row: Uuid) -> Value {
    let annotations: Value = w
        .pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT annotations FROM sequent_backend.scheduled_event WHERE id = $1",
            &[&row],
        )
        .await
        .unwrap()
        .get(0);
    annotations[FIRED_OUTCOME]["posts"][0].clone()
}

/// The (user, log type, description, details) of the entries of `kind`
/// about scheduled events: the USER entry's user with the SYSTEM entry's
/// log type, one per step, in order.
async fn entries(w: &World, kind: &str) -> Vec<(String, String, String, Value)> {
    w.pool
        .get()
        .await
        .unwrap()
        .query(
            "SELECT u.user_id, s.log_type, s.body->>'description', s.body->'details'
             FROM sequent_backend.signing_log_outbox s
             JOIN sequent_backend.signing_log_outbox u
                 ON u.step_id = s.step_id AND u.event_type = 'USER'
             WHERE s.election_event_id = $1 AND s.statement_kind = $2
                 AND s.event_type = 'SYSTEM'
                 AND s.body->'details' ? 'scheduled_event_id'
             ORDER BY s.id",
            &[&w.event, &kind],
        )
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3)))
        .collect()
}

async fn changes(w: &World) -> Vec<(String, String, String, Value)> {
    entries(w, "ScheduledOutcomeChanged").await
}

/// A waiting close request of the Post with one of its signatures.
async fn waiting_close_with_one_signature(w: &World) -> Uuid {
    set_post_status(
        w,
        json!({"voting_status": "OPEN", "kiosk_voting_status": "OPEN"}),
    )
    .await;
    let id = waiting_id(
        status_gate(
            w,
            &starter(w),
            VotingStatus::CLOSED,
            Some(vec![VotingStatusChannel::ONLINE]),
        )
        .await,
    );
    let signers = signers(w, SigningAction::CloseVoting, 1, "signer-early").await;
    let services = services(
        FakeVerifier::knowing(&signers.iter().map(|(_, c)| c).collect::<Vec<_>>()),
        executors(Arc::new(Dispatched::default())),
    );
    w.sign(&services, &signers[0].0, id, &signers[0].1, at(1))
        .await
        .unwrap();
    id
}

fn kind(outcome: &ScheduledOutcome) -> ScheduledOutcomeKind {
    outcome.explanation.outcome.clone()
}

#[tokio::test]
async fn covered_openings_and_closings_run_authorized_by_the_signed_configuration() {
    for (label, policy) in [("madrid-pe", REFUSE), ("dubai-pcg", RUN_AS_SYSTEM)] {
        let w = world(label).await;
        set_policy(&w, policy).await;
        require(&w, SigningAction::OpenVoting, 2).await;
        require(&w, SigningAction::CloseVoting, 2).await;
        require(&w, SigningAction::ApproveConfiguration, 2).await;
        let opens = add_row(&w, OPENS, Some(w.post), "2028-04-09T00:00").await;
        let closes = add_row(&w, CLOSES, None, "2028-05-08T19:00").await;
        let (request, code) = approve(&w, None, 2, "officer").await;
        let signers: Vec<String> = w
            .approvals(request)
            .await
            .into_iter()
            .map(|approval| approval.display_name.unwrap_or(approval.username))
            .collect();
        assert_eq!(signers.len(), 2);

        for row in [opens, closes] {
            let predicted = outcome_of(&w, row).await;
            assert_eq!(kind(&predicted), ScheduledOutcomeKind::Runs);
            assert_eq!(predicted.explanation.deciding, CheckId::Covered);
            let by = predicted.explanation.authorized_by.clone().unwrap();
            assert_eq!(by.code, code);
            assert_eq!(by.signers, signers);
        }

        // A close request in progress: the scheduled close cancels it and
        // keeps its partial signature in the history.
        assert!(!fire(&w, opens, VotingStatus::OPEN).await);
        let waiting = waiting_close_with_one_signature(&w).await;
        assert!(!fire(&w, closes, VotingStatus::CLOSED).await);

        let executed = entries(&w, "SigningActionExecuted").await;
        assert_eq!(executed.len(), 2);
        for (user, log_type, description, details) in &executed {
            assert_eq!(user, "scheduled-event");
            assert_eq!(log_type, "INFO");
            assert!(
                description.contains(&format!("authorized by the signed configuration {code}")),
                "{description}"
            );
            assert_eq!(details["authorized_by"]["code"], json!(code));
            assert_eq!(details["authorized_by"]["signers"], json!(signers));
            assert_eq!(details["explanation"]["outcome"], json!("runs"));
        }
        let close = &executed[1].3;
        assert_eq!(close["record"]["authorized_by"]["code"], json!(code));
        assert!(close["record"].get("unsigned").is_none());
        assert_eq!(close["record"]["signatures"], json!([]));
        assert_eq!(close["cancelled"][0]["request_id"], json!(waiting));
        let cancelled = w.request(waiting).await;
        assert_eq!(cancelled.status, SigningRequestStatus::Cancelled);
        assert_eq!(
            cancelled.cancel_reason,
            Some(CancelReason::ClosedOnSchedule)
        );
        assert_eq!(w.approvals(waiting).await.len(), 1);
        // Each row keeps what happened, for the Publish tab and the Post
        // dashboard: openings as well as closes.
        for (row, action) in [(opens, "open-voting"), (closes, "close-voting")] {
            let fired = fired_outcome(&w, row).await;
            assert_eq!(fired["action"], json!(action));
            assert_eq!(fired["election_id"], json!(w.post));
            assert_eq!(fired["outcome"], json!("runs"));
            assert_eq!(fired["unsigned"], json!(false));
            assert_eq!(fired["authorized_by"]["code"], json!(code));
            assert_eq!(fired["explanation"]["outcome"], json!("runs"));
        }
        w.assert_two_entries_per_step().await;
    }
}

#[tokio::test]
async fn editing_a_row_after_signing_drops_its_coverage_until_the_next_approval() {
    for policy in POLICIES {
        let w = world("edit-after-signing").await;
        set_policy(&w, policy).await;
        require(&w, SigningAction::OpenVoting, 2).await;
        require(&w, SigningAction::ApproveConfiguration, 2).await;
        let opens = add_row(&w, OPENS, Some(w.post), "2028-04-09T00:00").await;
        let (_, code) = approve(&w, None, 2, "first").await;
        // The approval's execution seeded the row's first prediction
        // silently: the row isn't what that write changed.
        assert!(changes(&w).await.is_empty());
        assert_eq!(
            kind(&outcome_of(&w, opens).await),
            ScheduledOutcomeKind::Runs
        );

        // An edit after signing, saved with its recompute.
        save_row(&w, opens, "2028-04-09T01:00", "ofov.admin").await;
        let logged = changes(&w).await;
        assert_eq!(logged.len(), 1);
        let (user, log_type, description, details) = &logged[0];
        assert_eq!(user, "ofov.admin");
        assert_eq!(log_type, "INFO");
        assert_eq!(details["scheduled_event_id"], json!(opens.to_string()));
        assert_eq!(details["before"]["outcome"], json!("runs"));
        assert_eq!(details["after"]["outcome"], json!("refused"));
        assert!(description.contains("now refused"), "{description}");
        assert!(
            description.contains(&format!("was: runs, authorized by configuration {code}")),
            "{description}"
        );
        assert!(description.contains("by ofov.admin"), "{description}");
        let predicted = outcome_of(&w, opens).await;
        assert_eq!(kind(&predicted), ScheduledOutcomeKind::Refused);
        let covered = predicted
            .explanation
            .checks
            .iter()
            .find(|check| check.id == CheckId::Covered)
            .unwrap();
        assert_eq!(
            covered.current.message_key,
            "scheduledOutcome.check.covered.changedBy"
        );
        assert_eq!(covered.current.params["edited_by"], json!("ofov.admin"));

        // Nothing changed: nothing more is logged.
        recompute(&w, "ofov.admin").await;
        assert_eq!(changes(&w).await.len(), 1);

        // The next approved publication covers it again.
        approve(&w, None, 2, "second").await;
        let logged = changes(&w).await;
        assert_eq!(logged.len(), 2);
        assert_eq!(logged[1].0, "configuration-manager");
        assert_eq!(logged[1].3["after"]["outcome"], json!("runs"));
        assert!(!fire(&w, opens, VotingStatus::OPEN).await);
    }
}

#[tokio::test]
async fn an_edited_opening_is_refused_at_its_time() {
    let w = world("edited-opening").await;
    require(&w, SigningAction::OpenVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let opens = add_row(&w, OPENS, Some(w.post), "2028-04-09T00:00").await;
    approve(&w, None, 2, "officer").await;
    edit_row(&w, opens, "2028-04-08T23:00").await;
    recompute(&w, "ofov.admin").await;
    let predicted = outcome_of(&w, opens).await;
    assert!(fire(&w, opens, VotingStatus::OPEN).await);
    let executed = entries(&w, "SigningActionExecuted").await;
    assert_eq!(executed.len(), 1);
    let (_, log_type, description, details) = &executed[0];
    assert_eq!(log_type, "ERROR");
    assert_eq!(details["reason"], json!("not-in-signed-configuration"));
    assert!(
        description.starts_with("Did not open voting"),
        "{description}"
    );
    // What the list said is what the scheduler did.
    assert_eq!(
        details["explanation"]["outcome"],
        json!(predicted.explanation.outcome)
    );
    assert_eq!(
        details["explanation"]["next_step"]["message_key"],
        json!("scheduledOutcome.nextStep.askSignersToOpen")
    );
}

#[tokio::test]
async fn an_uncovered_close_follows_the_policy_and_an_uncovered_opening_is_refused() {
    for policy in POLICIES {
        let w = world("uncovered").await;
        set_policy(&w, policy.clone()).await;
        require(&w, SigningAction::OpenVoting, 2).await;
        require(&w, SigningAction::CloseVoting, 2).await;
        let opens = add_row(&w, OPENS, Some(w.post), "2028-04-09T00:00").await;
        let closes = add_row(&w, CLOSES, Some(w.post), "2028-05-08T19:00").await;
        // Published without an approval: nothing covers the schedule.
        publish(&w, None).await;
        let open_predicted = outcome_of(&w, opens).await;
        let close_predicted = outcome_of(&w, closes).await;
        assert_eq!(kind(&open_predicted), ScheduledOutcomeKind::Refused);
        let expected = if policy == RUN_AS_SYSTEM {
            ScheduledOutcomeKind::RunsUnsigned
        } else {
            ScheduledOutcomeKind::Refused
        };
        assert_eq!(kind(&close_predicted), expected);

        let waiting = waiting_close_with_one_signature(&w).await;
        assert!(fire(&w, opens, VotingStatus::OPEN).await);
        let closed = !fire(&w, closes, VotingStatus::CLOSED).await;
        assert_eq!(closed, policy == RUN_AS_SYSTEM);

        let executed = entries(&w, "SigningActionExecuted").await;
        assert_eq!(executed.len(), 2);
        assert_eq!(executed[0].1, "ERROR");
        assert_eq!(
            executed[0].3["reason"],
            json!("not-in-signed-configuration")
        );
        let (_, log_type, description, details) = &executed[1];
        // The list's prediction is the fire-time outcome.
        assert_eq!(
            details["explanation"]["outcome"],
            json!(close_predicted.explanation.outcome)
        );
        let request = w.request(waiting).await;
        if policy == RUN_AS_SYSTEM {
            assert_eq!(log_type, "INFO");
            assert!(description.contains("without signatures"), "{description}");
            assert_eq!(details["reason"], json!("unsigned-scheduled-close"));
            assert_eq!(details["record"]["unsigned"], json!(true));
            assert!(details["record"].get("authorized_by").is_none());
            assert_eq!(
                details["cancelled"],
                json!([{"request_id": waiting, "action": "close-voting", "code": request.code, "signatures": 1, "required": 2}])
            );
            assert_eq!(request.cancel_reason, Some(CancelReason::ClosedOnSchedule));
        } else {
            assert_eq!(log_type, "ERROR");
            assert_eq!(details["reason"], json!("signing-required"));
            assert_eq!(request.status, SigningRequestStatus::Waiting);
        }
        assert_eq!(fired_outcome(&w, opens).await["outcome"], json!("refused"));
        let fired_close = fired_outcome(&w, closes).await;
        assert_eq!(
            fired_close["outcome"],
            json!(close_predicted.explanation.outcome)
        );
        assert_eq!(fired_close["unsigned"], json!(policy == RUN_AS_SYSTEM));
        // The partial signature stays either way.
        assert_eq!(w.approvals(waiting).await.len(), 1);
        w.assert_two_entries_per_step().await;
    }
}

async fn save_policy(
    w: &World,
    close: UnsignedScheduledClosePolicy,
) -> windmill::services::scheduled_outcome::SavedPolicies {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let saved = save_lifecycle_policies(
        &tx,
        w.tenant,
        w.event,
        &LifecyclePolicies {
            initialization_scope: InitializationScope::POST,
            unsigned_scheduled_close: close,
        },
        &actor("ofov.admin"),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    saved
}

#[tokio::test]
async fn a_loosened_policy_waits_for_the_next_publication_and_a_tightened_one_applies_at_once() {
    let w = world("policy-copies").await;
    require(&w, SigningAction::CloseVoting, 2).await;
    let closes = create_row(&w, CLOSES, Some(w.post), "2028-05-08T19:00", "ofov.admin").await;
    set_policy(&w, REFUSE).await;
    publish(&w, None).await;
    assert_eq!(changes(&w).await.len(), 1);
    assert_eq!(
        kind(&outcome_of(&w, closes).await),
        ScheduledOutcomeKind::Refused
    );

    // Loosening: no effect until the next publication, so nothing logged.
    let saved = save_policy(&w, RUN_AS_SYSTEM).await;
    assert_eq!(saved.applies, Some(ChangeApplies::Loosens));
    assert!(saved.changes.is_empty());
    let predicted = outcome_of(&w, closes).await;
    assert_eq!(kind(&predicted), ScheduledOutcomeKind::Refused);
    assert_eq!(predicted.explanation.deciding, CheckId::StricterCopy);
    assert_eq!(changes(&w).await.len(), 1);

    // The next publication carries it.
    publish(&w, None).await;
    assert_eq!(
        kind(&outcome_of(&w, closes).await),
        ScheduledOutcomeKind::RunsUnsigned
    );
    let logged = changes(&w).await;
    assert_eq!(logged.len(), 2);
    assert_eq!(logged[1].0, "configuration-manager");
    assert_eq!(logged[1].3["after"]["outcome"], json!("runs-unsigned"));

    // Tightening applies at once, and logs once.
    let saved = save_policy(&w, REFUSE).await;
    assert_eq!(saved.applies, Some(ChangeApplies::Tightens));
    assert_eq!(saved.changes.len(), 1);
    assert_eq!(
        kind(&outcome_of(&w, closes).await),
        ScheduledOutcomeKind::Refused
    );
    let logged = changes(&w).await;
    assert_eq!(logged.len(), 3);
    assert_eq!(logged[2].0, "ofov.admin");
    assert_eq!(logged[2].3["before"]["outcome"], json!("runs-unsigned"));
    assert_eq!(logged[2].3["after"]["outcome"], json!("refused"));
    // Saving the same again changes nothing and logs nothing.
    let saved = save_policy(&w, REFUSE).await;
    assert_eq!(saved.applies, None);
    assert_eq!(changes(&w).await.len(), 3);
    assert!(fire(&w, closes, VotingStatus::CLOSED).await);
}

#[tokio::test]
async fn before_the_first_publication_the_defaults_apply() {
    let w = world("nothing-published").await;
    require(&w, SigningAction::CloseVoting, 2).await;
    set_policy(&w, RUN_AS_SYSTEM).await;
    let closes = add_row(&w, CLOSES, Some(w.post), "2028-05-08T19:00").await;
    let predicted = outcome_of(&w, closes).await;
    assert_eq!(kind(&predicted), ScheduledOutcomeKind::Refused);
    assert_eq!(predicted.explanation.deciding, CheckId::Defaults);
    assert!(fire(&w, closes, VotingStatus::CLOSED).await);
    let executed = entries(&w, "SigningActionExecuted").await;
    assert_eq!(executed[0].3["explanation"]["deciding"], json!("defaults"));
}

async fn save_open_rule(
    w: &World,
    requirement: SigningRequirement,
) -> (Option<ChangeApplies>, usize) {
    let mut hasura = w.pool.get().await.unwrap();
    let htx = hasura.transaction().await.unwrap();
    let mut keycloak = w.pool.get().await.unwrap();
    let ktx = keycloak.transaction().await.unwrap();
    keycloak_tables(&ktx).await;
    // Two people who can sign open voting at the Post, so a Required rule
    // passes the capacity check.
    ktx.batch_execute(&format!(
        "INSERT INTO realm VALUES ('r1', 'tenant-{tenant}');
         INSERT INTO keycloak_role (id, name, realm_id) VALUES ('role', 'sign-open-voting', 'r1');
         INSERT INTO keycloak_group (id, name, realm_id) VALUES ('g', 'signers', 'r1');
         INSERT INTO group_role_mapping VALUES ('role', 'g');
         INSERT INTO group_attribute VALUES ('permission_labels', '{label}', 'g');
         INSERT INTO user_entity VALUES ('u1', 'signer-1', 'One', 'Last', true, 'r1', NULL),
             ('u2', 'signer-2', 'Two', 'Last', true, 'r1', NULL);
         INSERT INTO user_group_membership VALUES ('g', 'u1'), ('g', 'u2');",
        tenant = w.tenant,
        label = w.label,
    ))
    .await
    .unwrap();
    let revision: i64 = htx
        .query_one(
            "SELECT COALESCE(max(revision), 0) FROM sequent_backend.signing_rule
             WHERE election_event_id = $1 AND action = 'open-voting'",
            &[&w.event],
        )
        .await
        .unwrap()
        .get(0);
    let manager = caller("ofov.admin", &[Permissions::SIGNING_RULES_WRITE], &[]);
    let saved = save_rule(
        &htx,
        &ktx,
        &manager,
        w.tenant,
        w.event,
        &SaveRuleInput {
            action: SigningAction::OpenVoting,
            requirement,
            signatures: 2,
            requester_signing: RequesterSigning::Allowed,
            expires_minutes: None,
            roles: None,
            expected_revision: revision,
        },
    )
    .await
    .unwrap();
    htx.commit().await.unwrap();
    ktx.rollback().await.unwrap();
    (saved.outcome.applies, saved.outcome.outcome_changes.len())
}

#[tokio::test]
async fn a_loosened_rule_waits_for_the_next_publication_and_says_so() {
    let w = world("rule-copies").await;
    require(&w, SigningAction::OpenVoting, 2).await;
    let opens = create_row(&w, OPENS, Some(w.post), "2028-04-09T00:00", "ofov.admin").await;
    publish(&w, None).await;
    assert_eq!(
        kind(&outcome_of(&w, opens).await),
        ScheduledOutcomeKind::Refused
    );
    assert_eq!(changes(&w).await.len(), 1);

    // Open voting no longer needs signatures: manual openings at once, the
    // scheduled one after the next publication.
    let (applies, changed) = save_open_rule(&w, SigningRequirement::NotRequired).await;
    assert_eq!(applies, Some(ChangeApplies::Loosens));
    assert_eq!(changed, 0);
    let predicted = outcome_of(&w, opens).await;
    assert_eq!(kind(&predicted), ScheduledOutcomeKind::Refused);
    assert_eq!(predicted.explanation.deciding, CheckId::StricterCopy);
    assert_eq!(changes(&w).await.len(), 1);
    let rule_changed: String = w
        .pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT body->>'description' FROM sequent_backend.signing_log_outbox
             WHERE election_event_id = $1 AND statement_kind = 'SigningRuleChanged'
                 AND event_type = 'SYSTEM'",
            &[&w.event],
        )
        .await
        .unwrap()
        .get(0);
    assert!(
        rule_changed.ends_with(
            "Applies now to manual actions; to scheduled openings and closings after the next approved publication."
        ),
        "{rule_changed}"
    );
    publish(&w, None).await;
    assert_eq!(
        kind(&outcome_of(&w, opens).await),
        ScheduledOutcomeKind::Runs
    );
    assert_eq!(changes(&w).await.len(), 2);

    // Requiring signatures again applies at once and logs the change once.
    let (applies, changed) = save_open_rule(&w, SigningRequirement::Required).await;
    assert_eq!(applies, Some(ChangeApplies::Tightens));
    assert_eq!(changed, 1);
    assert_eq!(
        kind(&outcome_of(&w, opens).await),
        ScheduledOutcomeKind::Refused
    );
    let logged = changes(&w).await;
    assert_eq!(logged.len(), 3);
    assert_eq!(logged[2].0, "ofov.admin");
}

#[tokio::test]
async fn a_post_approval_covers_the_event_wide_close_at_that_post() {
    let w = world("post-approval").await;
    set_policy(&w, REFUSE).await;
    require(&w, SigningAction::CloseVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let closes = add_row(&w, CLOSES, None, "2028-05-08T19:00").await;
    let (_, code) = approve(&w, Some(w.post), 2, "post-officer").await;
    let predicted = outcomes(&w).await;
    assert_eq!(predicted.len(), 1);
    assert_eq!(predicted[0].election_id, Some(w.post));
    assert_eq!(kind(&predicted[0]), ScheduledOutcomeKind::Runs);
    assert_eq!(
        predicted[0]
            .explanation
            .authorized_by
            .as_ref()
            .unwrap()
            .code,
        code
    );
    // The event-wide close, as the event-wide task fires it.
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let refused = scheduled_change_needs_signatures(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        None,
        &VotingStatus::CLOSED,
        &closes.to_string(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(!refused);
    let executed = entries(&w, "SigningActionExecuted").await;
    assert_eq!(executed.len(), 1);
    assert_eq!(executed[0].3["election_id"], json!(w.post));
}

#[tokio::test]
async fn a_rule_that_needs_no_signatures_runs_and_logs_nothing() {
    let w = world("no-signatures").await;
    let opens = add_row(&w, OPENS, Some(w.post), "2028-04-09T00:00").await;
    let predicted = outcome_of(&w, opens).await;
    assert_eq!(kind(&predicted), ScheduledOutcomeKind::Runs);
    assert_eq!(predicted.explanation.deciding, CheckId::NeedsSignatures);
    assert!(!fire(&w, opens, VotingStatus::OPEN).await);
    assert!(entries(&w, "SigningActionExecuted").await.is_empty());
}

/// Another Post of the world's event.
async fn add_post(w: &World, label: &str) -> Uuid {
    let id = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.election
             (id, tenant_id, election_event_id, presentation, permission_label)
         VALUES ($1, $2, $3, $4, $5)",
        &[
            &id,
            &w.tenant,
            &w.event,
            &post_presentation(&format!("Post {label}")),
            &label,
        ],
    )
    .await;
    id
}

#[tokio::test]
async fn an_admin_alone_cannot_loosen_the_published_copy() {
    let w = world("forged-copy").await;
    set_policy(&w, REFUSE).await;
    require(&w, SigningAction::CloseVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let closes = add_row(&w, CLOSES, Some(w.post), "2028-05-08T19:00").await;
    let (_, _) = approve(&w, None, 2, "officer").await;
    // Out of the signed configuration: the policy decides.
    save_row(&w, closes, "2028-05-08T20:00", "ofov.admin").await;
    assert_eq!(
        kind(&outcome_of(&w, closes).await),
        ScheduledOutcomeKind::Refused
    );

    // Editable presentation remains client configuration; forged publication
    // completion/material authority is rejected before it can weaken the copy.
    set_policy(&w, RUN_AS_SYSTEM).await;
    let loose = serde_json::to_value(LifecycleSnapshotLoose::run_as_system()).unwrap();
    publication_attack_refused(
        &w,
        "UPDATE sequent_backend.ballot_publication
         SET annotations = jsonb_build_object('lifecycle_snapshot', $2::jsonb),
             published_at = now() + interval '1 hour'
         WHERE election_event_id = $1",
        &[&w.event, &loose],
    )
    .await;
    publication_attack_refused(
        &w,
        "INSERT INTO sequent_backend.ballot_publication
             (id, tenant_id, election_event_id, is_generated, election_ids, published_at,
              annotations)
         VALUES ($1, $2, $3, true, ARRAY[$4::uuid], now() + interval '2 hours',
                 jsonb_build_object('lifecycle_snapshot', $5::jsonb))",
        &[&Uuid::new_v4(), &w.tenant, &w.event, &w.post, &loose],
    )
    .await;
    let predicted = outcome_of(&w, closes).await;
    assert_eq!(kind(&predicted), ScheduledOutcomeKind::Refused);
    assert_eq!(predicted.explanation.deciding, CheckId::StricterCopy);
    publication_attack_refused(
        &w,
        "DELETE FROM sequent_backend.ballot_style WHERE election_event_id = $1",
        &[&w.event],
    )
    .await;
    // The protected styles still exist, so their restrictive FK also rejects
    // deleting the parent. Authorized event deletion removes both as trusted.
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let error = tx
        .execute(
            "DELETE FROM sequent_backend.ballot_publication WHERE election_event_id = $1",
            &[&w.event],
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.code(),
        Some(&tokio_postgres::error::SqlState::FOREIGN_KEY_VIOLATION)
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        kind(&outcome_of(&w, closes).await),
        ScheduledOutcomeKind::Refused
    );
    assert!(fire(&w, closes, VotingStatus::CLOSED).await);
}

/// A snapshot that lets every uncovered close run unsigned, as a forger
/// would write it.
struct LifecycleSnapshotLoose;

impl LifecycleSnapshotLoose {
    fn run_as_system() -> sequent_core::types::scheduled_outcome::LifecycleSnapshot {
        sequent_core::types::scheduled_outcome::LifecycleSnapshot {
            policies: LifecyclePolicies {
                initialization_scope: InitializationScope::POST,
                unsigned_scheduled_close: RUN_AS_SYSTEM,
            },
            ..Default::default()
        }
    }
}

#[tokio::test]
async fn an_event_wide_close_is_decided_at_each_post_it_applies_to() {
    let w = world("post-a").await;
    set_policy(&w, REFUSE).await;
    require(&w, SigningAction::CloseVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let other = add_post(&w, "post-b").await;
    let own_close = add_post(&w, "post-c").await;
    add_own_row(&w, CLOSES, own_close, "2028-05-08T18:00").await;
    let closes = add_row(&w, CLOSES, None, "2028-05-08T19:00").await;
    // Post A's approval covers the event-wide close at Post A only.
    let (_, code) = approve(&w, Some(w.post), 2, "post-a").await;
    let predicted: Vec<ScheduledOutcome> = outcomes(&w)
        .await
        .into_iter()
        .filter(|outcome| outcome.scheduled_event_id == closes.to_string())
        .collect();
    // Post C has its own close: the event-wide one doesn't apply there.
    assert_eq!(predicted.len(), 2);
    let at = |post| {
        predicted
            .iter()
            .find(|outcome| outcome.election_id == Some(post))
            .unwrap()
            .clone()
    };
    assert_eq!(kind(&at(w.post)), ScheduledOutcomeKind::Runs);
    assert_eq!(kind(&at(other)), ScheduledOutcomeKind::Refused);

    // For the whole event at once: refused; the covered Post logs that the
    // event-wide change was refused, with no explanation that says it runs.
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let refused = scheduled_change_needs_signatures(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        None,
        &VotingStatus::CLOSED,
        &closes.to_string(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(refused);
    let executed = entries(&w, "SigningActionExecuted").await;
    assert_eq!(executed.len(), 2);
    let held: Vec<&Value> = executed
        .iter()
        .map(|(_, _, _, details)| details)
        .filter(|details| details["reason"] == json!("event-wide-refused"))
        .collect();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0]["election_id"], json!(w.post));
    assert_eq!(held[0]["explanation"], Value::Null);
    assert_eq!(held[0]["outcome"], json!("refused"));
    for (_, log_type, _, details) in &executed {
        assert_eq!(log_type, "ERROR");
        assert_ne!(details["explanation"]["outcome"], json!("runs"));
    }

    // Per Post: the covered Post changes, the other doesn't.
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let runs = scheduled_change_for_posts(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &[w.post.to_string(), other.to_string()],
        &VotingStatus::CLOSED,
        &closes.to_string(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(runs, [w.post.to_string()]);
    let executed = entries(&w, "SigningActionExecuted").await;
    assert_eq!(executed.len(), 4);
    let entry_at = |post: Uuid| {
        executed[2..]
            .iter()
            .find(|(_, _, _, details)| details["election_id"] == json!(post))
            .unwrap()
            .clone()
    };
    let (_, log_type, _, details) = entry_at(w.post);
    assert_eq!(log_type, "INFO");
    assert_eq!(details["explanation"]["outcome"], json!("runs"));
    assert_eq!(details["authorized_by"]["code"], json!(code));
    let (_, log_type, _, details) = entry_at(other);
    assert_eq!(log_type, "ERROR");
    assert_eq!(details["explanation"]["outcome"], json!("refused"));
    assert_eq!(details["authorized_by"], Value::Null);
}

#[tokio::test]
async fn a_scheduled_close_cancels_only_what_it_makes_moot() {
    let w = world("moot-requests").await;
    set_policy(&w, RUN_AS_SYSTEM).await;
    require(&w, SigningAction::OpenVoting, 1).await;
    require(&w, SigningAction::CloseVoting, 2).await;
    w.execute(
        "UPDATE sequent_backend.election
         SET voting_channels = '{\"online\": true, \"kiosk\": true}' WHERE id = $1",
        &[&w.post],
    )
    .await;
    set_post_status(
        &w,
        json!({"voting_status": "OPEN", "kiosk_voting_status": "PAUSED"}),
    )
    .await;
    let close_online = waiting_id(
        status_gate(
            &w,
            &starter(&w),
            VotingStatus::CLOSED,
            Some(vec![VotingStatusChannel::ONLINE]),
        )
        .await,
    );
    let open_kiosk = waiting_id(
        status_gate(
            &w,
            &starter(&w),
            VotingStatus::OPEN,
            Some(vec![VotingStatusChannel::KIOSK]),
        )
        .await,
    );
    // A close of the kiosk channel only.
    let closes = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.scheduled_event
             (id, tenant_id, election_event_id, event_processor, cron_config, event_payload)
         VALUES ($1, $2, $3, 'END_VOTING_PERIOD', $4, $5)",
        &[
            &closes,
            &w.tenant,
            &w.event,
            &cron("2028-05-08T19:00"),
            &json!({"election_id": w.post.to_string(), "voting_channels": ["KIOSK"]}),
        ],
    )
    .await;
    publish(&w, None).await;
    assert!(!fire(&w, closes, VotingStatus::CLOSED).await);
    let fired = fired_outcome(&w, closes).await;
    assert_eq!(fired["channels"], json!(["KIOSK"]));
    assert_eq!(fired["record"]["channels"], json!(["KIOSK"]));
    assert!(fired["record"].get("code").is_none());
    assert!(fired["record"].get("payload_sha256").is_none());
    // The online close isn't what the schedule closed; the open is moot.
    assert_eq!(
        w.request(close_online).await.status,
        SigningRequestStatus::Waiting
    );
    let open = w.request(open_kiosk).await;
    assert_eq!(open.cancel_reason, Some(CancelReason::PayloadChanged));
    assert_eq!(fired["cancelled"][0]["request_id"], json!(open_kiosk));

    // Nothing left to close: recorded as such, nothing cancelled.
    set_post_status(
        &w,
        json!({"voting_status": "CLOSED", "kiosk_voting_status": "CLOSED"}),
    )
    .await;
    let again = add_row(&w, CLOSES, Some(w.post), "2028-05-08T20:00").await;
    assert!(!fire(&w, again, VotingStatus::CLOSED).await);
    let fired = fired_outcome(&w, again).await;
    assert_eq!(fired["nothing_to_change"], json!(true));
    assert!(fired.get("record").is_none());
    assert!(fired.get("cancelled").is_none());
    let executed = entries(&w, "SigningActionExecuted").await;
    let (_, _, description, _) = executed.last().unwrap();
    assert!(
        description.starts_with("Nothing to close voting"),
        "{description}"
    );
    assert_eq!(
        w.request(close_online).await.status,
        SigningRequestStatus::Waiting
    );
}

#[tokio::test]
async fn a_first_prediction_is_logged_only_for_the_row_its_write_made() {
    let w = world("first-prediction").await;
    require(&w, SigningAction::OpenVoting, 2).await;
    // A row from elsewhere: seeded silently.
    let quiet = add_row(&w, OPENS, Some(w.post), "2028-04-09T00:00").await;
    recompute(&w, "ofov.admin").await;
    assert!(changes(&w).await.is_empty());
    // A row created with its recompute: its first prediction is logged.
    let created = create_row(&w, CLOSES, Some(w.post), "2028-05-08T19:00", "ofov.admin").await;
    let logged = changes(&w).await;
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].0, "ofov.admin");
    assert_eq!(
        logged[0].3["scheduled_event_id"],
        json!(created.to_string())
    );
    assert_eq!(logged[0].3["before"], Value::Null);
    // A direct edit isn't attributed to the next writer.
    edit_row(&w, quiet, "2028-04-09T02:00").await;
    recompute(&w, "someone-else").await;
    let predicted = outcome_of(&w, quiet).await;
    assert_eq!(kind(&predicted), ScheduledOutcomeKind::Refused);
    assert_eq!(changes(&w).await.len(), 1);
}

#[tokio::test]
async fn a_locked_down_event_refuses_a_policy_save() {
    let w = world("locked-down").await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    windmill::postgres::trusted_write::trusted_write(&tx)
        .await
        .unwrap();
    tx.execute(
        "UPDATE sequent_backend.election_event SET presentation = '{\"locked_down\": \"locked-down\"}' WHERE id = $1",
        &[&w.event],
    ).await.unwrap();
    let error = save_lifecycle_policies(
        &tx,
        w.tenant,
        w.event,
        &LifecyclePolicies::default(),
        &actor("ofov.admin"),
    )
    .await
    .unwrap_err();
    assert!(error.downcast_ref::<LockedDown>().is_some(), "{error:#}");
}

#[tokio::test]
async fn the_snapshots_read_back_newest_and_newest_signed_per_target() {
    let w = world("snapshots").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let (request, code) = approve(&w, None, 2, "officer").await;
    // A later publication of the event without signatures (as when the
    // rule was off), and one of the Post.
    let unsigned = publish(&w, None).await;
    let post = publish(&w, Some(w.post)).await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let views = lifecycle_snapshots(&tx, w.tenant, w.event).await.unwrap();
    tx.rollback().await.unwrap();
    let shape: Vec<(Option<Uuid>, Uuid, bool, Option<String>)> = views
        .iter()
        .map(|view| {
            (
                view.election_id,
                view.publication_id,
                view.signed,
                view.approval_code.clone(),
            )
        })
        .collect();
    let signed_publication = views
        .iter()
        .find(|view| view.signed)
        .map(|view| view.publication_id)
        .unwrap();
    assert_eq!(
        shape,
        [
            (Some(w.post), post, false, None),
            (None, unsigned, false, None),
            (None, signed_publication, true, Some(code)),
        ]
    );
    assert_eq!(views[2].approval_request_id, Some(request));
}

/// A Post's own scheduled row, as the save path writes it (its task id).
async fn add_own_row(w: &World, processor: &str, post: Uuid, local: &str) -> Uuid {
    let id = add_row(w, processor, Some(post), local).await;
    let task_id = sequent_core::types::scheduled_event::generate_manage_date_task_name(
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&post.to_string()),
        &processor.parse().unwrap(),
    );
    w.execute(
        "UPDATE sequent_backend.scheduled_event SET task_id = $2 WHERE id = $1",
        &[&id, &task_id],
    )
    .await;
    id
}

async fn scheduled_event(
    w: &World,
    id: Uuid,
) -> sequent_core::types::scheduled_event::ScheduledEvent {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let found = windmill::postgres::scheduled_event::find_scheduled_event_by_id(
        &tx,
        Some(w.tenant.to_string()),
        Some(w.event.to_string()),
        &id.to_string(),
    )
    .await
    .unwrap()
    .unwrap();
    tx.rollback().await.unwrap();
    found
}

async fn fire_at_posts(
    w: &World,
    row: Uuid,
    posts: &[String],
    status: VotingStatus,
) -> Vec<String> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let runs = scheduled_change_for_posts(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        posts,
        &status,
        &row.to_string(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    runs
}

#[tokio::test]
async fn forged_own_rows_dont_take_posts_out_of_an_event_wide_check() {
    let w = world("forged-own-rows").await;
    require(&w, SigningAction::OpenVoting, 2).await;
    let other = add_post(&w, "post-b").await;
    let third = add_post(&w, "post-c").await;
    // Stopped "own" rows an admin inserted for two Posts, without the
    // Post's task id: they are not the Posts' own schedule.
    for post in [w.post, other] {
        let id = add_row(&w, OPENS, Some(post), "2028-04-08T00:00").await;
        w.execute(
            "UPDATE sequent_backend.scheduled_event SET stopped_at = now() WHERE id = $1",
            &[&id],
        )
        .await;
    }
    // A real own opening (task id, ONLINE) takes its Post out.
    add_own_row(&w, OPENS, third, "2028-04-08T00:00").await;
    let opens = add_row(&w, OPENS, None, "2028-04-09T00:00").await;
    let mut targets = {
        let event = scheduled_event(&w, opens).await;
        let mut client = w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let targets = event_wide_targets(&tx, &w.tenant.to_string(), &w.event.to_string(), &event)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
        targets
    };
    targets.sort();
    let mut expected = vec![w.post.to_string(), other.to_string()];
    expected.sort();
    assert_eq!(targets, expected);
    // The predictions are for the same Posts.
    let mut predicted: Vec<String> = outcomes(&w)
        .await
        .into_iter()
        .filter(|outcome| outcome.scheduled_event_id == opens.to_string())
        .map(|outcome| outcome.election_id.unwrap().to_string())
        .collect();
    predicted.sort();
    assert_eq!(predicted, expected);
    // Uncovered: nothing opens unsigned, each Post refused and logged.
    assert!(fire_at_posts(&w, opens, &targets, VotingStatus::OPEN)
        .await
        .is_empty());
    let executed = entries(&w, "SigningActionExecuted").await;
    assert_eq!(executed.len(), 2);
    for (_, log_type, _, details) in &executed {
        assert_eq!(log_type, "ERROR");
        assert_eq!(details["outcome"], json!("refused"));
    }
}

#[tokio::test]
async fn a_signed_transition_runs_once_and_not_late() {
    let w = world("replay").await;
    require(&w, SigningAction::OpenVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let opens = add_row(&w, OPENS, Some(w.post), "2028-04-09T00:00").await;
    // Due ten minutes ago: still on time.
    let soon = Uuid::new_v4();
    let late = Uuid::new_v4();
    for (id, minutes) in [(soon, 10), (late, 60)] {
        w.execute(
            "INSERT INTO sequent_backend.scheduled_event
                 (id, tenant_id, election_event_id, event_processor, cron_config, event_payload)
             VALUES ($1, $2, $3, 'END_VOTING_PERIOD',
                 jsonb_build_object('scheduled_date',
                     to_char((now() - make_interval(mins => $5)) AT TIME ZONE 'UTC',
                             'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')),
                 $4)",
            &[
                &id,
                &w.tenant,
                &w.event,
                &json!({"election_id": w.post.to_string()}),
                &minutes,
            ],
        )
        .await;
    }
    require(&w, SigningAction::CloseVoting, 2).await;
    approve(&w, None, 2, "officer").await;
    assert!(!fire(&w, opens, VotingStatus::OPEN).await);
    // A direct client cannot erase execution/cache evidence. Even rearming
    // while keeping that evidence does not replay the signed transition.
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let rejected = tx.execute(
        "UPDATE sequent_backend.scheduled_event SET stopped_at = NULL, annotations = '{}' WHERE id = $1",
        &[&opens],
    ).await.unwrap_err();
    assert_eq!(
        rejected.code(),
        Some(&tokio_postgres::error::SqlState::INSUFFICIENT_PRIVILEGE)
    );
    tx.rollback().await.unwrap();
    w.execute(
        "UPDATE sequent_backend.scheduled_event SET stopped_at = NULL WHERE id = $1",
        &[&opens],
    )
    .await;
    assert_eq!(
        kind(&outcome_of(&w, opens).await),
        ScheduledOutcomeKind::Refused
    );
    assert!(fire(&w, opens, VotingStatus::OPEN).await);
    let executed = entries(&w, "SigningActionExecuted").await;
    assert_eq!(executed.last().unwrap().3["reason"], json!("replay"));

    // A close released too late isn't covered; one on time is.
    set_post_status(&w, json!({"voting_status": "OPEN"})).await;
    assert!(fire(&w, late, VotingStatus::CLOSED).await);
    let executed = entries(&w, "SigningActionExecuted").await;
    assert_eq!(executed.last().unwrap().3["reason"], json!("late-fire"));
    assert!(!fire(&w, soon, VotingStatus::CLOSED).await);
}

#[tokio::test]
async fn changed_post_channels_drop_the_coverage() {
    let w = world("channels-changed").await;
    require(&w, SigningAction::OpenVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let opens = add_row(&w, OPENS, Some(w.post), "2028-04-09T00:00").await;
    approve(&w, None, 2, "officer").await;
    assert_eq!(
        kind(&outcome_of(&w, opens).await),
        ScheduledOutcomeKind::Runs
    );
    w.execute(
        "UPDATE sequent_backend.election
         SET voting_channels = '{\"online\": true, \"kiosk\": true}' WHERE id = $1",
        &[&w.post],
    )
    .await;
    let predicted = outcome_of(&w, opens).await;
    assert_eq!(kind(&predicted), ScheduledOutcomeKind::Refused);
    let covered = predicted
        .explanation
        .checks
        .iter()
        .find(|check| check.id == CheckId::Covered)
        .unwrap();
    assert_eq!(
        covered.current.message_key,
        "scheduledOutcome.check.covered.channelsChanged"
    );
    assert!(fire(&w, opens, VotingStatus::OPEN).await);
}

#[tokio::test]
async fn an_approval_without_a_lifecycle_keeps_the_previous_signed_copy() {
    let w = world("old-approval").await;
    let publication_id = publication(&w, None).await;
    let keep = |lifecycle: PublicationLifecycle| {
        let w = w.clone();
        async move {
            let mut client = w.pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            keep_publication_lifecycle(
                &tx,
                &w.tenant.to_string(),
                &w.event.to_string(),
                &publication_id.to_string(),
                None,
                &lifecycle,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }
    };
    let count = || {
        let w = w.clone();
        async move {
            let count: i64 = w
                .pool
                .get()
                .await
                .unwrap()
                .query_one(
                    "SELECT count(*) FROM sequent_backend.lifecycle_snapshot
                     WHERE election_event_id = $1",
                    &[&w.event],
                )
                .await
                .unwrap()
                .get(0);
            count
        }
    };
    let request = Uuid::new_v4();
    keep(PublicationLifecycle::Signed {
        request_id: request,
        snapshot: LifecycleSnapshotLoose::run_as_system(),
    })
    .await;
    assert_eq!(count().await, 1);
    keep(PublicationLifecycle::KeepPrevious).await;
    assert_eq!(count().await, 1);
    keep(PublicationLifecycle::Current).await;
    assert_eq!(count().await, 2);
}

/// Closes as the status service would, in the status JSON only (the board
/// is out of these tests).
struct SqlCloser;

#[async_trait]
impl PostCloser for SqlCloser {
    async fn close(
        &self,
        tx: &Transaction<'_>,
        _tenant_id: Uuid,
        _election_event_id: Uuid,
        election_id: Uuid,
        channels: &[VotingStatusChannel],
    ) -> anyhow::Result<()> {
        let mut status = serde_json::Map::new();
        for channel in channels {
            let key = match channel {
                VotingStatusChannel::ONLINE => "voting_status",
                VotingStatusChannel::KIOSK => "kiosk_voting_status",
                VotingStatusChannel::EARLY_VOTING => "early_voting_status",
                VotingStatusChannel::TELEPHONE => "telephone_voting_status",
            };
            status.insert(key.to_owned(), json!("CLOSED"));
        }
        // This fake implements the validated status writer at the port boundary.
        windmill::postgres::trusted_write::trusted_write(tx).await?;
        tx.execute(
            "UPDATE sequent_backend.election
             SET status = COALESCE(status, '{}'::jsonb) || $2::jsonb WHERE id = $1",
            &[&election_id, &Value::Object(status)],
        )
        .await?;
        Ok(())
    }
}

async fn enforce(w: &World) -> Vec<String> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let closed =
        enforce_signed_closes(&tx, &w.tenant.to_string(), &w.event.to_string(), &SqlCloser)
            .await
            .unwrap();
    tx.commit().await.unwrap();
    closed
        .into_iter()
        .map(|decision| decision.election_id)
        .collect()
}

/// A close of the Post due `minutes` from now (negative: in the past).
async fn close_due(w: &World, minutes: i32) -> Uuid {
    let id = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.scheduled_event
             (id, tenant_id, election_event_id, event_processor, cron_config, event_payload)
         VALUES ($1, $2, $3, 'END_VOTING_PERIOD',
             jsonb_build_object('scheduled_date',
                 to_char((now() + make_interval(mins => $5)) AT TIME ZONE 'UTC',
                         'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')),
             $4)",
        &[
            &id,
            &w.tenant,
            &w.event,
            &json!({"election_id": w.post.to_string()}),
            &minutes,
        ],
    )
    .await;
    id
}

async fn set_due(w: &World, id: Uuid, minutes: i32) {
    w.execute(
        "UPDATE sequent_backend.scheduled_event
         SET cron_config = jsonb_build_object('scheduled_date',
             to_char((now() + make_interval(mins => $2)) AT TIME ZONE 'UTC',
                     'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'))
         WHERE id = $1",
        &[&id, &minutes],
    )
    .await;
}

#[tokio::test]
async fn non_canonical_own_rows_dont_take_posts_out_of_an_event_wide_check() {
    let w = world("spelled-ids").await;
    require(&w, SigningAction::OpenVoting, 2).await;
    let other = add_post(&w, "post-b").await;
    // "Own" rows whose Post id is spelled otherwise, with a task id built
    // from that spelling and ONLINE: not the Posts' own rows.
    for spelled in [w.post.to_string().to_uppercase(), format!("{{{other}}}")] {
        let id = Uuid::new_v4();
        let task_id = sequent_core::types::scheduled_event::generate_manage_date_task_name(
            &w.tenant.to_string(),
            &w.event.to_string(),
            Some(&spelled),
            &sequent_core::types::scheduled_event::EventProcessors::START_VOTING_PERIOD,
        );
        w.execute(
            "INSERT INTO sequent_backend.scheduled_event
                 (id, tenant_id, election_event_id, event_processor, cron_config, event_payload,
                  task_id, stopped_at)
             VALUES ($1, $2, $3, 'START_VOTING_PERIOD', $4, $5, $6, now())",
            &[
                &id,
                &w.tenant,
                &w.event,
                &cron("2028-04-08T00:00"),
                &json!({"election_id": spelled, "voting_channels": ["ONLINE"]}),
                &task_id,
            ],
        )
        .await;
    }
    let opens = add_row(&w, OPENS, None, "2028-04-09T00:00").await;
    let event = scheduled_event(&w, opens).await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let decisions = event_wide_decision(&tx, &w.tenant.to_string(), &w.event.to_string(), &event)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let mut posts: Vec<String> = decisions.iter().map(|d| d.election_id.clone()).collect();
    posts.sort();
    let mut expected = vec![w.post.to_string(), other.to_string()];
    expected.sort();
    assert_eq!(posts, expected);
    assert!(decisions.iter().all(|decision| !decision.runs));
    let executed = entries(&w, "SigningActionExecuted").await;
    assert_eq!(executed.len(), 2);
    assert!(executed.iter().all(|(_, log_type, ..)| log_type == "ERROR"));
}

#[tokio::test]
async fn a_signed_transition_with_nothing_to_change_still_runs_only_once() {
    let w = world("nothing-then-replay").await;
    require(&w, SigningAction::CloseVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let closes = add_row(&w, CLOSES, Some(w.post), "2028-05-08T19:00").await;
    approve(&w, None, 2, "officer").await;
    set_post_status(&w, json!({"voting_status": "CLOSED"})).await;
    assert!(!fire(&w, closes, VotingStatus::CLOSED).await);
    assert_eq!(
        fired_outcome(&w, closes).await["nothing_to_change"],
        json!(true)
    );
    // Re-opened and the row re-armed: the signed close doesn't run again.
    set_post_status(&w, json!({"voting_status": "OPEN"})).await;
    assert!(fire(&w, closes, VotingStatus::CLOSED).await);
    let executed = entries(&w, "SigningActionExecuted").await;
    assert_eq!(executed.last().unwrap().3["reason"], json!("replay"));
}

#[tokio::test]
async fn a_signed_close_closes_at_its_time_whatever_happens_to_the_row() {
    for change in ["edit", "stop", "archive", "delete"] {
        let w = world(change).await;
        require(&w, SigningAction::CloseVoting, 2).await;
        require(&w, SigningAction::ApproveConfiguration, 2).await;
        set_post_status(&w, json!({"voting_status": "OPEN"})).await;
        let closes = close_due(&w, -5).await;
        let (_, code) = approve(&w, None, 2, "officer").await;
        // An admin alone tries to extend voting.
        match change {
            "edit" => set_due(&w, closes, 600).await,
            "stop" => {
                w.execute(
                    "UPDATE sequent_backend.scheduled_event SET stopped_at = now() WHERE id = $1",
                    &[&closes],
                )
                .await
            }
            "archive" => {
                w.execute(
                    "UPDATE sequent_backend.scheduled_event SET archived_at = now() WHERE id = $1",
                    &[&closes],
                )
                .await
            }
            _ => {
                w.execute(
                    "DELETE FROM sequent_backend.scheduled_event WHERE id = $1",
                    &[&closes],
                )
                .await
            }
        }
        assert_eq!(enforce(&w).await, [w.post.to_string()], "{change}");
        assert_eq!(
            post_status(&w).await.unwrap()["voting_status"],
            json!("CLOSED")
        );
        let executed = entries(&w, "SigningActionExecuted").await;
        let (_, log_type, description, details) = executed.last().unwrap();
        assert_eq!(log_type, "INFO");
        assert_eq!(details["reason"], json!("signed-close"));
        assert_eq!(details["authorized_by"]["code"], json!(code));
        assert!(description.contains(&code), "{description}");
        // Once.
        set_post_status(&w, json!({"voting_status": "OPEN"})).await;
        assert!(enforce(&w).await.is_empty(), "{change}");
    }
}

#[tokio::test]
async fn only_a_newer_approval_moves_a_signed_close() {
    let w = world("moved-close").await;
    require(&w, SigningAction::CloseVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    set_post_status(&w, json!({"voting_status": "OPEN"})).await;
    let closes = close_due(&w, 60).await;
    approve(&w, None, 2, "first").await;
    assert!(enforce(&w).await.is_empty());
    // Moved earlier without signatures: the signed time still holds.
    set_due(&w, closes, -5).await;
    assert!(enforce(&w).await.is_empty());
    // A newer approval of the new time: it closes then.
    approve(&w, None, 2, "second").await;
    assert_eq!(enforce(&w).await, [w.post.to_string()]);
}

#[tokio::test]
async fn an_approval_of_a_publication_published_meanwhile_fails() {
    let w = world("published-meanwhile").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let id = publication(&w, None).await;
    let who = caller("configuration-manager", &[Permissions::PUBLISH_WRITE], &[]);
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let outcome = gate_publication(
        &tx,
        &who,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &id.to_string(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let request_id = match outcome {
        GuardOutcome::SigningRequired(summary) => summary.id,
        GuardOutcome::Proceed => panic!("expected a request"),
    };
    // Raw completion cannot bypass the pending request.
    publication_attack_refused(
        &w,
        "UPDATE sequent_backend.ballot_publication SET published_at = now() WHERE id = $1",
        &[&id],
    )
    .await;
    // A competing trusted server publication is still detected by the effect.
    publication_fixture_write(
        &w,
        "UPDATE sequent_backend.ballot_publication SET published_at = now() WHERE id = $1",
        &[&id],
    )
    .await;
    let request = w.request(request_id).await;
    let tx = client.transaction().await.unwrap();
    let error = windmill::services::signing::actions::configuration::publish(
        &tx,
        &request,
        &EffectProgress::default(),
    )
    .await
    .unwrap_err();
    tx.rollback().await.unwrap();
    assert_eq!(
        error
            .downcast_ref::<windmill::services::signing::actions::EffectRefused>()
            .unwrap()
            .code,
        "payload-changed"
    );
}

#[tokio::test]
async fn a_tick_dispatches_a_signed_close_even_after_the_live_row_was_deleted() {
    let w = world("tick-deleted-close").await;
    require(&w, SigningAction::CloseVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    set_post_status(&w, json!({"voting_status": "OPEN"})).await;
    let closes = close_due(&w, -5).await;
    approve(&w, None, 2, "officer").await;
    w.execute(
        "DELETE FROM sequent_backend.scheduled_event WHERE id = $1",
        &[&closes],
    )
    .await;

    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let expected = (w.tenant.to_string(), w.event.to_string());
    let dispatched = std::cell::RefCell::new(Vec::new());
    let failed = windmill::tasks::enforce_signed_closes::dispatch_signed_close_checks(
        &tx,
        |tenant, event| {
            if (&tenant, &event) == (&expected.0, &expected.1) {
                dispatched.borrow_mut().push((tenant, event));
            }
            async { Ok(()) }
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(failed, 0);
    assert_eq!(dispatched.into_inner(), [expected]);

    let tx = client.transaction().await.unwrap();
    let retained = windmill::services::signing::actions::voting::retained_signed_closes(
        &tx, w.tenant, w.event,
    )
    .await
    .unwrap();
    assert_eq!(retained.len(), 1);
    assert_eq!(retained[0].scheduled_event_id, closes.to_string());
    assert_eq!(retained[0].election_id, w.post);
    assert!(retained[0].fired_at.is_none());
    tx.rollback().await.unwrap();

    // The dispatched worker has its own transaction; the deleted live row
    // cannot hide the signed deadline from either the tick or enforcement.
    assert_eq!(enforce(&w).await, [w.post.to_string()]);
    assert_eq!(
        post_status(&w).await.unwrap()["voting_status"],
        json!("CLOSED")
    );
    let tx = client.transaction().await.unwrap();
    let fired = windmill::services::signing::actions::voting::retained_signed_closes(
        &tx, w.tenant, w.event,
    )
    .await
    .unwrap();
    assert_eq!(fired.len(), 1);
    assert!(fired[0].fired_at.is_some());
    assert_eq!(fired[0].fingerprint, retained[0].fingerprint);
    tx.rollback().await.unwrap();
    assert!(enforce(&w).await.is_empty());
}

#[tokio::test]
async fn a_failed_signed_close_dispatch_does_not_skip_other_events() {
    let first = world("tick-broker-failure").await;
    let second = world("tick-broker-healthy").await;
    for w in [&first, &second] {
        require(w, SigningAction::ApproveConfiguration, 2).await;
        close_due(w, -5).await;
        approve(w, None, 2, "officer").await;
    }
    let mut client = first.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seen = std::cell::RefCell::new(Vec::new());
    let failed =
        windmill::tasks::enforce_signed_closes::dispatch_signed_close_checks(&tx, |_, event| {
            let fails = event == first.event.to_string();
            seen.borrow_mut().push(event);
            async move {
                if fails {
                    Err(anyhow::anyhow!("broker unavailable"))
                } else {
                    Ok(())
                }
            }
        })
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(failed, 1);
    let seen = seen.into_inner();
    assert!(seen.contains(&first.event.to_string()));
    assert!(seen.contains(&second.event.to_string()));
}

#[tokio::test]
async fn signed_closes_enforce_each_channel_after_live_rows_disappear() {
    let w = world("signed-multiple-channels").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    w.execute("UPDATE sequent_backend.election SET voting_channels = '{\"online\": true, \"kiosk\": true}' WHERE id = $1", &[&w.post]).await;
    set_post_status(
        &w,
        json!({"voting_status": "OPEN", "kiosk_voting_status": "OPEN"}),
    )
    .await;
    let online = close_due(&w, -10).await;
    w.execute(
        "UPDATE sequent_backend.scheduled_event
         SET event_payload = event_payload || '{\"voting_channels\": [\"ONLINE\"]}'::jsonb WHERE id = $1",
        &[&online],
    ).await;
    let kiosk = close_due(&w, -5).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_payload = event_payload || '{\"voting_channels\": [\"KIOSK\"]}'::jsonb WHERE id = $1", &[&kiosk]).await;
    approve(&w, None, 2, "officer").await;
    w.execute(
        "DELETE FROM sequent_backend.scheduled_event WHERE id = ANY($1)",
        &[&vec![online, kiosk]],
    )
    .await;
    assert_eq!(enforce(&w).await.len(), 2);
    let status = post_status(&w).await.unwrap();
    assert_eq!(status["voting_status"], json!("CLOSED"));
    assert_eq!(status["kiosk_voting_status"], json!("CLOSED"));
    assert!(enforce(&w).await.is_empty());
}

#[tokio::test]
async fn an_unrelated_signed_opening_cannot_extend_online_close() {
    let w = world("signed-unrelated-opening").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    w.execute("UPDATE sequent_backend.election SET voting_channels = '{\"online\": true, \"kiosk\": true}' WHERE id = $1", &[&w.post]).await;
    set_post_status(
        &w,
        json!({"voting_status": "OPEN", "kiosk_voting_status": "PAUSED"}),
    )
    .await;
    let online = close_due(&w, -10).await;
    let kiosk = close_due(&w, -5).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_processor = 'START_VOTING_PERIOD', event_payload = event_payload || '{\"voting_channels\": [\"KIOSK\"]}'::jsonb WHERE id = $1", &[&kiosk]).await;
    approve(&w, None, 2, "officer").await;
    // The gate records the effect; its caller applies status in the same transaction.
    // The board is outside these tests, as with SqlCloser below.
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert!(!scheduled_change_needs_signatures(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&w.post.to_string()),
        &VotingStatus::OPEN,
        &kiosk.to_string(),
    )
    .await
    .unwrap());
    windmill::postgres::trusted_write::trusted_write(&tx)
        .await
        .unwrap();
    tx.execute(
        "UPDATE sequent_backend.election SET status = status || '{\"kiosk_voting_status\": \"OPEN\"}'::jsonb WHERE id=$1",
        &[&w.post],
    ).await.unwrap();
    tx.commit().await.unwrap();
    w.execute(
        "DELETE FROM sequent_backend.scheduled_event WHERE id = ANY($1)",
        &[&vec![online, kiosk]],
    )
    .await;
    assert_eq!(enforce(&w).await, [w.post.to_string()]);
    let status = post_status(&w).await.unwrap();
    assert_eq!(status["voting_status"], json!("CLOSED"));
    assert_eq!(status["kiosk_voting_status"], json!("OPEN"));
}

#[tokio::test]
async fn a_kiosk_only_own_close_does_not_replace_the_signed_event_close() {
    let w = world("signed-own-kiosk").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    set_post_status(&w, json!({"voting_status": "OPEN"})).await;
    let event_close = close_due(&w, -5).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_payload = '{\"voting_channels\": [\"ONLINE\"]}'::jsonb WHERE id = $1", &[&event_close]).await;
    let kiosk = close_due(&w, 60).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_payload = event_payload || '{\"voting_channels\": [\"KIOSK\"]}'::jsonb WHERE id = $1", &[&kiosk]).await;
    approve(&w, None, 2, "officer").await;
    w.execute(
        "DELETE FROM sequent_backend.scheduled_event WHERE id = ANY($1)",
        &[&vec![event_close, kiosk]],
    )
    .await;
    assert_eq!(enforce(&w).await, [w.post.to_string()]);
    assert_eq!(
        post_status(&w).await.unwrap()["voting_status"],
        json!("CLOSED")
    );
}

#[tokio::test]
async fn a_signed_post_row_cannot_authorize_a_different_post() {
    let w = world("signed-target-scope").await;
    let other = add_post(&w, "other").await;
    require(&w, SigningAction::OpenVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let opens = add_row(&w, OPENS, Some(w.post), "2028-04-09T00:00").await;
    approve(&w, None, 2, "officer").await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let runs = scheduled_change_for_posts(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &[other.to_string()],
        &VotingStatus::OPEN,
        &opens.to_string(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(runs.is_empty());
    assert!(entries(&w, "SigningActionExecuted").await.is_empty());
}

#[tokio::test]
async fn a_signed_opening_supersedes_only_its_channel_of_a_close() {
    let w = world("signed-partial-supersession").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    w.execute("UPDATE sequent_backend.election SET voting_channels = '{\"online\": true, \"kiosk\": true}' WHERE id = $1", &[&w.post]).await;
    set_post_status(
        &w,
        json!({"voting_status": "OPEN", "kiosk_voting_status": "PAUSED"}),
    )
    .await;
    let close = close_due(&w, -10).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_payload = event_payload || '{\"voting_channels\": [\"ONLINE\", \"KIOSK\"]}'::jsonb WHERE id = $1", &[&close]).await;
    let opening = close_due(&w, -5).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_processor = 'START_VOTING_PERIOD', event_payload = event_payload || '{\"voting_channels\": [\"KIOSK\"]}'::jsonb WHERE id = $1", &[&opening]).await;
    approve(&w, None, 2, "officer").await;
    assert!(!fire(&w, opening, VotingStatus::OPEN).await);
    set_post_status(
        &w,
        json!({"voting_status": "OPEN", "kiosk_voting_status": "OPEN"}),
    )
    .await;
    assert_eq!(enforce(&w).await, [w.post.to_string()]);
    let status = post_status(&w).await.unwrap();
    assert_eq!(status["voting_status"], json!("CLOSED"));
    assert_eq!(status["kiosk_voting_status"], json!("OPEN"));
    let executed = entries(&w, "SigningActionExecuted").await;
    assert_eq!(executed.last().unwrap().3["channels"], json!(["ONLINE"]));
}

#[tokio::test]
async fn archived_events_do_not_execute_or_record_signed_closes() {
    let w = world("signed-archived-event").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    set_post_status(&w, json!({"voting_status": "OPEN"})).await;
    close_due(&w, -5).await;
    approve(&w, None, 2, "officer").await;
    let before = entries(&w, "SigningActionExecuted").await.len();
    w.execute(
        "UPDATE sequent_backend.election_event SET is_archived = true WHERE id = $1",
        &[&w.event],
    )
    .await;
    assert!(enforce(&w).await.is_empty());
    assert_eq!(
        post_status(&w).await.unwrap()["voting_status"],
        json!("OPEN")
    );
    assert_eq!(entries(&w, "SigningActionExecuted").await.len(), before);
}

/// The maintained window read on every ballot, without scheduler execution.
async fn cast_window_end(w: &World, post: Uuid) -> Option<String> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let config = windmill::postgres::election::get_cast_vote_configuration(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &post.to_string(),
    )
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    config.dates.end_date
}

async fn signed_row_date(w: &World, row: Uuid) -> String {
    w.pool.get().await.unwrap().query_one(
        "SELECT cron_config->>'scheduled_date' FROM sequent_backend.scheduled_event WHERE id = $1",
        &[&row],
    ).await.unwrap().get(0)
}

#[tokio::test]
async fn ballot_deadline_keeps_a_signed_close_after_live_edits_or_deletion() {
    let w = world("cast-signed-close").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    set_post_status(&w, json!({"voting_status": "OPEN"})).await;
    let close = close_due(&w, -5).await;
    let task = sequent_core::types::scheduled_event::generate_manage_date_task_name(
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&w.post.to_string()),
        &sequent_core::types::scheduled_event::EventProcessors::END_VOTING_PERIOD,
    );
    w.execute(
        "UPDATE sequent_backend.scheduled_event SET task_id = $2 WHERE id = $1",
        &[&close, &task],
    )
    .await;
    let signed_date = signed_row_date(&w, close).await;
    approve(&w, None, 2, "officer").await;
    assert_eq!(cast_window_end(&w, w.post).await, Some(signed_date.clone()));
    set_due(&w, close, 60).await;
    assert_eq!(cast_window_end(&w, w.post).await, Some(signed_date.clone()));
    w.execute(
        "DELETE FROM sequent_backend.scheduled_event WHERE id = $1",
        &[&close],
    )
    .await;
    assert_eq!(cast_window_end(&w, w.post).await, Some(signed_date));
    // The signed bound does not depend on beat having changed this status.
    assert_eq!(
        post_status(&w).await.unwrap()["voting_status"],
        json!("OPEN")
    );
}

#[tokio::test]
async fn only_a_new_executed_approval_moves_the_ballot_signed_close_bound() {
    let w = world("cast-approved-reschedule").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let close = close_due(&w, -5).await;
    let first = signed_row_date(&w, close).await;
    approve(&w, None, 2, "first").await;
    set_due(&w, close, 60).await;
    let second = signed_row_date(&w, close).await;
    assert_eq!(cast_window_end(&w, w.post).await, Some(first));
    approve(&w, None, 2, "second").await;
    assert_eq!(cast_window_end(&w, w.post).await, Some(second.clone()));
    let new_post = add_post(&w, "new-post").await;
    // This close belongs to the original Post, not to a newly added Post.
    assert_eq!(cast_window_end(&w, new_post).await, None);
}

#[tokio::test]
async fn a_signed_kiosk_own_row_cannot_hide_the_common_online_ballot_bound() {
    let w = world("cast-signed-common-close").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let common = close_due(&w, -5).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_payload = '{\"voting_channels\": [\"ONLINE\"]}' WHERE id = $1", &[&common]).await;
    let own = close_due(&w, 60).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_payload = event_payload || '{\"voting_channels\": [\"KIOSK\"]}'::jsonb WHERE id = $1", &[&own]).await;
    let date = signed_row_date(&w, common).await;
    approve(&w, None, 2, "officer").await;
    w.execute(
        "DELETE FROM sequent_backend.scheduled_event WHERE id = ANY($1)",
        &[&vec![common, own]],
    )
    .await;
    assert_eq!(cast_window_end(&w, w.post).await, Some(date.clone()));
    let new_post = add_post(&w, "new-post").await;
    assert_eq!(cast_window_end(&w, new_post).await, Some(date));
}

#[tokio::test]
async fn a_later_signed_online_opening_supersedes_only_the_older_ballot_bound() {
    let w = world("cast-signed-new-period").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    close_due(&w, -10).await;
    let opening = close_due(&w, -5).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_processor = 'START_VOTING_PERIOD' WHERE id = $1", &[&opening]).await;
    let next_close = close_due(&w, 60).await;
    let date = signed_row_date(&w, next_close).await;
    approve(&w, None, 2, "officer").await;
    set_post_status(&w, json!({"voting_status": "PAUSED"})).await;
    assert!(!fire(&w, opening, VotingStatus::OPEN).await);
    assert_eq!(cast_window_end(&w, w.post).await, Some(date));
}

#[tokio::test]
async fn an_unexecuted_signed_opening_cannot_erase_the_previous_signed_deadline() {
    let w = world("cast-unexecuted-new-period").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    set_post_status(&w, json!({"voting_status": "OPEN"})).await;
    let close = close_due(&w, -10).await;
    let original_deadline = signed_row_date(&w, close).await;
    let opening = close_due(&w, -5).await;
    w.execute(
        "UPDATE sequent_backend.scheduled_event SET event_processor = 'START_VOTING_PERIOD' WHERE id = $1",
        &[&opening],
    )
    .await;
    close_due(&w, 60).await;
    approve(&w, None, 2, "officer").await;
    // Beat missed the first close. An operator deletes the later opening
    // before it executes; merely passing its signed time cannot reopen a
    // period or remove the old close's independent ballot acceptance bound.
    w.execute(
        "DELETE FROM sequent_backend.scheduled_event WHERE id = $1",
        &[&opening],
    )
    .await;
    assert_eq!(cast_window_end(&w, w.post).await, Some(original_deadline));
    assert_eq!(enforce(&w).await, [w.post.to_string()]);
    assert_eq!(
        post_status(&w).await.unwrap()["voting_status"],
        json!("CLOSED")
    );
}

#[tokio::test]
async fn an_offline_only_opening_effect_cannot_erase_the_online_signed_deadline() {
    let w = world("cast-partial-opening-proof").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    w.execute(
        "UPDATE sequent_backend.election SET voting_channels = '{\"online\":true,\"kiosk\":true}' WHERE id = $1",
        &[&w.post],
    )
    .await;
    set_post_status(
        &w,
        json!({"voting_status":"OPEN","kiosk_voting_status":"NOT_STARTED"}),
    )
    .await;
    let close = close_due(&w, -10).await;
    let original_deadline = signed_row_date(&w, close).await;
    let opening = close_due(&w, -5).await;
    w.execute(
        "UPDATE sequent_backend.scheduled_event SET event_processor='START_VOTING_PERIOD', event_payload=event_payload || '{\"voting_channels\":[\"ONLINE\",\"KIOSK\"]}'::jsonb WHERE id=$1",
        &[&opening],
    )
    .await;
    close_due(&w, 60).await;
    approve(&w, None, 2, "officer").await;
    // Opening requires no individual signatures in this configuration. A
    // disabled ONLINE channel does not change, while the KIOSK can open.
    w.execute(
        "UPDATE sequent_backend.election SET voting_channels='{\"online\":false,\"kiosk\":true}' WHERE id=$1",
        &[&w.post],
    )
    .await;
    assert!(!fire(&w, opening, VotingStatus::OPEN).await);
    assert_eq!(
        fired_outcome(&w, opening).await["channels"],
        json!(["KIOSK"])
    );
    assert_eq!(cast_window_end(&w, w.post).await, Some(original_deadline));
}

#[tokio::test]
async fn a_noop_signed_opening_keeps_the_previous_signed_deadline() {
    let w = world("cast-noop-opening-proof").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    set_post_status(&w, json!({"voting_status":"OPEN"})).await;
    let close = close_due(&w, -10).await;
    let original = signed_row_date(&w, close).await;
    let opening = close_due(&w, -5).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_processor='START_VOTING_PERIOD' WHERE id=$1", &[&opening]).await;
    close_due(&w, 60).await;
    approve(&w, None, 2, "officer").await;
    assert!(!fire(&w, opening, VotingStatus::OPEN).await);
    assert_eq!(
        fired_outcome(&w, opening).await["nothing_to_change"],
        json!(true)
    );
    assert_eq!(cast_window_end(&w, w.post).await, Some(original));
    assert_eq!(enforce(&w).await, [w.post.to_string()]);
}

#[tokio::test]
async fn fired_opening_proof_binds_post_fingerprint_channel_time_and_transaction() {
    let w = world("cast-exact-opening-proof").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let other = add_post(&w, "proof-other-post").await;
    let close = close_due(&w, -10).await;
    let original = signed_row_date(&w, close).await;
    let opening = close_due(&w, -5).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_processor='START_VOTING_PERIOD' WHERE id=$1", &[&opening]).await;
    let next = close_due(&w, 60).await;
    let next_date = signed_row_date(&w, next).await;
    approve(&w, None, 2, "officer").await;
    let fingerprint: String = w.pool.get().await.unwrap().query_one(
        "SELECT fingerprint FROM sequent_backend.signed_voting_boundary WHERE election_id=$1 AND scheduled_event_id=$2",
        &[&w.post, &opening.to_string()],
    ).await.unwrap().get(0);
    let cases = [
        (
            "other Post",
            other,
            fingerprint.clone(),
            Some(json!(["ONLINE"])),
            -1,
        ),
        (
            "different fingerprint",
            w.post,
            "f".repeat(64),
            Some(json!(["ONLINE"])),
            -1,
        ),
        (
            "offline effect",
            w.post,
            fingerprint.clone(),
            Some(json!(["KIOSK"])),
            -1,
        ),
        (
            "no effect",
            w.post,
            fingerprint.clone(),
            Some(json!([])),
            -1,
        ),
        (
            "legacy unknown effect",
            w.post,
            fingerprint.clone(),
            None,
            -1,
        ),
        (
            "future effect",
            w.post,
            fingerprint.clone(),
            Some(json!(["ONLINE"])),
            60,
        ),
        (
            "effect before opening",
            w.post,
            fingerprint.clone(),
            Some(json!(["ONLINE"])),
            -20,
        ),
    ];
    let mut client = w.pool.get().await.unwrap();
    for (reason, post, fingerprint, channels, minutes) in cases {
        let tx = client.transaction().await.unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.lifecycle_fired (tenant_id,election_event_id,scheduled_event_id,election_id,fingerprint,executed_channels,fired_at)
             VALUES ($1,$2,$3,$4,$5,$6,clock_timestamp()+make_interval(mins=>$7))",
            &[&w.tenant, &w.event, &opening, &post, &fingerprint, &channels, &minutes],
        ).await.unwrap();
        let config = windmill::postgres::election::get_cast_vote_configuration(
            &tx,
            &w.tenant.to_string(),
            &w.event.to_string(),
            &w.post.to_string(),
        )
        .await
        .unwrap();
        assert_eq!(config.dates.end_date, Some(original.clone()), "{reason}");
        let shown = windmill::postgres::election::get_display_voting_closes(
            &tx,
            &w.tenant.to_string(),
            &w.event.to_string(),
            &[w.post.to_string()],
        )
        .await
        .unwrap();
        assert_eq!(
            shown[&w.post.to_string()].scheduled_at,
            Some(original.clone()),
            "{reason}"
        );
        let retained = windmill::services::signing::actions::voting::retained_signed_closes(
            &tx, w.tenant, w.event,
        )
        .await
        .unwrap();
        assert!(
            retained.iter().any(|retained| {
                retained.scheduled_event_id == close.to_string()
                    && retained.election_id == w.post
                    && retained.channels.contains(&VotingStatusChannel::ONLINE)
            }),
            "{reason}"
        );
        tx.rollback().await.unwrap();
    }
    // A real channel effect rolled back with its task is not execution proof.
    set_post_status(&w, json!({"voting_status":"PAUSED"})).await;
    let tx = client.transaction().await.unwrap();
    assert!(!scheduled_change_needs_signatures(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&w.post.to_string()),
        &VotingStatus::OPEN,
        &opening.to_string(),
    )
    .await
    .unwrap());
    tx.rollback().await.unwrap();
    assert_eq!(cast_window_end(&w, w.post).await, Some(original));
    assert!(!fire(&w, opening, VotingStatus::OPEN).await);
    assert_eq!(cast_window_end(&w, w.post).await, Some(next_date));
}

#[tokio::test]
async fn signed_own_opening_precedence_survives_live_task_tampering_or_deletion() {
    for deleted in [false, true] {
        let w = world("signed-own-opening").await;
        require(&w, SigningAction::ApproveConfiguration, 2).await;
        let global = add_row(&w, OPENS, None, "2099-04-09T00:00").await;
        let own = add_row(&w, OPENS, Some(w.post), "2099-04-09T01:00").await;
        let task = sequent_core::types::scheduled_event::generate_manage_date_task_name(
            &w.tenant.to_string(),
            &w.event.to_string(),
            Some(&w.post.to_string()),
            &sequent_core::types::scheduled_event::EventProcessors::START_VOTING_PERIOD,
        );
        w.execute(
            "UPDATE sequent_backend.scheduled_event SET task_id=$2 WHERE id=$1",
            &[&own, &task],
        )
        .await;
        assert!(!outcomes(&w)
            .await
            .iter()
            .any(|outcome| outcome.scheduled_event_id == global.to_string()
                && outcome.election_id == Some(w.post)));
        approve(&w, None, 2, "original").await;
        if deleted {
            w.execute(
                "DELETE FROM sequent_backend.scheduled_event WHERE id=$1",
                &[&own],
            )
            .await;
        } else {
            w.execute(
                "UPDATE sequent_backend.scheduled_event SET task_id='dummy' WHERE id=$1",
                &[&own],
            )
            .await;
        }
        let predicted = outcome_of(&w, global).await;
        assert_eq!(kind(&predicted), ScheduledOutcomeKind::Refused);
        assert_eq!(predicted.explanation.deciding, CheckId::Covered);
        assert!(predicted
            .explanation
            .checks
            .iter()
            .any(|check| check.current.message_key
                == "scheduledOutcome.check.covered.overriddenBySignedPostRow"));
        assert!(fire(&w, global, VotingStatus::OPEN).await);
        let mut client = w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let fired: i64 = tx
            .query_one(
                "SELECT count(*) FROM sequent_backend.lifecycle_fired WHERE election_event_id=$1",
                &[&w.event],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(fired, 0);
        tx.rollback().await.unwrap();
        if !deleted {
            w.execute(
                "DELETE FROM sequent_backend.scheduled_event WHERE id=$1",
                &[&own],
            )
            .await;
        }
        approve(&w, None, 2, "replacement").await;
        assert_eq!(
            kind(&outcome_of(&w, global).await),
            ScheduledOutcomeKind::Runs
        );
    }
}

#[tokio::test]
async fn signed_cast_bounds_cover_kiosk_early_and_telephone_channels() {
    let w = world("cast-signed-channel-bounds").await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let close = close_due(&w, -5).await;
    w.execute("UPDATE sequent_backend.scheduled_event SET event_payload = event_payload || '{\"voting_channels\": [\"KIOSK\", \"EARLY_VOTING\", \"TELEPHONE\"]}'::jsonb WHERE id = $1", &[&close]).await;
    let date = signed_row_date(&w, close).await;
    approve(&w, None, 2, "officer").await;
    w.execute(
        "DELETE FROM sequent_backend.scheduled_event WHERE id = $1",
        &[&close],
    )
    .await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let config = windmill::postgres::election::get_cast_vote_configuration(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &w.post.to_string(),
    )
    .await
    .unwrap();
    assert_eq!(config.dates.end_date, None);
    for channel in [
        VotingStatusChannel::KIOSK,
        VotingStatusChannel::EARLY_VOTING,
        VotingStatusChannel::TELEPHONE,
    ] {
        assert_eq!(config.signed_close_dates.get(&channel), Some(&date));
    }
    assert_eq!(config.signed_close_dates.len(), 3);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_refused_live_close_cannot_tighten_the_signed_ballot_deadline() {
    let w = world("cast-refused-live-close").await;
    require(&w, SigningAction::CloseVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let close = close_due(&w, 60).await;
    let task = sequent_core::types::scheduled_event::generate_manage_date_task_name(
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&w.post.to_string()),
        &sequent_core::types::scheduled_event::EventProcessors::END_VOTING_PERIOD,
    );
    w.execute(
        "UPDATE sequent_backend.scheduled_event SET task_id=$2 WHERE id=$1",
        &[&close, &task],
    )
    .await;
    let signed = signed_row_date(&w, close).await;
    approve(&w, None, 2, "officer").await;
    set_due(&w, close, -5).await;
    assert_eq!(
        kind(&outcome_of(&w, close).await),
        ScheduledOutcomeKind::Refused
    );
    assert_eq!(cast_window_end(&w, w.post).await, Some(signed));
}

#[tokio::test]
async fn both_run_as_system_copies_allow_a_live_close_to_tighten_the_ballot_deadline() {
    let w = world("cast-allowed-live-close").await;
    require(&w, SigningAction::CloseVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    set_policy(&w, RUN_AS_SYSTEM).await;
    let close = close_due(&w, 60).await;
    let task = sequent_core::types::scheduled_event::generate_manage_date_task_name(
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&w.post.to_string()),
        &sequent_core::types::scheduled_event::EventProcessors::END_VOTING_PERIOD,
    );
    w.execute(
        "UPDATE sequent_backend.scheduled_event SET task_id=$2 WHERE id=$1",
        &[&close, &task],
    )
    .await;
    approve(&w, None, 2, "officer").await;
    set_due(&w, close, -5).await;
    assert_eq!(
        kind(&outcome_of(&w, close).await),
        ScheduledOutcomeKind::RunsUnsigned
    );
    assert_eq!(
        cast_window_end(&w, w.post).await,
        Some(signed_row_date(&w, close).await)
    );
}

#[tokio::test]
async fn optional_live_close_keeps_the_legacy_ballot_deadline() {
    let w = world("cast-optional-live-close").await;
    let close = close_due(&w, 60).await;
    let task = sequent_core::types::scheduled_event::generate_manage_date_task_name(
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&w.post.to_string()),
        &sequent_core::types::scheduled_event::EventProcessors::END_VOTING_PERIOD,
    );
    w.execute(
        "UPDATE sequent_backend.scheduled_event SET task_id=$2 WHERE id=$1",
        &[&close, &task],
    )
    .await;
    assert_eq!(
        cast_window_end(&w, w.post).await,
        Some(signed_row_date(&w, close).await)
    );
}

#[tokio::test]
async fn a_forged_kept_snapshot_cannot_authorize_a_live_ballot_deadline() {
    let w = world("cast-forged-close-policy").await;
    require(&w, SigningAction::CloseVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    let close = close_due(&w, 60).await;
    let task = sequent_core::types::scheduled_event::generate_manage_date_task_name(
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&w.post.to_string()),
        &sequent_core::types::scheduled_event::EventProcessors::END_VOTING_PERIOD,
    );
    w.execute(
        "UPDATE sequent_backend.scheduled_event SET task_id=$2 WHERE id=$1",
        &[&close, &task],
    )
    .await;
    let signed = signed_row_date(&w, close).await;
    approve(&w, None, 2, "officer").await;
    set_policy(&w, RUN_AS_SYSTEM).await;
    let forged = serde_json::to_value(LifecycleSnapshotLoose::run_as_system()).unwrap();
    w.execute(
        "UPDATE sequent_backend.lifecycle_snapshot SET snapshot=$2 WHERE election_event_id=$1",
        &[&w.event, &forged],
    )
    .await;
    set_due(&w, close, -5).await;
    assert_eq!(cast_window_end(&w, w.post).await, Some(signed.clone()));
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let closes = windmill::postgres::election::get_display_voting_closes(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &[w.post.to_string()],
    )
    .await
    .unwrap();
    assert_eq!(closes[&w.post.to_string()].scheduled_at, Some(signed));
    let views = lifecycle_snapshots(&tx, w.tenant, w.event).await.unwrap();
    assert!(views.iter().all(|view| view.snapshot.close_voting.required));
    let state = windmill::services::scheduled_outcome::fire_time_state(
        &tx,
        w.tenant,
        w.event,
        &close.to_string(),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        state
            .0
            .explain(
                &state.1,
                Some(w.post),
                windmill::services::scheduled_outcome::Moment::FireTime
            )
            .outcome,
        ScheduledOutcomeKind::Refused
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        kind(&outcome_of(&w, close).await),
        ScheduledOutcomeKind::Refused
    );
}

#[tokio::test]
async fn an_unsigned_refused_close_has_no_ballot_or_display_deadline() {
    let w = world("cast-no-authorized-close").await;
    require(&w, SigningAction::CloseVoting, 2).await;
    let close = close_due(&w, 60).await;
    let task = sequent_core::types::scheduled_event::generate_manage_date_task_name(
        &w.tenant.to_string(),
        &w.event.to_string(),
        Some(&w.post.to_string()),
        &sequent_core::types::scheduled_event::EventProcessors::END_VOTING_PERIOD,
    );
    w.execute(
        "UPDATE sequent_backend.scheduled_event SET task_id=$2 WHERE id=$1",
        &[&close, &task],
    )
    .await;
    assert_eq!(cast_window_end(&w, w.post).await, None);
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let closes = windmill::postgres::election::get_display_voting_closes(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &[w.post.to_string()],
    )
    .await
    .unwrap();
    assert!(closes.contains_key(&w.post.to_string()));
    assert_eq!(closes[&w.post.to_string()].scheduled_at, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn completed_policy_subject_is_authoritative_and_invalid_signed_references_refuse() {
    let w = world("cast-completed-policy-reference").await;
    require(&w, SigningAction::CloseVoting, 2).await;
    require(&w, SigningAction::ApproveConfiguration, 2).await;
    set_policy(&w, RUN_AS_SYSTEM).await;
    let (request, _) = approve(&w, None, 2, "officer").await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.signing_request SET status='completed' WHERE id=$1",
        &[&request],
    )
    .await
    .unwrap();
    let allowed = || async {
        tx.query_one(
            "SELECT sequent_backend.live_voting_close_allowed($1,$2,$3)",
            &[&w.tenant, &w.event, &w.post],
        )
        .await
        .unwrap()
        .get::<_, bool>(0)
    };
    assert!(
        allowed().await,
        "the validated publication effect phase reads its completed subject"
    );
    tx.execute("UPDATE sequent_backend.lifecycle_snapshot SET ballot_publication_id=gen_random_uuid() WHERE approval_request_id=$1", &[&request]).await.unwrap();
    assert!(
        !allowed().await,
        "a different publication cannot use the kept policy fallback"
    );
    tx.execute("UPDATE sequent_backend.lifecycle_snapshot SET ballot_publication_id=(SELECT (subject->>'ballot_publication_id')::uuid FROM sequent_backend.signing_request WHERE id=$1) WHERE approval_request_id=$1", &[&request]).await.unwrap();
    assert!(allowed().await);
    tx.execute(
        "UPDATE sequent_backend.signing_request SET scope_key='|||wrong-target' WHERE id=$1",
        &[&request],
    )
    .await
    .unwrap();
    assert!(
        !allowed().await,
        "a different target cannot use the kept policy fallback"
    );
    tx.execute("UPDATE sequent_backend.signing_request SET scope_key='|||event', subject=jsonb_set(subject,'{ballot_publication_id}','\"invalid\"') WHERE id=$1", &[&request]).await.unwrap();
    assert!(
        !allowed().await,
        "a malformed subject publication refuses predictably"
    );
    tx.execute("UPDATE sequent_backend.lifecycle_snapshot SET approval_request_id=gen_random_uuid() WHERE approval_request_id=$1", &[&request]).await.unwrap();
    assert!(
        !allowed().await,
        "a missing approval cannot use the kept policy fallback"
    );
    tx.rollback().await.unwrap();
}
