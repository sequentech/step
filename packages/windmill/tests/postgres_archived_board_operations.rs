// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The board's approvals bound to an archived election event, against a
//! private migrated Hasura database. An archived event is finished: they serve
//! only the export and the deletion of its data. An approval event that is
//! archived approves no import, whenever the board signed.
//!
//! Each test commits its own tenant. The routes' own checks run before the
//! board's gate; the tests call the check they share with the tasks.
//!
//! Needs `HASURA_DB__*`.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;

use chrono::{Duration, Utc};
use sequent_core::election_lifecycle::ElectoralPhase;
use sequent_core::signing::{RequesterSigning, SigningAction, StaffCertificateRegistration};
use sequent_core::types::permissions::Permissions;
use serde_json::json;
use signing::*;
use uuid::Uuid;
use windmill::postgres::signing::{insert_staff_certificate, NewStaffCertificate};
use windmill::postgres::trusted_write;
use windmill::services::board_operations::{
    bind, require_unarchived_phase, source_revision, validate, BindingKind,
};
use windmill::services::election_lifecycle::state;
use windmill::services::signing::board_gate::{gate, BoardGateOutcome};
use windmill::services::signing::SigningCaller;

const ARCHIVED: &str =
    "The election event is archived: only the export and deletion of its data remain possible";
const ARCHIVE: &str = "UPDATE sequent_backend.election_event SET is_archived = true WHERE id = $1";
const MOVE: &str =
    "UPDATE sequent_backend.election_lifecycle SET phase = $1 WHERE election_event_id = $2";

/// An approval event: no election and no area.
async fn approval_event() -> World {
    let pool = schema::pool().await;
    let (tenant, event) = (Uuid::new_v4(), Uuid::new_v4());
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("tenant-{tenant}")],
        )
        .await
        .unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&event, &tenant],
        )
        .await
        .unwrap();
    drop(client);
    World {
        pool,
        tenant,
        event,
        post: Uuid::new_v4(),
        area: Uuid::new_v4(),
        label: String::new(),
    }
}

/// Moves the event to `phase` in the server's own write.
async fn move_to(w: &World, phase: &str) {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    tx.execute(MOVE, &[&phase, &w.event]).await.unwrap();
    tx.commit().await.unwrap();
}

/// A board member who signs the actions of these tests.
fn member(user: &str) -> SigningCaller {
    caller(
        user,
        &[
            Permissions::SIGNING_REQUESTS_CANCEL,
            SigningAction::ImportElectionData.sign_permission(),
            SigningAction::StartTally.sign_permission(),
            SigningAction::ExportExecutionData.sign_permission(),
        ],
        &[],
    )
}

