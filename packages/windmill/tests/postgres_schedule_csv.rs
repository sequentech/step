// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Importing and exporting the schedule as CSV against the database: the
//! rows the import writes pass the scheduled event checks, a second import
//! updates the same events, a file with errors writes nothing, and an export
//! imports back to the same schedule.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use serde_json::{json, Value};
use uuid::Uuid;
use windmill::services::schedule_csv::{
    export_schedule, import_schedule, preview_schedule, schedule_import_log_step, ImportOutcome,
    RowsWithErrors, ScheduleAuthor, ScheduleEntry, ScheduleRowError,
};
use windmill::services::signing::log::stage;

/// Whole hours, the fractional offsets and Cairo, cycled over the Posts.
const POST_ZONES: [&str; 8] = [
    "Asia/Dubai",
    "Asia/Kolkata",
    "Asia/Kathmandu",
    "Asia/Tehran",
    "Asia/Yangon",
    "Africa/Cairo",
    "America/Toronto",
    "Asia/Manila",
];

struct Config {
    primary: &'static str,
    post_zones: Vec<&'static str>,
    posts: usize,
}

fn overseas() -> Config {
    Config {
        primary: "Asia/Manila",
        post_zones: POST_ZONES.to_vec(),
        posts: 104,
    }
}

fn association() -> Config {
    Config {
        primary: "Europe/Madrid",
        post_zones: vec!["Europe/Madrid", "Atlantic/Canary"],
        posts: 4,
    }
}

struct Scope {
    tenant: Uuid,
    event: Uuid,
}

impl Scope {
    fn tenant(&self) -> String {
        self.tenant.to_string()
    }
    fn event(&self) -> String {
        self.event.to_string()
    }
}

async fn setup(tx: &Transaction<'_>, config: &Config) -> Scope {
    let scope = Scope {
        tenant: Uuid::new_v4(),
        event: Uuid::new_v4(),
    };
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&scope.tenant, &format!("tenant-{}", scope.tenant)],
    )
    .await
    .unwrap();
    let mut configured = vec![config.primary];
    configured.extend(
        config
            .post_zones
            .iter()
            .filter(|zone| **zone != config.primary),
    );
    let presentation = json!({
        "timezones": {"configured": configured, "primary": config.primary, "logs": "election"}
    });
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol, presentation)
         VALUES ($1, $2, 'RSA256', $3)",
        &[&scope.event, &scope.tenant, &presentation],
    )
    .await
    .unwrap();
    for index in 0..config.posts {
        let zone = config.post_zones[index % config.post_zones.len()];
        // The alias and name in the election's language, as the Admin
        // Portal shows them.
        let presentation = json!({
            "language_conf": {"default_language_code": "es"},
            "i18n": {
                "en": {"alias": format!("en-{index:03}"), "name": format!("Post {index:03}")},
                "es": {"alias": format!("post-{index:03}"), "name": format!("Puesto {index:03}")}
            },
            "timezone": zone
        });
        tx.execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation)
             VALUES ($1, $2, $3, $4)",
            &[&Uuid::new_v4(), &scope.tenant, &scope.event, &presentation],
        )
        .await
        .unwrap();
    }
    scope
}

/// Enrollment and voting open at `opens` local time in every Post; the
/// event closes both in the primary zone.
fn schedule_csv(config: &Config, opens: &str) -> String {
    let mut lines =
        vec!["election_alias,event_type,local_date_time,timezone,voting_channels".to_string()];
    for index in 0..config.posts {
        lines.push(format!(
            "post-{index:03},START_ENROLLMENT_PERIOD,2028-02-09T{opens},,"
        ));
        lines.push(format!(
            "post-{index:03},START_VOTING_PERIOD,2028-04-09T{opens},,ONLINE|KIOSK"
        ));
    }
    lines.push("ALL,END_ENROLLMENT_PERIOD,2028-03-09T00:00,,".to_string());
    lines.push("ALL,END_VOTING_PERIOD,2028-05-08T19:00,,ONLINE|KIOSK".to_string());
    lines.join("\n")
}

