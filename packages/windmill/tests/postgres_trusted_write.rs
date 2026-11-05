// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The database refuses a change of a voting status or of the lockdown
//! unless the server marked its transaction as trusted
//! (`postgres::trusted_write`). Writes that keep those values, such as an
//! admin form saving the whole record, still pass.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::{Pool, Transaction};
use sequent_core::ballot::{ElectionEventStatus, ElectionStatus};
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;
use windmill::postgres::trusted_write;
use windmill::services::election_event_status::{
    save_event_voting_status, update_election_voting_status_impl,
};
use windmill::tasks::manage_election_event_lockdown::manage_election_event_lockdown_wrapped;

struct World {
    pool: Pool,
    tenant: Uuid,
    event: Uuid,
    post: Uuid,
}

const POST_STATUS: &str = r#"{
    "is_published": true,
    "voting_status": "OPEN",
    "kiosk_voting_status": "NOT_STARTED",
    "init_report": "allowed",
    "allow_tally": "disallowed",
    "voting_period_dates": {"first_started_at": "2028-04-09T00:00:00.000Z"}
}"#;

const EVENT_STATUS: &str = r#"{"is_published": true, "voting_status": "OPEN"}"#;

/// A tenant, an event and its Post (election), committed.
async fn world() -> World {
    let pool = schema::pool().await;
    let w = World {
        pool,
        tenant: Uuid::new_v4(),
        event: Uuid::new_v4(),
        post: Uuid::new_v4(),
    };
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    // Voting is open and the event locked down, as only the server sets them.
    trusted_write(&tx).await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&w.tenant, &format!("tenant-{}", w.tenant)],
    )
    .await
    .unwrap();
    let event_status: Value = serde_json::from_str(EVENT_STATUS).unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event
             (id, tenant_id, encryption_protocol, status, presentation)
         VALUES ($1, $2, 'RSA256', $3, $4)",
        &[
            &w.event,
            &w.tenant,
            &event_status,
            &json!({"locked_down": "locked-down", "language_conf": {"default_language_code": "en"}}),
        ],
    )
    .await
    .unwrap();
    let post_status: Value = serde_json::from_str(POST_STATUS).unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, status, presentation)
         VALUES ($1, $2, $3, $4, $5)",
        &[&w.post, &w.tenant, &w.event, &post_status, &json!({"grace_period_policy": "grace-period-without-alert", "grace_period_secs": 60})],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    w
}

/// Inserts an election of the world's event with `status` and
/// `presentation`, untrusted.
async fn insert_post(
    tx: &Transaction<'_>,
    w: &World,
    status: Option<&Value>,
    presentation: &Value,
) -> Result<Uuid, String> {
    let id = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, status, presentation)
         VALUES ($1, $2, $3, $4, $5)",
        &[&id, &w.tenant, &w.event, &status, presentation],
    )
    .await
    .map(|_| id)
    .map_err(sqlstate)
}

/// Inserts an election event of the world's tenant, untrusted.
async fn insert_event(
    tx: &Transaction<'_>,
    w: &World,
    status: Option<&Value>,
    presentation: Option<&Value>,
) -> Result<u64, String> {
    tx.execute(
        "INSERT INTO sequent_backend.election_event
             (id, tenant_id, encryption_protocol, status, presentation)
         VALUES ($1, $2, 'RSA256', $3, $4)",
        &[&Uuid::new_v4(), &w.tenant, &status, &presentation],
    )
    .await
    .map_err(sqlstate)
}

async fn set_post_presentation(
    tx: &Transaction<'_>,
    post: Uuid,
    presentation: &Value,
) -> Result<u64, String> {
    tx.execute(
        "UPDATE sequent_backend.election SET presentation = $2 WHERE id = $1",
        &[&post, presentation],
    )
    .await
    .map_err(sqlstate)
}

async fn set_post_status(tx: &Transaction<'_>, w: &World, status: &Value) -> Result<u64, String> {
    tx.execute(
        "UPDATE sequent_backend.election SET status = $2 WHERE id = $1",
        &[&w.post, status],
    )
    .await
    .map_err(sqlstate)
}

async fn set_event_status(tx: &Transaction<'_>, w: &World, status: &Value) -> Result<u64, String> {
    tx.execute(
        "UPDATE sequent_backend.election_event SET status = $2 WHERE id = $1",
        &[&w.event, status],
    )
    .await
    .map_err(sqlstate)
}

