// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot box seals in the Hasura database (migration
//! `1791000001600_ballot_box_seal` and the ones after it): the seal rows are
//! permanent and follow their status flow, the seal record's document is
//! kept, and the event's seal settings are locked once voting has opened.
//! The guard of the ballots is in the event's electoral-log database: its
//! tests are electoral-log's `tests/ballot_box.rs`.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;

use chrono::{Duration, Utc};
use deadpool_postgres::Transaction;
use serde_json::{json, Value};
use tokio_postgres::types::ToSql;
use uuid::Uuid;
use windmill::postgres::ballot_box_seal::{
    any_for_election, any_for_event, get_for_box, get_for_update, insert_pending, list_due_pending,
    list_for_elections, list_open_work_ids, list_sealed_unpublished, mark_failed, mark_published,
    mark_sealed, set_close_provenance, BallotBoxSealStatus, ClosedBy, ClosedBySigner,
    NewPendingSeal, PublishedFields, SealLogRecord, SealedFields,
};
use windmill::postgres::trusted_write;

#[derive(Clone, Copy)]
struct World {
    tenant: Uuid,
    event: Uuid,
    election: Uuid,
    area: Uuid,
    other_area: Uuid,
}

const SEAL_AT_CLOSE: &str = "seal-at-close";
const DO_NOT_SEAL: &str = "do-not-seal";

/// A tenant, an event that seals at close, one election and two areas.
async fn world(tx: &Transaction<'_>) -> World {
    world_with_policy(tx, SEAL_AT_CLOSE).await
}

/// A tenant, an event with the given seal policy, one election and two
/// areas.
async fn world_with_policy(tx: &Transaction<'_>, policy: &str) -> World {
    let w = World {
        tenant: Uuid::new_v4(),
        event: Uuid::new_v4(),
        election: Uuid::new_v4(),
        area: Uuid::new_v4(),
        other_area: Uuid::new_v4(),
    };
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&w.tenant, &format!("tenant-{}", w.tenant)],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event
             (id, tenant_id, encryption_protocol, status, presentation)
         VALUES ($1, $2, 'RSA256', $3, $4)",
        &[
            &w.event,
            &w.tenant,
            &json!({"is_published": true, "voting_status": "NOT_STARTED"}),
            &json!({ "ballot_box_seal_policy": policy }),
        ],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
         VALUES ($1, $2, $3)",
        &[&w.election, &w.tenant, &w.event],
    )
    .await
    .unwrap();
    for area in [w.area, w.other_area] {
        tx.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, 'Area')",
            &[&area, &w.tenant, &w.event],
        )
        .await
        .unwrap();
    }
    w
}

fn pending(w: &World, area: Uuid, deadline_in: Duration) -> NewPendingSeal {
    let now = Utc::now();
    NewPendingSeal {
        tenant_id: w.tenant,
        election_event_id: w.event,
        election_id: w.election,
        area_id: area,
        closed_at: now,
        grace_deadline: now + deadline_in,
        close_request_id: None,
        closed_by: ClosedBy::User {
            username: Some("admin".into()),
        },
    }
}

/// Where a seal's record is on the event's electoral log.
fn log_record() -> SealLogRecord {
    SealLogRecord {
        delivery_id: "ballot-box-seal:event:election:area".into(),
        log_uid: Uuid::from_u128(42),
        leaf_index: 7,
    }
}

