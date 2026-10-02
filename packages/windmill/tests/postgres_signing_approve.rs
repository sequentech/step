// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The approve step (design §5): a request runs its action once, with its
//! last signature and never before it; refusals are logged and committed;
//! one person fills one slot (§5a); executor failures keep the signatures;
//! trustee gates; expiry and the sweeper.
//!
//! Each test commits its own tenant, since the approve step owns its
//! transactions. Rule numbers and labels come from the configuration each
//! test uses, and the tests run under two of them.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;

use async_trait::async_trait;
use sequent_core::signing::{
    CertificateCheckId, RequesterSigning, SigningAction, SigningRequestStatus,
};
use sequent_core::types::permissions::Permissions;
use serde_json::json;
use signing::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;
use windmill::postgres::signing::*;
use windmill::services::signing::executors::{
    ExecutionOutcome, PostCommit, PostCommitTask, SigningExecutor,
};
use windmill::services::signing::guard::consume_gate;
use windmill::services::signing::requests::{
    cancel, claim_dispatched, expire_overdue_requests, finish_dispatched,
    redispatch_unexecuted_requests, DispatchedResult, EXECUTION_LEASE_SECONDS,
    MAX_EXECUTION_ATTEMPTS,
};
use windmill::services::signing::{SigningCaller, SigningError};

const ACTION: SigningAction = SigningAction::CloseVoting;

/// Two configurations: a Post label and how many sign.
const PRESETS: [(&str, u16); 2] = [("madrid-pe", 3), ("faculty-of-science", 2)];

fn fake(behaviour: FakeBehaviour) -> Arc<FakeExecutor> {
    Arc::new(FakeExecutor::new(ACTION, behaviour))
}

/// `n` signers with their own registered certificates.
async fn signers(w: &World, n: usize) -> Vec<(SigningCaller, TestCert)> {
    let mut signers = vec![];
    for i in 0..n {
        let user = format!("sbei-{i}");
        let certificate = cert(&user, &user, &user);
        w.register(&user, &certificate, None).await;
        signers.push((w.signer(&user, ACTION), certificate));
    }
    signers
}

fn verifier_for(signers: &[(SigningCaller, TestCert)]) -> FakeVerifier {
    FakeVerifier::knowing(&signers.iter().map(|(_, c)| c).collect::<Vec<_>>())
}