async fn set_presentation(
    tx: &Transaction<'_>,
    w: &World,
    presentation: &Value,
) -> Result<u64, String> {
    tx.execute(
        "UPDATE sequent_backend.election_event SET presentation = $2 WHERE id = $1",
        &[&w.event, presentation],
    )
    .await
    .map_err(sqlstate)
}

fn sqlstate(error: tokio_postgres::Error) -> String {
    error
        .code()
        .map(|code| code.code().to_owned())
        .unwrap_or_else(|| error.to_string())
}

/// The stored Post status with `changes` merged in.
fn post_status_with(changes: Value) -> Value {
    let mut status: Value = serde_json::from_str(POST_STATUS).unwrap();
    for (key, value) in changes.as_object().unwrap() {
        status[key] = value.clone();
    }
    status
}

#[tokio::test]
async fn an_untrusted_change_of_a_voting_status_is_refused() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    for changes in [
        json!({"voting_status": "CLOSED"}),
        json!({"kiosk_voting_status": "OPEN"}),
        json!({"early_voting_status": "OPEN"}),
        json!({"telephone_voting_status": "OPEN"}),
        // A later close lets grace-period votes in after closing.
        json!({"voting_period_dates": {
            "first_started_at": "2028-04-09T00:00:00.000Z",
            "last_stopped_at": "2028-05-08T11:00:00Z"
        }}),
    ] {
        let tx = client.transaction().await.unwrap();
        assert_eq!(
            set_post_status(&tx, &w, &post_status_with(changes.clone())).await,
            Err("42501".to_owned()),
            "{changes}"
        );
    }
    // Clearing the whole status resets the voting status too.
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        set_post_status(&tx, &w, &Value::Null).await,
        Err("42501".to_owned())
    );
    drop(tx);
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        set_event_status(
            &tx,
            &w,
            &json!({"is_published": true, "voting_status": "CLOSED"})
        )
        .await,
        Err("42501".to_owned())
    );
}

#[tokio::test]
async fn an_untrusted_write_that_keeps_the_voting_status_passes() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    // Other keys change; the voting values are re-serialized as the server
    // does: defaults spelled out and dates in another format, same instant.
    let resaved = post_status_with(json!({
        "allow_tally": "allowed",
        "init_report": "disallowed",
        "early_voting_status": "NOT_STARTED",
        "telephone_voting_status": "NOT_STARTED",
        "voting_period_dates": {
            "first_started_at": "2028-04-09T04:00:00+04:00",
            "last_started_at": null
        },
        "kiosk_voting_period_dates": {}
    }));
    assert_eq!(set_post_status(&tx, &w, &resaved).await, Ok(1));
    assert_eq!(
        set_event_status(
            &tx,
            &w,
            &json!({"is_published": false, "voting_status": "OPEN", "kiosk_voting_status": "NOT_STARTED"})
        )
        .await,
        Ok(1)
    );
    // A save of other columns, with the status as loaded.
    assert_eq!(
        tx.execute(
            "UPDATE sequent_backend.election SET description = 'x', status = status WHERE id = $1",
            &[&w.post],
        )
        .await
        .map_err(sqlstate),
        Ok(1)
    );
}

#[tokio::test]
async fn an_untrusted_change_of_the_lockdown_is_refused() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    for presentation in [
        json!({"locked_down": "not-locked-down", "language_conf": {"default_language_code": "en"}}),
        json!({"language_conf": {"default_language_code": "en"}}),
        Value::Null,
    ] {
        let tx = client.transaction().await.unwrap();
        assert_eq!(
            set_presentation(&tx, &w, &presentation).await,
            Err("42501".to_owned()),
            "{presentation}"
        );
    }
}

#[tokio::test]
async fn an_untrusted_presentation_save_that_keeps_the_lockdown_passes() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        set_presentation(
            &tx,
            &w,
            &json!({"locked_down": "locked-down", "language_conf": {"default_language_code": "es"}})
        )
        .await,
        Ok(1)
    );
    // An event that was never locked down: no key and "not-locked-down" agree.
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&Uuid::new_v4(), &w.tenant],
    )
    .await
    .unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE sequent_backend.election_event
             SET presentation = '{\"locked_down\": \"not-locked-down\"}'
             WHERE tenant_id = $1 AND id <> $2",
            &[&w.tenant, &w.event],
        )
        .await
        .map_err(sqlstate),
        Ok(1)
    );
}