/// The event's active scheduled events: task id, cron_config, payload.
async fn stored(tx: &Transaction<'_>, scope: &Scope) -> Vec<(String, Value, Value)> {
    tx.query(
        "SELECT task_id, cron_config, event_payload FROM sequent_backend.scheduled_event
         WHERE tenant_id = $1 AND election_event_id = $2 AND archived_at IS NULL
         ORDER BY task_id",
        &[&scope.tenant, &scope.event],
    )
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.get(0), row.get(1), row.get(2)))
    .collect()
}

fn without_rows(entries: &[ScheduleEntry]) -> Vec<ScheduleEntry> {
    let mut entries: Vec<ScheduleEntry> = entries
        .iter()
        .cloned()
        .map(|entry| ScheduleEntry { row: 0, ..entry })
        .collect();
    entries.sort_by_key(|entry| (entry.election_id.clone(), entry.event_processor.to_string()));
    entries
}

#[tokio::test]
async fn an_import_writes_the_schedule_and_a_second_one_updates_it() {
    for config in [overseas(), association()] {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let scope = setup(&tx, &config).await;
        let rows = 2 * config.posts + 2;

        let first = schedule_csv(&config, "00:00");
        let (outcome, entries) =
            import_schedule(&tx, &scope.tenant(), &scope.event(), first.as_bytes())
                .await
                .unwrap();
        assert_eq!(
            outcome,
            ImportOutcome {
                created: rows as u64,
                updated: 0
            }
        );
        let events = stored(&tx, &scope).await;
        assert_eq!(events.len(), rows);

        // Every row keeps its wall time, its zone and the instant, with
        // the payload the voting period check requires.
        for entry in &entries {
            let task_id = entry.task_id(&scope.tenant(), &scope.event());
            let (_, cron_config, payload) =
                events.iter().find(|(task, _, _)| *task == task_id).unwrap();
            assert_eq!(cron_config["local"], json!(entry.local));
            assert_eq!(cron_config["timezone"], json!(entry.time_zone));
            assert_eq!(cron_config["scheduled_date"], json!(entry.scheduled_date));
            assert_eq!(payload["election_id"], json!(entry.election_id));
        }
        let close_task = format!(
            "tenant_{}_event_{}_END_VOTING_PERIOD",
            scope.tenant, scope.event
        );
        let (_, close, _) = events
            .iter()
            .find(|(task, _, _)| *task == close_task)
            .unwrap();
        assert_eq!(close["timezone"], json!(config.primary));
        assert_eq!(close["local"], json!("2028-05-08T19:00"));

        // A corrected file updates the same events.
        let second = schedule_csv(&config, "01:00");
        let (outcome, entries) =
            import_schedule(&tx, &scope.tenant(), &scope.event(), second.as_bytes())
                .await
                .unwrap();
        assert_eq!(
            outcome,
            ImportOutcome {
                created: 0,
                updated: rows as u64
            }
        );
        let events = stored(&tx, &scope).await;
        assert_eq!(events.len(), rows);
        let opening = entries[1].task_id(&scope.tenant(), &scope.event());
        let (_, cron_config, _) = events.iter().find(|(task, _, _)| *task == opening).unwrap();
        assert_eq!(cron_config["local"], json!("2028-04-09T01:00"));

        // An export imports back to the same schedule.
        let exported = export_schedule(&tx, &scope.tenant(), &scope.event())
            .await
            .unwrap();
        let again = preview_schedule(&tx, &scope.tenant(), &scope.event(), exported.as_bytes())
            .await
            .unwrap();
        assert_eq!(again.preview.errors, 0);
        assert_eq!(without_rows(&again.entries), without_rows(&entries));

        tx.rollback().await.unwrap();
    }
}

#[tokio::test]
async fn a_file_with_an_error_writes_nothing() {
    for config in [overseas(), association()] {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let scope = setup(&tx, &config).await;
        let first = schedule_csv(&config, "00:00");
        import_schedule(&tx, &scope.tenant(), &scope.event(), first.as_bytes())
            .await
            .unwrap();
        let before = stored(&tx, &scope).await;

        // Every row is fine but the last: the same type twice.
        let broken = format!(
            "{}\npost-000,START_VOTING_PERIOD,2028-04-10T00:00,,",
            schedule_csv(&config, "01:00")
        );
        let error = import_schedule(&tx, &scope.tenant(), &scope.event(), broken.as_bytes())
            .await
            .unwrap_err();
        let refusal = error.downcast_ref::<RowsWithErrors>().unwrap();
        assert_eq!(refusal.0.errors, 1);
        assert_eq!(stored(&tx, &scope).await, before);

        tx.rollback().await.unwrap();
    }
}

