// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The board-operation gate: a member repeats the operation after the board
//! signs. A completed request is consumed once, and one that no longer
//! authorises the operation is replaced by a new request that waits for
//! signatures; its cancellation names why.
//!
//! Each test commits its own tenant, and every call to the gate commits, as
//! `authorize_operation` does for both outcomes.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;

use chrono::{Duration, Utc};
use sequent_core::signing::{CancelReason, RequesterSigning, SigningAction, SigningRequestStatus};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use signing::*;
use uuid::Uuid;
use windmill::services::signing::board_gate::{gate, BoardGateOutcome};
use windmill::services::signing::{allowed_by_permission, SigningCaller};

const ACTION: SigningAction = SigningAction::ExportExecutionData;
const REVOKE: &str = "UPDATE sequent_backend.staff_certificate
    SET status = 'revoked', revoked_at = now(), revoked_by = 'officer'
    WHERE tenant_id = $1 AND user_id = 'member-1'";
const EXPIRE: &str = "UPDATE sequent_backend.staff_certificate
    SET not_after = clock_timestamp() - interval '1 second'
    WHERE tenant_id = $1 AND user_id = 'member-1'";
const TIME_LIMIT_PASSED: &str = "UPDATE sequent_backend.signing_request
    SET expires_at = clock_timestamp() - interval '1 second'
    WHERE id = $1";

fn member(user: &str) -> SigningCaller {
    caller(
        user,
        &[
            Permissions::SIGNING_REQUESTS_CANCEL,
            ACTION.sign_permission(),
        ],
        &[],
    )
}

/// A member with a registered certificate that is valid now, as the gate
/// reads it from the database's clock.
async fn signer(w: &World, user: &str) -> (SigningCaller, TestCert) {
    let mut certificate = cert(user, user, user);
    certificate.identity.not_before = Utc::now() - Duration::days(1);
    certificate.identity.not_after = Utc::now() + Duration::days(1);
    w.register(user, &certificate, None).await;
    (member(user), certificate)
}

