// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The way into post-processing against a private migrated Hasura database: the
//! rule of the workflow, an election that can never be counted because a seal
//! failed, and an archived event, which stays in preparation, where the board
//! exports and then deletes its data.
//!
//! Each test commits its own tenant. The archive itself and the realm, the
//! documents and the electoral-log database of a deletion need the other
//! services; the export's receipt stands in for the archive.
//!
//! Needs `HASURA_DB__*`.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;

use chrono::{Duration, Utc};
use deadpool_postgres::Transaction;
use sequent_core::signing::{RequesterSigning, SigningAction, StaffCertificateRegistration};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use signing::*;
use tokio_postgres::types::ToSql;
use uuid::Uuid;
use windmill::postgres::ballot_box_seal::{
    insert_pending, mark_failed, mark_published, mark_sealed, ClosedBy, NewPendingSeal,
    PublishedFields, SealLogRecord, SealedFields,
};
use windmill::postgres::election_event::delete_election_event;
use windmill::postgres::signing::{insert_staff_certificate, NewStaffCertificate};
use windmill::postgres::trusted_write;
use windmill::services::ballot_box_seal::refuse_sealed_event_delete;
use windmill::services::board_operations::{
    authorize_deletion, bind, is_authorized_deletion, require_export, source_revision, validate,
    BindingKind,
};
use windmill::services::delete_election_event::{
    archive_protocol_boards, find_election_event_deletion_refusal, ElectionEventDeletionRefusal,
};
use windmill::services::election_lifecycle::{
    require_execution_data_phase, start_post_processing, state,
};
use windmill::services::signing::board_gate::{gate, BoardGateOutcome};
use windmill::services::signing::SigningCaller;

const WRONG_PHASE: &str = "the election event is not in the required electoral phase";
const NOT_DETERMINED: &str = "the result is not determined yet";
const BACKWARDS: &str = "Electoral phase cannot move backwards";
const STAYS: &str = "An archived election event stays in preparation";
const NOT_POST_PROCESSING: &str =
    "This operation requires electoral phase PostProcessing; current phase is Preparation";
const ARCHIVE: &str = "UPDATE sequent_backend.election_event SET is_archived = true WHERE id = $1";
const MOVE: &str =
    "UPDATE sequent_backend.election_lifecycle SET phase = $1 WHERE election_event_id = $2";
const UP: &str = include_str!(
    "../../../hasura/migrations/backend-db/1793020015000_archived_event_deletion/up.sql"
);
const DOWN: &str = include_str!(
    "../../../hasura/migrations/backend-db/1793020015000_archived_event_deletion/down.sql"
);

/// An approval event: no election and no area.
async fn empty_world() -> World {
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

async fn second_election(w: &World) -> Uuid {
    let id = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
         VALUES ($1, $2, $3)",
        &[&id, &w.tenant, &w.event],
    )
    .await;
    id
}

async fn second_area(w: &World) -> Uuid {
    let id = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
         VALUES ($1, $2, $3, 'Other')",
        &[&id, &w.tenant, &w.event],
    )
    .await;
    id
}

async fn archive(w: &World) {
    w.execute(ARCHIVE, &[&w.event]).await;
}

/// Runs `sql` in the server's own write.
async fn server_write(w: &World, sql: &str, params: &[&(dyn ToSql + Sync)]) {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    tx.execute(sql, params).await.unwrap();
    tx.commit().await.unwrap();
}

async fn evaluating(w: &World) {
    server_write(w, MOVE, &[&"execution", &w.event]).await;
    server_write(w, MOVE, &[&"evaluation", &w.event]).await;
}

async fn rules(w: &World) {
    for action in [
        SigningAction::ExportExecutionData,
        SigningAction::DeleteExecutionData,
    ] {
        w.rule(action, 1, RequesterSigning::NotAllowed, None).await;
    }
}

/// The board's action, committed when it is allowed.
async fn start(w: &World) -> anyhow::Result<()> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    start_post_processing(&tx, &w.tenant.to_string(), &w.event.to_string()).await?;
    tx.commit().await.unwrap();
    Ok(())
}