fn sealed_fields() -> SealedFields {
    SealedFields {
        sealed_at: Utc::now(),
        ballots_in_box: 2,
        ballots_counted: 1,
        seal_hash: "ab".repeat(64),
        manifest: vec![1, 2, 3],
        signed_message: vec![4, 5, 6],
        log_record: log_record(),
    }
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

/// A failed seal keeps the record of its failure, or none when the event
/// has no electoral log, and is final.
#[tokio::test]
async fn a_failed_seal_is_final() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    insert_pending(
        &tx,
        &[
            pending(&w, w.area, Duration::zero()),
            pending(&w, w.other_area, Duration::zero()),
        ],
    )
    .await
    .unwrap();
    let seals = list_for_elections(&tx, &w.tenant, &w.event, &[w.election])
        .await
        .unwrap();
    let (with_record, without_record) = (seals[0].id, seals[1].id);
    // Half a record is refused.
    let partial = "UPDATE sequent_backend.ballot_box_seal
         SET status = 'failed', failure_reason = 'x', log_delivery_id = 'd' WHERE id = $1";
    assert_eq!(refused(&tx, partial, &[&with_record]).await.0, "42501");
    mark_failed(&tx, &with_record, "ballot id mismatch", Some(&log_record()))
        .await
        .unwrap();
    mark_failed(&tx, &without_record, "no board", None)
        .await
        .unwrap();
    let row = get_for_update(&tx, &with_record).await.unwrap().unwrap();
    assert_eq!(row.status, BallotBoxSealStatus::Failed);
    assert_eq!(row.log_record, Some(log_record()));
    let row = get_for_update(&tx, &without_record).await.unwrap().unwrap();
    assert_eq!(row.log_record, None);
    for sql in [
        "UPDATE sequent_backend.ballot_box_seal SET failure_reason = 'other' WHERE id = $1",
        "UPDATE sequent_backend.ballot_box_seal SET log_leaf_index = 8 WHERE id = $1",
        "UPDATE sequent_backend.ballot_box_seal SET status = 'pending' WHERE id = $1",
    ] {
        assert_eq!(refused(&tx, sql, &[&with_record]).await.0, "42501", "{sql}");
    }
    assert!(mark_sealed(&tx, &without_record, &sealed_fields())
        .await
        .is_err());
    assert!(any_for_event(&tx, &w.tenant, &w.event).await.unwrap());
}