#[tokio::test]
async fn a_trusted_transaction_changes_the_guarded_values() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    assert_eq!(
        set_post_status(
            &tx,
            &w,
            &post_status_with(json!({"voting_status": "CLOSED"}))
        )
        .await,
        Ok(1)
    );
    assert_eq!(
        set_event_status(&tx, &w, &json!({"voting_status": "CLOSED"})).await,
        Ok(1)
    );
    assert_eq!(
        set_presentation(&tx, &w, &json!({"locked_down": "not-locked-down"})).await,
        Ok(1)
    );
    tx.commit().await.unwrap();

    // The mark ends with its transaction.
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        set_post_status(&tx, &w, &post_status_with(json!({"voting_status": "OPEN"}))).await,
        Err("42501".to_owned())
    );
}

/// Runs the lockdown task for a new scheduled event of `processor`,
/// committed, and returns the event's lockdown and the details of the
/// entries the task queued in the signing log outbox (USER, SYSTEM).
async fn run_lockdown(
    client: &mut deadpool_postgres::Object,
    tenant: Uuid,
    event: Uuid,
    processor: &str,
) -> (Value, Uuid, Vec<(String, String, Value)>) {
    let scheduled: Uuid = client
        .query_one(
            "INSERT INTO sequent_backend.scheduled_event
                 (tenant_id, election_event_id, event_processor)
             VALUES ($1, $2, $3) RETURNING id",
            &[&tenant, &event, &processor],
        )
        .await
        .unwrap()
        .get(0);
    let tx = client.transaction().await.unwrap();
    manage_election_event_lockdown_wrapped(
        &tx,
        tenant.to_string(),
        event.to_string(),
        scheduled.to_string(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let presentation: Value = client
        .query_one(
            "SELECT presentation FROM sequent_backend.election_event WHERE id = $1",
            &[&event],
        )
        .await
        .unwrap()
        .get(0);
    let entries = client
        .query(
            "SELECT event_type, username, body FROM sequent_backend.signing_log_outbox
             WHERE election_event_id = $1 AND statement_kind = 'LockdownChanged'
               AND body -> 'details' ->> 'scheduled_event_id' = $2
             ORDER BY id",
            &[&event, &scheduled.to_string()],
        )
        .await
        .unwrap()
        .into_iter()
        .map(|row| {
            (
                row.get::<_, String>(0),
                row.get::<_, Option<String>>(1).unwrap_or_default(),
                row.get::<_, Value>(2),
            )
        })
        .collect();
    (presentation["locked_down"].clone(), scheduled, entries)
}

/// The lockdown task (START/END_LOCKDOWN_PERIOD) still locks and unlocks,
/// and logs each change in the electoral log, in its own transaction.
#[tokio::test]
async fn the_lockdown_task_changes_the_lockdown_and_logs_it() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    for (processor, expected, locked_down, previous, description) in [
        (
            "END_LOCKDOWN_PERIOD",
            "not-locked-down",
            false,
            true,
            "Lifted the lockdown of the election event on schedule.",
        ),
        (
            "START_LOCKDOWN_PERIOD",
            "locked-down",
            true,
            false,
            "Locked down the election event on schedule.",
        ),
    ] {
        let (lockdown, scheduled, entries) =
            run_lockdown(&mut client, w.tenant, w.event, processor).await;
        assert_eq!(lockdown, expected, "{processor}");
        let details = json!({
            "locked_down": locked_down,
            "previous": previous,
            "source": "scheduled",
            "scheduled_event_id": scheduled.to_string(),
        });
        let body = json!({"description": description, "details": details});
        assert_eq!(
            entries,
            [
                (
                    "USER".to_owned(),
                    "scheduled-event".to_owned(),
                    body.clone()
                ),
                ("SYSTEM".to_owned(), String::new(), body),
            ],
            "{processor}"
        );
    }
}

/// An event that has no presentation yet is locked down too.
#[tokio::test]
async fn the_lockdown_task_locks_down_an_event_without_presentation() {
    let w = world().await;
    let event = Uuid::new_v4();
    let mut client = w.pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&event, &w.tenant],
        )
        .await
        .unwrap();
    let (lockdown, _, entries) =
        run_lockdown(&mut client, w.tenant, event, "START_LOCKDOWN_PERIOD").await;
    assert_eq!(lockdown, "locked-down");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].2["details"]["previous"], false);
}

