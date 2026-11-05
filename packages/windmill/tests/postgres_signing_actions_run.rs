// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! How a signed protected action runs once its last signature arrives: it
//! is dispatched once; its task claims it with a token, checks the state
//! is as signed, acts, and reports. A refusal or a failure after acting
//! fails the request; a failure before acting leaves it for the sweeper. A
//! copy of the task holding an old token does nothing. Closing keeps the
//! closing signatures as its seal record, feeds it to the seal hook after
//! the close in the same transaction, and cancels a waiting open.
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

use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::signing::{CancelReason, RequesterSigning, SigningAction, SigningRequestStatus};
use serde_json::{json, Value};
use signing::*;
use signing_actions::*;
use windmill::postgres::signing::release_signing_execution;
use windmill::services::signing::actions::{
    claim, run_claimed, run_dispatched, NoSeal, ProductionEffects, RunOutcome, EFFECT_PARTIAL,
};
use windmill::services::signing::requests::redispatch_unexecuted_requests;

/// `redispatch_unexecuted_requests` sweeps every tenant. The tests that
/// call it backdate their own request's `completed_at` first, which makes
/// it a candidate for the other's sweep, so they take turns.
static REDISPATCH: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Two configurations: a Post label and how many sign.
const PRESETS: [(&str, u16); 2] = [("madrid-pe", 2), ("faculty-of-science", 3)];

/// A world whose Post's channels are all open, with a close voting rule.
async fn open_post(label: &str, required: u16) -> World {
    let w = world(label).await;
    set_post_status(
        &w,
        json!({"voting_status": "OPEN", "kiosk_voting_status": "OPEN",
               "telephone_voting_status": "OPEN", "early_voting_status": "OPEN"}),
    )
    .await;
    w.rule(
        SigningAction::CloseVoting,
        required,
        RequesterSigning::NotAllowed,
        Some(60),
    )
    .await;
    w
}

async fn close_online(w: &World) -> uuid::Uuid {
    waiting_id(
        status_gate(
            w,
            &starter(w),
            VotingStatus::CLOSED,
            Some(vec![VotingStatusChannel::ONLINE]),
        )
        .await,
    )
}

