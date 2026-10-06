// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The windmill futures these tests await are deep.
#![recursion_limit = "256"]

//! Scheduling against the migrated schema (VOTE-LIFECYCLE §5): saving a
//! wall time and zone, the event-wide close in the voting window
//! projection, event-wide ALLOW_* events, lifecycle windows and the tz
//! database recompute. Every test rolls its transaction back.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::types::scheduled_event::{generate_manage_date_task_name, EventProcessors};
use serde_json::{json, Value};
use uuid::Uuid;
use windmill::services::election_dates::{save_schedule, InvalidSchedule, ScheduleInput};
use windmill::services::schedule_recompute;
use windmill::services::signing::log::Actor;
use windmill::tasks::manage_election_allow_tally::manage_election_allow_tally_wrapped;
use windmill::tasks::manage_election_event_date::manage_election_event_date_wrapped;
use windmill::tasks::manage_election_lifecycle_window::manage_election_lifecycle_window_wrapped;

#[derive(Clone, Copy)]
struct Event {
    tenant: Uuid,
    event: Uuid,
}

impl Event {
    fn t(&self) -> String {
        self.tenant.to_string()
    }
    fn e(&self) -> String {
        self.event.to_string()
    }
}

/// An event whose presentation configures `zones` with `primary`.
async fn event(tx: &Transaction<'_>, primary: &str, zones: &[&str]) -> Event {
    let (tenant, event) = (Uuid::new_v4(), Uuid::new_v4());
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol, presentation)
         VALUES ($1, $2, 'RSA256', $3)",
        &[
            &event,
            &tenant,
            &json!({"timezones": {"configured": zones, "primary": primary, "logs": "election"}}),
        ],
    )
    .await
    .unwrap();
    Event { tenant, event }
}

async fn election(tx: &Transaction<'_>, e: Event, zone: Option<&str>) -> Uuid {
    let id = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation)
         VALUES ($1, $2, $3, $4)",
        &[&id, &e.tenant, &e.event, &json!({"timezone": zone})],
    )
    .await
    .unwrap();
    id
}

async fn schedule(
    tx: &Transaction<'_>,
    e: Event,
    election: Option<Uuid>,
    processor: EventProcessors,
    payload: Value,
    cron: Value,
) -> Uuid {
    let election_text = election.map(|id| id.to_string());
    tx.query_one(
        "INSERT INTO sequent_backend.scheduled_event
             (tenant_id, election_event_id, event_processor, task_id, event_payload, cron_config)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        &[
            &e.tenant,
            &e.event,
            &processor.to_string(),
            &generate_manage_date_task_name(&e.t(), &e.e(), election_text.as_deref(), &processor),
            &payload,
            &cron,
        ],
    )
    .await
    .unwrap()
    .get(0)
}

/// (start_date, end_date) of the election's window row, if any.
async fn window(
    tx: &Transaction<'_>,
    e: Event,
    election: Uuid,
) -> Option<(Option<String>, Option<String>)> {
    tx.query_opt(
        "SELECT start_date, end_date FROM sequent_backend.election_voting_window
         WHERE tenant_id = $1 AND election_event_id = $2 AND election_id = $3",
        &[&e.tenant, &e.event, &election],
    )
    .await
    .unwrap()
    .map(|row| (row.get(0), row.get(1)))
}

async fn row(tx: &Transaction<'_>, table: &str, id: Uuid) -> Value {
    tx.query_one(
        &format!("SELECT to_jsonb(r) FROM sequent_backend.{table} r WHERE r.id = $1"),
        &[&id],
    )
    .await
    .unwrap()
    .get(0)
}

fn admin() -> Actor {
    Actor {
        user_id: "admin".into(),
        username: "admin".into(),
    }
}

fn some(text: &str) -> Option<String> {
    Some(text.to_owned())
}