/// The voting status action's write still passes. Its electoral log entry
/// comes after the write and needs a bulletin board, which this event has
/// not: the error is the log's, and the status already changed.
#[tokio::test]
async fn the_voting_status_action_changes_the_voting_status() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let error = update_election_voting_status_impl(
        w.tenant.to_string(),
        None,
        None,
        w.event.to_string(),
        w.post.to_string(),
        VotingStatus::PAUSED,
        VotingStatusChannel::ONLINE,
        None,
        &tx,
    )
    .await
    .unwrap_err();
    assert!(format!("{error:?}").contains("bulletin board"), "{error:?}");
    let status: Value = tx
        .query_one(
            "SELECT status FROM sequent_backend.election WHERE id = $1",
            &[&w.post],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(status["voting_status"], "PAUSED");
}

#[tokio::test]
async fn an_untrusted_insert_starts_from_the_default_state() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let open = json!({"voting_status": "OPEN"});
    let refused = [
        json!({"voting_status": "OPEN"}),
        json!({"early_voting_status": "PAUSED"}),
        json!({"voting_period_dates": {"last_stopped_at": "2028-05-08T11:00:00Z"}}),
    ];
    for status in &refused {
        let tx = client.transaction().await.unwrap();
        assert_eq!(
            insert_post(&tx, &w, Some(status), &json!({})).await,
            Err("42501".to_owned()),
            "{status}"
        );
    }
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        insert_event(&tx, &w, Some(&open), None).await,
        Err("42501".to_owned())
    );
    drop(tx);
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        insert_event(&tx, &w, None, Some(&json!({"locked_down": "locked-down"}))).await,
        Err("42501".to_owned())
    );
    drop(tx);

    // The defaults, spelled out or not, pass; so does a trusted insert.
    let tx = client.transaction().await.unwrap();
    let defaults = serde_json::to_value(ElectionStatus::default()).unwrap();
    assert!(insert_post(&tx, &w, None, &json!({})).await.is_ok());
    assert!(insert_post(&tx, &w, Some(&defaults), &json!({}))
        .await
        .is_ok());
    assert_eq!(
        insert_event(
            &tx,
            &w,
            Some(&serde_json::to_value(ElectionEventStatus::default()).unwrap()),
            Some(&json!({"locked_down": "not-locked-down"}))
        )
        .await,
        Ok(1)
    );
    trusted_write(&tx).await.unwrap();
    assert!(insert_post(&tx, &w, Some(&open), &json!({})).await.is_ok());
    assert_eq!(
        insert_event(
            &tx,
            &w,
            Some(&open),
            Some(&json!({"locked_down": "locked-down"}))
        )
        .await,
        Ok(1)
    );
}

/// The grace period lets votes in after a close: it may change until voting
/// starts, and then only through the server.
#[tokio::test]
async fn the_grace_period_changes_only_before_voting_starts() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let grace = |policy: &str, secs: u64| json!({"grace_period_policy": policy, "grace_period_secs": secs, "sort_order": 1});
    // Voting is open at the world's Post.
    for presentation in [
        grace("grace-period-without-alert", 3600),
        grace("no-grace-period", 60),
        json!({"grace_period_secs": 60}),
    ] {
        let tx = client.transaction().await.unwrap();
        assert_eq!(
            set_post_presentation(&tx, w.post, &presentation).await,
            Err("42501".to_owned()),
            "{presentation}"
        );
    }
    let tx = client.transaction().await.unwrap();
    // Other keys stay writable, with the grace period as stored.
    assert_eq!(
        set_post_presentation(&tx, w.post, &grace("grace-period-without-alert", 60)).await,
        Ok(1)
    );
    trusted_write(&tx).await.unwrap();
    assert_eq!(
        set_post_presentation(&tx, w.post, &grace("no-grace-period", 0)).await,
        Ok(1)
    );
    drop(tx);

    // Before voting starts it is configuration like any other; no settings
    // and the explicit defaults are the same.
    let tx = client.transaction().await.unwrap();
    let post = insert_post(&tx, &w, None, &json!({})).await.unwrap();
    for presentation in [
        json!({"grace_period_policy": "no-grace-period", "grace_period_secs": 0}),
        grace("grace-period-without-alert", 3600),
        json!({}),
    ] {
        assert_eq!(
            set_post_presentation(&tx, post, &presentation).await,
            Ok(1),
            "{presentation}"
        );
    }
}

