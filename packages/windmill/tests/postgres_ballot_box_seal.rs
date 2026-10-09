// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot box seals (migration `1791000001600_ballot_box_seal`): seals are
//! permanent, a box whose seal is past `pending` takes no `cast_vote`
//! writes, the sealer's advisory lock orders casts around the seal, and the
//! event's seal policy is locked once voting has opened.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;

use chrono::{Duration, Utc};
use deadpool_postgres::Transaction;
use serde_json::{json, Value};
use std::time::Duration as StdDuration;
use tokio_postgres::types::ToSql;
use uuid::Uuid;
use windmill::postgres::ballot_box_seal::{
    any_for_election, any_for_event, ballot_box_lock_key, get_for_box, get_for_update,
    insert_pending, list_due_pending, list_for_elections, list_open_work_ids,
    list_sealed_unpublished, lock_box, mark_failed, mark_published, mark_sealed,
    set_close_provenance, BallotBoxSealStatus, ClosedBy, ClosedBySigner, NewPendingSeal,
    PublishedFields, SealedFields, BALLOT_BOX_SEALED_ERROR,
    BALLOT_BOX_SEAL_REQUIRES_READ_COMMITTED_ERROR,
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

fn sealed_fields() -> SealedFields {
    SealedFields {
        sealed_at: Utc::now(),
        ballots_in_box: 2,
        ballots_counted: 1,
        seal_hash: "ab".repeat(64),
        manifest: vec![1, 2, 3],
        signed_message: vec![4, 5, 6],
    }
}

/// Casts a ballot of a new voter into `area`.
async fn cast(tx: &Transaction<'_>, w: &World, area: Uuid) -> Result<u64, tokio_postgres::Error> {
    tx.execute(
        "INSERT INTO sequent_backend.cast_vote
             (tenant_id, election_event_id, election_id, area_id, voter_id_string, status)
         VALUES ($1, $2, $3, $4, $5, 'valid')",
        &[
            &w.tenant,
            &w.event,
            &w.election,
            &area,
            &Uuid::new_v4().to_string(),
        ],
    )
    .await
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

#[tokio::test]
async fn lock_key_is_the_sql_key() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    let sql: String = tx
        .query_one(
            "SELECT sequent_backend.ballot_box_lock_key($1, $2, $3, $4)",
            &[&w.tenant, &w.event, &w.election, &w.area],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(
        sql,
        ballot_box_lock_key(&w.tenant, &w.event, &w.election, &w.area)
    );
}

#[tokio::test]
async fn a_sealed_box_refuses_every_cast_vote_write() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    assert_eq!(
        insert_pending(&tx, &[pending(&w, w.area, Duration::zero())])
            .await
            .unwrap(),
        1
    );
    // Pending: the box still takes ballots.
    cast(&tx, &w, w.area).await.unwrap();
    let seal = list_for_elections(&tx, &w.tenant, &w.event, &[w.election])
        .await
        .unwrap()
        .remove(0);
    mark_sealed(&tx, &seal.id, &sealed_fields()).await.unwrap();

    let insert = "INSERT INTO sequent_backend.cast_vote
             (tenant_id, election_event_id, election_id, area_id, voter_id_string)
         VALUES ($1, $2, $3, $4, 'late')";
    let refusal = refused(&tx, insert, &[&w.tenant, &w.event, &w.election, &w.area]).await;
    assert_eq!(
        refusal,
        ("42501".to_owned(), BALLOT_BOX_SEALED_ERROR.to_owned())
    );
    let update = "UPDATE sequent_backend.cast_vote SET status = 'discarded'
         WHERE election_id = $1 AND area_id = $2";
    assert_eq!(
        refused(&tx, update, &[&w.election, &w.area]).await.1,
        BALLOT_BOX_SEALED_ERROR
    );
    let delete = "DELETE FROM sequent_backend.cast_vote WHERE election_id = $1 AND area_id = $2";
    assert_eq!(
        refused(&tx, delete, &[&w.election, &w.area]).await.1,
        BALLOT_BOX_SEALED_ERROR
    );
    // Moving a ballot out of or into a sealed box is refused too.
    cast(&tx, &w, w.other_area).await.unwrap();
    let move_in = "UPDATE sequent_backend.cast_vote SET area_id = $2
         WHERE election_id = $1 AND area_id = $3";
    assert_eq!(
        refused(&tx, move_in, &[&w.election, &w.area, &w.other_area])
            .await
            .1,
        BALLOT_BOX_SEALED_ERROR
    );
    // Other boxes are untouched.
    cast(&tx, &w, w.other_area).await.unwrap();
}

#[tokio::test]
async fn a_failed_box_stays_locked() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = world(&tx).await;
    insert_pending(&tx, &[pending(&w, w.area, Duration::zero())])
        .await
        .unwrap();
    let seal = list_for_elections(&tx, &w.tenant, &w.event, &[w.election])
        .await
        .unwrap()
        .remove(0);
    mark_failed(&tx, &seal.id, "ballot id mismatch")
        .await
        .unwrap();
    let insert = "INSERT INTO sequent_backend.cast_vote
             (tenant_id, election_event_id, election_id, area_id, voter_id_string)
         VALUES ($1, $2, $3, $4, 'late')";
    assert_eq!(
        refused(&tx, insert, &[&w.tenant, &w.event, &w.election, &w.area])
            .await
            .1,
        BALLOT_BOX_SEALED_ERROR
    );
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
    mark_sealed(&tx, &id, &sealed_fields()).await.unwrap();
    // Sealed rows don't move back, nor take a failure or new provenance.
    assert!(mark_failed(&tx, &id, "late").await.is_err());
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
    let tamper = "UPDATE sequent_backend.ballot_box_seal SET seal_hash = 'cd' WHERE id = $1";
    assert_eq!(refused(&tx, tamper, &[&id]).await.0, "42501");
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
    tx.execute(
        "UPDATE sequent_backend.ballot_box_seal SET log_entry_id = 7 WHERE id = $1",
        &[&id],
    )
    .await
    .unwrap();
    let rewrite = "UPDATE sequent_backend.ballot_box_seal SET log_entry_id = 8 WHERE id = $1";
    assert_eq!(refused(&tx, rewrite, &[&id]).await.0, "42501");
    let published = PublishedFields {
        log_entry_id: 7,
        public_document_id: Uuid::new_v4(),
        public_path: format!("ballot-box-seals/{}/{}.json", w.election, w.area),
        published_at: Utc::now(),
    };
    mark_published(&tx, &id, &published).await.unwrap();
    let final_update = "UPDATE sequent_backend.ballot_box_seal SET public_path = 'x' WHERE id = $1";
    assert_eq!(refused(&tx, final_update, &[&id]).await.0, "42501");
    let row = get_for_update(&tx, &id).await.unwrap().unwrap();
    assert_eq!(row.status, BallotBoxSealStatus::Published);
    assert_eq!(row.manifest, Some(vec![1, 2, 3]));
    assert_eq!(row.public_path, Some(published.public_path));
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
        refused(&tx, "TRUNCATE sequent_backend.cast_vote", &[])
            .await
            .1,
        BALLOT_BOX_SEALED_ERROR
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

/// A cast that arrives while the sealer holds the box waits, then sees the
/// seal and is refused.
#[tokio::test]
async fn a_cast_during_the_seal_waits_and_is_refused() {
    let pool = schema::pool().await;
    let w = {
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = world(&tx).await;
        insert_pending(&tx, &[pending(&w, w.area, Duration::zero())])
            .await
            .unwrap();
        tx.commit().await.unwrap();
        w
    };
    let mut sealer_client = pool.get().await.unwrap();
    let sealer = sealer_client.transaction().await.unwrap();
    let seal = list_for_elections(&sealer, &w.tenant, &w.event, &[w.election])
        .await
        .unwrap()
        .remove(0);
    lock_box(&sealer, &seal.lock_key()).await.unwrap();

    let caster_pool = pool.clone();
    let (tenant, event, election, area) = (w.tenant, w.event, w.election, w.area);
    let caster = tokio::spawn(async move {
        let mut client = caster_pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = World {
            tenant,
            event,
            election,
            area,
            other_area: area,
        };
        let result = cast(&tx, &w, area).await;
        result.map_err(|error| error.as_db_error().map(|db| db.message().to_owned()))
    });
    tokio::time::sleep(StdDuration::from_millis(300)).await;
    assert!(!caster.is_finished(), "the cast waits for the seal");

    get_for_update(&sealer, &seal.id).await.unwrap().unwrap();
    mark_sealed(&sealer, &seal.id, &sealed_fields())
        .await
        .unwrap();
    sealer.commit().await.unwrap();

    let result = tokio::time::timeout(StdDuration::from_secs(10), caster)
        .await
        .expect("the cast resumes after the seal")
        .unwrap();
    assert_eq!(result, Err(Some(BALLOT_BOX_SEALED_ERROR.to_owned())));
}

/// Above READ COMMITTED a write's snapshot could miss a seal committed while
/// it waited for the box lock, so a seal-at-close event refuses it; an event
/// that doesn't seal is unaffected.
#[tokio::test]
async fn repeatable_read_writes_are_refused_only_when_the_event_seals() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .await
        .unwrap();
    let sealing = world_with_policy(&tx, SEAL_AT_CLOSE).await;
    let not_sealing = world_with_policy(&tx, DO_NOT_SEAL).await;
    let insert = "INSERT INTO sequent_backend.cast_vote
             (tenant_id, election_event_id, election_id, area_id, voter_id_string)
         VALUES ($1, $2, $3, $4, 'voter')";
    assert_eq!(
        refused(
            &tx,
            insert,
            &[
                &sealing.tenant,
                &sealing.event,
                &sealing.election,
                &sealing.area
            ],
        )
        .await,
        (
            "42501".to_owned(),
            BALLOT_BOX_SEAL_REQUIRES_READ_COMMITTED_ERROR.to_owned()
        )
    );
    cast(&tx, &not_sealing, not_sealing.area).await.unwrap();
}