#[tokio::test]
async fn seals_are_permanent_and_follow_their_status_flow() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    assert!(!any_for_event(&tx, &w.tenant, &w.event).await.unwrap());
    insert_pending(&tx, &[pending(&w, w.area, Duration::zero())])
        .await
        .unwrap();
    // A second close inserts nothing.
    assert_eq!(
        insert_pending(&tx, &[pending(&w, w.area, Duration::zero())])
            .await
            .unwrap(),
        0
    );
    let id = list_for_elections(&tx, &w.tenant, &w.event, &[w.election])
        .await
        .unwrap()[0]
        .id;
    assert!(any_for_event(&tx, &w.tenant, &w.event).await.unwrap());

    // Inserting past pending, deleting and changing identity are refused.
    let (code, _) = refused(
        &tx,
        "INSERT INTO sequent_backend.ballot_box_seal
             (tenant_id, election_event_id, election_id, area_id, status,
              closed_at, grace_deadline, closed_by)
         VALUES ($1, $2, $3, $4, 'sealed', now(), now(), '{\"kind\": \"scheduled\"}')",
        &[&w.tenant, &w.event, &w.election, &w.other_area],
    )
    .await;
    assert_eq!(code, "42501");
    let delete = "DELETE FROM sequent_backend.ballot_box_seal WHERE id = $1";
    assert_eq!(refused(&tx, delete, &[&id]).await.0, "42501");
    let identity = "UPDATE sequent_backend.ballot_box_seal SET area_id = $2 WHERE id = $1";
    assert_eq!(
        refused(&tx, identity, &[&id, &w.other_area]).await.0,
        "42501"
    );
    let skip = "UPDATE sequent_backend.ballot_box_seal
         SET status = 'published', published_at = now() WHERE id = $1";
    assert_eq!(refused(&tx, skip, &[&id]).await.0, "42501");

    // The signed close completes the provenance while pending.
    let signed = ClosedBy::Signed {
        signers: Some(vec![ClosedBySigner {
            name: "Chair".into(),
            certificate_sha256: "ab".into(),
        }]),
        signing_code: Some("CODE".into()),
    };
    let request = Uuid::new_v4();
    assert_eq!(
        set_close_provenance(
            &tx,
            &w.tenant,
            &w.event,
            &w.election,
            Some(request),
            &signed
        )
        .await
        .unwrap(),
        1
    );

    let locked = get_for_update(&tx, &id).await.unwrap().unwrap();
    assert_eq!(locked.status, BallotBoxSealStatus::Pending);
    assert_eq!(locked.close_request_id, Some(request));
    assert_eq!(locked.closed_by, signed);
    // A seal is stored with its record on the electoral log.
    let no_record = "UPDATE sequent_backend.ballot_box_seal
         SET status = 'sealed', sealed_at = now(), ballots_in_box = 0, ballots_counted = 0,
             seal_hash = 'ab', manifest = '\\x01', signed_message = '\\x02'
         WHERE id = $1";
    assert_eq!(refused(&tx, no_record, &[&id]).await.0, "42501");
    mark_sealed(&tx, &id, &sealed_fields()).await.unwrap();
    assert_eq!(
        get_for_update(&tx, &id).await.unwrap().unwrap().log_record,
        Some(log_record())
    );
    // Sealed rows don't move back, nor take a failure or new provenance.
    assert!(mark_failed(&tx, &id, "late", None).await.is_err());
    assert_eq!(
        set_close_provenance(
            &tx,
            &w.tenant,
            &w.event,
            &w.election,
            None,
            &ClosedBy::Scheduled
        )
        .await
        .unwrap(),
        0
    );
    for tamper in [
        "UPDATE sequent_backend.ballot_box_seal SET seal_hash = 'cd' WHERE id = $1",
        "UPDATE sequent_backend.ballot_box_seal SET log_leaf_index = 8 WHERE id = $1",
    ] {
        assert_eq!(refused(&tx, tamper, &[&id]).await.0, "42501", "{tamper}");
    }
    assert_eq!(
        list_sealed_unpublished(&tx, 1000)
            .await
            .unwrap()
            .iter()
            .filter(|seal| seal.id == id)
            .count(),
        1
    );

    // The publication fields are set once, then published is final.
    let document = Uuid::new_v4();
    tx.execute(
        "UPDATE sequent_backend.ballot_box_seal SET public_document_id = $2 WHERE id = $1",
        &[&id, &document],
    )
    .await
    .unwrap();
    let rewrite =
        "UPDATE sequent_backend.ballot_box_seal SET public_document_id = $2 WHERE id = $1";
    assert_eq!(
        refused(&tx, rewrite, &[&id, &Uuid::new_v4()]).await.0,
        "42501"
    );
    // A public record has a public path (voting never opened here, so the
    // record policy is still free).
    set_record_policy(&tx, &w, json!("public")).await;
    let published = PublishedFields {
        public_document_id: document,
        public_path: Some(format!("ballot-box-seals/{}/{}.json", w.election, w.area)),
        published_at: Utc::now(),
    };
    mark_published(&tx, &id, &published).await.unwrap();
    let final_update = "UPDATE sequent_backend.ballot_box_seal SET public_path = 'x' WHERE id = $1";
    assert_eq!(refused(&tx, final_update, &[&id]).await.0, "42501");
    let row = get_for_update(&tx, &id).await.unwrap().unwrap();
    assert_eq!(row.status, BallotBoxSealStatus::Published);
    assert_eq!(row.manifest, Some(vec![1, 2, 3]));
    assert_eq!(row.public_path, published.public_path);
}

/// A restricted seal record is a private document: the seal is published
/// with its document id and no public path, and is then final.
#[tokio::test]
async fn a_restricted_record_is_published_without_a_public_path() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    insert_pending(&tx, &[pending(&w, w.area, Duration::zero())])
        .await
        .unwrap();
    let id = list_for_elections(&tx, &w.tenant, &w.event, &[w.election])
        .await
        .unwrap()[0]
        .id;
    mark_sealed(&tx, &id, &sealed_fields()).await.unwrap();
    // The document id is still required.
    let no_document = "UPDATE sequent_backend.ballot_box_seal
         SET status = 'published', published_at = now() WHERE id = $1";
    assert_eq!(refused(&tx, no_document, &[&id]).await.0, "42501");
    let published = PublishedFields {
        public_document_id: Uuid::new_v4(),
        public_path: None,
        published_at: Utc::now(),
    };
    mark_published(&tx, &id, &published).await.unwrap();
    let row = get_for_update(&tx, &id).await.unwrap().unwrap();
    assert_eq!(row.status, BallotBoxSealStatus::Published);
    assert_eq!(row.public_document_id, Some(published.public_document_id));
    assert_eq!(row.public_path, None);
    // A public path can't be added afterwards.
    let late_path = "UPDATE sequent_backend.ballot_box_seal SET public_path = 'x' WHERE id = $1";
    assert_eq!(refused(&tx, late_path, &[&id]).await.0, "42501");
}