/// The member repeats the operation, committed.
async fn repeat(w: &World, who: &SigningCaller, subject: Value) -> BoardGateOutcome {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let outcome = gate(
        &tx,
        who,
        w.tenant,
        w.event,
        ACTION,
        "export-execution-data",
        subject,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    outcome
}

fn waiting(outcome: BoardGateOutcome) -> Uuid {
    match outcome {
        BoardGateOutcome::Waiting(summary) => summary.id,
        other => panic!("expected a waiting request, got {other:?}"),
    }
}

fn authorized(outcome: BoardGateOutcome) -> Uuid {
    match outcome {
        BoardGateOutcome::Authorized(id) => id,
        other => panic!("expected an authorisation, got {other:?}"),
    }
}

/// A board of a requester and two signers, with `requester`'s request
/// completed.
async fn completed_request(
    w: &World,
    requester: &SigningCaller,
) -> (Uuid, Vec<(SigningCaller, TestCert)>) {
    w.rule(ACTION, 2, RequesterSigning::NotAllowed, None).await;
    let signers = vec![signer(w, "member-1").await, signer(w, "member-2").await];
    let id = waiting(repeat(w, requester, subject(1)).await);
    let services = services(
        FakeVerifier::knowing(&[&signers[0].1, &signers[1].1]),
        vec![],
    );
    for (who, certificate) in &signers {
        w.sign(&services, who, id, certificate, at(1))
            .await
            .unwrap();
    }
    assert_eq!(w.request(id).await.status, SigningRequestStatus::Completed);
    (id, signers)
}

#[tokio::test]
async fn a_completed_request_authorises_the_operation_once() {
    let w = world("board-gate-once").await;
    let requester = member("member-0");
    let (id, _) = completed_request(&w, &requester).await;

    assert_eq!(authorized(repeat(&w, &requester, subject(1)).await), id);
    assert_eq!(w.request(id).await.status, SigningRequestStatus::Executed);
    assert_eq!(w.entries("SigningActionExecuted").await.len(), 1);

    let next = waiting(repeat(&w, &requester, subject(1)).await);
    assert_ne!(next, id);
    assert!(w.entries("SigningRequestCancelled").await.is_empty());
    assert_eq!(w.request(id).await.status, SigningRequestStatus::Executed);
}

#[tokio::test]
async fn a_request_that_no_longer_authorises_is_replaced_naming_why() {
    let revoker = allowed_by_permission(Permissions::SIGNING_CERTIFICATES_REVOKE);
    for (label, reason, cancelled_by, note, allowed_by) in [
        // As the officer who revoked the certificate would have ended it.
        (
            "revoked",
            CancelReason::CertificateRevoked,
            "officer",
            Value::Null,
            Some(&revoker),
        ),
        // The others record no permission a board member does not hold.
        (
            "expired",
            CancelReason::Superseded,
            "member-0",
            json!("A required signing certificate expired"),
            None,
        ),
        (
            "rule changed",
            CancelReason::Superseded,
            "member-0",
            json!("The signing rule changed"),
            None,
        ),
        (
            "time limit passed",
            CancelReason::Superseded,
            "member-0",
            json!("Its time limit passed"),
            None,
        ),
    ] {
        let w = world(label).await;
        let requester = member("member-0");
        let (id, signers) = completed_request(&w, &requester).await;
        match label {
            "revoked" => w.execute(REVOKE, &[&w.tenant]).await,
            "expired" => w.execute(EXPIRE, &[&w.tenant]).await,
            "rule changed" => w.rule(ACTION, 2, RequesterSigning::NotAllowed, None).await,
            _ => w.execute(TIME_LIMIT_PASSED, &[&id]).await,
        }

        let next = waiting(repeat(&w, &requester, subject(1)).await);

        assert_ne!(next, id, "{label}");
        let old = w.request(id).await;
        assert_eq!(old.status, SigningRequestStatus::Cancelled, "{label}");
        assert_eq!(old.cancel_reason, Some(reason), "{label}");
        assert_eq!(old.cancelled_by.as_deref(), Some(cancelled_by), "{label}");
        let new = w.request(next).await;
        assert_eq!(new.status, SigningRequestStatus::Waiting, "{label}");
        assert_ne!(new.code, old.code, "{label}");
        assert!(w.approvals(next).await.is_empty(), "{label}");
        let cancelled = w.entries("SigningRequestCancelled").await;
        assert_eq!(cancelled.len(), 1, "{label}");
        assert_eq!(cancelled[0].0.as_deref(), Some(cancelled_by), "{label}");
        let details = &cancelled[0].1;
        assert_eq!(details["reason"], json!(reason.to_string()), "{label}");
        assert_eq!(details["note"], note, "{label}");
        assert_eq!(details["voided_approvals"].as_array().unwrap().len(), 2);
        assert_eq!(details.get("allowed_by"), allowed_by, "{label}");

        // Repeating again finds the same request, not another one.
        assert_eq!(waiting(repeat(&w, &requester, subject(1)).await), next);

        // The member with a valid certificate signs again, with another member.
        let (third, third_certificate) = signer(&w, "member-3").await;
        let services = services(
            FakeVerifier::knowing(&[&signers[1].1, &third_certificate]),
            vec![],
        );
        w.sign(&services, &signers[1].0, next, &signers[1].1, at(1))
            .await
            .unwrap();
        w.sign(&services, &third, next, &third_certificate, at(1))
            .await
            .unwrap();
        assert_eq!(authorized(repeat(&w, &requester, subject(1)).await), next);
        assert_eq!(
            w.request(next).await.status,
            SigningRequestStatus::Executed,
            "{label}"
        );
        w.assert_two_entries_per_step().await;
    }
}

#[tokio::test]
async fn a_completed_request_for_something_else_is_replaced_as_changed() {
    let w = world("board-gate-changed").await;
    let requester = member("member-0");
    let (id, _) = completed_request(&w, &requester).await;

    let next = waiting(repeat(&w, &requester, subject(2)).await);

    assert_ne!(next, id);
    assert_eq!(
        w.request(id).await.cancel_reason,
        Some(CancelReason::PayloadChanged)
    );
    let cancelled = w.entries("SigningRequestCancelled").await;
    assert_eq!(cancelled[0].1["reason"], json!("payload-changed"));
    assert_eq!(cancelled[0].1["note"], Value::Null);
}