#[tokio::test]
async fn the_event_wide_close_closes_every_post_without_its_own_in_the_voting_window() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx, "Asia/Manila", &["Asia/Manila", "Asia/Dubai"]).await;
    let (own, opens, nothing) = (
        election(&tx, e, None).await,
        election(&tx, e, Some("Asia/Dubai")).await,
        election(&tx, e, None).await,
    );
    for post in [own, opens] {
        schedule(
            &tx,
            e,
            Some(post),
            EventProcessors::START_VOTING_PERIOD,
            json!({"election_id": post.to_string()}),
            json!({"scheduled_date": "2028-04-08T20:00:00Z"}),
        )
        .await;
    }
    schedule(
        &tx,
        e,
        Some(own),
        EventProcessors::END_VOTING_PERIOD,
        json!({"election_id": own.to_string()}),
        json!({"scheduled_date": "2028-04-20T00:00:00Z"}),
    )
    .await;
    let common = schedule(
        &tx,
        e,
        None,
        EventProcessors::END_VOTING_PERIOD,
        json!({"election_id": null}),
        json!({"scheduled_date": "2028-05-08T11:00:00Z", "local": "2028-05-08T19:00", "timezone": "Asia/Manila"}),
    )
    .await;

    let start = some("2028-04-08T20:00:00Z");
    assert_eq!(
        window(&tx, e, own).await,
        Some((start.clone(), some("2028-04-20T00:00:00Z")))
    );
    assert_eq!(
        window(&tx, e, opens).await,
        Some((start.clone(), some("2028-05-08T11:00:00Z")))
    );
    assert_eq!(
        window(&tx, e, nothing).await,
        Some((None, some("2028-05-08T11:00:00Z")))
    );
    // A Post created after the close gets it too.
    let late = election(&tx, e, None).await;
    assert_eq!(
        window(&tx, e, late).await,
        Some((None, some("2028-05-08T11:00:00Z")))
    );

    // Moving the close moves every Post without its own.
    tx.execute(
        "UPDATE sequent_backend.scheduled_event
         SET cron_config = '{\"scheduled_date\": \"2028-05-09T11:00:00Z\"}' WHERE id = $1",
        &[&common],
    )
    .await
    .unwrap();
    assert_eq!(
        window(&tx, e, opens).await,
        Some((start.clone(), some("2028-05-09T11:00:00Z")))
    );
    assert_eq!(
        window(&tx, e, own).await,
        Some((start.clone(), some("2028-04-20T00:00:00Z")))
    );

    // A kiosk-only close doesn't end online voting; archiving it removes it.
    tx.execute(
        "UPDATE sequent_backend.scheduled_event
         SET event_payload = '{\"election_id\": null, \"voting_channels\": [\"KIOSK\"]}' WHERE id = $1",
        &[&common],
    )
    .await
    .unwrap();
    assert_eq!(window(&tx, e, opens).await, Some((start.clone(), None)));
    assert_eq!(window(&tx, e, nothing).await, None);
    tx.execute(
        "UPDATE sequent_backend.scheduled_event SET event_payload = '{\"election_id\": null}', archived_at = now() WHERE id = $1",
        &[&common],
    )
    .await
    .unwrap();
    assert_eq!(window(&tx, e, opens).await, Some((start.clone(), None)));
    assert_eq!(window(&tx, e, late).await, None);

    // A close that already ran with a date the scheduler can't read (no
    // offset) is skipped, not an error: writes go on.
    let stale = schedule(
        &tx,
        e,
        None,
        EventProcessors::END_VOTING_PERIOD,
        json!({"election_id": null}),
        json!({"scheduled_date": "2028-05-08T19:00:00"}),
    )
    .await;
    tx.execute(
        "UPDATE sequent_backend.scheduled_event SET stopped_at = now() WHERE id = $1",
        &[&stale],
    )
    .await
    .unwrap();
    let newest = election(&tx, e, None).await;
    assert_eq!(window(&tx, e, newest).await, None);
    assert_eq!(window(&tx, e, opens).await, Some((start, None)));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn saving_a_wall_time_stores_the_instant_in_the_posts_zone_and_answers_warnings() {
    // The two configurations: expectations follow each one's zones.
    for (primary, post_zone, expected_start) in [
        ("Asia/Manila", "Asia/Dubai", "2028-04-08T20:00:00Z"),
        ("Europe/Madrid", "Atlantic/Canary", "2028-04-08T23:00:00Z"),
    ] {
        let mut client = schema::pool().await.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let e = event(&tx, primary, &[primary, post_zone]).await;
        let post = election(&tx, e, Some(post_zone)).await;
        let post_text = post.to_string();

        let saved = save_schedule(
            &tx,
            &e.t(),
            &e.e(),
            Some(&post_text),
            &EventProcessors::START_VOTING_PERIOD,
            &ScheduleInput {
                local_date_time: some("2028-04-09T00:00"),
                ..Default::default()
            },
            Some(vec![VotingStatusChannel::ONLINE]),
            &admin(),
        )
        .await
        .unwrap();
        assert_eq!(
            saved.scheduled_date.as_deref(),
            Some(expected_start),
            "{primary}"
        );
        let stored: Value = tx
            .query_one(
                "SELECT cron_config FROM sequent_backend.scheduled_event WHERE task_id = $1",
                &[&generate_manage_date_task_name(
                    &e.t(),
                    &e.e(),
                    Some(&post_text),
                    &EventProcessors::START_VOTING_PERIOD,
                )],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(
            stored,
            json!({"cron": null, "scheduled_date": expected_start, "local": "2028-04-09T00:00", "timezone": post_zone})
        );

        // The event-wide close is in the primary; a 10-day window warns.
        let saved = save_schedule(
            &tx,
            &e.t(),
            &e.e(),
            None,
            &EventProcessors::END_VOTING_PERIOD,
            &ScheduleInput {
                local_date_time: some("2028-04-19T00:00"),
                ..Default::default()
            },
            None,
            &admin(),
        )
        .await
        .unwrap();
        assert_eq!(saved.warnings.len(), 1, "{:?}", saved.warnings);
        assert_eq!(saved.warnings[0].code, "voting-window-days");
        assert_eq!(
            saved.warnings[0].election_id.as_deref(),
            Some(post_text.as_str())
        );
        assert_eq!(saved.warnings[0].params["time_zone"], json!(post_zone));
        assert_eq!(
            window(&tx, e, post).await.and_then(|(_, end)| end),
            saved.scheduled_date
        );

        // Refusals: an instant without an offset, an event-wide window.
        for (election, processor, input) in [
            (
                Some(post_text.as_str()),
                EventProcessors::END_VOTING_PERIOD,
                ScheduleInput {
                    scheduled_date: some("2028-05-08T19:00:00"),
                    ..Default::default()
                },
            ),
            (
                None,
                EventProcessors::START_TEST_VOTING,
                ScheduleInput {
                    local_date_time: some("2028-03-01T09:00"),
                    ..Default::default()
                },
            ),
        ] {
            let err = save_schedule(
                &tx,
                &e.t(),
                &e.e(),
                election,
                &processor,
                &input,
                None,
                &admin(),
            )
            .await
            .unwrap_err();
            assert!(err.downcast_ref::<InvalidSchedule>().is_some(), "{err:?}");
        }
        let rows: i64 = tx
            .query_one(
                "SELECT count(*) FROM sequent_backend.scheduled_event WHERE election_event_id = $1",
                &[&e.event],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(rows, 2);
        tx.rollback().await.unwrap();
    }
}

#[tokio::test]
async fn event_wide_allow_events_apply_to_every_post_and_are_marked_done() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx, "UTC", &["UTC"]).await;
    let posts = [election(&tx, e, None).await, election(&tx, e, None).await];
    let date = json!({"scheduled_date": "2026-01-01T00:00:00Z"});
    let init = schedule(
        &tx,
        e,
        None,
        EventProcessors::ALLOW_INIT_REPORT,
        json!({"election_id": null, "allow_init": false}),
        date.clone(),
    )
    .await;
    let tally = schedule(
        &tx,
        e,
        None,
        EventProcessors::ALLOW_TALLY,
        json!({"election_id": null}),
        date.clone(),
    )
    .await;
    let end = schedule(
        &tx,
        e,
        None,
        EventProcessors::ALLOW_VOTING_PERIOD_END,
        json!({"election_id": null, "allow_voting_period_end": false}),
        date,
    )
    .await;
    for id in [init, tally, end] {
        manage_election_event_date_wrapped(&tx, e.t(), e.e(), id.to_string())
            .await
            .unwrap();
        assert_ne!(
            row(&tx, "scheduled_event", id).await["stopped_at"],
            Value::Null
        );
    }
    for post in posts {
        let election = row(&tx, "election", post).await;
        assert_eq!(election["status"]["init_report"], json!("disallowed"));
        assert_eq!(election["status"]["allow_tally"], json!("allowed"));
        assert_eq!(
            election["presentation"]["voting_period_end"],
            json!("disallowed")
        );
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_posts_own_row_wins_over_the_event_wide_one_at_fire_time() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx, "UTC", &["UTC"]).await;
    let (own, other) = (election(&tx, e, None).await, election(&tx, e, None).await);
    // This fixture seeds a preexisting state rather than exercising a raw write.
    windmill::postgres::trusted_write::trusted_write(&tx)
        .await
        .unwrap();
    tx.execute(
        "UPDATE sequent_backend.election
         SET status = '{\"voting_status\": \"OPEN\", \"allow_tally\": \"disallowed\"}',
             voting_channels = '{\"online\": true}'
         WHERE id = $1",
        &[&own],
    )
    .await
    .unwrap();
    // Not open: the common close has nothing to do there.
    tx.execute(
        "UPDATE sequent_backend.election SET status = '{\"allow_tally\": \"disallowed\"}' WHERE id = $1",
        &[&other],
    )
    .await
    .unwrap();
    let past = json!({"scheduled_date": "2026-01-01T00:00:00Z"});
    // X closes on 10 May with its own row; the common close is earlier.
    schedule(
        &tx,
        e,
        Some(own),
        EventProcessors::END_VOTING_PERIOD,
        json!({"election_id": own.to_string()}),
        json!({"scheduled_date": "2099-05-10T00:00:00Z"}),
    )
    .await;
    let common = schedule(
        &tx,
        e,
        None,
        EventProcessors::END_VOTING_PERIOD,
        json!({"election_id": null}),
        past.clone(),
    )
    .await;
    // X's own ALLOW_TALLY is still to come; the event-wide one has run.
    schedule(
        &tx,
        e,
        Some(own),
        EventProcessors::ALLOW_TALLY,
        json!({"election_id": own.to_string()}),
        json!({"scheduled_date": "2099-05-11T00:00:00Z"}),
    )
    .await;
    let tally = schedule(
        &tx,
        e,
        None,
        EventProcessors::ALLOW_TALLY,
        json!({"election_id": null}),
        past,
    )
    .await;
    for id in [common, tally] {
        manage_election_event_date_wrapped(&tx, e.t(), e.e(), id.to_string())
            .await
            .unwrap();
        assert_ne!(
            row(&tx, "scheduled_event", id).await["stopped_at"],
            Value::Null
        );
    }
    let x = row(&tx, "election", own).await;
    assert_eq!(x["status"]["voting_status"], json!("OPEN"));
    assert_eq!(x["status"]["allow_tally"], json!("disallowed"));
    assert_eq!(
        row(&tx, "election", other).await["status"]["allow_tally"],
        json!("allowed")
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_task_queued_before_its_row_moved_later_waits_for_the_new_time() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx, "UTC", &["UTC"]).await;
    let post = election(&tx, e, None).await;
    let moved = schedule(
        &tx,
        e,
        None,
        EventProcessors::ALLOW_TALLY,
        json!({"election_id": null}),
        json!({"scheduled_date": "2099-01-01T00:00:00Z"}),
    )
    .await;
    manage_election_event_date_wrapped(&tx, e.t(), e.e(), moved.to_string())
        .await
        .unwrap();
    assert_eq!(
        row(&tx, "scheduled_event", moved).await["stopped_at"],
        Value::Null
    );
    assert_ne!(
        row(&tx, "election", post).await["status"]["allow_tally"],
        json!("allowed")
    );
    let window = schedule(
        &tx,
        e,
        Some(post),
        EventProcessors::START_TEST_VOTING,
        json!({"election_id": post.to_string()}),
        json!({"scheduled_date": "2099-01-01T00:00:00Z"}),
    )
    .await;
    manage_election_lifecycle_window_wrapped(
        &tx,
        &e.t(),
        &e.e(),
        &window.to_string(),
        Some(&post.to_string()),
    )
    .await
    .unwrap();
    assert_eq!(
        row(&tx, "scheduled_event", window).await["stopped_at"],
        Value::Null
    );
    tx.rollback().await.unwrap();
}