/// Sets the event's Seal Record Publication policy.
async fn set_record_policy(tx: &Transaction<'_>, w: &World, policy: Value) {
    tx.execute(
        "UPDATE sequent_backend.election_event
         SET presentation = jsonb_set(presentation, '{ballot_box_seal_record_policy}', $2)
         WHERE id = $1",
        &[&w.event, &policy],
    )
    .await
    .unwrap();
}

/// A sealed seal of the world's first box, ready to publish.
async fn sealed_seal(tx: &Transaction<'_>, w: &World) -> Uuid {
    insert_pending(tx, &[pending(w, w.area, Duration::zero())])
        .await
        .unwrap();
    let id = list_for_elections(tx, &w.tenant, &w.event, &[w.election])
        .await
        .unwrap()[0]
        .id;
    mark_sealed(tx, &id, &sealed_fields()).await.unwrap();
    id
}

/// R10 S2: a seal reaches `published` with a public path exactly when the
/// event's effective record policy is public.
#[tokio::test]
async fn the_public_path_follows_the_record_policy() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let publish = "UPDATE sequent_backend.ballot_box_seal
         SET status = 'published', public_document_id = $2,
             public_path = $3, published_at = now()
         WHERE id = $1";
    let path = Some("ballot-box-seals/e/a.json".to_string());
    let none: Option<String> = None;

    // Restricted (unset): a public path is refused.
    let w = world(&tx).await;
    let id = sealed_seal(&tx, &w).await;
    let (code, message) = refused(&tx, publish, &[&id, &Uuid::new_v4(), &path]).await;
    assert_eq!(code, "42501", "{message}");
    // Spelt out, the same.
    set_record_policy(&tx, &w, json!("restricted")).await;
    assert_eq!(
        refused(&tx, publish, &[&id, &Uuid::new_v4(), &path])
            .await
            .0,
        "42501"
    );

    // Public: a missing public path is refused, a path is accepted.
    let w = world(&tx).await;
    set_record_policy(&tx, &w, json!("public")).await;
    let id = sealed_seal(&tx, &w).await;
    let (code, message) = refused(&tx, publish, &[&id, &Uuid::new_v4(), &none]).await;
    assert_eq!(code, "42501", "{message}");
    tx.execute(publish, &[&id, &Uuid::new_v4(), &path])
        .await
        .unwrap();
}