/// `expire_overdue_requests` expires every tenant's overdue requests. The
/// tests that make a request overdue take turns with it, so it never
/// expires a request another test is about to expire itself.
static EXPIRY: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn refused(result: Result<impl std::fmt::Debug, SigningError>) -> CertificateCheckId {
    match result {
        Err(SigningError::Refused { check, .. }) => check,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[tokio::test]
async fn the_last_signature_runs_the_action_once_and_none_before_it() {
    for (label, required) in PRESETS {
        let w = world(label).await;
        w.rule(ACTION, required, RequesterSigning::NotAllowed, None)
            .await;
        let signers = signers(&w, usize::from(required)).await;
        let executor = fake(FakeBehaviour::Succeed(json!({"closed": true})));
        let services = services(verifier_for(&signers), vec![executor.clone()]);
        let requester = caller("operator", &[], &[&w.label]);
        let request = w.start(&requester, ACTION, subject(1), at(0)).await;
        assert_eq!(request.required, i32::from(required), "{label}");

        for (i, (signer, certificate)) in signers.iter().enumerate() {
            let outcome = w
                .sign(&services, signer, request.id, certificate, at(1))
                .await
                .unwrap();
            let k = i as i64 + 1;
            assert_eq!(outcome.count, k, "{label}");
            assert_eq!(outcome.required, i32::from(required));
            if k < i64::from(required) {
                assert_eq!(outcome.status, SigningRequestStatus::Waiting, "{label}");
                assert!(executor.runs().is_empty(), "{label}: ran at {k}");
            } else {
                assert_eq!(outcome.status, SigningRequestStatus::Executed, "{label}");
            }
        }
        assert_eq!(
            executor.runs(),
            vec![(request.id, usize::from(required))],
            "{label}"
        );
        // The Logs table: the action is run by its last signer.
        let last = &signers.last().unwrap().0;
        let executed = w.entries("SigningActionExecuted").await;
        assert_eq!(executed.len(), 1);
        assert_eq!(executed[0].0.as_deref(), Some(last.user_id.as_str()));
        let row = w.request(request.id).await;
        assert_eq!(row.status, SigningRequestStatus::Executed);
        assert_eq!(row.execution_result, Some(json!({"closed": true})));
        assert!(row.completed_at.is_some() && row.executed_at.is_some());

        // A signature after the action ran is refused and runs nothing.
        let late = w.signer("late", ACTION);
        let late_certificate = cert("late", "late", "late");
        w.register("late", &late_certificate, None).await;
        let services = services_with(&signers, &late_certificate, executor.clone());
        assert!(matches!(
            w.sign(&services, &late, request.id, &late_certificate, at(2))
                .await,
            Err(SigningError::Closed { .. })
        ));
        assert_eq!(executor.runs().len(), 1);

        let mut expected = vec!["SigningRequestCreated".to_string()];
        expected.extend(vec![
            "SigningRequestSigned".to_string();
            usize::from(required)
        ]);
        expected.push("SigningRequestCompleted".into());
        expected.push("SigningActionExecuted".into());
        assert_eq!(w.steps().await, expected, "{label}");
        w.assert_two_entries_per_step().await;
        let described: Vec<String> = w.outbox().await.into_iter().map(|row| row.5).collect();
        let code = &request.code;
        assert!(described.contains(&format!(
            "Started signing request {code} to close voting: {required} signatures needed"
        )));
        assert!(described.contains(&format!(
            "Signature verified on {code}: {required} of {required}"
        )));
        assert!(described.contains(&format!("Ran Close voting for signing request {code}")));
    }
}

fn services_with(
    signers: &[(SigningCaller, TestCert)],
    extra: &TestCert,
    executor: Arc<FakeExecutor>,
) -> windmill::services::signing::approve::SigningServices {
    let mut certificates: Vec<&TestCert> = signers.iter().map(|(_, c)| c).collect();
    certificates.push(extra);
    services(FakeVerifier::knowing(&certificates), vec![executor])
}

#[tokio::test]
async fn two_concurrent_final_signatures_run_the_action_once() {
    for (label, required) in PRESETS {
        let w = world(label).await;
        w.rule(ACTION, required, RequesterSigning::NotAllowed, None)
            .await;
        // One signer more than needed: the last two sign at the same time.
        let signers = signers(&w, usize::from(required) + 1).await;
        let executor = fake(FakeBehaviour::Succeed(json!({})));
        let services = services(verifier_for(&signers), vec![executor.clone()]);
        let request = w
            .start(&caller("operator", &[], &[]), ACTION, subject(2), at(0))
            .await;
        let (first, last_two) = signers.split_at(usize::from(required) - 1);
        for (signer, certificate) in first {
            w.sign(&services, signer, request.id, certificate, at(1))
                .await
                .unwrap();
        }
        let (a, b) = tokio::join!(
            w.sign(&services, &last_two[0].0, request.id, &last_two[0].1, at(1)),
            w.sign(&services, &last_two[1].0, request.id, &last_two[1].1, at(1)),
        );
        let outcomes = [a, b];
        let completed = outcomes.iter().filter(|outcome| outcome.is_ok()).count();
        assert_eq!(completed, 1, "{label}: {outcomes:?}");
        assert!(
            outcomes
                .iter()
                .any(|outcome| matches!(outcome, Err(SigningError::Closed { .. }))),
            "{label}: {outcomes:?}"
        );
        assert_eq!(executor.runs().len(), 1, "{label}");
        assert_eq!(w.approvals(request.id).await.len(), usize::from(required));
        assert_eq!(
            w.request(request.id).await.status,
            SigningRequestStatus::Executed
        );
    }
}

#[tokio::test]
async fn expired_and_cancelled_requests_refuse_signatures_and_theirs_do_not_carry_over() {
    let _turn = EXPIRY.lock().await;
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::NotAllowed, Some(30))
        .await;
    let signers = signers(&w, 2).await;
    let executor = fake(FakeBehaviour::Succeed(json!({})));
    let services = services(verifier_for(&signers), vec![executor.clone()]);
    let requester = caller("operator", &[], &[]);

    // Expired: one signature in time, the next one after its time.
    let expiring = w.start(&requester, ACTION, subject(3), at(0)).await;
    assert_eq!(expiring.expires_at, Some(at(30)));
    w.sign(&services, &signers[0].0, expiring.id, &signers[0].1, at(1))
        .await
        .unwrap();
    w.make_overdue(expiring.id).await;
    assert!(matches!(
        w.sign(&services, &signers[1].0, expiring.id, &signers[1].1, at(30))
            .await,
        Err(SigningError::Closed { .. })
    ));
    let row = w.request(expiring.id).await;
    assert_eq!(row.status, SigningRequestStatus::Expired);
    assert_eq!(w.approvals(expiring.id).await.len(), 1);

    // A new request for the same scope starts from zero.
    let again = w.start(&requester, ACTION, subject(3), at(31)).await;
    assert_ne!(again.id, expiring.id);
    let outcome = w
        .sign(&services, &signers[0].0, again.id, &signers[0].1, at(32))
        .await
        .unwrap();
    assert_eq!(
        (outcome.count, outcome.status),
        (1, SigningRequestStatus::Waiting)
    );

    // Cancelled by its requester: refuses the next signature.
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let cancelled = cancel(&tx, &requester, w.tenant, again.id, Some("wrong channel"))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        cancelled.cancel_reason,
        Some(sequent_core::signing::CancelReason::ByRequester)
    );
    assert!(matches!(
        w.sign(&services, &signers[1].0, again.id, &signers[1].1, at(33))
            .await,
        Err(SigningError::Closed { .. })
    ));
    let fresh = w.start(&requester, ACTION, subject(3), at(34)).await;
    let outcome = w
        .sign(&services, &signers[1].0, fresh.id, &signers[1].1, at(35))
        .await
        .unwrap();
    assert_eq!(outcome.count, 1);
    assert!(executor.runs().is_empty());
    w.assert_two_entries_per_step().await;
    assert!(w
        .steps()
        .await
        .contains(&"SigningRequestExpired".to_string()));
    assert!(w
        .steps()
        .await
        .contains(&"SigningRequestCancelled".to_string()));
}