#[tokio::test]
async fn closing_runs_once_with_its_last_signature_and_seals_the_closing_signatures() {
    for (label, required) in PRESETS {
        let w = open_post(label, required).await;
        w.rule(
            SigningAction::OpenVoting,
            1,
            RequesterSigning::NotAllowed,
            Some(60),
        )
        .await;
        // An open of another channel waits too; closing cancels it.
        set_post_status(
            &w,
            json!({"voting_status": "OPEN", "kiosk_voting_status": "PAUSED"}),
        )
        .await;
        let open = waiting_id(
            status_gate(
                &w,
                &starter(&w),
                VotingStatus::OPEN,
                Some(vec![VotingStatusChannel::KIOSK]),
            )
            .await,
        );
        let id = close_online(&w).await;
        assert_eq!(
            w.request(id).await.subject,
            json!({"channels": ["ONLINE"], "from": ["ONLINE=OPEN"]})
        );

        let dispatched = Dispatched::default();
        let signers = signers(
            &w,
            SigningAction::CloseVoting,
            usize::from(required),
            "sbei",
        )
        .await;
        let services = services(
            FakeVerifier::knowing(&signers.iter().map(|(_, c)| c).collect::<Vec<_>>()),
            windmill::services::signing::actions::executors(std::sync::Arc::new(
                dispatched.clone(),
            )),
        );
        let (last, first) = signers.split_last().unwrap();
        for (signer, certificate) in first {
            w.sign(&services, signer, id, certificate, at(1))
                .await
                .unwrap();
        }
        assert!(
            dispatched.tasks().is_empty(),
            "k-1 signatures dispatch nothing"
        );
        let outcome = w
            .sign(&services, &last.0, id, &last.1, at(2))
            .await
            .unwrap();
        assert_eq!(outcome.status, SigningRequestStatus::Completed);
        let tasks = dispatched.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(task_status(&w, &tasks[0]).await, "IN_PROGRESS");

        let effects = FakeEffects::new(Effect::Succeed);
        let seal = RecordingSeal::default();
        let mut client = w.pool.get().await.unwrap();
        let result = match run_dispatched(&mut client, &effects, &seal, &tasks[0])
            .await
            .unwrap()
        {
            RunOutcome::Executed(result) => result,
            other => panic!("expected the action to run, got {other:?}"),
        };
        // A second copy of the task can't claim it.
        assert_eq!(
            run_dispatched(&mut client, &effects, &seal, &tasks[0])
                .await
                .unwrap(),
            RunOutcome::NotClaimed
        );
        assert_eq!(effects.runs(), 1);

        let request = w.request(id).await;
        assert_eq!(request.status, SigningRequestStatus::Executed);
        assert_eq!(request.execution_result.as_ref(), Some(&result));
        let signatures = result["signatures"].as_array().unwrap();
        assert_eq!(signatures.len(), usize::from(required));
        let mut fingerprints: Vec<&str> = signatures
            .iter()
            .map(|s| s["certificate_fingerprint"].as_str().unwrap())
            .collect();
        fingerprints.sort();
        let mut expected: Vec<&str> = signers
            .iter()
            .map(|(_, c)| c.identity.fingerprint_sha256.as_str())
            .collect();
        expected.sort();
        assert_eq!(fingerprints, expected);
        assert_eq!(result["code"], json!(request.code));
        assert_eq!(result["channels"], json!(["ONLINE"]));
        assert_eq!(result["from"], json!(["ONLINE=OPEN"]));
        assert!(result["closed_at"].is_string());
        // One summary per country seal, each signed by the closing
        // signatures the hook was fed; no single seal of the Post.
        let signed_by = sealed_by(usize::from(required), &request.code);
        let expected_seals: Vec<Value> = sealed_countries()
            .into_iter()
            .map(|(area_id, area_name, hash, ballots)| {
                json!({
                    "area_id": area_id.to_string(),
                    "area_name": area_name,
                    "hash_algorithm": "SHA-512",
                    "hash": hash,
                    "ballots": ballots,
                    "signed_by": signed_by,
                })
            })
            .collect();
        assert_eq!(result["seals"], Value::Array(expected_seals));
        assert!(result.get("seal").is_none());
        {
            let fed = seal.0.lock().unwrap();
            assert_eq!(fed.len(), 1);
            let (record, status) = &fed[0];
            assert_eq!(record.signatures.len(), usize::from(required));
            // The hook runs after the close, in its transaction: it sees
            // the channel closed.
            assert_eq!(
                status
                    .as_ref()
                    .and_then(|status| status.get("voting_status")),
                Some(&json!("CLOSED"))
            );
        }
        assert_eq!(task_status(&w, &tasks[0]).await, "SUCCESS");

        // The log's SigningActionExecuted entry carries the seal record and
        // its seals.
        let logged: Value = client
            .query_one(
                "SELECT body FROM sequent_backend.signing_log_outbox
                 WHERE election_event_id = $1 AND statement_kind = 'SigningActionExecuted'
                     AND event_type = 'SYSTEM'",
                &[&w.event],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(
            logged["details"]["result"]["signatures"]
                .as_array()
                .map(Vec::len),
            Some(usize::from(required))
        );
        assert_eq!(logged["details"]["result"]["seals"], result["seals"]);
        // Closing cancelled the waiting open of the Post.
        let open = w.request(open).await;
        assert_eq!(open.status, SigningRequestStatus::Cancelled);
        assert_eq!(open.cancel_reason, Some(CancelReason::PayloadChanged));
        w.assert_two_entries_per_step().await;
    }
}

#[tokio::test]
async fn without_a_seal_hook_the_close_records_no_seals() {
    let w = open_post("unsealed-post", 1).await;
    let id = close_online(&w).await;
    let dispatched = Dispatched::default();
    sign_all(&w, SigningAction::CloseVoting, id, 1, "closer", &dispatched).await;
    let mut client = w.pool.get().await.unwrap();
    let result = match run_dispatched(
        &mut client,
        &FakeEffects::new(Effect::Succeed),
        &NoSeal,
        &dispatched.tasks()[0],
    )
    .await
    .unwrap()
    {
        RunOutcome::Executed(result) => result,
        other => panic!("expected the action to run, got {other:?}"),
    };
    // The record still lists the closing signature; there is no seal yet.
    assert_eq!(result["signatures"].as_array().map(Vec::len), Some(1));
    assert_eq!(result["seals"], json!([]));
    assert!(result.get("seal").is_none());
    assert_eq!(
        post_status(&w)
            .await
            .and_then(|status| status.get("voting_status").cloned()),
        Some(json!("CLOSED"))
    );
}

#[tokio::test]
async fn a_failure_before_acting_is_tried_again_and_a_running_retry_is_in_progress() {
    let _turn = REDISPATCH.lock().await;
    let w = open_post("retry-post", 1).await;
    let id = close_online(&w).await;
    let dispatched = Dispatched::default();
    let services = sign_all(&w, SigningAction::CloseVoting, id, 1, "sbei", &dispatched).await;
    let task = dispatched.tasks()[0];

    let mut client = w.pool.get().await.unwrap();
    let failing = FakeEffects::new(Effect::FailBefore);
    assert!(matches!(
        run_dispatched(&mut client, &failing, &RecordingSeal::default(), &task)
            .await
            .unwrap(),
        RunOutcome::Retry(_)
    ));
    assert_eq!(w.request(id).await.status, SigningRequestStatus::Completed);
    assert_eq!(task_status(&w, &task).await, "FAILED");
    let description: Option<String> = client
        .query_one(
            "SELECT description FROM sequent_backend.election WHERE id = $1",
            &[&w.post],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(description, None, "the effect's write was undone");

    // The sweeper sends the same task again; the claim marks it running.
    w.execute(
        "UPDATE sequent_backend.signing_request
         SET completed_at = clock_timestamp() - interval '10 minutes' WHERE id = $1",
        &[&id],
    )
    .await;
    assert_eq!(
        redispatch_unexecuted_requests(&mut client, &services.executors)
            .await
            .unwrap(),
        1
    );
    assert_eq!(dispatched.tasks(), vec![task, task]);
    let token = claim(&mut client, &task).await.unwrap().expect("a claim");
    assert_eq!(task_status(&w, &task).await, "IN_PROGRESS");
    let effects = FakeEffects::new(Effect::Succeed);
    assert!(matches!(
        run_claimed(
            &mut client,
            &effects,
            &RecordingSeal::default(),
            &task,
            token
        )
        .await
        .unwrap(),
        RunOutcome::Executed(_)
    ));
    assert_eq!(w.request(id).await.status, SigningRequestStatus::Executed);
    assert_eq!(effects.runs(), 1);
}

#[tokio::test]
async fn a_failure_after_acting_or_a_refusal_fails_the_request_for_a_person() {
    let _turn = REDISPATCH.lock().await;
    for (effect, code) in [
        (Effect::FailAfter, EFFECT_PARTIAL),
        (Effect::Refuse, "state-changed"),
    ] {
        let w = open_post("failing-post", 1).await;
        let id = close_online(&w).await;
        let dispatched = Dispatched::default();
        let services = sign_all(&w, SigningAction::CloseVoting, id, 1, "sbei", &dispatched).await;
        let task = dispatched.tasks()[0];
        let mut client = w.pool.get().await.unwrap();
        assert_eq!(
            run_dispatched(
                &mut client,
                &FakeEffects::new(effect),
                &RecordingSeal::default(),
                &task
            )
            .await
            .unwrap(),
            RunOutcome::Failed(code.to_owned())
        );
        let request = w.request(id).await;
        assert_eq!(request.status, SigningRequestStatus::Failed);
        assert_eq!(
            request.execution_result.unwrap()["error"]["code"],
            json!(code)
        );
        assert_eq!(task_status(&w, &task).await, "FAILED");
        // Never tried again.
        w.execute(
            "UPDATE sequent_backend.signing_request
             SET completed_at = clock_timestamp() - interval '10 minutes' WHERE id = $1",
            &[&id],
        )
        .await;
        assert_eq!(
            redispatch_unexecuted_requests(&mut client, &services.executors)
                .await
                .unwrap(),
            0
        );
        let logged: String = client
            .query_one(
                "SELECT log_type FROM sequent_backend.signing_log_outbox
                 WHERE election_event_id = $1 AND statement_kind = 'SigningActionExecuted'
                     AND event_type = 'SYSTEM'",
                &[&w.event],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(logged, "ERROR");
    }
}

#[tokio::test]
async fn a_copy_holding_an_old_token_does_nothing() {
    let w = open_post("token-post", 1).await;
    let id = close_online(&w).await;
    let dispatched = Dispatched::default();
    sign_all(&w, SigningAction::CloseVoting, id, 1, "sbei", &dispatched).await;
    let task = dispatched.tasks()[0];
    let mut client = w.pool.get().await.unwrap();

    // Copy A claims; its lease is given up (the sweeper releases a failed
    // or stale claim), and copy B claims again.
    let old = claim(&mut client, &task).await.unwrap().expect("a claim");
    {
        let tx = client.transaction().await.unwrap();
        release_signing_execution(&tx, w.tenant, w.event, id)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let new = claim(&mut client, &task)
        .await
        .unwrap()
        .expect("a second claim");
    assert_ne!(old, new);
    let effects = FakeEffects::new(Effect::Succeed);
    assert_eq!(
        run_claimed(&mut client, &effects, &RecordingSeal::default(), &task, old)
            .await
            .unwrap(),
        RunOutcome::NotClaimed
    );
    assert_eq!(effects.runs(), 0);
    assert!(matches!(
        run_claimed(&mut client, &effects, &RecordingSeal::default(), &task, new)
            .await
            .unwrap(),
        RunOutcome::Executed(_)
    ));
    assert_eq!(effects.runs(), 1);
}

#[tokio::test]
async fn an_effect_that_committed_is_only_reported_again() {
    let w = open_post("committed-post", 1).await;
    let id = close_online(&w).await;
    let dispatched = Dispatched::default();
    sign_all(&w, SigningAction::CloseVoting, id, 1, "sbei", &dispatched).await;
    let task = dispatched.tasks()[0];
    let mut client = w.pool.get().await.unwrap();
    let token = claim(&mut client, &task).await.unwrap().unwrap();
    // The effect committed its result; the task died before reporting.
    let kept = json!({"channels": ["ONLINE"], "from": ["ONLINE=OPEN"], "kept": true});
    w.execute(
        "UPDATE sequent_backend.signing_request SET execution_result = $2 WHERE id = $1",
        &[&id, &kept],
    )
    .await;
    let effects = FakeEffects::new(Effect::Succeed);
    assert_eq!(
        run_claimed(
            &mut client,
            &effects,
            &RecordingSeal::default(),
            &task,
            token
        )
        .await
        .unwrap(),
        RunOutcome::Executed(kept.clone())
    );
    assert_eq!(effects.runs(), 0);
    assert_eq!(w.request(id).await.execution_result, Some(kept));
}

/// Today's status change: it checks what was signed before acting, and this
/// event has no bulletin board, so the change itself fails after it began.
#[tokio::test]
async fn the_status_change_refuses_a_changed_post_and_fails_after_acting() {
    let w = open_post("production-post", 1).await;
    let changed = close_online(&w).await;
    let dispatched = Dispatched::default();
    sign_all(
        &w,
        SigningAction::CloseVoting,
        changed,
        1,
        "first",
        &dispatched,
    )
    .await;
    // Someone paused online voting meanwhile.
    set_post_status(&w, json!({"voting_status": "PAUSED"})).await;
    let mut client = w.pool.get().await.unwrap();
    let effects = ProductionEffects::default();
    assert_eq!(
        run_dispatched(
            &mut client,
            &effects,
            &RecordingSeal::default(),
            &dispatched.tasks()[0]
        )
        .await
        .unwrap(),
        RunOutcome::Failed("state-changed".into())
    );
    assert_eq!(
        post_status(&w).await,
        Some(json!({"voting_status": "PAUSED"}))
    );

    let id = close_online(&w).await;
    let dispatched = Dispatched::default();
    sign_all(&w, SigningAction::CloseVoting, id, 1, "second", &dispatched).await;
    assert_eq!(
        run_dispatched(
            &mut client,
            &effects,
            &RecordingSeal::default(),
            &dispatched.tasks()[0]
        )
        .await
        .unwrap(),
        RunOutcome::Failed(EFFECT_PARTIAL.into())
    );
    // The status write rolled back with the effect.
    assert_eq!(
        post_status(&w).await,
        Some(json!({"voting_status": "PAUSED"}))
    );
}
