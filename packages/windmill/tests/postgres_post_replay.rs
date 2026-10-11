// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The 104-Post replay (VOTE-LIFECYCLE acceptance): every Post opens voting
//! at 00:00 its own time on 9 April 2028 and one event-wide close ends voting
//! for every Post at 19:00 in the primary zone on 8 May. Run under the COMELEC
//! preset (104 Posts, including 27 synthetic additions; 80 zones, Manila
//! primary) and the Madrid association. This proves timezone behavior, not
//! official Annex A fidelity.
//!
//! The schedule is saved as the Admin Portal saves it (wall time + zone), and
//! imported again as the 210-row CSV with enrollment. Every expected instant
//! comes from the tz database (chrono-tz) and the configuration, never from a
//! literal.

#![recursion_limit = "256"]

#[path = "support/lifecycle_configurations.rs"]
mod lifecycle_configurations;
#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Duration, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use deadpool_postgres::Transaction;
use lifecycle_configurations::{comelec_preset, madrid_association, Configuration, Post};
use sequent_core::types::scheduled_event::{generate_voting_period_dates, EventProcessors};
use std::collections::BTreeSet;
use uuid::Uuid;
use windmill::postgres::election::get_elections;
use windmill::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use windmill::services::election_dates::{get_election_dates, save_schedule, ScheduleInput};
use windmill::services::enrollment_windows::{election_windows, PostElection};
use windmill::services::schedule_csv::{
    export_schedule, import_schedule, preview_schedule, ImportOutcome,
};
use windmill::services::signing::log::Actor;

const OPENS: &str = "2028-04-09T00:00";
const CLOSES: &str = "2028-05-08T19:00";
const ENROLLMENT_OPENS: &str = "2028-02-09T00:00";
/// Enrollment closes for every Post one hour before voting does.
const ENROLLMENT_CLOSES: &str = "2028-05-08T18:00";

struct Scope {
    tenant: Uuid,
    event: Uuid,
    /// Each Post's election id, in the configuration's order.
    elections: Vec<Uuid>,
}

impl Scope {
    fn tenant(&self) -> String {
        self.tenant.to_string()
    }
    fn event(&self) -> String {
        self.event.to_string()
    }
}

async fn setup(tx: &Transaction<'_>, config: &Configuration) -> Scope {
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
        &[&event, &tenant, &config.presentation],
    )
    .await
    .unwrap();
    let mut elections = vec![];
    for post in &config.posts {
        let id = Uuid::new_v4();
        tx.execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation)
             VALUES ($1, $2, $3, $4)",
            &[&id, &tenant, &event, &config.election_presentation(post)],
        )
        .await
        .unwrap();
        elections.push(id);
    }
    Scope {
        tenant,
        event,
        elections,
    }
}

fn zone(name: &str) -> Tz {
    name.parse()
        .unwrap_or_else(|_| panic!("{name} isn't in the tz database"))
}

/// The instant a wall time names in a zone, straight from the tz database.
/// Midnight on 9 April and 19:00 on 8 May exist exactly once in every zone
/// of both configurations; a gap or an overlap would fail here.
fn instant(local: &str, zone: Tz) -> DateTime<Utc> {
    let naive = NaiveDateTime::parse_from_str(local, "%Y-%m-%dT%H:%M").unwrap();
    zone.from_local_datetime(&naive)
        .single()
        .unwrap_or_else(|| panic!("{local} isn't a single instant in {}", zone.name()))
        .with_timezone(&Utc)
}

fn parse(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap_or_else(|err| panic!("{text:?} isn't RFC 3339: {err}"))
        .with_timezone(&Utc)
}

fn offset_minutes(zone: Tz, at: DateTime<Utc>) -> i32 {
    zone.offset_from_utc_datetime(&at.naive_utc())
        .fix()
        .local_minus_utc()
        / 60
}