#[tokio::test]
async fn the_requester_signs_only_when_the_rule_allows_it() {
    for (policy, allowed) in [
        (RequesterSigning::NotAllowed, false),
        (RequesterSigning::Allowed, true),
    ] {
        let w = world("madrid-pe").await;
        w.rule(ACTION, 2, policy, None).await;
        let signers = signers(&w, 1).await;
        let (requester, certificate) = &signers[0];
        let services = services(
            verifier_for(&signers),
            vec![fake(FakeBehaviour::Succeed(json!({})))],
        );
        let request = w.start(requester, ACTION, subject(4), at(0)).await;
        let outcome = w
            .sign(&services, requester, request.id, certificate, at(1))
            .await;
        match (allowed, outcome) {
            (true, Ok(outcome)) => assert_eq!(outcome.count, 1),
            (false, Err(SigningError::Forbidden(_))) => {
                assert!(w.approvals(request.id).await.is_empty())
            }
            (_, other) => panic!("{policy}: {other:?}"),
        }
    }
}

#[tokio::test]
async fn one_person_fills_one_slot_of_a_request() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 3, RequesterSigning::NotAllowed, None).await;
    let alice_certificate = cert("alice", "alice-key", "Alice");
    w.register("alice", &alice_certificate, None).await;
    // A Security Officer linked Alice's certificate, a reissue of her key,
    // and a second certificate issued to her, to her other accounts.
    let reissued = cert("alice-reissued", "alice-key", "Alice (reissued)");
    let second = cert("alice-second", "alice-other-key", "Alice");
    w.register("alice-trustee", &alice_certificate, Some("alice"))
        .await;
    w.register("alice-2", &reissued, Some("alice")).await;
    w.register("alice-3", &second, Some("alice")).await;
    let services = services(
        FakeVerifier::knowing(&[&alice_certificate, &reissued, &second]),
        vec![fake(FakeBehaviour::Succeed(json!({})))],
    );
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(5), at(0))
        .await;
    let alice = w.signer("alice", ACTION);
    w.sign(&services, &alice, request.id, &alice_certificate, at(1))
        .await
        .unwrap();

    // The same account again.
    assert_eq!(
        refused(
            w.sign(&services, &alice, request.id, &alice_certificate, at(2))
                .await
        ),
        CertificateCheckId::AlreadySigned
    );
    // Her certificate from another account; a reissued certificate of her
    // key; a second certificate with her name.
    for (account, certificate) in [
        ("alice-trustee", &alice_certificate),
        ("alice-2", &reissued),
        ("alice-3", &second),
    ] {
        assert_eq!(
            refused(
                w.sign(
                    &services,
                    &w.signer(account, ACTION),
                    request.id,
                    certificate,
                    at(3)
                )
                .await
            ),
            CertificateCheckId::AlreadySigned,
            "{account}"
        );
    }
    assert_eq!(w.approvals(request.id).await.len(), 1);
    let row = w.request(request.id).await;
    assert_eq!(row.status, SigningRequestStatus::Waiting);
    // Each refusal is logged with its ERROR entry and committed.
    let refusals: Vec<_> = w
        .outbox()
        .await
        .into_iter()
        .filter(|row| row.1 == "SigningSignatureRefused")
        .collect();
    assert_eq!(refusals.len(), 8);
    assert!(refusals
        .iter()
        .filter(|row| row.2 == "SYSTEM")
        .all(|row| row.3 == "ERROR"));
    w.assert_two_entries_per_step().await;
}

#[tokio::test]
async fn only_signers_of_the_post_with_the_permission_sign() {
    for (label, other) in [
        ("madrid-pe", "tokyo-pe"),
        ("faculty-of-science", "faculty-of-law"),
    ] {
        let w = world(label).await;
        w.rule(ACTION, 2, RequesterSigning::NotAllowed, None).await;
        let certificate = cert("jose", "jose", "Jose");
        w.register("jose", &certificate, None).await;
        let services = services(
            FakeVerifier::knowing(&[&certificate]),
            vec![fake(FakeBehaviour::Succeed(json!({})))],
        );
        let request = w
            .start(&caller("operator", &[], &[]), ACTION, subject(6), at(0))
            .await;
        let outside = caller("jose", &[ACTION.sign_permission()], &[other]);
        let wrong_permission = caller(
            "jose",
            &[SigningAction::OpenVoting.sign_permission()],
            &[label],
        );
        // Without labels a signer sees no labelled Post, as in Hasura, so
        // signs for none.
        let unlabeled = caller("jose", &[ACTION.sign_permission()], &[]);
        for denied in [&outside, &wrong_permission, &unlabeled] {
            assert!(matches!(
                w.sign(&services, denied, request.id, &certificate, at(1))
                    .await,
                Err(SigningError::Forbidden(_))
            ));
        }
        let of_the_post = caller("jose", &[ACTION.sign_permission()], &[label]);
        assert_eq!(
            w.sign(&services, &of_the_post, request.id, &certificate, at(1))
                .await
                .unwrap()
                .count,
            1
        );
    }
}