/// The event-wide voting status action (manual and scheduled) writes every
/// election's status and the event's as the server's own write. Its
/// electoral log entries, which come first, need the board, so this drives
/// the write step it ends with.
#[tokio::test]
async fn the_event_wide_voting_status_action_writes_under_the_guard() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let elections = windmill::postgres::election::get_elections(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
    )
    .await
    .unwrap();
    let mut closed: ElectionStatus = serde_json::from_str(POST_STATUS).unwrap();
    closed.voting_status = VotingStatus::CLOSED;
    let elections_status: HashMap<String, ElectionStatus> = elections
        .iter()
        .map(|election| (election.id.clone(), closed.clone()))
        .collect();
    let mut event_status: ElectionEventStatus = serde_json::from_str(EVENT_STATUS).unwrap();
    event_status.voting_status = VotingStatus::CLOSED;
    save_event_voting_status(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &elections,
        &elections_status,
        &event_status,
    )
    .await
    .unwrap();
    let statuses: (Value, Value) = {
        let row = tx
            .query_one(
                "SELECT e.status, ev.status FROM sequent_backend.election e
                 JOIN sequent_backend.election_event ev ON ev.id = e.election_event_id
                 WHERE e.id = $1",
                &[&w.post],
            )
            .await
            .unwrap();
        (row.get(0), row.get(1))
    };
    assert_eq!(statuses.0["voting_status"], "CLOSED");
    assert_eq!(statuses.1["voting_status"], "CLOSED");
    tx.commit().await.unwrap();

    // The same writes outside the action are refused.
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        set_event_status(&tx, &w, &json!({"voting_status": "OPEN"})).await,
        Err("42501".to_owned())
    );
}

/// Recorded channel dates remain proof that voting started even after a
/// trusted reset of its current status.
#[tokio::test]
async fn a_reset_channel_does_not_make_the_grace_period_editable() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    set_post_status(
        &tx,
        &w,
        &post_status_with(json!({"voting_status": "NOT_STARTED"})),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        set_post_presentation(&tx, w.post, &json!({"grace_period_secs": 3600})).await,
        Err("42501".to_owned())
    );
}

#[tokio::test]
async fn an_incomplete_event_wide_status_map_never_marks_the_transaction() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let elections = windmill::postgres::election::get_elections(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
    )
    .await
    .unwrap();
    let error = save_event_voting_status(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &elections,
        &HashMap::new(),
        &ElectionEventStatus::default(),
    )
    .await
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("Missing voting status for election"));
    assert_eq!(
        set_event_status(&tx, &w, &json!({"voting_status": "CLOSED"})).await,
        Err("42501".to_owned())
    );
}

#[tokio::test]
async fn initialization_evidence_cannot_be_inserted_or_changed_directly() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert_eq!(tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, initialization_report_generated) VALUES ($1, $2, $3, true)",
        &[&Uuid::new_v4(), &w.tenant, &w.event],
    ).await.map_err(sqlstate), Err("42501".to_owned()));
    drop(tx);
    let tx = client.transaction().await.unwrap();
    assert_eq!(tx.execute("UPDATE sequent_backend.election SET initialization_report_generated = true WHERE id = $1", &[&w.post]).await.map_err(sqlstate), Err("42501".to_owned()));
    drop(tx);
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    windmill::postgres::election::set_election_initialization_report_generated(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &w.post.to_string(),
        &true,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert_eq!(tx.execute("UPDATE sequent_backend.election SET initialization_report_generated = false WHERE id = $1", &[&w.post]).await.map_err(sqlstate), Err("42501".to_owned()));
}