#[tokio::test]
async fn a_corrected_import_re_arms_an_event_that_already_ran() {
    // With the scheduler's re-arm rule (an edit to a future time clears
    // `stopped_at`), re-importing a corrected time for an event that already
    // ran updates it and arms it again.
    let config = association();
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let scope = setup(&tx, &config).await;
    let first = schedule_csv(&config, "00:00");
    let (_, entries) = import_schedule(&tx, &scope.tenant(), &scope.event(), first.as_bytes())
        .await
        .unwrap();
    // The opening of post-000 (file line 3) has run.
    let fired = entries[1].task_id(&scope.tenant(), &scope.event());
    tx.execute(
        "UPDATE sequent_backend.scheduled_event SET stopped_at = now() WHERE task_id = $1",
        &[&fired],
    )
    .await
    .unwrap();

    let second = schedule_csv(&config, "01:00");
    let (outcome, _) = import_schedule(&tx, &scope.tenant(), &scope.event(), second.as_bytes())
        .await
        .unwrap();
    assert_eq!(outcome.created, 0);
    assert_eq!(outcome.updated, entries.len() as u64);
    let stopped: Option<chrono::DateTime<chrono::Utc>> = tx
        .query_one(
            "SELECT stopped_at FROM sequent_backend.scheduled_event WHERE task_id = $1",
            &[&fired],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(stopped, None, "the corrected future time re-arms the event");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn a_file_without_channels_keeps_each_event_s_channels() {
    let config = association();
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let scope = setup(&tx, &config).await;
    import_schedule(
        &tx,
        &scope.tenant(),
        &scope.event(),
        schedule_csv(&config, "00:00").as_bytes(),
    )
    .await
    .unwrap();
    let without = "election_alias,event_type,local_date_time\n\
                   post-000,START_VOTING_PERIOD,2028-04-09T02:00\n";
    let (outcome, entries) =
        import_schedule(&tx, &scope.tenant(), &scope.event(), without.as_bytes())
            .await
            .unwrap();
    assert_eq!(outcome.updated, 1);
    let task = entries[0].task_id(&scope.tenant(), &scope.event());
    let events = stored(&tx, &scope).await;
    let (_, cron_config, payload) = events.iter().find(|(t, _, _)| *t == task).unwrap();
    assert_eq!(cron_config["local"], json!("2028-04-09T02:00"));
    assert_eq!(payload["voting_channels"], json!(["ONLINE", "KIOSK"]));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn the_import_entry_is_staged_in_the_import_transaction() {
    let config = association();
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let scope = setup(&tx, &config).await;
    let csv = schedule_csv(&config, "00:00");
    let (outcome, _) = import_schedule(&tx, &scope.tenant(), &scope.event(), csv.as_bytes())
        .await
        .unwrap();
    let author = ScheduleAuthor {
        user_id: "admin-1".to_string(),
        username: Some("ada".to_string()),
    };
    let step = schedule_import_log_step(
        &scope.tenant(),
        &scope.event(),
        "document-1",
        csv.as_bytes(),
        &outcome,
        &author,
    )
    .unwrap();
    stage(&tx, &step).await.unwrap();
    let rows = tx
        .query(
            "SELECT statement_kind, event_type, username, body FROM sequent_backend.signing_log_outbox
             WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY entry",
            &[&scope.tenant, &scope.event],
        )
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let kind: String = rows[0].get(0);
    let event_type: String = rows[0].get(1);
    let username: Option<String> = rows[0].get(2);
    let body: Value = rows[0].get(3);
    assert_eq!(kind, "ScheduleImported");
    assert_eq!(event_type, "USER");
    assert_eq!(username.as_deref(), Some("ada"));
    assert_eq!(body["details"]["created"], json!(outcome.created));
    assert_eq!(body["details"]["document_id"], json!("document-1"));
    tx.rollback().await.unwrap();
}