#[tokio::test]
async fn certificate_refusals_are_logged_committed_and_answered_with_their_check() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::NotAllowed, None).await;
    let signers = signers(&w, 1).await;
    let (maria, certificate) = &signers[0];
    let unregistered = cert("jose", "jose", "Jose");
    let services = services(
        FakeVerifier::knowing(&[certificate, &unregistered])
            .refusing(certificate, CertificateCheckId::NotRevoked),
        vec![fake(FakeBehaviour::Succeed(json!({})))],
    );
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(7), at(0))
        .await;
    assert_eq!(
        refused(
            w.sign(&services, maria, request.id, certificate, at(1))
                .await
        ),
        CertificateCheckId::NotRevoked
    );
    assert_eq!(
        refused(
            w.sign(
                &services,
                &w.signer("jose", ACTION),
                request.id,
                &unregistered,
                at(1)
            )
            .await
        ),
        CertificateCheckId::Registered
    );
    // A signature over another payload.
    let good = services_with(
        &signers,
        &unregistered,
        fake(FakeBehaviour::Succeed(json!({}))),
    );
    let mut client = w.pool.get().await.unwrap();
    let wrong = windmill::services::signing::approve::approve(
        &mut client,
        &good,
        maria,
        w.tenant,
        &windmill::services::signing::approve::ApproveInput {
            request_id: request.id,
            chain_pem: vec![certificate.identity.pem.clone()],
            algorithm: sequent_core::signing::SignatureAlgorithm::RsaPkcs1Sha256,
            payload_signature: signature_over("another payload"),
            document_signature: None,
            pdf_cms: None,
            revision: None,
        },
        at(1),
    )
    .await;
    assert_eq!(refused(wrong), CertificateCheckId::Signature);
    assert!(w.approvals(request.id).await.is_empty());
    // Maria's second refusal within the throttle is answered, not logged.
    let steps = w.steps().await;
    assert_eq!(
        steps,
        [
            "SigningRequestCreated",
            "SigningSignatureRefused",
            "SigningSignatureRefused"
        ]
    );
    w.assert_two_entries_per_step().await;
}

#[tokio::test]
async fn a_failing_executor_fails_the_request_and_keeps_its_signatures() {
    for behaviour in [
        FakeBehaviour::Fail("the board is down".into()),
        FakeBehaviour::WriteThenFail("failed after writing".into()),
    ] {
        let w = world("madrid-pe").await;
        w.rule(ACTION, 2, RequesterSigning::NotAllowed, None).await;
        let signers = signers(&w, 2).await;
        let executor = fake(behaviour.clone());
        let services = services(verifier_for(&signers), vec![executor.clone()]);
        let request = w
            .start(&caller("operator", &[], &[]), ACTION, subject(8), at(0))
            .await;
        for (signer, certificate) in &signers {
            w.sign(&services, signer, request.id, certificate, at(1))
                .await
                .unwrap();
        }
        let row = w.request(request.id).await;
        assert_eq!(row.status, SigningRequestStatus::Failed, "{behaviour:?}");
        assert!(row.executed_at.is_none());
        // What the executor wrote before failing was undone.
        assert_eq!(row.open_failures, 0);
        // A stable code, never the executor's own error.
        assert_eq!(
            row.execution_result,
            Some(
                json!({"error": {"code": "execution-failed", "message": "The action could not run."}})
            )
        );
        let logged = serde_json::to_string(
            &w.outbox()
                .await
                .iter()
                .map(|r| r.5.clone())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert!(!logged.contains("board is down") && !logged.contains("after writing"));
        assert_eq!(w.approvals(request.id).await.len(), 2);
        assert_eq!(executor.runs().len(), 1);
        let executed: Vec<_> = w
            .outbox()
            .await
            .into_iter()
            .filter(|row| row.1 == "SigningActionExecuted")
            .map(|row| (row.2, row.3))
            .collect();
        assert_eq!(
            executed,
            [
                ("USER".into(), "INFO".into()),
                ("SYSTEM".into(), "ERROR".into())
            ]
        );
        assert_eq!(
            w.entries("SigningActionExecuted").await[0].0.as_deref(),
            Some(signers[1].0.user_id.as_str())
        );
    }
}

#[tokio::test]
async fn a_deferred_action_without_an_executor_fails() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 1, RequesterSigning::NotAllowed, None).await;
    let signers = signers(&w, 1).await;
    let services = services(verifier_for(&signers), vec![]);
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(9), at(0))
        .await;
    let outcome = w
        .sign(&services, &signers[0].0, request.id, &signers[0].1, at(1))
        .await
        .unwrap();
    assert_eq!(outcome.status, SigningRequestStatus::Failed);
    assert_eq!(
        w.request(request.id).await.execution_result.unwrap()["error"]["code"],
        "no-executor"
    );
}

/// Dispatches a task that counts how often it is sent.
struct Dispatching {
    sent: Arc<AtomicUsize>,
    task_execution_id: Uuid,
}