/// The local calendar dates the voting window `[start, end)` touches in a
/// zone, and the hours of voting on its last local day.
fn local_window(zone: Tz, start: DateTime<Utc>, end: DateTime<Utc>) -> (i64, i64) {
    let first = start.with_timezone(&zone).date_naive();
    let last = (end - Duration::seconds(1))
        .with_timezone(&zone)
        .date_naive();
    let last_day_start = start
        .with_timezone(&zone)
        .naive_local()
        .max(last.and_hms_opt(0, 0, 0).unwrap());
    let hours = (end.with_timezone(&zone).naive_local() - last_day_start).num_hours();
    ((last - first).num_days() + 1, hours)
}

fn admin() -> Actor {
    Actor {
        user_id: "admin".into(),
        username: "admin".into(),
    }
}

fn wall_time(local: &str) -> ScheduleInput {
    ScheduleInput {
        local_date_time: Some(local.to_string()),
        time_zone: None,
        scheduled_date: None,
    }
}

/// (start_date, end_date) of a Post's voting window row.
async fn voting_window(
    tx: &Transaction<'_>,
    scope: &Scope,
    election: Uuid,
) -> (Option<String>, Option<String>) {
    let row = tx
        .query_one(
            "SELECT start_date, end_date FROM sequent_backend.election_voting_window
             WHERE tenant_id = $1 AND election_event_id = $2 AND election_id = $3",
            &[&scope.tenant, &scope.event, &election],
        )
        .await
        .unwrap();
    (row.get(0), row.get(1))
}

/// What vote casting (`election_voting_window`), the voter screens
/// (`get_election_dates`, the ballot styles' dates) and the scheduler's own
/// window (`generate_voting_period_dates`) give each Post: its own opening in
/// its zone and the event-wide close.
async fn assert_every_post_opens_locally_and_closes_together(
    tx: &Transaction<'_>,
    config: &Configuration,
    scope: &Scope,
) {
    let primary = zone(&config.primary());
    let close = instant(CLOSES, primary);
    let scheduled = find_scheduled_event_by_election_event_id(tx, &scope.tenant(), &scope.event())
        .await
        .unwrap();
    let elections = get_elections(tx, &scope.tenant(), &scope.event())
        .await
        .unwrap();
    for (post, election_id) in config.posts.iter().zip(&scope.elections) {
        let post_zone = zone(&config.zone_of(post));
        let opening = instant(OPENS, post_zone);
        let label = format!("{} ({})", post.name, post_zone.name());

        let (start, end) = voting_window(tx, scope, *election_id).await;
        assert_eq!(
            start.as_deref().map(parse),
            Some(opening),
            "{label}: window start"
        );
        assert_eq!(
            end.as_deref().map(parse),
            Some(close),
            "{label}: window end"
        );

        let election = elections
            .iter()
            .find(|election| election.id == election_id.to_string())
            .unwrap();
        let dates = get_election_dates(election, scheduled.clone()).unwrap();
        let dates = dates.scheduled_event_dates.unwrap();
        let start = &dates[&EventProcessors::START_VOTING_PERIOD.to_string()];
        let end = &dates[&EventProcessors::END_VOTING_PERIOD.to_string()];
        assert_eq!(
            start.scheduled_at.as_deref().map(parse),
            Some(opening),
            "{label}"
        );
        assert_eq!(start.timezone.as_deref(), Some(post_zone.name()), "{label}");
        assert_eq!(
            end.scheduled_at.as_deref().map(parse),
            Some(close),
            "{label}"
        );
        assert_eq!(end.timezone.as_deref(), Some(primary.name()), "{label}");

        let period = generate_voting_period_dates(
            scheduled.clone(),
            &scope.tenant(),
            &scope.event(),
            Some(&election_id.to_string()),
        )
        .unwrap();
        assert_eq!(
            period.start_date.as_deref().map(parse),
            Some(opening),
            "{label}"
        );
        assert_eq!(
            period.end_date.as_deref().map(parse),
            Some(close),
            "{label}"
        );
    }
}