/// A cast already inside its transaction when the sealer arrives holds the
/// box: the sealer's lock waits for its commit, then the seal sees it.
#[tokio::test]
async fn a_cast_before_the_seal_commits_first_and_is_sealed() {
    let pool = schema::pool().await;
    let w = {
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let w = world(&tx).await;
        insert_pending(&tx, &[pending(&w, w.area, Duration::zero())])
            .await
            .unwrap();
        tx.commit().await.unwrap();
        w
    };
    let mut caster_client = pool.get().await.unwrap();
    let caster = caster_client.transaction().await.unwrap();
    cast(&caster, &w, w.area).await.unwrap();

    let sealer_pool = pool.clone();
    let sealer = tokio::spawn(async move {
        let mut client = sealer_pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let key = ballot_box_lock_key(&w.tenant, &w.event, &w.election, &w.area);
        lock_box(&tx, &key).await.unwrap();
        let seal = list_for_elections(&tx, &w.tenant, &w.event, &[w.election])
            .await
            .unwrap()
            .remove(0);
        get_for_update(&tx, &seal.id).await.unwrap().unwrap();
        let in_box: i64 = tx
            .query_one(
                "SELECT count(*) FROM sequent_backend.cast_vote
                 WHERE tenant_id = $1 AND election_event_id = $2
                   AND election_id = $3 AND area_id = $4",
                &[&w.tenant, &w.event, &w.election, &w.area],
            )
            .await
            .unwrap()
            .get(0);
        mark_sealed(&tx, &seal.id, &sealed_fields()).await.unwrap();
        tx.commit().await.unwrap();
        in_box
    });
    tokio::time::sleep(StdDuration::from_millis(300)).await;
    assert!(!sealer.is_finished(), "the sealer waits for the cast");

    caster.commit().await.unwrap();
    let in_box = tokio::time::timeout(StdDuration::from_secs(10), sealer)
        .await
        .expect("the sealer resumes after the cast commits")
        .unwrap();
    assert_eq!(in_box, 1, "the seal sees the cast that committed first");
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