struct CountingTask(Arc<AtomicUsize>);

#[async_trait]
impl PostCommitTask for CountingTask {
    async fn send(&self) -> anyhow::Result<()> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait]
impl SigningExecutor for Dispatching {
    fn action(&self) -> SigningAction {
        ACTION
    }

    async fn execute(
        &self,
        _tx: &deadpool_postgres::Transaction<'_>,
        _request: &SigningRequestRow,
        _approvals: &[SigningApprovalRow],
    ) -> anyhow::Result<ExecutionOutcome> {
        Ok(ExecutionOutcome::Dispatched {
            task_execution_id: self.task_execution_id,
            task: PostCommit::SendTask(Box::new(CountingTask(self.sent.clone()))),
        })
    }

    async fn redispatch(&self, _request: &SigningRequestRow) -> anyhow::Result<Option<PostCommit>> {
        Ok(Some(PostCommit::SendTask(Box::new(CountingTask(
            self.sent.clone(),
        )))))
    }
}

/// A completed request whose task was dispatched, and what its task sent.
async fn dispatched(
    w: &World,
) -> (
    Uuid,
    Uuid,
    Arc<AtomicUsize>,
    windmill::services::signing::executors::SigningExecutorRegistry,
) {
    w.rule(ACTION, 1, RequesterSigning::NotAllowed, None).await;
    let signers = signers(w, 1).await;
    let sent = Arc::new(AtomicUsize::new(0));
    let task_execution_id = Uuid::new_v4();
    let executor: Arc<dyn SigningExecutor> = Arc::new(Dispatching {
        sent: sent.clone(),
        task_execution_id,
    });
    let services = services(verifier_for(&signers), vec![executor]);
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(10), at(0))
        .await;
    let outcome = w
        .sign(&services, &signers[0].0, request.id, &signers[0].1, at(1))
        .await
        .unwrap();
    assert_eq!(outcome.status, SigningRequestStatus::Completed);
    assert_eq!(sent.load(Ordering::SeqCst), 1, "sent after the commit");
    let row = w.request(request.id).await;
    assert_eq!(row.task_execution_id, Some(task_execution_id));
    (request.id, task_execution_id, sent, services.executors)
}

/// Ages the request's completion (and claim) by `seconds`.
async fn age(w: &World, id: Uuid, completed: i64, claimed: Option<i64>) {
    w.execute(
        "UPDATE sequent_backend.signing_request SET
             completed_at = clock_timestamp() - make_interval(secs => $2::bigint),
             execution_started_at = CASE WHEN $3::bigint IS NULL THEN execution_started_at
                 ELSE clock_timestamp() - make_interval(secs => $3::bigint) END
         WHERE id = $1",
        &[&id, &completed, &claimed],
    )
    .await;
}

async fn claim(w: &World, id: Uuid, task: Uuid) -> bool {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let claimed = claim_dispatched(&tx, w.tenant, w.event, id, task)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    claimed
}

async fn finish(w: &World, id: Uuid, task: Uuid, result: DispatchedResult) -> bool {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let finished = finish_dispatched(&tx, w.tenant, w.event, id, task, result)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    finished
}

#[tokio::test]
async fn a_dispatched_execution_is_claimed_once_and_reported_by_its_task() {
    let w = world("madrid-pe").await;
    let (id, task, _, _) = dispatched(&w).await;
    // Reporting before claiming, or for another task, is refused.
    assert!(!finish(&w, id, task, DispatchedResult::Executed(None)).await);
    assert!(!claim(&w, id, Uuid::new_v4()).await);
    // The task claims it; a duplicate copy of the task can't.
    assert!(claim(&w, id, task).await);
    assert!(!claim(&w, id, task).await);
    assert!(!finish(&w, id, Uuid::new_v4(), DispatchedResult::Executed(None)).await);
    assert!(
        finish(
            &w,
            id,
            task,
            DispatchedResult::Executed(Some(json!({"done": 1})))
        )
        .await
    );
    assert!(
        !finish(
            &w,
            id,
            task,
            DispatchedResult::Failed {
                code: "late".into()
            }
        )
        .await
    );
    let row = w.request(id).await;
    assert_eq!(row.status, SigningRequestStatus::Executed);
    assert_eq!(row.execution_result, Some(json!({"done": 1})));
    assert_eq!(row.execution_attempts, 1);
    // Reported by the task, still named for the signer who completed it.
    let executed = w.entries("SigningActionExecuted").await;
    assert_eq!(executed.len(), 1);
    assert_eq!(executed[0].0.as_deref(), Some("sbei-0"));
    w.assert_two_entries_per_step().await;
}