/// The schedule as the Admin Portal saves it: each Post's opening at local
/// midnight in its zone (the input's default zone), then the one close in
/// the primary.
#[tokio::test]
async fn every_post_opens_at_local_midnight_and_the_event_wide_close_ends_them_all() {
    for config in [comelec_preset(), madrid_association()] {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let scope = setup(&tx, &config).await;
        let primary = zone(&config.primary());

        for (post, election_id) in config.posts.iter().zip(&scope.elections) {
            let post_zone = zone(&config.zone_of(post));
            let saved = save_schedule(
                &tx,
                &scope.tenant(),
                &scope.event(),
                Some(&election_id.to_string()),
                &EventProcessors::START_VOTING_PERIOD,
                &wall_time(OPENS),
                None,
                &admin(),
                None,
            )
            .await
            .unwrap_or_else(|err| panic!("{}: {err:?}", post.name));
            assert_eq!(
                saved.scheduled_date.as_deref().map(parse),
                Some(instant(OPENS, post_zone)),
                "{} opens at {OPENS} {}",
                post.name,
                post_zone.name()
            );
        }
        let saved = save_schedule(
            &tx,
            &scope.tenant(),
            &scope.event(),
            None,
            &EventProcessors::END_VOTING_PERIOD,
            &wall_time(CLOSES),
            None,
            &admin(),
            None,
        )
        .await
        .unwrap();
        let close = instant(CLOSES, primary);
        assert_eq!(saved.scheduled_date.as_deref().map(parse), Some(close));

        // The rows keep the wall time and the zone with the instant.
        let rows = tx
            .query(
                "SELECT event_payload->>'election_id', cron_config FROM sequent_backend.scheduled_event
                 WHERE tenant_id = $1 AND election_event_id = $2 AND archived_at IS NULL",
                &[&scope.tenant, &scope.event],
            )
            .await
            .unwrap();
        assert_eq!(rows.len(), config.posts.len() + 1, "{}", config.label);
        for row in rows {
            let election: Option<String> = row.get(0);
            let cron: serde_json::Value = row.get(1);
            let (local, zone_name) = match election {
                Some(id) => {
                    let index = scope
                        .elections
                        .iter()
                        .position(|election| election.to_string() == id)
                        .unwrap();
                    (OPENS, config.zone_of(&config.posts[index]))
                }
                None => (CLOSES, config.primary()),
            };
            assert_eq!(cron["local"], local);
            assert_eq!(cron["timezone"], zone_name.as_str());
            assert_eq!(
                parse(cron["scheduled_date"].as_str().unwrap()),
                instant(local, zone(&zone_name))
            );
        }

        // The close's warnings name the Posts whose local window isn't 30
        // calendar days or whose last local day is short (they warn, never
        // block): computed here from the tz database.
        let mut expected_days = BTreeSet::new();
        let mut expected_short = BTreeSet::new();
        for (post, election_id) in config.posts.iter().zip(&scope.elections) {
            let post_zone = zone(&config.zone_of(post));
            let (days, hours) = local_window(post_zone, instant(OPENS, post_zone), close);
            if days != 30 {
                expected_days.insert(election_id.to_string());
            }
            if hours < 12 {
                expected_short.insert(election_id.to_string());
            }
        }
        let warned = |code: &str| -> BTreeSet<String> {
            saved
                .warnings
                .iter()
                .filter(|warning| warning.code == code)
                .filter_map(|warning| warning.election_id.clone())
                .collect()
        };
        if config.posts.len() == 104 {
            // Ticket D4: far west of Manila the common close falls early on
            // 8 May (Honolulu at 01:00) or even on 7 May (Pago Pago), so
            // those Posts are warned; the save still goes through.
            let id_of = |zone: &str| {
                let index = config
                    .posts
                    .iter()
                    .position(|post| post.time_zone.as_deref() == Some(zone))
                    .unwrap();
                scope.elections[index].to_string()
            };
            assert!(expected_days.contains(&id_of("Pacific/Pago_Pago")));
            assert!(expected_short.contains(&id_of("Pacific/Honolulu")));
            assert!(!expected_days.contains(&id_of("Asia/Tokyo")));
        }
        assert_eq!(
            warned("voting-window-days"),
            expected_days,
            "{}",
            config.label
        );
        assert_eq!(warned("short-last-day"), expected_short, "{}", config.label);

        assert_every_post_opens_locally_and_closes_together(&tx, &config, &scope).await;
        tx.rollback().await.unwrap();
    }
}