async fn outbox_kinds(tx: &Transaction<'_>, e: Event) -> Vec<(String, String)> {
    tx.query(
        "SELECT statement_kind, event_type FROM sequent_backend.signing_log_outbox
         WHERE election_event_id = $1 ORDER BY id",
        &[&e.event],
    )
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.get(0), row.get(1)))
    .collect()
}

#[tokio::test]
async fn a_lifecycle_window_opens_for_its_post_is_logged_and_marked_done() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx, "UTC", &["UTC"]).await;
    let post = election(&tx, e, None).await;
    let post_text = post.to_string();
    let id = schedule(
        &tx,
        e,
        Some(post),
        EventProcessors::START_FINAL_TESTING,
        json!({"election_id": post_text}),
        json!({"scheduled_date": "2026-01-01T00:00:00Z"}),
    )
    .await;
    tx.execute(
        "UPDATE sequent_backend.scheduled_event
         SET annotations = '{\"schedule_recompute\": {}, \"note\": 1}' WHERE id = $1",
        &[&id],
    )
    .await
    .unwrap();
    manage_election_lifecycle_window_wrapped(
        &tx,
        &e.t(),
        &e.e(),
        &id.to_string(),
        Some(&post_text),
    )
    .await
    .unwrap();
    let election = row(&tx, "election", post).await;
    assert_eq!(
        election["status"]["lifecycle_windows"],
        json!({"FINAL_TESTING": "OPEN"})
    );
    let fired = row(&tx, "scheduled_event", id).await;
    assert_ne!(fired["stopped_at"], Value::Null);
    assert_eq!(fired["annotations"], json!({"note": 1}));
    assert_eq!(
        outbox_kinds(&tx, e).await,
        [
            ("LifecycleWindowChanged".to_owned(), "USER".to_owned()),
            ("LifecycleWindowChanged".to_owned(), "SYSTEM".to_owned())
        ]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_recomputed_instant_is_only_recorded_until_it_is_applied_and_logged() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx, "Asia/Dubai", &["Asia/Dubai"]).await;
    let post = election(&tx, e, None).await;
    // Stored as if Dubai were +03:00; the tz database says +04:00.
    let id = schedule(
        &tx,
        e,
        Some(post),
        EventProcessors::START_VOTING_PERIOD,
        json!({"election_id": post.to_string()}),
        json!({"scheduled_date": "2099-04-08T21:00:00Z", "local": "2099-04-09T00:00", "timezone": "Asia/Dubai"}),
    )
    .await;
    let now: DateTime<Utc> = Utc::now();
    assert!(schedule_recompute::check_all(&tx, now).await.unwrap() >= 1);
    let checked = row(&tx, "scheduled_event", id).await;
    assert_eq!(
        checked["cron_config"]["scheduled_date"],
        json!("2099-04-08T21:00:00Z")
    );
    assert_eq!(
        checked["annotations"]["schedule_recompute"]["scheduled_date"],
        json!("2099-04-08T20:00:00Z")
    );
    assert!(outbox_kinds(&tx, e).await.is_empty());

    let updated = schedule_recompute::apply(&tx, &e.t(), &e.e(), &admin(), now)
        .await
        .unwrap();
    assert_eq!(updated, 1);
    let applied = row(&tx, "scheduled_event", id).await;
    assert_eq!(
        applied["cron_config"]["scheduled_date"],
        json!("2099-04-08T20:00:00Z")
    );
    assert_eq!(applied["annotations"].get("schedule_recompute"), None);
    assert_eq!(
        window(&tx, e, post)
            .await
            .and_then(|(start, _)| start)
            .as_deref(),
        Some("2099-04-08T20:00:00Z")
    );
    assert_eq!(
        outbox_kinds(&tx, e).await,
        [
            ("ScheduleRecomputeApplied".to_owned(), "USER".to_owned()),
            ("ScheduleRecomputeApplied".to_owned(), "SYSTEM".to_owned()),
            ("ScheduledOutcomeChanged".to_owned(), "USER".to_owned()),
            ("ScheduledOutcomeChanged".to_owned(), "SYSTEM".to_owned())
        ]
    );
    assert_eq!(
        schedule_recompute::apply(&tx, &e.t(), &e.e(), &admin(), now)
            .await
            .unwrap(),
        0
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn an_election_task_queued_before_its_row_moved_later_waits_and_then_runs() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx, "UTC", &["UTC"]).await;
    let post = election(&tx, e, None).await;
    tx.execute(
        "UPDATE sequent_backend.election SET status = '{\"allow_tally\": \"disallowed\"}' WHERE id = $1",
        &[&post],
    )
    .await
    .unwrap();
    let id = schedule(
        &tx,
        e,
        Some(post),
        EventProcessors::ALLOW_TALLY,
        json!({"election_id": post.to_string()}),
        json!({"scheduled_date": "2099-01-01T00:00:00Z"}),
    )
    .await;
    let run =
        || manage_election_allow_tally_wrapped(&tx, e.t(), e.e(), id.to_string(), post.to_string());
    run().await.unwrap();
    assert_eq!(
        row(&tx, "scheduled_event", id).await["stopped_at"],
        Value::Null
    );
    assert_eq!(
        row(&tx, "election", post).await["status"]["allow_tally"],
        json!("disallowed")
    );
    // At its time it runs and is marked done.
    tx.execute(
        "UPDATE sequent_backend.scheduled_event
         SET cron_config = '{\"scheduled_date\": \"2026-01-01T00:00:00Z\"}' WHERE id = $1",
        &[&id],
    )
    .await
    .unwrap();
    run().await.unwrap();
    assert_ne!(
        row(&tx, "scheduled_event", id).await["stopped_at"],
        Value::Null
    );
    assert_eq!(
        row(&tx, "election", post).await["status"]["allow_tally"],
        json!("allowed")
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn an_archived_event_does_not_record_a_scheduled_transition_as_executed() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx, "Europe/Madrid", &["Europe/Madrid"]).await;
    election(&tx, e, None).await;
    let id = schedule(
        &tx,
        e,
        None,
        EventProcessors::END_VOTING_PERIOD,
        json!({"election_id": null}),
        json!({"scheduled_date": "2020-04-09T00:00:00Z"}),
    )
    .await;
    tx.execute(
        "UPDATE sequent_backend.election_event SET is_archived = true WHERE id = $1",
        &[&e.event],
    )
    .await
    .unwrap();
    manage_election_event_date_wrapped(&tx, e.t(), e.e(), id.to_string())
        .await
        .unwrap();
    let stored = row(&tx, "scheduled_event", id).await;
    assert!(stored["annotations"]["fired_outcome"].is_null());
    assert!(stored["stopped_at"].is_null());
    assert!(outbox_kinds(&tx, e).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_queued_event_wide_task_cannot_execute_a_row_retargeted_to_a_post() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx, "Europe/Madrid", &["Europe/Madrid"]).await;
    let post = election(&tx, e, None).await;
    let id = schedule(
        &tx,
        e,
        Some(post),
        EventProcessors::END_VOTING_PERIOD,
        json!({"election_id": post.to_string()}),
        json!({"scheduled_date": "2020-04-09T00:00:00Z"}),
    )
    .await;
    let error = manage_election_event_date_wrapped(&tx, e.t(), e.e(), id.to_string())
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("no longer event-wide"),
        "{error:#}"
    );
    let stored = row(&tx, "scheduled_event", id).await;
    assert!(stored["annotations"]["fired_outcome"].is_null());
    assert!(stored["stopped_at"].is_null());
    assert!(outbox_kinds(&tx, e).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn stopped_schedule_rejects_nonfuture_edits_and_rearms_only_for_the_future() {
    use windmill::postgres::scheduled_event::update_scheduled_event;
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx, "UTC", &["UTC"]).await;
    let id = schedule(
        &tx,
        e,
        None,
        EventProcessors::ALLOW_TALLY,
        json!({"election_id": null}),
        json!({"scheduled_date": "2000-01-01T00:00:00Z"}),
    )
    .await;
    tx.execute(
        "UPDATE sequent_backend.scheduled_event SET stopped_at = now() WHERE id = $1",
        &[&id],
    )
    .await
    .unwrap();
    let original = row(&tx, "scheduled_event", id).await;
    let now: chrono::DateTime<chrono::Utc> =
        tx.query_one("SELECT NOW()", &[]).await.unwrap().get(0);
    let now = now.to_rfc3339();
    for date in [
        Some("2001-01-01T00:00:00Z"),
        Some(now.as_str()),
        None,
        Some("invalid"),
    ] {
        let cron = serde_json::from_value(json!({"scheduled_date": date})).unwrap();
        assert_eq!(
            update_scheduled_event(&tx, &e.t(), &id.to_string(), cron, None)
                .await
                .unwrap(),
            0
        );
        assert_eq!(row(&tx, "scheduled_event", id).await, original);
    }
    let cron = serde_json::from_value(json!({"scheduled_date": "2099-01-01T00:00:00Z"})).unwrap();
    assert_eq!(
        update_scheduled_event(&tx, &e.t(), &id.to_string(), cron, None)
            .await
            .unwrap(),
        1
    );
    assert!(row(&tx, "scheduled_event", id).await["stopped_at"].is_null());
    tx.execute(
        "UPDATE sequent_backend.scheduled_event SET archived_at = NOW() WHERE id = $1",
        &[&id],
    )
    .await
    .unwrap();
    let archived = row(&tx, "scheduled_event", id).await;
    let cron = serde_json::from_value(json!({"scheduled_date": "2099-02-01T00:00:00Z"})).unwrap();
    assert_eq!(
        update_scheduled_event(&tx, &e.t(), &id.to_string(), cron, None)
            .await
            .unwrap(),
        0
    );
    assert_eq!(row(&tx, "scheduled_event", id).await, archived);
}