#[tokio::test]
async fn the_sweeper_resends_only_unclaimed_stale_or_failed_executions_a_few_times() {
    let w = world("madrid-pe").await;
    let (id, task, sent, executors) = dispatched(&w).await;
    let mut client = w.pool.get().await.unwrap();
    let resent = |sent: &Arc<AtomicUsize>| sent.load(Ordering::SeqCst) - 1;

    // Completed a moment ago: not yet.
    redispatch_unexecuted_requests(&mut client, &executors)
        .await
        .unwrap();
    assert_eq!(resent(&sent), 0);
    // Unclaimed for three minutes: sent again.
    age(&w, id, 180, None).await;
    redispatch_unexecuted_requests(&mut client, &executors)
        .await
        .unwrap();
    assert_eq!(resent(&sent), 1);
    // Claimed by a task still running (within the lease): not sent.
    assert!(claim(&w, id, task).await);
    age(&w, id, 180, Some(60)).await;
    redispatch_unexecuted_requests(&mut client, &executors)
        .await
        .unwrap();
    assert_eq!(resent(&sent), 1);
    // A stale claim is sent again, and its new copy may claim it.
    age(&w, id, 3600, Some(EXECUTION_LEASE_SECONDS + 60)).await;
    redispatch_unexecuted_requests(&mut client, &executors)
        .await
        .unwrap();
    assert_eq!(resent(&sent), 2);
    assert!(claim(&w, id, task).await);
    // A failed task is sent again at once.
    w.execute(
        "INSERT INTO sequent_backend.tasks_execution
             (id, tenant_id, election_event_id, name, type, execution_status, executed_by_user)
         VALUES ($1, $2, $3, 'signing', 'signing', 'FAILED', 'system')",
        &[&task, &w.tenant, &w.event],
    )
    .await;
    redispatch_unexecuted_requests(&mut client, &executors)
        .await
        .unwrap();
    assert_eq!(resent(&sent), 3);
    assert_eq!(w.request(id).await.execution_started_at, None);
    // After the last attempt the request fails instead.
    assert!(claim(&w, id, task).await);
    assert_eq!(
        w.request(id).await.execution_attempts,
        MAX_EXECUTION_ATTEMPTS
    );
    redispatch_unexecuted_requests(&mut client, &executors)
        .await
        .unwrap();
    assert_eq!(resent(&sent), 3);
    let row = w.request(id).await;
    assert_eq!(row.status, SigningRequestStatus::Failed);
    assert_eq!(
        row.execution_result.unwrap()["error"]["code"],
        "execution-attempts-exhausted"
    );
    let executed = w.entries("SigningActionExecuted").await;
    assert_eq!(executed.len(), 1);
    assert_eq!(executed[0].0.as_deref(), Some("sbei-0"));
    w.assert_two_entries_per_step().await;
}

#[tokio::test]
async fn overdue_requests_expire_for_their_requester() {
    let _turn = EXPIRY.lock().await;
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::NotAllowed, Some(60))
        .await;
    let requester = caller("operator", &[], &[]);
    let due = w.start(&requester, ACTION, subject(11), at(0)).await;
    let mut client = w.pool.get().await.unwrap();
    expire_overdue_requests(&mut client).await.unwrap();
    assert_eq!(
        w.request(due.id).await.status,
        SigningRequestStatus::Waiting
    );
    w.make_overdue(due.id).await;
    expire_overdue_requests(&mut client).await.unwrap();
    assert_eq!(
        w.request(due.id).await.status,
        SigningRequestStatus::Expired
    );
    let expired: Vec<_> = w
        .outbox()
        .await
        .into_iter()
        .filter(|row| row.1 == "SigningRequestExpired")
        .collect();
    assert_eq!(expired.len(), 2);
    assert_eq!(expired[0].4.as_deref(), Some("operator"));
    assert_eq!(expired[1].4, None);
}

#[tokio::test]
async fn a_trustee_gate_completes_and_only_its_trustee_consumes_it() {
    let action = SigningAction::ConfirmKeyShare;
    let w = world("madrid-pe").await;
    // A trustee step needs its trustee's signature, whatever the number.
    w.rule(action, 3, RequesterSigning::NotAllowed, None).await;
    let trustee_id = Uuid::new_v4();
    let client = w.pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.trustee (id, name, tenant_id) VALUES ($1, 'trustee-1', $2)",
            &[&trustee_id, &w.tenant],
        )
        .await
        .unwrap();
    let mut trustee = caller("trustee-user", &[action.sign_permission()], &[]);
    trustee.trustee = Some("trustee-1".into());
    let mut other = caller("trustee-user-2", &[action.sign_permission()], &[]);
    other.trustee = Some("trustee-2".into());
    let certificate = cert("trustee", "trustee", "Trustee");
    w.register("trustee-user", &certificate, None).await;
    w.register("trustee-user-2", &certificate, Some("trustee-user"))
        .await;
    let services = services(FakeVerifier::knowing(&[&certificate]), vec![]);
    let key_share = json!({"keys_ceremony_id": "k", "trustee_id": trustee_id, "key_share_sha256": sha("share")});
    let request = GuardRequestBuilder::trustee(&w, action, trustee_id, key_share.clone());
    let summary = match w
        .guard_request(&trustee, &request.build(), at(0))
        .await
        .unwrap()
    {
        windmill::services::signing::guard::GuardOutcome::SigningRequired(summary) => summary,
        other => panic!("{other:?}"),
    };
    assert_eq!(summary.required, 1);
    assert!(matches!(
        w.sign(&services, &other, summary.id, &certificate, at(1))
            .await,
        Err(SigningError::Forbidden(_))
    ));
    let outcome = w
        .sign(&services, &trustee, summary.id, &certificate, at(1))
        .await
        .unwrap();
    assert_eq!(outcome.status, SigningRequestStatus::Completed);

    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert!(matches!(
        consume_gate(
            &tx,
            &trustee,
            w.tenant,
            w.event,
            SigningAction::ContributeKeyShare,
            summary.id,
            &key_share
        )
        .await,
        Err(SigningError::NotFound(_))
    ));
    drop(tx);
    for (who, expected, subject) in [
        (&other, "forbidden", key_share.clone()),
        (
            &trustee,
            "conflict",
            json!({"keys_ceremony_id": "k", "trustee_id": trustee_id, "key_share_sha256": sha("other")}),
        ),
        (&trustee, "executed", key_share.clone()),
        (&trustee, "closed", key_share.clone()),
    ] {
        let tx = client.transaction().await.unwrap();
        let result = consume_gate(&tx, who, w.tenant, w.event, action, summary.id, &subject).await;
        let got = match &result {
            Ok(row) => row.status.to_string(),
            Err(SigningError::Forbidden(_)) => "forbidden".into(),
            Err(SigningError::Conflict(_)) => "conflict".into(),
            Err(SigningError::Closed { .. }) => "closed".into(),
            Err(other) => panic!("{other:?}"),
        };
        assert_eq!(got, expected);
        tx.commit().await.unwrap();
    }
    w.assert_two_entries_per_step().await;
}