/// The offsets and the DST change the acceptance names are in the COMELEC
/// preset's replay: +05:30, +05:45, +03:30 and +06:30 at the opening, and
/// Cairo opening before its change on 28 April and closing after it.
#[test]
fn the_comelec_replay_covers_the_fractional_offsets_and_cairo_s_dst_change() {
    let config = comelec_preset();
    let primary = zone(&config.primary());
    let close = instant(CLOSES, primary);
    let offsets: BTreeSet<i32> = config
        .posts
        .iter()
        .map(|post| {
            let post_zone = zone(&config.zone_of(post));
            offset_minutes(post_zone, instant(OPENS, post_zone))
        })
        .collect();
    for minutes in [5 * 60 + 30, 5 * 60 + 45, 3 * 60 + 30, 6 * 60 + 30] {
        assert!(offsets.contains(&minutes), "no Post at {minutes} minutes");
    }

    let cairo = config
        .posts
        .iter()
        .find(|post| post.time_zone.as_deref() == Some("Africa/Cairo"))
        .expect("a Post in Cairo");
    let cairo_zone = zone(&config.zone_of(cairo));
    let opening = instant(OPENS, cairo_zone);
    let change = instant("2028-04-28T12:00", cairo_zone);
    assert!(opening < change && change < close);
    assert!(
        offset_minutes(cairo_zone, close) > offset_minutes(cairo_zone, opening),
        "Cairo's clocks go forward between the opening and the close"
    );
    // 104 Posts, 80 configured zones: every Post's zone and the primary.
    assert_eq!(config.posts.len(), 104);
    let configured: BTreeSet<String> = config.configured().into_iter().collect();
    assert_eq!(configured.len(), 80);
    let mut used: BTreeSet<String> = config
        .posts
        .iter()
        .map(|post| config.zone_of(post))
        .collect();
    used.insert(config.primary());
    assert_eq!(used, configured);
    // Every configured name is the tz database's canonical one.
    for name in &configured {
        assert_eq!(
            windmill::services::time_zones::parse_zone(name)
                .unwrap()
                .name(),
            name
        );
    }
}

fn replay_csv(config: &Configuration) -> String {
    let mut lines =
        vec!["election_alias,event_type,local_date_time,timezone,voting_channels".to_string()];
    for post in &config.posts {
        lines.push(format!(
            "{},START_ENROLLMENT_PERIOD,{ENROLLMENT_OPENS},,",
            post.alias
        ));
        lines.push(format!("{},START_VOTING_PERIOD,{OPENS},,", post.alias));
    }
    lines.push(format!("ALL,END_ENROLLMENT_PERIOD,{ENROLLMENT_CLOSES},,"));
    lines.push(format!("ALL,END_VOTING_PERIOD,{CLOSES},,"));
    lines.join("\n") + "\n"
}