fn lifecycle_processors() -> [EventProcessors; 5] {
    [
        EventProcessors::ALLOW_TALLY,
        EventProcessors::ALLOW_INIT_REPORT,
        EventProcessors::ALLOW_VOTING_PERIOD_END,
        EventProcessors::START_TEST_VOTING,
        EventProcessors::START_LOCKDOWN_PERIOD,
    ]
}

async fn run_lifecycle_task(
    tx: &Transaction<'_>,
    e: Event,
    id: Uuid,
    post: Uuid,
    processor: &EventProcessors,
) -> anyhow::Result<()> {
    use windmill::tasks::{
        manage_election_event_lockdown::manage_election_event_lockdown_wrapped,
        manage_election_init_report::manage_election_init_report_wrapped,
        manage_election_voting_period_end::manage_election_voting_period_end_wrapped,
    };
    match processor {
        EventProcessors::ALLOW_TALLY => {
            manage_election_allow_tally_wrapped(tx, e.t(), e.e(), id.to_string(), post.to_string())
                .await
        }
        EventProcessors::ALLOW_INIT_REPORT => {
            manage_election_init_report_wrapped(tx, e.t(), e.e(), id.to_string(), post.to_string())
                .await
        }
        EventProcessors::ALLOW_VOTING_PERIOD_END => {
            manage_election_voting_period_end_wrapped(
                tx,
                e.t(),
                e.e(),
                id.to_string(),
                post.to_string(),
            )
            .await
        }
        EventProcessors::START_LOCKDOWN_PERIOD => {
            manage_election_event_lockdown_wrapped(tx, e.t(), e.e(), id.to_string()).await
        }
        EventProcessors::START_TEST_VOTING => {
            manage_election_lifecycle_window_wrapped(
                tx,
                &e.t(),
                &e.e(),
                &id.to_string(),
                Some(&post.to_string()),
            )
            .await
        }
        _ => unreachable!("test processor"),
    }
}