/// A trustee request.
struct GuardRequestBuilder {
    action: SigningAction,
    scope: windmill::services::signing::guard::RequestScope,
    subject: serde_json::Value,
}

impl GuardRequestBuilder {
    fn trustee(
        w: &World,
        action: SigningAction,
        trustee_id: Uuid,
        subject: serde_json::Value,
    ) -> Self {
        GuardRequestBuilder {
            action,
            scope: windmill::services::signing::guard::RequestScope {
                tenant_id: w.tenant,
                election_event_id: w.event,
                election_id: None,
                area_id: None,
                trustee_id: Some(trustee_id),
                subject_key: Some("k".into()),
            },
            subject,
        }
    }

    fn build(&self) -> windmill::services::signing::guard::GuardRequest {
        windmill::services::signing::guard::GuardRequest {
            action: self.action,
            scope: self.scope.clone(),
            subject: self.subject.clone(),
            document: None,
            config_revision: None,
        }
    }
}

/// The `check` the refusal steps logged, in order.
async fn refusal_checks(w: &World) -> Vec<String> {
    w.pool
        .get()
        .await
        .unwrap()
        .query(
            "SELECT body->'details'->>'check' FROM sequent_backend.signing_log_outbox
             WHERE election_event_id = $1 AND statement_kind = 'SigningSignatureRefused'
                 AND entry = 1 ORDER BY id",
            &[&w.event],
        )
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect()
}

#[tokio::test]
async fn who_may_not_sign_is_refused_with_a_logged_reason_once_per_burst() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::NotAllowed, None).await;
    let certificate = cert("reader", "reader", "Reader");
    w.register("reader", &certificate, None).await;
    let services = services(FakeVerifier::knowing(&[&certificate]), vec![]);
    let operator = caller("operator", &[ACTION.sign_permission()], &["madrid-pe"]);
    let request = w.start(&operator, ACTION, subject(12), at(0)).await;
    let reader = caller("reader", &[Permissions::SIGNING_REQUESTS_READ], &[]);
    let outsider = caller("outsider", &[ACTION.sign_permission()], &["tokyo-pe"]);
    for who in [&reader, &reader, &outsider, &operator] {
        assert!(matches!(
            w.sign(&services, who, request.id, &certificate, at(1))
                .await,
            Err(SigningError::Forbidden(_))
        ));
    }
    // The reader's second attempt within the throttle is answered, not
    // logged; nothing is told about the request.
    assert_eq!(
        refusal_checks(&w).await,
        ["forbidden", "outside-post", "requester-not-allowed"]
    );
    assert!(w.approvals(request.id).await.is_empty());
    w.assert_two_entries_per_step().await;
}