/// The certificate of `user`, valid now and registered as its first use for
/// an action of the whole event does: bound to no election.
async fn registered(w: &World, user: &str) -> TestCert {
    let mut certificate = cert(user, user, user);
    certificate.identity.not_before = Utc::now() - Duration::days(1);
    certificate.identity.not_after = Utc::now() + Duration::days(1);
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let identity = &certificate.identity;
    insert_staff_certificate(
        &tx,
        &NewStaffCertificate {
            tenant_id: w.tenant,
            election_event_id: w.event,
            user_id: user.to_owned(),
            username: user.to_owned(),
            election_id: None,
            fingerprint_sha256: identity.fingerprint_sha256.clone(),
            spki_sha256: identity.spki_sha256.clone(),
            holder_sha256: identity.holder_sha256.clone(),
            serial: identity.serial.clone(),
            subject: identity.subject.clone(),
            issuer: identity.issuer.clone(),
            not_before: identity.not_before,
            not_after: identity.not_after,
            pem: identity.pem.clone(),
            registration: StaffCertificateRegistration::FirstUse,
            linked_to: None,
            registered_by: user.to_owned(),
            user_display_name: Some(format!("{user} display")),
            registered_by_name: None,
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    certificate
}

/// The board's approval of `operation`: a member asks, the board signs and
/// the member repeats the operation, as the board's gate takes it.
async fn approved(
    w: &World,
    certificate: &TestCert,
    action: SigningAction,
    operation: &str,
) -> Uuid {
    let (tenant, event) = (w.tenant.to_string(), w.event.to_string());
    let requester = member("member-0");
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let subject = json!({
        "operation": operation,
        "parameters": {},
        "lifecycle_generation": state(&tx, &tenant, &event).await.unwrap().generation,
        "source_revision": source_revision(&tx, &tenant, &event).await.unwrap(),
        "permission_labels": [],
    });
    let outcome = gate(
        &tx,
        &requester,
        w.tenant,
        w.event,
        action,
        operation,
        subject.clone(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let BoardGateOutcome::Waiting(request) = outcome else {
        panic!("expected a waiting request, got {outcome:?}");
    };
    w.sign(
        &services(FakeVerifier::knowing(&[certificate]), vec![]),
        &member("member-1"),
        request.id,
        certificate,
        at(1),
    )
    .await
    .unwrap();
    let tx = client.transaction().await.unwrap();
    let outcome = gate(
        &tx, &requester, w.tenant, w.event, action, operation, subject,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let BoardGateOutcome::Authorized(id) = outcome else {
        panic!("expected an authorisation, got {outcome:?}");
    };
    id
}

fn refusal<T: std::fmt::Debug>(result: anyhow::Result<T>) -> String {
    result.unwrap_err().to_string()
}

#[tokio::test]
async fn an_archived_approval_event_approves_no_import() {
    let w = approval_event().await;
    for action in [
        SigningAction::ImportElectionData,
        SigningAction::ExportExecutionData,
    ] {
        w.rule(action, 1, RequesterSigning::NotAllowed, None).await;
    }
    let certificate = registered(&w, "member-1").await;
    let (tenant, event) = (w.tenant.to_string(), w.event.to_string());
    let mut client = w.pool.get().await.unwrap();

    // Before the archiving, the board's approval imports.
    let before = approved(
        &w,
        &certificate,
        SigningAction::ImportElectionData,
        "import-election-event",
    )
    .await;
    let task = Uuid::new_v4().to_string();
    let tx = client.transaction().await.unwrap();
    bind(&tx, &tenant, &event, before, BindingKind::Import, &task)
        .await
        .unwrap();
    validate(&tx, &tenant, &event, BindingKind::Import, &task, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    w.execute(ARCHIVE, &[&w.event]).await;

    let tx = client.transaction().await.unwrap();
    // The route asks the board for no import.
    assert_eq!(
        refusal(require_unarchived_phase(&tx, &tenant, &event, ElectoralPhase::Preparation).await),
        ARCHIVED
    );
    // The import the board approved before is refused.
    assert_eq!(
        refusal(validate(&tx, &tenant, &event, BindingKind::Import, &task, None).await),
        ARCHIVED
    );
    tx.rollback().await.unwrap();

    // So is an approval completed afterwards, as the board's gate alone
    // would give it: no import takes it.
    let after = approved(
        &w,
        &certificate,
        SigningAction::ImportElectionData,
        "import-election-event",
    )
    .await;
    let other_task = Uuid::new_v4().to_string();
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        refusal(
            bind(
                &tx,
                &tenant,
                &event,
                after,
                BindingKind::Import,
                &other_task
            )
            .await
        ),
        ARCHIVED
    );
    tx.rollback().await.unwrap();

    // The export that archiving allows still takes the board's approval.
    let export = approved(
        &w,
        &certificate,
        SigningAction::ExportExecutionData,
        "export-election-event",
    )
    .await;
    let document = Uuid::new_v4().to_string();
    let tx = client.transaction().await.unwrap();
    bind(&tx, &tenant, &event, export, BindingKind::Export, &document)
        .await
        .unwrap();
    validate(&tx, &tenant, &event, BindingKind::Export, &document, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

/// An event archived after Preparation, as only older data has it: the
/// count the board approved is refused once the event is archived.
#[tokio::test]
async fn an_event_archived_after_preparation_counts_no_votes() {
    let w = world("archived-count").await;
    w.rule(
        SigningAction::StartTally,
        1,
        RequesterSigning::NotAllowed,
        None,
    )
    .await;
    let certificate = registered(&w, "member-1").await;
    let (tenant, event) = (w.tenant.to_string(), w.event.to_string());
    move_to(&w, "execution").await;
    move_to(&w, "evaluation").await;
    let count = approved(&w, &certificate, SigningAction::StartTally, "create-tally").await;
    let session = Uuid::new_v4().to_string();
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    bind(&tx, &tenant, &event, count, BindingKind::Tally, &session)
        .await
        .unwrap();
    validate(&tx, &tenant, &event, BindingKind::Tally, &session, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    w.execute(ARCHIVE, &[&w.event]).await;

    let tx = client.transaction().await.unwrap();
    assert_eq!(
        refusal(require_unarchived_phase(&tx, &tenant, &event, ElectoralPhase::Evaluation).await),
        ARCHIVED
    );
    assert_eq!(
        refusal(validate(&tx, &tenant, &event, BindingKind::Tally, &session, None).await),
        ARCHIVED
    );
    tx.rollback().await.unwrap();
}