#[tokio::test]
async fn archived_event_cannot_execute_queued_lifecycle_tasks() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let e = event(&tx, "UTC", &["UTC"]).await;
    let post = election(&tx, e, None).await;
    tx.execute(
        "UPDATE sequent_backend.election_event SET is_archived = true WHERE id = $1",
        &[&e.event],
    )
    .await
    .unwrap();
    let before_post = row(&tx, "election", post).await;
    let before_event = row(&tx, "election_event", e.event).await;
    for processor in lifecycle_processors() {
        let id = schedule(
            &tx,
            e,
            Some(post),
            processor.clone(),
            json!({"election_id": post}),
            json!({"scheduled_date": "2000-01-01T00:00:00Z"}),
        )
        .await;
        run_lifecycle_task(&tx, e, id, post, &processor)
            .await
            .unwrap();
        assert_eq!(row(&tx, "election", post).await, before_post, "{processor}");
        assert_eq!(
            row(&tx, "election_event", e.event).await,
            before_event,
            "{processor}"
        );
    }
}

#[tokio::test]
async fn queued_lifecycle_tasks_recheck_direct_schedule_edits_before_deciding_to_fire() {
    use std::time::Duration;
    let pool = schema::pool().await;
    #[derive(Clone, Copy)]
    enum Edit {
        Postpone,
        Archive,
    }
    for processor in lifecycle_processors() {
        for edit in [Edit::Postpone, Edit::Archive] {
            let mut editor = pool.get().await.unwrap();
            let tx = editor.transaction().await.unwrap();
            let e = event(&tx, "UTC", &["UTC"]).await;
            let post = election(&tx, e, None).await;
            let id = schedule(
                &tx,
                e,
                Some(post),
                processor.clone(),
                json!({"election_id": post}),
                json!({"scheduled_date": "2000-01-01T00:00:00Z"}),
            )
            .await;
            let before_post = row(&tx, "election", post).await;
            let before_event = row(&tx, "election_event", e.event).await;
            tx.commit().await.unwrap();
            let mut worker = pool.get().await.unwrap();
            let worker_pid: i32 = worker
                .query_one("SELECT pg_backend_pid()", &[])
                .await
                .unwrap()
                .get(0);
            let tx = editor.transaction().await.unwrap();
            let sql = match edit {
            Edit::Postpone => r#"UPDATE sequent_backend.scheduled_event SET cron_config = '{"scheduled_date": "2099-01-01T00:00:00Z"}' WHERE id = $1"#,
            Edit::Archive => "UPDATE sequent_backend.scheduled_event SET stopped_at = NOW(), archived_at = NOW() WHERE id = $1",
        };
            tx.execute(sql, &[&id]).await.unwrap();
            let task_processor = processor.clone();
            let task = tokio::spawn(async move {
                let tx = worker.transaction().await.unwrap();
                let result = run_lifecycle_task(&tx, e, id, post, &task_processor).await;
                tx.commit().await.unwrap();
                result
            });
            // Observe the actual row-lock wait rather than assuming the worker has started.
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if task.is_finished() {
                        panic!("queued task executed before the schedule edit committed");
                    }
                    let blocked: bool = tx
                        .query_one(
                            "SELECT cardinality(pg_blocking_pids($1)) > 0",
                            &[&worker_pid],
                        )
                        .await
                        .unwrap()
                        .get(0);
                    if blocked {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("queued task must lock its scheduled row");
            tx.commit().await.unwrap();
            let result = tokio::time::timeout(Duration::from_secs(5), task)
                .await
                .unwrap()
                .unwrap();
            match edit {
                Edit::Archive => assert!(result
                    .unwrap_err()
                    .to_string()
                    .contains("Can't find scheduled event")),
                Edit::Postpone => result.unwrap(),
            }
            let tx = editor.transaction().await.unwrap();
            assert_eq!(row(&tx, "election", post).await, before_post);
            assert_eq!(row(&tx, "election_event", e.event).await, before_event);
            let stored = row(&tx, "scheduled_event", id).await;
            assert_eq!(
                stored["stopped_at"].is_null(),
                matches!(edit, Edit::Postpone)
            );
        }
    }
}