/// R10 S1: once a seal is published, its record's document row can't be
/// changed or deleted: the record would no longer be found. Before, and for
/// other documents, nothing changes.
#[tokio::test]
async fn a_published_record_keeps_its_document() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    let document = Uuid::new_v4();
    let other = Uuid::new_v4();
    for id in [document, other] {
        tx.execute(
            "INSERT INTO sequent_backend.document
                 (id, tenant_id, election_event_id, name, media_type, size, is_public)
             VALUES ($1, $2, $3, 'ballot-box-seals/e/a.json', 'application/json', 3, false)",
            &[&id, &w.tenant, &w.event],
        )
        .await
        .unwrap();
    }
    let rename = "UPDATE sequent_backend.document SET name = 'renamed' WHERE id = $1";
    let delete = "DELETE FROM sequent_backend.document WHERE id = $1";
    let id = sealed_seal(&tx, &w).await;
    // Not published yet: the publisher may still rewrite it.
    tx.execute(
        "UPDATE sequent_backend.document SET size = 4 WHERE id = $1",
        &[&document],
    )
    .await
    .unwrap();
    mark_published(
        &tx,
        &id,
        &PublishedFields {
            public_document_id: document,
            public_path: None,
            published_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    for sql in [rename, delete] {
        let (code, message) = refused(&tx, sql, &[&document]).await;
        assert_eq!(code, "42501", "{message}");
        assert!(message.contains("seal record"), "{message}");
    }
    // A trusted write is refused too.
    trusted_write(&tx).await.unwrap();
    assert_eq!(refused(&tx, delete, &[&document]).await.0, "42501");
    let (code, _) = refused(&tx, "TRUNCATE sequent_backend.document CASCADE", &[]).await;
    assert_eq!(code, "42501");
    // Another document is free.
    tx.execute(rename, &[&other]).await.unwrap();
    tx.execute(delete, &[&other]).await.unwrap();
}

/// R10 S3: on a seal-at-close event, the record policy is `restricted`,
/// `public`, absent or JSON null, on every write; anything else is refused
/// with a clear error, also before voting opens. A policy-off event doesn't
/// read it, so its writes are as before.
#[tokio::test]
async fn an_unknown_record_policy_is_refused_on_every_write() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    let set = "UPDATE sequent_backend.election_event
         SET presentation = jsonb_set(presentation, '{ballot_box_seal_record_policy}', $2)
         WHERE id = $1";
    for value in [
        json!("PUBLIC"),
        json!(""),
        json!(true),
        json!({"policy": "public"}),
    ] {
        let (code, message) = refused(&tx, set, &[&w.event, &value]).await;
        assert_eq!(code, "22023", "{value}: {message}");
        assert!(
            message.contains("ballot_box_seal_record_policy"),
            "{message}"
        );
    }
    // An insert too.
    let (code, _) = refused(
        &tx,
        "INSERT INTO sequent_backend.election_event
             (id, tenant_id, encryption_protocol, presentation)
         VALUES ($1, $2, 'RSA256', $3)",
        &[
            &Uuid::new_v4(),
            &w.tenant,
            &json!({"ballot_box_seal_policy": SEAL_AT_CLOSE,
                    "ballot_box_seal_record_policy": "Public"}),
        ],
    )
    .await;
    assert_eq!(code, "22023");
    for value in [json!("restricted"), json!("public"), Value::Null] {
        tx.execute(set, &[&w.event, &value]).await.unwrap();
    }

    // A policy-off event keeps any value, and can't become seal-at-close
    // with an unknown one.
    let off = world_with_policy(&tx, DO_NOT_SEAL).await;
    tx.execute(set, &[&off.event, &json!("PUBLIC")])
        .await
        .unwrap();
    let (code, _) = refused(
        &tx,
        "UPDATE sequent_backend.election_event
         SET presentation = jsonb_set(presentation, '{ballot_box_seal_policy}', '\"seal-at-close\"')
         WHERE id = $1",
        &[&off.event],
    )
    .await;
    assert_eq!(code, "22023");
}

/// R10 S3, S6: after voting opened, JSON null and a missing record
/// policy both read as restricted, so moving between them is accepted.
#[tokio::test]
async fn a_null_record_policy_is_restricted_once_voting_opened() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    open_voting(&tx, &w).await;
    set_record_policy(&tx, &w, Value::Null).await;
    tx.execute(
        "UPDATE sequent_backend.election_event
         SET presentation = presentation - 'ballot_box_seal_record_policy' WHERE id = $1",
        &[&w.event],
    )
    .await
    .unwrap();
    set_record_policy(&tx, &w, Value::Null).await;
    set_record_policy(&tx, &w, json!("restricted")).await;
    let (code, _) = refused(
        &tx,
        "UPDATE sequent_backend.election_event
         SET presentation = jsonb_set(presentation, '{ballot_box_seal_record_policy}', '\"public\"')
         WHERE id = $1",
        &[&w.event],
    )
    .await;
    assert_eq!(code, "42501");
}

#[tokio::test]
async fn due_pending_waits_for_the_grace_deadline() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    insert_pending(
        &tx,
        &[
            pending(&w, w.area, Duration::zero()),
            pending(&w, w.other_area, Duration::minutes(15)),
        ],
    )
    .await
    .unwrap();
    let due = |seals: Vec<windmill::postgres::ballot_box_seal::BallotBoxSeal>| {
        seals
            .into_iter()
            .filter(|seal| seal.election_event_id == w.event)
            .map(|seal| seal.area_id)
            .collect::<Vec<_>>()
    };
    let now = Utc::now() + Duration::seconds(1);
    assert_eq!(
        due(list_due_pending(&tx, now, 1000).await.unwrap()),
        vec![w.area]
    );
    assert_eq!(
        due(list_due_pending(&tx, now + Duration::minutes(15), 1000)
            .await
            .unwrap())
        .len(),
        2
    );
}