/// The same schedule with enrollment as a CSV (210 rows for the 104 Posts):
/// every row previews in its Post's time and in the primary, the import
/// writes them all, and importing it again updates the same events.
#[tokio::test]
async fn the_replay_csv_previews_and_imports_cleanly() {
    for config in [comelec_preset(), madrid_association()] {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let scope = setup(&tx, &config).await;
        let primary = zone(&config.primary());
        let csv = replay_csv(&config);
        let rows = 2 * config.posts.len() + 2;
        if config.posts.len() == 104 {
            assert_eq!(rows, 210);
        }

        let parsed = preview_schedule(&tx, &scope.tenant(), &scope.event(), csv.as_bytes())
            .await
            .unwrap();
        let preview = &parsed.preview;
        assert_eq!(preview.errors, 0, "{}: {:?}", config.label, preview.rows);
        assert_eq!(preview.ok, rows as u64);
        assert_eq!(preview.posts, config.posts.len() as u64);
        assert_eq!(preview.primary_time_zone, primary.name());
        assert_eq!(preview.rows.len(), rows);
        for row in &preview.rows {
            assert_eq!(row.error_code, None, "row {}", row.row);
            assert_eq!(row.note_code, None, "row {}", row.row);
            let post: Option<&Post> = config
                .posts
                .iter()
                .find(|post| post.alias == row.election_alias);
            let row_zone = match post {
                Some(post) => zone(&config.zone_of(post)),
                None => {
                    assert_eq!(row.election_alias, "ALL");
                    primary
                }
            };
            let expected = instant(&row.local, row_zone);
            assert_eq!(row.time_zone, row_zone.name(), "row {}", row.row);
            let shown = DateTime::parse_from_rfc3339(row.instant.as_deref().unwrap()).unwrap();
            assert_eq!(shown.with_timezone(&Utc), expected, "row {}", row.row);
            assert_eq!(
                shown.offset().local_minus_utc() / 60,
                offset_minutes(row_zone, expected),
                "row {}: shown with the row zone's offset",
                row.row
            );
            assert_eq!(
                row.primary_local.as_deref(),
                Some(
                    expected
                        .with_timezone(&primary)
                        .format("%Y-%m-%dT%H:%M")
                        .to_string()
                        .as_str()
                ),
                "row {}",
                row.row
            );
        }

        let (outcome, _) = import_schedule(&tx, &scope.tenant(), &scope.event(), csv.as_bytes())
            .await
            .unwrap();
        assert_eq!(
            outcome,
            ImportOutcome {
                created: rows as u64,
                updated: 0
            }
        );
        assert_every_post_opens_locally_and_closes_together(&tx, &config, &scope).await;

        let exported = export_schedule(&tx, &scope.tenant(), &scope.event())
            .await
            .unwrap();
        let round_trip =
            preview_schedule(&tx, &scope.tenant(), &scope.event(), exported.as_bytes())
                .await
                .unwrap();
        assert_eq!(round_trip.preview.errors, 0);
        assert_eq!(round_trip.preview.rows.len(), rows);
        let (outcome, _) =
            import_schedule(&tx, &scope.tenant(), &scope.event(), exported.as_bytes())
                .await
                .unwrap();
        assert_eq!(
            outcome,
            ImportOutcome {
                created: 0,
                updated: rows as u64
            }
        );
        assert_every_post_opens_locally_and_closes_together(&tx, &config, &scope).await;
        let scheduled =
            find_scheduled_event_by_election_event_id(&tx, &scope.tenant(), &scope.event())
                .await
                .unwrap();
        let elections: Vec<PostElection> = config
            .posts
            .iter()
            .zip(&scope.elections)
            .map(|(post, id)| PostElection {
                id: id.to_string(),
                presentation: Some(
                    serde_json::from_value(config.election_presentation(post)).unwrap(),
                ),
            })
            .collect();
        let event = serde_json::from_value(config.presentation.clone()).unwrap();
        let enrollment = election_windows(Some(&event), &elections, &scheduled);
        assert_eq!(enrollment.len(), config.posts.len());
        for (post, id) in config.posts.iter().zip(&scope.elections) {
            let window = &enrollment[&id.to_string()];
            assert_eq!(
                window.opens_at.as_deref().map(parse),
                Some(instant(ENROLLMENT_OPENS, zone(&config.zone_of(post))))
            );
            let close = window.closes_at.as_deref().map(parse).unwrap();
            assert_eq!(close, instant(ENROLLMENT_CLOSES, primary));
            assert_eq!(instant(CLOSES, primary) - close, Duration::hours(1));
        }
        tx.rollback().await.unwrap();
    }
}