#[tokio::test]
async fn a_document_signature_goes_only_with_its_kind_of_action() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::NotAllowed, None).await;
    let signers = signers(&w, 1).await;
    let services = services(verifier_for(&signers), vec![]);
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(14), at(0))
        .await;
    let row = w.request(request.id).await;
    let mut client = w.pool.get().await.unwrap();
    let base = windmill::services::signing::approve::ApproveInput {
        request_id: request.id,
        chain_pem: vec![signers[0].1.identity.pem.clone()],
        algorithm: sequent_core::signing::SignatureAlgorithm::RsaPkcs1Sha256,
        payload_signature: signature_over(&row.canonical_payload),
        document_signature: None,
        pdf_cms: None,
        revision: None,
    };
    for input in [
        windmill::services::signing::approve::ApproveInput {
            document_signature: Some(vec![1]),
            ..base.clone()
        },
        windmill::services::signing::approve::ApproveInput {
            pdf_cms: Some(vec![1]),
            revision: Some(1),
            ..base.clone()
        },
    ] {
        assert!(matches!(
            windmill::services::signing::approve::approve(
                &mut client,
                &services,
                &signers[0].0,
                w.tenant,
                &input,
                at(1)
            )
            .await,
            Err(SigningError::Invalid {
                reason: windmill::services::signing::InvalidReason::Document,
                ..
            })
        ));
    }

    // A PDF action fails closed while nothing takes PDF signatures.
    let reports = SigningAction::GenerateReports;
    w.rule(reports, 1, RequesterSigning::NotAllowed, None).await;
    let signer = w.signer("sbei-0", reports);
    let pdf = w
        .guard_request(
            &caller("operator", &[], &[]),
            &windmill::services::signing::guard::GuardRequest {
                action: reports,
                scope: w.scope(reports),
                subject: json!({"report_type": "r", "document_sha256": sha("pdf"), "template_id": null}),
                document: Some(windmill::services::signing::guard::SigningDocument {
                    document_id: None,
                    sha256: sha("pdf"),
                }),
                config_revision: None,
            },
            at(0),
        )
        .await
        .unwrap();
    let windmill::services::signing::guard::GuardOutcome::SigningRequired(pdf) = pdf else {
        panic!("{pdf:?}")
    };
    let pdf_row = w.request(pdf.id).await;
    let input = windmill::services::signing::approve::ApproveInput {
        request_id: pdf.id,
        payload_signature: signature_over(&pdf_row.canonical_payload),
        pdf_cms: Some(vec![1]),
        revision: Some(1),
        ..base.clone()
    };
    assert!(matches!(
        windmill::services::signing::approve::approve(
            &mut client,
            &services,
            &signer,
            w.tenant,
            &input,
            at(1)
        )
        .await,
        Err(SigningError::Invalid {
            reason: windmill::services::signing::InvalidReason::Document,
            ..
        })
    ));
    assert!(w.approvals(pdf.id).await.is_empty());
}

#[tokio::test]
async fn a_revoked_certificate_of_a_counted_signature_cancels_the_request() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::NotAllowed, None).await;
    let signers = signers(&w, 2).await;
    let executor = fake(FakeBehaviour::Succeed(json!({})));
    let services = services(verifier_for(&signers), vec![executor.clone()]);
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(15), at(0))
        .await;
    w.sign(&services, &signers[0].0, request.id, &signers[0].1, at(1))
        .await
        .unwrap();
    // The first signer's key is revoked after they signed.
    w.execute(
        "UPDATE sequent_backend.staff_certificate
         SET status = 'revoked', revoked_at = now(), revoked_by = 'officer'
         WHERE tenant_id = $1 AND user_id = 'sbei-0'",
        &[&w.tenant],
    )
    .await;
    assert!(matches!(
        w.sign(&services, &signers[1].0, request.id, &signers[1].1, at(2))
            .await,
        Err(SigningError::Closed {
            status: SigningRequestStatus::Cancelled,
            ..
        })
    ));
    assert!(executor.runs().is_empty());
    let row = w.request(request.id).await;
    assert_eq!(
        (row.status, row.cancel_reason),
        (
            SigningRequestStatus::Cancelled,
            Some(sequent_core::signing::CancelReason::CertificateRevoked)
        )
    );
    assert_eq!(w.approvals(request.id).await.len(), 1);
    assert_eq!(
        w.steps().await.last().map(String::as_str),
        Some("SigningRequestCancelled")
    );
    w.assert_two_entries_per_step().await;
}

#[tokio::test]
async fn a_first_signature_registers_the_certificate_to_its_signer() {
    let w = world("madrid-pe").await;
    w.rule(ACTION, 2, RequesterSigning::NotAllowed, None).await;
    let ana_certificate = cert("ana", "ana", "Ana");
    let services = services(
        FakeVerifier::knowing(&[&ana_certificate]).registering_on_first_use(),
        vec![fake(FakeBehaviour::Succeed(json!({})))],
    );
    let request = w
        .start(&caller("operator", &[], &[]), ACTION, subject(13), at(0))
        .await;
    let ana = w.signer("ana", ACTION);
    let outcome = w
        .sign(&services, &ana, request.id, &ana_certificate, at(1))
        .await
        .unwrap();
    assert_eq!(outcome.count, 1);
    let approvals = w.approvals(request.id).await;
    assert_eq!(approvals[0].display_name.as_deref(), Some("ana display"));
    let client = w.pool.get().await.unwrap();
    let registered: (String, String, Option<String>) = client
        .query_one(
            "SELECT user_id, registration, user_display_name FROM sequent_backend.staff_certificate
             WHERE id = $1",
            &[&approvals[0].certificate_id],
        )
        .await
        .map(|row| (row.get(0), row.get(1), row.get(2)))
        .unwrap();
    assert_eq!(
        registered,
        (
            "ana".to_string(),
            "first-use".to_string(),
            Some("ana display".to_string())
        )
    );
    assert_eq!(
        w.steps().await,
        [
            "SigningRequestCreated",
            "SigningCertificateRegistered",
            "SigningRequestSigned"
        ]
    );
    w.assert_two_entries_per_step().await;
}