#[tokio::test]
async fn sealed_tables_refuse_truncate() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    insert_pending(&tx, &[pending(&w, w.area, Duration::zero())])
        .await
        .unwrap();
    assert_eq!(
        refused(&tx, "TRUNCATE sequent_backend.ballot_box_seal", &[])
            .await
            .0,
        "42501"
    );
    let id = list_for_elections(&tx, &w.tenant, &w.event, &[w.election])
        .await
        .unwrap()[0]
        .id;
    mark_sealed(&tx, &id, &sealed_fields()).await.unwrap();
    assert_eq!(
        refused(&tx, "TRUNCATE sequent_backend.ballot_box_seal", &[])
            .await
            .0,
        "42501"
    );
}

#[tokio::test]
async fn the_seal_policy_locks_once_voting_opened() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    let set_policy = "UPDATE sequent_backend.election_event
         SET presentation = jsonb_set(presentation, '{ballot_box_seal_policy}', $2)
         WHERE id = $1";
    // Before voting opens it changes freely.
    tx.execute(set_policy, &[&w.event, &json!("do-not-seal")])
        .await
        .unwrap();
    tx.execute(set_policy, &[&w.event, &json!("seal-at-close")])
        .await
        .unwrap();

    trusted_write(&tx).await.unwrap();
    let started: Value = json!({
        "voting_status": "CLOSED",
        "voting_period_dates": {"first_started_at": "2028-04-09T00:00:00.000Z"}
    });
    tx.execute(
        "UPDATE sequent_backend.election SET status = $2 WHERE id = $1",
        &[&w.election, &started],
    )
    .await
    .unwrap();

    // Even a trusted write can't change it now; keeping it passes.
    assert_eq!(
        refused(&tx, set_policy, &[&w.event, &json!("do-not-seal")])
            .await
            .0,
        "42501"
    );
    tx.execute(set_policy, &[&w.event, &json!("seal-at-close")])
        .await
        .unwrap();
}

#[tokio::test]
async fn the_seal_policy_locks_once_the_event_opened() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    trusted_write(&tx).await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.election_event SET status = $2 WHERE id = $1",
        &[
            &w.event,
            &json!({"is_published": true, "voting_status": "OPEN"}),
        ],
    )
    .await
    .unwrap();
    // Missing and unknown values read as do-not-seal, so dropping the key
    // changes the policy and is refused.
    assert_eq!(
        refused(
            &tx,
            "UPDATE sequent_backend.election_event
             SET presentation = presentation - 'ballot_box_seal_policy' WHERE id = $1",
            &[&w.event],
        )
        .await
        .0,
        "42501"
    );
}

/// Marks voting as having opened on the world's election (a trusted
/// write): it started once and is closed now.
async fn open_voting(tx: &Transaction<'_>, w: &World) {
    trusted_write(tx).await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.election SET status = $2 WHERE id = $1",
        &[
            &w.election,
            &json!({
                "voting_status": "CLOSED",
                "voting_period_dates": {"first_started_at": "2028-04-09T00:00:00.000Z"}
            }),
        ],
    )
    .await
    .unwrap();
}