async fn refusal(w: &World) -> String {
    start(w).await.unwrap_err().to_string()
}

async fn lifecycle(w: &World) -> (String, i64) {
    let row = w
        .pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT phase, generation FROM sequent_backend.election_lifecycle
             WHERE election_event_id = $1",
            &[&w.event],
        )
        .await
        .unwrap();
    (row.get(0), row.get(1))
}

async fn phase_changes(w: &World) -> Vec<Value> {
    w.pool
        .get()
        .await
        .unwrap()
        .query(
            "SELECT details FROM sequent_backend.lifecycle_notice
             WHERE election_event_id = $1 AND kind = 'ElectoralPhaseChanged'",
            &[&w.event],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect()
}

/// A pending seal, as the close of the box creates it.
async fn pending_seal(w: &World, election: Uuid, area: Uuid) -> Uuid {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let now = Utc::now();
    insert_pending(
        &tx,
        &[NewPendingSeal {
            tenant_id: w.tenant,
            election_event_id: w.event,
            election_id: election,
            area_id: area,
            closed_at: now,
            grace_deadline: now,
            close_request_id: None,
            closed_by: ClosedBy::Scheduled,
        }],
    )
    .await
    .unwrap();
    let id = tx
        .query_one(
            "SELECT id FROM sequent_backend.ballot_box_seal
             WHERE election_id = $1 AND area_id = $2",
            &[&election, &area],
        )
        .await
        .unwrap()
        .get(0);
    tx.commit().await.unwrap();
    id
}

fn log_record(seal: Uuid) -> SealLogRecord {
    SealLogRecord {
        delivery_id: format!("ballot-box-seal:{seal}"),
        log_uid: Uuid::new_v4(),
        leaf_index: 7,
    }
}

async fn fail_seal(w: &World, seal: Uuid) {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    mark_failed(&tx, &seal, "unreadable ballot", Some(&log_record(seal)))
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn seal_it(w: &World, seal: Uuid) {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    mark_sealed(
        &tx,
        &seal,
        &SealedFields {
            sealed_at: Utc::now(),
            ballots_in_box: 2,
            ballots_counted: 1,
            seal_hash: "ab".repeat(64),
            manifest: vec![1, 2, 3],
            signed_message: vec![4, 5, 6],
            log_record: log_record(seal),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

async fn publish_it(w: &World, seal: Uuid) {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    mark_published(
        &tx,
        &seal,
        &PublishedFields {
            public_document_id: Uuid::new_v4(),
            public_path: None,
            published_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

async fn seal_status(w: &World, seal: Uuid) -> String {
    w.pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT status FROM sequent_backend.ballot_box_seal WHERE id = $1",
            &[&seal],
        )
        .await
        .unwrap()
        .get(0)
}

/// A completed results tally of `election`.
async fn counted(w: &World, election: Uuid) {
    results_tally(w, election, "SUCCESS").await
}

/// A results tally of `election` that ended with `status`.
async fn results_tally(w: &World, election: Uuid, status: &str) {
    let (session, ceremony) = (Uuid::new_v4(), Uuid::new_v4());
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.keys_ceremony
             (id, tenant_id, election_event_id, trustee_ids, threshold)
         VALUES ($1, $2, $3, '{}', 1)",
        &[&ceremony, &w.tenant, &w.event],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tally_session
             (id, tenant_id, election_event_id, election_ids, keys_ceremony_id, threshold,
              tally_type, is_execution_completed, execution_status)
         VALUES ($1, $2, $3, $4, $5, 1, 'ELECTORAL_RESULTS', true, $6)",
        &[
            &session,
            &w.tenant,
            &w.event,
            &vec![election],
            &ceremony,
            &status,
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

/// Runs `sql` expecting a refusal, rolling back to before it; returns the
/// refusal's SQLSTATE and message.
async fn refused(
    tx: &Transaction<'_>,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> (String, String) {
    tx.batch_execute("SAVEPOINT refused").await.unwrap();
    let error = tx
        .execute(sql, params)
        .await
        .expect_err("the statement should be refused");
    tx.batch_execute("ROLLBACK TO SAVEPOINT refused")
        .await
        .unwrap();
    let db = error.as_db_error().expect("a database error");
    (db.code().code().to_owned(), db.message().to_owned())
}

fn backwards() -> (String, String) {
    ("42501".to_owned(), BACKWARDS.to_owned())
}

fn stays() -> (String, String) {
    ("42501".to_owned(), STAYS.to_owned())
}

#[tokio::test]
async fn no_event_enters_post_processing_from_preparation_archived_or_not() {
    for label in ["not-archived", "archived"] {
        let w = world(label).await;
        rules(&w).await;
        if label == "archived" {
            archive(&w).await;
        }

        assert_eq!(refusal(&w).await, WRONG_PHASE);

        assert_eq!(lifecycle(&w).await, ("preparation".to_owned(), 0));
        assert!(phase_changes(&w).await.is_empty());
    }
}

#[tokio::test]
async fn the_trigger_keeps_an_archived_event_in_preparation() {
    let w = world("trigger").await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.execute(ARCHIVE, &[&w.event]).await.unwrap();

    // Without the server's own mark nothing moves.
    assert_eq!(
        refused(&tx, MOVE, &[&"execution", &w.event]).await,
        (
            "42501".to_owned(),
            "Election lifecycle changes require the server workflow".to_owned()
        )
    );
    trusted_write(&tx).await.unwrap();
    for phase in ["execution", "evaluation"] {
        assert_eq!(refused(&tx, MOVE, &[&phase, &w.event]).await, stays());
    }
    assert_eq!(
        refused(&tx, MOVE, &[&"post_processing", &w.event]).await,
        backwards()
    );
    // Its other changes go through.
    tx.execute(
        "UPDATE sequent_backend.election_lifecycle SET generation = generation + 1
         WHERE election_event_id = $1",
        &[&w.event],
    )
    .await
    .unwrap();
    tx.execute(MOVE, &[&"preparation", &w.event]).await.unwrap();

    // An event that isn't archived opens voting.
    tx.execute(
        "UPDATE sequent_backend.election_event SET is_archived = false WHERE id = $1",
        &[&w.event],
    )
    .await
    .unwrap();
    tx.execute(MOVE, &[&"execution", &w.event]).await.unwrap();
}

/// Only the archiving route archives an event, and only in preparation: an
/// event archived later by other means keeps the moves of its phase.
#[tokio::test]
async fn an_event_archived_after_preparation_keeps_its_other_moves() {
    let w = world("trigger-rest").await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    tx.execute(MOVE, &[&"execution", &w.event]).await.unwrap();
    tx.execute(ARCHIVE, &[&w.event]).await.unwrap();

    for phase in ["preparation", "post_processing"] {
        assert_eq!(refused(&tx, MOVE, &[&phase, &w.event]).await, backwards());
    }
    tx.execute(MOVE, &[&"evaluation", &w.event]).await.unwrap();
    for phase in ["preparation", "execution"] {
        assert_eq!(refused(&tx, MOVE, &[&phase, &w.event]).await, backwards());
    }
    tx.execute(MOVE, &[&"post_processing", &w.event])
        .await
        .unwrap();
}

#[tokio::test]
async fn the_migration_goes_down_and_up() {
    let deletes_in_preparation = "SELECT pg_get_functiondef(
            'sequent_backend.authorized_execution_deletion(uuid,uuid)'::regprocedure)
        LIKE '%is_archived%'";
    let w = world("migration").await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    tx.execute(ARCHIVE, &[&w.event]).await.unwrap();

    tx.batch_execute(DOWN).await.unwrap();
    let deletes: bool = tx
        .query_one(deletes_in_preparation, &[])
        .await
        .unwrap()
        .get(0);
    assert!(!deletes);
    tx.batch_execute("SAVEPOINT moved").await.unwrap();
    tx.execute(MOVE, &[&"execution", &w.event]).await.unwrap();
    tx.batch_execute("ROLLBACK TO SAVEPOINT moved")
        .await
        .unwrap();

    tx.batch_execute(UP).await.unwrap();
    let deletes: bool = tx
        .query_one(deletes_in_preparation, &[])
        .await
        .unwrap()
        .get(0);
    assert!(deletes);
    assert_eq!(refused(&tx, MOVE, &[&"execution", &w.event]).await, stays());
    // The other tests keep the migrated database.
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn the_execution_data_of_an_event_in_preparation_needs_its_archiving() {
    let w = world("gate").await;
    let (tenant, event) = (w.tenant.to_string(), w.event.to_string());
    let document = Uuid::new_v4().to_string();
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        require_execution_data_phase(&tx, &tenant, &event)
            .await
            .unwrap_err()
            .to_string(),
        NOT_POST_PROCESSING
    );
    for kind in [BindingKind::Export, BindingKind::Deletion] {
        assert_eq!(
            validate(&tx, &tenant, &event, kind, &document, None)
                .await
                .unwrap_err()
                .to_string(),
            NOT_POST_PROCESSING
        );
    }
    tx.rollback().await.unwrap();

    archive(&w).await;
    let tx = client.transaction().await.unwrap();
    require_execution_data_phase(&tx, &tenant, &event)
        .await
        .unwrap();
    // What it still needs is the board's approval.
    for kind in [BindingKind::Export, BindingKind::Deletion] {
        assert_eq!(
            validate(&tx, &tenant, &event, kind, &document, None)
                .await
                .unwrap_err()
                .to_string(),
            "A durable election-board authorisation is required"
        );
    }
    // The voters' register is read in post-processing only.
    assert_eq!(
        validate(
            &tx,
            &tenant,
            &event,
            BindingKind::VoterExport,
            &document,
            None
        )
        .await
        .unwrap_err()
        .to_string(),
        NOT_POST_PROCESSING
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_failed_seal_finishes_its_election_once_none_of_its_seals_can_change() {
    let w = world("failed-seal").await;
    let other_election = second_election(&w).await;
    let other_area = second_area(&w).await;
    evaluating(&w).await;
    let failed = pending_seal(&w, w.post, w.area).await;
    let other_box = pending_seal(&w, w.post, other_area).await;

    assert_eq!(refusal(&w).await, NOT_DETERMINED);
    // The other seal of the election may still succeed or fail.
    fail_seal(&w, failed).await;
    assert_eq!(refusal(&w).await, NOT_DETERMINED);
    seal_it(&w, other_box).await;
    assert_eq!(refusal(&w).await, NOT_DETERMINED);
    publish_it(&w, other_box).await;
    // The other election must still be counted.
    assert_eq!(refusal(&w).await, NOT_DETERMINED);
    assert_eq!(lifecycle(&w).await.0, "evaluation");
    assert!(phase_changes(&w).await.is_empty());

    counted(&w, other_election).await;
    start(&w).await.unwrap();

    assert_eq!(lifecycle(&w).await, ("post_processing".to_owned(), 1));
    assert_eq!(
        phase_changes(&w).await,
        vec![json!({
            "from": "evaluation", "to": "post_processing",
            "investigation": false, "failed_seal": true,
        })]
    );
    // Nothing is undone.
    assert_eq!(seal_status(&w, failed).await, "failed");
    assert_eq!(seal_status(&w, other_box).await, "published");
}

#[tokio::test]
async fn an_election_with_a_seal_that_failed_can_be_the_only_one() {
    let w = world("only-failed").await;
    evaluating(&w).await;
    let failed = pending_seal(&w, w.post, w.area).await;
    fail_seal(&w, failed).await;

    start(&w).await.unwrap();

    assert_eq!(phase_changes(&w).await[0]["failed_seal"], json!(true));
}

#[tokio::test]
async fn a_counted_event_keeps_the_ordinary_way() {
    let w = world("counted").await;
    evaluating(&w).await;
    assert_eq!(refusal(&w).await, NOT_DETERMINED);
    // A tally that failed counted nothing.
    results_tally(&w, w.post, "FAILED").await;
    assert_eq!(refusal(&w).await, NOT_DETERMINED);
    counted(&w, w.post).await;

    start(&w).await.unwrap();

    assert_eq!(
        phase_changes(&w).await,
        vec![json!({
            "from": "evaluation", "to": "post_processing",
            "investigation": false, "failed_seal": false,
        })]
    );
}

#[tokio::test]
async fn an_event_without_elections_has_no_result_in_evaluation() {
    let w = empty_world().await;
    evaluating(&w).await;

    assert_eq!(refusal(&w).await, NOT_DETERMINED);
}

#[tokio::test]
async fn an_unrecoverable_event_still_needs_no_result() {
    let w = world("unrecoverable").await;
    let _ = second_election(&w).await;
    evaluating(&w).await;
    let failed = pending_seal(&w, w.post, w.area).await;
    fail_seal(&w, failed).await;
    server_write(
        &w,
        "UPDATE sequent_backend.election_lifecycle SET safety = 'unrecoverable'
         WHERE election_event_id = $1",
        &[&w.event],
    )
    .await;

    start(&w).await.unwrap();

    // The other election isn't counted: the failed seal isn't what let it through.
    assert_eq!(
        phase_changes(&w).await,
        vec![json!({
            "from": "evaluation", "to": "post_processing",
            "investigation": true, "failed_seal": false,
        })]
    );
}

fn member(user: &str) -> SigningCaller {
    caller(
        user,
        &[
            Permissions::SIGNING_REQUESTS_CANCEL,
            SigningAction::ExportExecutionData.sign_permission(),
            SigningAction::DeleteExecutionData.sign_permission(),
        ],
        &[],
    )
}

/// Registers `certificate` as its first use for an action of the whole event
/// does: bound to no election, which an approval event doesn't have anyway.
async fn register_for_the_event(w: &World, user: &str, certificate: &TestCert) {
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
}

/// The operation's board approval: the member asks, the board signs and the
/// member repeats the operation, as the route does.
async fn approved(
    w: &World,
    certificate: &TestCert,
    action: SigningAction,
    operation: &str,
    labels: &[String],
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
        "permission_labels": labels,
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
    assert_eq!(id, request.id);
    id
}

/// The board's export and deletion of an event, as the routes and the tasks run
/// them. `refusal` is what the deletion task tolerates.
async fn export_then_delete(w: &World, refusal: Option<ElectionEventDeletionRefusal>) {
    let (tenant, event) = (w.tenant.to_string(), w.event.to_string());
    let labels: Vec<String> = (!w.label.is_empty())
        .then(|| w.label.clone())
        .into_iter()
        .collect();
    let mut certificate = cert("member-1", "member-1", "member-1");
    certificate.identity.not_before = Utc::now() - Duration::days(1);
    certificate.identity.not_after = Utc::now() + Duration::days(1);
    register_for_the_event(w, "member-1", &certificate).await;

    let export = approved(
        w,
        &certificate,
        SigningAction::ExportExecutionData,
        "export-election-event",
        &labels,
    )
    .await;
    let document = Uuid::new_v4();
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    bind(
        &tx,
        &tenant,
        &event,
        export,
        BindingKind::Export,
        &document.to_string(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // What the export task reads and records around the archive.
    let tx = client.transaction().await.unwrap();
    validate(
        &tx,
        &tenant,
        &event,
        BindingKind::Export,
        &document.to_string(),
        None,
    )
    .await
    .unwrap();
    let evidence: Value = tx
        .query_one(
            "SELECT sequent_backend.execution_security_evidence($1::text::uuid,$2::text::uuid)",
            &[&tenant, &event],
        )
        .await
        .unwrap()
        .get(0);
    assert!(evidence.is_object(), "{evidence}");
    assert_eq!(
        require_export(&tx, &tenant, &event)
            .await
            .unwrap_err()
            .to_string(),
        "Export the complete, current election execution data before requesting deletion"
    );
    let revision = source_revision(&tx, &tenant, &event).await.unwrap();
    let generation = state(&tx, &tenant, &event).await.unwrap().generation;
    trusted_write(&tx).await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.execution_export
             (tenant_id, election_event_id, document_id, request_id, source_revision,
              lifecycle_generation, manifest_sha256, archive_sha256, signing_public_key)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7, 'public key')",
        &[
            &w.tenant,
            &w.event,
            &document,
            &export,
            &revision,
            &generation,
            &"ab".repeat(32),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let deletion = approved(
        w,
        &certificate,
        SigningAction::DeleteExecutionData,
        "delete-execution-data",
        &labels,
    )
    .await;
    let task = Uuid::new_v4().to_string();
    let tx = client.transaction().await.unwrap();
    require_export(&tx, &tenant, &event).await.unwrap();
    bind(&tx, &tenant, &event, deletion, BindingKind::Deletion, &task)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // Refused without the task's authorisation.
    let tx = client.transaction().await.unwrap();
    assert!(!is_authorized_deletion(&tx, &tenant, &event).await.unwrap());
    assert_eq!(
        delete_election_event(&tx, &tenant, &event)
            .await
            .unwrap_err()
            .to_string(),
        "Deleting election execution data requires its current export and board authorisation"
    );
    tx.rollback().await.unwrap();

    let tx = client.transaction().await.unwrap();
    let signed_labels = authorize_deletion(&tx, &tenant, &event, &task)
        .await
        .unwrap();
    assert_eq!(signed_labels, labels);
    assert_eq!(
        find_election_event_deletion_refusal(&tx, &tenant, &event, &signed_labels)
            .await
            .unwrap(),
        refusal
    );
    refuse_sealed_event_delete(&tx, &tenant, &event)
        .await
        .unwrap();
    archive_protocol_boards(&tx, &tenant, &event).await.unwrap();
    delete_election_event(&tx, &tenant, &event).await.unwrap();
    tx.commit().await.unwrap();

    for (table, column) in [
        ("election_event", "id"),
        ("election_lifecycle", "election_event_id"),
        ("election_execution_revision", "election_event_id"),
        ("election", "election_event_id"),
        ("ballot_box_seal", "election_event_id"),
    ] {
        let left: i64 = client
            .query_one(
                &format!("SELECT count(*) FROM sequent_backend.{table} WHERE {column} = $1"),
                &[&w.event],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(left, 0, "{table}");
    }
}

#[tokio::test]
async fn an_approval_event_is_exported_and_deleted_once_archived() {
    let w = empty_world().await;
    // The event's revision exists although nothing has happened to it.
    let revision: i64 = w
        .pool
        .get()
        .await
        .unwrap()
        .query_one(
            "SELECT revision FROM sequent_backend.election_execution_revision
             WHERE election_event_id = $1",
            &[&w.event],
        )
        .await
        .unwrap()
        .get(0);
    assert!(revision >= 1);
    rules(&w).await;
    archive(&w).await;

    export_then_delete(&w, None).await;
}

#[tokio::test]
async fn an_archived_event_is_exported_and_deleted_in_preparation() {
    let w = world("archived-deleted").await;
    rules(&w).await;
    archive(&w).await;
    assert_eq!(lifecycle(&w).await, ("preparation".to_owned(), 0));

    export_then_delete(&w, None).await;
}

#[tokio::test]
async fn an_archived_event_in_its_lockdown_is_exported_and_deleted() {
    let w = world("archived-locked").await;
    rules(&w).await;
    server_write(
        &w,
        r#"UPDATE sequent_backend.election_event
           SET is_archived = true, presentation = '{"locked_down": "locked-down"}'
           WHERE id = $1"#,
        &[&w.event],
    )
    .await;

    // The refusal the deletion task tolerates, and the only one there is.
    export_then_delete(&w, Some(ElectionEventDeletionRefusal::LockedDown)).await;
}

#[tokio::test]
async fn an_event_with_a_failed_seal_is_exported_and_deleted() {
    let w = world("failed-seal-deleted").await;
    rules(&w).await;
    evaluating(&w).await;
    let failed = pending_seal(&w, w.post, w.area).await;
    fail_seal(&w, failed).await;

    start(&w).await.unwrap();

    export_then_delete(&w, None).await;
}