#[tokio::test]
async fn scheduler_execution_evidence_cannot_be_spoofed_or_removed() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let fake = json!({"fired_outcome":{"at":"2028-01-01T00:00:00Z","posts":[]}});
    assert_eq!(tx.execute("INSERT INTO sequent_backend.scheduled_event (tenant_id,election_event_id,annotations) VALUES ($1,$2,$3)", &[&w.tenant,&w.event,&fake]).await.map_err(sqlstate),Err("42501".into()));
    drop(tx);
    let tx = client.transaction().await.unwrap();
    let id: Uuid = tx.query_one("INSERT INTO sequent_backend.scheduled_event (tenant_id,election_event_id,annotations) VALUES ($1,$2,'{}') RETURNING id", &[&w.tenant,&w.event]).await.unwrap().get(0);
    tx.commit().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE sequent_backend.scheduled_event SET annotations=$2 WHERE id=$1",
            &[&id, &fake]
        )
        .await
        .map_err(sqlstate),
        Err("42501".into())
    );
    drop(tx);
    let tx = client.transaction().await.unwrap();
    windmill::services::scheduled_outcome::record_fired_outcome(
        &tx,
        w.tenant,
        w.event,
        &id.to_string(),
        &json!({"at":"2028-01-01T00:00:00Z","posts":[{"election_id":w.post,"outcome":"runs"}]}),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert_eq!(tx.execute(r#"UPDATE sequent_backend.scheduled_event SET annotations=annotations || '{"operator_note":"allowed"}'::jsonb WHERE id=$1"#, &[&id]).await.map_err(sqlstate),Ok(1));
    tx.commit().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert_eq!(tx.execute("UPDATE sequent_backend.scheduled_event SET annotations=annotations-'fired_outcome' WHERE id=$1", &[&id]).await.map_err(sqlstate),Err("42501".into()));
}

#[tokio::test]
async fn prediction_cache_integrity_keeps_scheduler_attribution_and_audit_baselines() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let fake = json!({"predicted_outcome":{"fingerprint":"forged","edited_by":"somebody-else","outcomes":[]}});
    let tx = client.transaction().await.unwrap();
    assert_eq!(tx.execute("INSERT INTO sequent_backend.scheduled_event (tenant_id,election_event_id,annotations) VALUES ($1,$2,$3)", &[&w.tenant,&w.event,&fake]).await.map_err(sqlstate),Err("42501".into()));
    drop(tx);
    let tx = client.transaction().await.unwrap();
    let id:Uuid=tx.query_one("INSERT INTO sequent_backend.scheduled_event (tenant_id,election_event_id,event_processor,cron_config,event_payload) VALUES ($1,$2,'START_VOTING_PERIOD',$3,$4) RETURNING id", &[&w.tenant,&w.event,&json!({"scheduled_date":"2099-01-01T00:00:00Z"}), &json!({"election_id":w.post})]).await.unwrap().get(0);
    tx.commit().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE sequent_backend.scheduled_event SET annotations=$2 WHERE id=$1",
            &[&id, &fake]
        )
        .await
        .map_err(sqlstate),
        Err("42501".into())
    );
    drop(tx);
    let tx = client.transaction().await.unwrap();
    windmill::services::scheduled_outcome::recompute_predictions(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        &windmill::services::signing::log::Actor {
            user_id: "operator".into(),
            username: "operator".into(),
        },
    )
    .await
    .unwrap();
    let cache:Option<Value>=tx.query_one("SELECT annotations->'predicted_outcome' FROM sequent_backend.scheduled_event WHERE id=$1", &[&id]).await.unwrap().get(0);
    assert!(cache.is_some());
    tx.commit().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert_eq!(tx.execute("UPDATE sequent_backend.scheduled_event SET annotations=annotations-'predicted_outcome' WHERE id=$1", &[&id]).await.map_err(sqlstate),Err("42501".into()));
}

#[tokio::test]
async fn a_started_post_cannot_change_identity_or_event_without_server_authority() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let target_event = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&target_event, &w.tenant],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    for (column, replacement) in [("election_event_id", target_event), ("id", Uuid::new_v4())] {
        let tx = client.transaction().await.unwrap();
        let error = tx
            .execute(
                &format!("UPDATE sequent_backend.election SET {column} = $2 WHERE id = $1"),
                &[&w.post, &replacement],
            )
            .await
            .expect_err("started voting cannot be detached from its authorized Post and event");
        let error = error.as_db_error().expect("database refusal");
        assert_eq!(error.code().code(), "42501");
        assert_eq!(
            error.message(),
            "A Post with voting or initialization evidence can only change identity or event through a trusted server transaction"
        );
    }

    let tx = client.transaction().await.unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE sequent_backend.election SET id = id, election_event_id = election_event_id,
             annotations = '{\"operator_note\":\"allowed\"}' WHERE id = $1",
            &[&w.post],
        )
        .await
        .unwrap(),
        1
    );
    tx.commit().await.unwrap();
    let tx = client.transaction().await.unwrap();
    trusted_write(&tx).await.unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE sequent_backend.election SET election_event_id = $2 WHERE id = $1",
            &[&w.post, &target_event],
        )
        .await
        .unwrap(),
        1
    );
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn an_initialized_post_cannot_move_but_a_new_post_can() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let target_event = Uuid::new_v4();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&target_event, &w.tenant],
    )
    .await
    .unwrap();
    let fresh_post = insert_post(&tx, &w, None, &json!({})).await.unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE sequent_backend.election SET election_event_id = $2 WHERE id = $1",
            &[&fresh_post, &target_event],
        )
        .await
        .unwrap(),
        1
    );
    trusted_write(&tx).await.unwrap();
    tx.execute(
        "UPDATE sequent_backend.election SET initialization_report_generated = true WHERE id = $1",
        &[&fresh_post],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let error = tx
        .execute(
            "UPDATE sequent_backend.election SET election_event_id = $2 WHERE id = $1",
            &[&fresh_post, &w.event],
        )
        .await
        .expect_err("initialization evidence is scoped to the original event");
    let error = error.as_db_error().expect("database refusal");
    assert_eq!(error.code().code(), "42501");
    assert_eq!(
        error.message(),
        "A Post with voting or initialization evidence can only change identity or event through a trusted server transaction"
    );
}