/// The Seal Record Publication policy is locked with the other settings the
/// seal relies on once voting opened on a seal-at-close event; its effective
/// value counts (unset is restricted).
#[tokio::test]
async fn the_record_policy_locks_once_voting_opened() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    let set_record_policy = "UPDATE sequent_backend.election_event
         SET presentation = jsonb_set(presentation, '{ballot_box_seal_record_policy}', $2)
         WHERE id = $1";
    let drop_record_policy = "UPDATE sequent_backend.election_event
         SET presentation = presentation - 'ballot_box_seal_record_policy' WHERE id = $1";
    // Before voting opens it changes freely.
    tx.execute(set_record_policy, &[&w.event, &json!("public")])
        .await
        .unwrap();
    tx.execute(drop_record_policy, &[&w.event]).await.unwrap();

    open_voting(&tx, &w).await;

    // Unset reads as restricted: spelling it out passes, making it public
    // doesn't, even for a trusted write.
    tx.execute(set_record_policy, &[&w.event, &json!("restricted")])
        .await
        .unwrap();
    let (code, message) = refused(&tx, set_record_policy, &[&w.event, &json!("public")]).await;
    assert_eq!(code, "42501");
    assert!(
        message.contains("ballot_box_seal_record_policy"),
        "{message}"
    );
    tx.execute(drop_record_policy, &[&w.event]).await.unwrap();
}

/// A public record policy stays public once voting opened: dropping the key
/// would make it restricted.
#[tokio::test]
async fn a_public_record_policy_stays_public_once_voting_opened() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    tx.execute(
        "UPDATE sequent_backend.election_event
         SET presentation = presentation || '{\"ballot_box_seal_record_policy\": \"public\"}'
         WHERE id = $1",
        &[&w.event],
    )
    .await
    .unwrap();
    open_voting(&tx, &w).await;
    let (code, message) = refused(
        &tx,
        "UPDATE sequent_backend.election_event
         SET presentation = presentation - 'ballot_box_seal_record_policy' WHERE id = $1",
        &[&w.event],
    )
    .await;
    assert_eq!(code, "42501");
    assert!(
        message.contains("ballot_box_seal_record_policy"),
        "{message}"
    );
}

/// An event that doesn't seal never locks the record policy.
#[tokio::test]
async fn the_record_policy_is_free_when_the_event_does_not_seal() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world_with_policy(&tx, DO_NOT_SEAL).await;
    open_voting(&tx, &w).await;
    tx.execute(
        "UPDATE sequent_backend.election_event
         SET presentation = presentation || '{\"ballot_box_seal_record_policy\": \"public\"}'
         WHERE id = $1",
        &[&w.event],
    )
    .await
    .unwrap();
}

/// One box's full seal comes from `get_for_box`; the lists leave the large
/// columns out; the dispatcher reads ids only.
#[tokio::test]
async fn reads_take_only_what_they_need() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    insert_pending(
        &tx,
        &[
            pending(&w, w.area, -Duration::seconds(1)),
            pending(&w, w.other_area, Duration::hours(1)),
        ],
    )
    .await
    .unwrap();
    let seals = list_for_elections(&tx, &w.tenant, &w.event, &[w.election])
        .await
        .unwrap();
    let sealed = seals.iter().find(|seal| seal.area_id == w.area).unwrap();
    let not_due = seals
        .iter()
        .find(|seal| seal.area_id == w.other_area)
        .unwrap();
    mark_sealed(&tx, &sealed.id, &sealed_fields())
        .await
        .unwrap();

    let full = get_for_box(&tx, &w.tenant, &w.event, &w.election, &w.area)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(full.id, sealed.id);
    assert_eq!(full.manifest, Some(sealed_fields().manifest));
    assert_eq!(full.signed_message, Some(sealed_fields().signed_message));
    assert!(
        get_for_box(&tx, &w.tenant, &w.event, &w.election, &Uuid::new_v4())
            .await
            .unwrap()
            .is_none()
    );

    let listed = list_sealed_unpublished(&tx, 1000)
        .await
        .unwrap()
        .into_iter()
        .find(|seal| seal.id == sealed.id)
        .unwrap();
    assert_eq!(listed.manifest, None);
    assert_eq!(listed.signed_message, None);
    assert_eq!(listed.seal_hash, Some("ab".repeat(64)));

    let ids = list_open_work_ids(&tx, Utc::now(), 1000).await.unwrap();
    assert!(ids.contains(&sealed.id));
    assert!(!ids.contains(&not_due.id), "not due yet");

    assert!(any_for_election(&tx, &w.tenant, &w.event, &w.election)
        .await
        .unwrap());
    assert!(!any_for_election(&tx, &w.tenant, &w.event, &Uuid::new_v4())
        .await
        .unwrap());
}