#[tokio::test]
async fn a_not_started_post_cannot_detach_its_signed_voting_boundary() {
    let w = world().await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let post = insert_post(&tx, &w, None, &json!({})).await.unwrap();
    // Seed the private projection at its public database seam. Voting has
    // not started yet, but this Post already has a retained signed deadline.
    tx.execute(
        "INSERT INTO sequent_backend.signed_voting_boundary
         (tenant_id, election_event_id, election_id, approval_request_id,
          scheduled_event_id, event_processor, channels, scheduled_date, scheduled_at)
         VALUES ($1, $2, $3, $4, 'signed-close', 'END_VOTING_PERIOD', '[\"ONLINE\"]',
                 '2099-01-01T00:00:00Z', '2099-01-01T00:00:00Z')",
        &[&w.tenant, &w.event, &post, &Uuid::new_v4()],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let error = tx
        .execute(
            "UPDATE sequent_backend.election SET id = $2 WHERE id = $1",
            &[&post, &Uuid::new_v4()],
        )
        .await
        .expect_err("a new identity cannot escape a retained signed deadline");
    let error = error.as_db_error().expect("database refusal");
    assert_eq!(error.code().code(), "42501");
    assert_eq!(
        error.message(),
        "A Post with voting or initialization evidence can only change identity or event through a trusted server transaction"
    );
}

#[tokio::test]
async fn published_initialization_policies_without_schedules_keep_the_post_identity() {
    for event_wide in [false, true] {
        let w = world().await;
        let mut client = w.pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let post = insert_post(&tx, &w, None, &json!({})).await.unwrap();
        let target = if event_wide { None } else { Some(post) };
        // Publication authority exists even before voting/initialization;
        // no schedule means there is no signed-boundary projection row.
        tx.execute(
            "INSERT INTO sequent_backend.lifecycle_snapshot
             (tenant_id, election_event_id, ballot_publication_id, election_id, snapshot)
             VALUES ($1, $2, $3, $4, $5)",
            &[
                &w.tenant,
                &w.event,
                &Uuid::new_v4(),
                &target,
                &json!({"initialization_report_policies": {post.to_string(): "required"}, "schedule": []}),
            ],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let error = tx
            .execute(
                "UPDATE sequent_backend.election SET id = $2 WHERE id = $1",
                &[&post, &Uuid::new_v4()],
            )
            .await
            .expect_err("published report requirements cannot be dropped by changing identity");
        let error = error.as_db_error().expect("database refusal");
        assert_eq!(error.code().code(), "42501");
        assert_eq!(
            error.message(),
            "A Post with voting or initialization evidence can only change identity or event through a trusted server transaction"
        );
        drop(tx);
        let tx = client.transaction().await.unwrap();
        assert_eq!(
            tx.execute(
                "UPDATE sequent_backend.election SET id = id WHERE id = $1",
                &[&post],
            )
            .await
            .unwrap(),
            1
        );
        trusted_write(&tx).await.unwrap();
        assert_eq!(
            tx.execute(
                "UPDATE sequent_backend.election SET id = $2 WHERE id = $1",
                &[&post, &Uuid::new_v4()],
            )
            .await
            .unwrap(),
            1
        );
        tx.commit().await.unwrap();
    }
}
