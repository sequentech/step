// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Snapshot passes: a pass shows what it counted, an unchanged pass writes
//! nothing and counts again only when something it counts from moved, a
//! change writes only the scopes it changed while earlier revisions keep
//! what they showed, and pruning keeps the shown run.

#[path = "support/schema.rs"]
mod schema;

use chrono::Duration;
use deadpool_postgres::{Client, Pool};
use sequent_core::monitoring::config::Settings;
use sequent_core::monitoring::payload::ScopePayload;
use sequent_core::monitoring::presets;
use sequent_core::monitoring::sources::{DataSourceId, Measure, PendingProducer};
use serde_json::json;
use uuid::Uuid;
use windmill::postgres::monitoring_config::EventRef;
use windmill::services::monitoring::projection::refresh_voter_activity;
use windmill::services::monitoring::snapshot::{
    complete_snapshot, count_event, count_run, live_snapshot, prune_snapshots, read_scope,
    request_election_set, scope_catalogue, start_run, PassOutcome, Recount, ScopeRead,
    RECOUNT_EVERY,
};
use windmill::tasks::refresh_monitoring_snapshot::events_to_prune;

fn settings() -> Settings {
    presets::load("comelec")
        .unwrap()
        .unwrap()
        .set
        .settings
        .clone()
        .unwrap()
}

struct Event {
    event: EventRef,
    madrid: Uuid,
    tokyo: Uuid,
}

async fn seed(client: &mut Client) -> Event {
    let event = EventRef {
        tenant_id: Uuid::new_v4(),
        election_event_id: Uuid::new_v4(),
    };
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&event.tenant_id, &format!("tenant-{}", event.tenant_id)],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event.election_event_id, &event.tenant_id],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.monitoring_event (tenant_id, election_event_id)
         VALUES ($1, $2)",
        &[&event.tenant_id, &event.election_event_id],
    )
    .await
    .unwrap();
    let mut ids = Vec::new();
    for (name, region, status) in [("Madrid", "Europe", "OPEN"), ("Tokyo", "Asia", "CLOSED")] {
        let id = Uuid::new_v4();
        tx.execute(
            "INSERT INTO sequent_backend.election
                 (id, tenant_id, election_event_id, presentation, annotations, status)
             VALUES ($1, $2, $3, $4, $5, $6)",
            &[
                &id,
                &event.tenant_id,
                &event.election_event_id,
                &json!({"i18n": {"en": {"name": name}}}),
                &json!({"miru:geographical-region": region}),
                &json!({"voting_status": status}),
            ],
        )
        .await
        .unwrap();
        ids.push(id);
    }
    tx.commit().await.unwrap();
    Event {
        event,
        madrid: ids[0],
        tokyo: ids[1],
    }
}

async fn voter(
    client: &Client,
    event: &Event,
    voter: &str,
    election: Uuid,
    region: &str,
    voted: bool,
) {
    client
        .execute(
            "INSERT INTO sequent_backend.monitoring_voter
                 (tenant_id, election_event_id, election_id, voter_id, region, country, dims,
                  first_voted_at, attributes_hash, settings_revision)
             VALUES ($1, $2, $3, $4, $5, 'Spain', '{\"sex\": \"F\"}',
                     CASE WHEN $6 THEN now() - interval '1 hour' END, 'h', 1)",
            &[
                &event.event.tenant_id,
                &event.event.election_event_id,
                &election,
                &voter,
                &region,
                &voted,
            ],
        )
        .await
        .unwrap();
}

async fn pass(client: &mut Client, event: &Event, settings: &Settings) -> PassOutcome {
    count_event(client, event.event, settings, 1, 1)
        .await
        .unwrap()
}

async fn read(
    client: &mut Client,
    event: &Event,
    revision: i64,
    source: DataSourceId,
    set: &str,
    scope: &str,
) -> ScopeRead {
    let tx = client.transaction().await.unwrap();
    let read = read_scope(&tx, event.event, revision, source, set, scope)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    read
}

fn payload(read: ScopeRead) -> ScopePayload {
    match read {
        ScopeRead::Payload { text, .. } => serde_json::from_str(&text).unwrap(),
        other => panic!("no payload: {other:?}"),
    }
}

async fn full_set(client: &mut Client, event: &Event) -> String {
    let tx = client.transaction().await.unwrap();
    let key = request_election_set(&tx, event.event, &[event.madrid, event.tokyo])
        .await
        .unwrap();
    tx.commit().await.unwrap();
    key
}

async fn live(client: &mut Client, event: &Event) -> i64 {
    let tx = client.transaction().await.unwrap();
    let live = live_snapshot(&tx, event.event).await.unwrap().unwrap();
    tx.commit().await.unwrap();
    live.revision
}

async fn open_figures(client: &Client, event: &Event) -> i64 {
    client
        .query_one(
            "SELECT count(*) FROM sequent_backend.monitoring_snapshot_figure
             WHERE tenant_id = $1 AND election_event_id = $2 AND to_revision IS NULL",
            &[&event.event.tenant_id, &event.event.election_event_id],
        )
        .await
        .unwrap()
        .get(0)
}

async fn client(pool: &Pool) -> Client {
    pool.get().await.unwrap()
}

#[tokio::test]
async fn a_pass_shows_what_it_counted_and_an_unchanged_one_writes_nothing() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = settings();
    let event = seed(&mut client).await;
    voter(&client, &event, "ana", event.madrid, "Europe", true).await;
    voter(&client, &event, "ana", event.tokyo, "Asia", false).await;
    voter(&client, &event, "ben", event.tokyo, "Asia", false).await;

    let PassOutcome::Completed {
        revision,
        figures_written,
    } = pass(&mut client, &event, &settings).await
    else {
        panic!("the first pass completes");
    };
    assert!(figures_written > 0);
    assert_eq!(live(&mut client, &event).await, revision);
    let set = full_set(&mut client, &event).await;

    let turnout = payload(
        read(
            &mut client,
            &event,
            revision,
            DataSourceId::VoterTurnout,
            &set,
            "event",
        )
        .await,
    );
    assert_eq!(turnout.totals[&Measure::Registered], 2);
    assert_eq!(turnout.totals[&Measure::Voted], 1);
    let asia = payload(
        read(
            &mut client,
            &event,
            revision,
            DataSourceId::VoterTurnout,
            &set,
            "region=Asia",
        )
        .await,
    );
    assert_eq!(asia.totals[&Measure::Registered], 2);
    assert_eq!(asia.totals[&Measure::Voted], 0, "ana voted in Madrid");
    let poll = payload(
        read(
            &mut client,
            &event,
            revision,
            DataSourceId::PollStatus,
            &set,
            "event",
        )
        .await,
    );
    assert_eq!(poll.totals[&Measure::Posts], 2);
    assert_eq!(poll.totals[&Measure::Closed], 1);
    assert_eq!(
        read(
            &mut client,
            &event,
            revision,
            DataSourceId::TestVoting,
            &set,
            "event"
        )
        .await,
        ScopeRead::NotConnected {
            reason: PendingProducer::TestElectionDesignation
        }
    );
    assert_eq!(
        read(
            &mut client,
            &event,
            revision,
            DataSourceId::VoterTurnout,
            &set,
            "country=Italy"
        )
        .await,
        ScopeRead::Empty,
        "counted, nobody there"
    );
    assert_eq!(
        read(
            &mut client,
            &event,
            revision,
            DataSourceId::VoterTurnout,
            "0000000000000000",
            "event"
        )
        .await,
        ScopeRead::NotCounted
    );
    {
        let tx = client.transaction().await.unwrap();
        let catalogue = scope_catalogue(&tx, event.event, revision, &set)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(catalogue.regions, vec!["Asia", "Europe"]);
        let mut posts = vec![event.madrid, event.tokyo];
        posts.sort();
        assert_eq!(catalogue.posts, posts);
        assert_eq!(catalogue.countries, vec!["Spain"]);
    }

    assert_eq!(
        pass(&mut client, &event, &settings).await,
        PassOutcome::Unchanged {
            shown: Some(revision),
            recount: Recount::Skipped,
        },
        "nothing it counts from moved"
    );
    assert_eq!(
        live(&mut client, &event).await,
        revision,
        "viewers keep what they have"
    );
    let runs: i64 = client
        .query_one(
            "SELECT count(*) FROM sequent_backend.monitoring_snapshot_run
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&event.event.tenant_id, &event.event.election_event_id],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(runs, 1, "the unchanged pass left no run");
    let checked: bool = client
        .query_one(
            "SELECT checked_at IS NOT NULL FROM sequent_backend.monitoring_snapshot_run
             WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3",
            &[
                &event.event.tenant_id,
                &event.event.election_event_id,
                &revision,
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert!(checked);
}

#[tokio::test]
async fn a_change_writes_only_its_scopes_and_earlier_revisions_keep_theirs() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = settings();
    let event = seed(&mut client).await;
    voter(&client, &event, "ana", event.madrid, "Europe", true).await;
    voter(&client, &event, "ben", event.tokyo, "Asia", false).await;
    let PassOutcome::Completed {
        revision: first, ..
    } = pass(&mut client, &event, &settings).await
    else {
        panic!();
    };
    let set = full_set(&mut client, &event).await;
    let open_before = open_figures(&client, &event).await;

    client
        .execute(
            "UPDATE sequent_backend.monitoring_voter SET first_voted_at = now()
             WHERE tenant_id = $1 AND election_event_id = $2 AND voter_id = 'ben'",
            &[&event.event.tenant_id, &event.event.election_event_id],
        )
        .await
        .unwrap();
    let PassOutcome::Completed {
        revision: second,
        figures_written,
    } = pass(&mut client, &event, &settings).await
    else {
        panic!("a change completes a run");
    };
    assert!(second > first);
    assert_eq!(live(&mut client, &event).await, second);
    assert_eq!(
        open_figures(&client, &event).await,
        open_before,
        "each scope still shows one figure"
    );
    let europe_first = read(
        &mut client,
        &event,
        first,
        DataSourceId::VoterTurnout,
        &set,
        "region=Europe",
    )
    .await;
    let europe_second = read(
        &mut client,
        &event,
        second,
        DataSourceId::VoterTurnout,
        &set,
        "region=Europe",
    )
    .await;
    assert_eq!(europe_first, europe_second, "Europe did not change");
    let written: i64 = client
        .query_one(
            "SELECT count(*) FROM sequent_backend.monitoring_snapshot_figure
             WHERE tenant_id = $1 AND election_event_id = $2 AND from_revision = $3",
            &[
                &event.event.tenant_id,
                &event.event.election_event_id,
                &second,
            ],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(written as usize, figures_written);
    assert!(
        figures_written < open_before as usize,
        "only the scopes that changed"
    );

    let before = payload(
        read(
            &mut client,
            &event,
            first,
            DataSourceId::VoterTurnout,
            &set,
            "event",
        )
        .await,
    );
    let after = payload(
        read(
            &mut client,
            &event,
            second,
            DataSourceId::VoterTurnout,
            &set,
            "event",
        )
        .await,
    );
    assert_eq!(
        before.totals[&Measure::Voted],
        1,
        "the earlier revision shows what it showed"
    );
    assert_eq!(after.totals[&Measure::Voted], 2);

    let pruned = prune_snapshots(&mut client, event.event, Duration::zero())
        .await
        .unwrap();
    assert_eq!(pruned.runs, 1);
    assert!(pruned.figures > 0);
    let tx = client.transaction().await.unwrap();
    assert_eq!(
        complete_snapshot(&tx, event.event, first).await.unwrap(),
        None
    );
    assert!(complete_snapshot(&tx, event.event, second)
        .await
        .unwrap()
        .is_some());
    tx.commit().await.unwrap();
    assert_eq!(
        read(
            &mut client,
            &event,
            first,
            DataSourceId::VoterTurnout,
            &set,
            "event"
        )
        .await,
        ScopeRead::NotCounted
    );
    let still = payload(
        read(
            &mut client,
            &event,
            second,
            DataSourceId::VoterTurnout,
            &set,
            "event",
        )
        .await,
    );
    assert_eq!(
        still.totals[&Measure::Voted],
        2,
        "the shown run keeps all its figures"
    );
    let orphans: i64 = client
        .query_one(
            "SELECT count(*) FROM sequent_backend.monitoring_snapshot_payload p
             WHERE tenant_id = $1 AND election_event_id = $2
               AND NOT EXISTS (SELECT 1 FROM sequent_backend.monitoring_snapshot_figure f
                               WHERE f.tenant_id = p.tenant_id
                                 AND f.election_event_id = p.election_event_id
                                 AND f.payload_sha256 = p.sha256)",
            &[&event.event.tenant_id, &event.event.election_event_id],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(orphans, 0);
}

#[tokio::test]
async fn a_set_a_viewer_asks_for_is_counted_by_the_next_pass() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = settings();
    let event = seed(&mut client).await;
    voter(&client, &event, "ana", event.madrid, "Europe", true).await;
    voter(&client, &event, "ben", event.tokyo, "Asia", false).await;
    pass(&mut client, &event, &settings).await;
    let tx = client.transaction().await.unwrap();
    let tokyo_only = request_election_set(&tx, event.event, &[event.tokyo])
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let shown = live(&mut client, &event).await;
    assert_eq!(
        read(
            &mut client,
            &event,
            shown,
            DataSourceId::VoterTurnout,
            &tokyo_only,
            "event"
        )
        .await,
        ScopeRead::NotCounted,
        "not counted until the next pass"
    );
    let PassOutcome::Completed { revision, .. } = pass(&mut client, &event, &settings).await else {
        panic!("a new set completes a run");
    };
    let tokyo = payload(
        read(
            &mut client,
            &event,
            revision,
            DataSourceId::VoterTurnout,
            &tokyo_only,
            "event",
        )
        .await,
    );
    assert_eq!(tokyo.totals[&Measure::Registered], 1);
    assert_eq!(tokyo.totals[&Measure::Voted], 0);
    let poll = payload(
        read(
            &mut client,
            &event,
            revision,
            DataSourceId::PollStatus,
            &tokyo_only,
            "event",
        )
        .await,
    );
    assert_eq!(poll.totals[&Measure::Posts], 1);
}

/// Moves a run's times back by `hours`, as if it had run then. Runs are
/// final, so this goes around the tables' triggers.
async fn backdate(client: &mut Client, event: &Event, revision: i64, hours: i32) {
    let tx = client.transaction().await.unwrap();
    tx.batch_execute("SET LOCAL session_replication_role = replica")
        .await
        .unwrap();
    tx.execute(
        "UPDATE sequent_backend.monitoring_snapshot_run
         SET started_at = started_at - make_interval(hours => $4),
             finished_at = finished_at - make_interval(hours => $4),
             as_of = as_of - make_interval(hours => $4),
             checked_at = checked_at - make_interval(hours => $4)
         WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3",
        &[
            &event.event.tenant_id,
            &event.event.election_event_id,
            &revision,
            &hours,
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

async fn is_kept(client: &mut Client, event: &Event, revision: i64) -> bool {
    let tx = client.transaction().await.unwrap();
    let kept = complete_snapshot(&tx, event.event, revision)
        .await
        .unwrap()
        .is_some();
    tx.commit().await.unwrap();
    kept
}

async fn vote(client: &Client, event: &Event, voter: &str) {
    client
        .execute(
            "UPDATE sequent_backend.monitoring_voter SET first_voted_at = now()
             WHERE tenant_id = $1 AND election_event_id = $2 AND voter_id = $3",
            &[
                &event.event.tenant_id,
                &event.event.election_event_id,
                &voter,
            ],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn a_run_shown_for_hours_is_kept_for_the_window_after_it_is_replaced() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = settings();
    let event = seed(&mut client).await;
    voter(&client, &event, "ana", event.madrid, "Europe", false).await;
    voter(&client, &event, "ben", event.tokyo, "Asia", false).await;
    let PassOutcome::Completed {
        revision: first, ..
    } = pass(&mut client, &event, &settings).await
    else {
        panic!();
    };
    // Shown for three hours, then replaced.
    backdate(&mut client, &event, first, 3).await;
    vote(&client, &event, "ana").await;
    let PassOutcome::Completed {
        revision: second, ..
    } = pass(&mut client, &event, &settings).await
    else {
        panic!();
    };
    let window = Duration::hours(2);
    let pruned = prune_snapshots(&mut client, event.event, window)
        .await
        .unwrap();
    assert_eq!(pruned.runs, 0);
    assert!(
        is_kept(&mut client, &event, first).await,
        "a viewer shown it a moment ago can still export it"
    );

    // Replaced more than the window ago.
    backdate(&mut client, &event, second, 3).await;
    vote(&client, &event, "ben").await;
    let PassOutcome::Completed {
        revision: third, ..
    } = pass(&mut client, &event, &settings).await
    else {
        panic!();
    };
    let closed_before: i64 = client
        .query_one(
            "SELECT count(*) FROM sequent_backend.monitoring_snapshot_figure
             WHERE tenant_id = $1 AND election_event_id = $2 AND to_revision IS NOT NULL
               AND to_revision <= $3",
            &[
                &event.event.tenant_id,
                &event.event.election_event_id,
                &second,
            ],
        )
        .await
        .unwrap()
        .get(0);
    let pruned = prune_snapshots(&mut client, event.event, window)
        .await
        .unwrap();
    assert_eq!(pruned.runs, 1);
    assert_eq!(
        pruned.figures, closed_before as u64,
        "every figure deleted is counted"
    );
    assert!(!is_kept(&mut client, &event, first).await);
    assert!(
        is_kept(&mut client, &event, second).await,
        "replaced a moment ago"
    );
    assert!(is_kept(&mut client, &event, third).await, "the shown run");
}

#[tokio::test]
async fn a_failing_pass_leaves_the_shown_run_shown() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = settings();
    let event = seed(&mut client).await;
    voter(&client, &event, "ana", event.madrid, "Europe", false).await;
    let PassOutcome::Completed {
        revision: shown, ..
    } = pass(&mut client, &event, &settings).await
    else {
        panic!();
    };
    // Every figure this event writes from now on is refused.
    let tenant = event.event.tenant_id;
    let name = format!("refuse_{}", tenant.simple());
    client
        .batch_execute(&format!(
            "CREATE FUNCTION public.{name}() RETURNS trigger AS $$
             BEGIN RAISE EXCEPTION 'refused for the test'; END; $$ LANGUAGE plpgsql;
             CREATE TRIGGER {name} BEFORE INSERT ON sequent_backend.monitoring_snapshot_figure
             FOR EACH ROW WHEN (NEW.tenant_id = '{tenant}')
             EXECUTE FUNCTION public.{name}();"
        ))
        .await
        .unwrap();
    vote(&client, &event, "ana").await;
    let failed = count_event(&mut client, event.event, &settings, 1, 1).await;
    client
        .batch_execute(&format!(
            "DROP TRIGGER {name} ON sequent_backend.monitoring_snapshot_figure;
             DROP FUNCTION public.{name}();"
        ))
        .await
        .unwrap();
    assert!(failed.is_err(), "the failure is the pass's error");
    assert_eq!(live(&mut client, &event).await, shown);
    let (status, error): (String, Option<String>) = client
        .query_one(
            "SELECT status, error FROM sequent_backend.monitoring_snapshot_run
             WHERE tenant_id = $1 AND election_event_id = $2 AND revision > $3",
            &[
                &event.event.tenant_id,
                &event.event.election_event_id,
                &shown,
            ],
        )
        .await
        .map(|row| (row.get(0), row.get(1)))
        .unwrap();
    assert_eq!(status, "FAILED");
    assert!(error.unwrap().contains("refused for the test"));
    assert!(matches!(
        pass(&mut client, &event, &settings).await,
        PassOutcome::Completed { .. }
    ));
}

#[tokio::test]
async fn a_pass_a_later_one_overtook_is_superseded() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = settings();
    let event = seed(&mut client).await;
    voter(&client, &event, "ana", event.madrid, "Europe", false).await;
    let slow = start_run(&mut client, event.event).await.unwrap();
    let PassOutcome::Completed {
        revision: later, ..
    } = pass(&mut client, &event, &settings).await
    else {
        panic!();
    };
    assert_eq!(
        count_run(&mut client, event.event, slow, &settings, 1, 1)
            .await
            .unwrap(),
        PassOutcome::Superseded { revision: slow }
    );
    assert_eq!(live(&mut client, &event).await, later);
}

#[tokio::test]
async fn regions_read_from_areas_scope_posts_and_sign_ins() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = presets::load("campus")
        .unwrap()
        .unwrap()
        .set
        .settings
        .clone()
        .unwrap();
    let event = seed(&mut client).await;
    let tenant = event.event.tenant_id;
    let election_event = event.event.election_event_id;
    let north = Uuid::new_v4();
    for (election, area, campus) in [
        (event.madrid, north, Some("North")),
        (event.tokyo, Uuid::new_v4(), None),
    ] {
        let contest = Uuid::new_v4();
        client
            .execute(
                "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
                 VALUES ($1, $2, $3, $4)",
                &[&contest, &tenant, &election_event, &election],
            )
            .await
            .unwrap();
        client
            .execute(
                "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name, annotations)
                 VALUES ($1, $2, $3, 'area', $4)",
                &[&area, &tenant, &election_event, &campus.map(|campus| json!({"campus": campus}))],
            )
            .await
            .unwrap();
        client
            .execute(
                "INSERT INTO sequent_backend.area_contest
                     (id, tenant_id, election_event_id, area_id, contest_id)
                 VALUES ($1, $2, $3, $4, $5)",
                &[&Uuid::new_v4(), &tenant, &election_event, &area, &contest],
            )
            .await
            .unwrap();
    }
    // A voter whose own record names a campus the areas do not.
    voter(&client, &event, "ana", event.tokyo, "South", false).await;
    client
        .execute(
            "INSERT INTO sequent_backend.monitoring_login_counter
                 (tenant_id, election_event_id, bucket_start, event_type, registration, area_id,
                  attempts)
             VALUES ($1, $2, date_bin('15 minutes', now(), '2000-01-01'), 'LOGIN', 'REGISTERED',
                     $3, 4)",
            &[&tenant, &election_event, &north],
        )
        .await
        .unwrap();
    let PassOutcome::Completed { revision, .. } = pass(&mut client, &event, &settings).await else {
        panic!();
    };
    let set = full_set(&mut client, &event).await;
    let tx = client.transaction().await.unwrap();
    let catalogue = scope_catalogue(&tx, event.event, revision, &set)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(catalogue.regions, vec!["North", "South"]);
    let north_polls = payload(
        read(
            &mut client,
            &event,
            revision,
            DataSourceId::PollStatus,
            &set,
            "region=North",
        )
        .await,
    );
    assert_eq!(
        north_polls.totals[&Measure::Posts],
        1,
        "Madrid, by its area"
    );
    let south_polls = payload(
        read(
            &mut client,
            &event,
            revision,
            DataSourceId::PollStatus,
            &set,
            "region=South",
        )
        .await,
    );
    assert_eq!(
        south_polls.totals[&Measure::Posts],
        1,
        "Tokyo, by its voter"
    );
    let sign_ins = payload(
        read(
            &mut client,
            &event,
            revision,
            DataSourceId::AccessSecurity,
            &set,
            "region=North",
        )
        .await,
    );
    assert_eq!(sign_ins.totals[&Measure::Logins], 4);
}

fn unchanged(shown: i64, recount: Recount) -> PassOutcome {
    PassOutcome::Unchanged {
        shown: Some(shown),
        recount,
    }
}

/// A pass under `settings_revision` and `config_generation` counts a new
/// run after `shown`, and the pass after it finds nothing moved.
async fn counted_again_under(
    client: &mut Client,
    event: &Event,
    settings: &Settings,
    settings_revision: i32,
    config_generation: i64,
    shown: i64,
    what: &str,
) -> i64 {
    let outcome = count_event(
        client,
        event.event,
        settings,
        settings_revision,
        config_generation,
    )
    .await
    .unwrap();
    let PassOutcome::Completed { revision, .. } = outcome else {
        panic!("{what}: counted again into a new run, not {outcome:?}");
    };
    assert!(revision > shown, "{what}");
    assert_eq!(
        count_event(
            client,
            event.event,
            settings,
            settings_revision,
            config_generation
        )
        .await
        .unwrap(),
        unchanged(revision, Recount::Skipped),
        "{what}: then nothing moved"
    );
    revision
}

async fn counted_again(
    client: &mut Client,
    event: &Event,
    settings: &Settings,
    shown: i64,
    what: &str,
) -> i64 {
    counted_again_under(client, event, settings, 1, 1, shown, what).await
}

/// Runs `statement` with the event's tenant and id as `$1` and `$2`.
async fn change(client: &Client, event: &Event, statement: &str) {
    client
        .execute(
            statement,
            &[&event.event.tenant_id, &event.event.election_event_id],
        )
        .await
        .unwrap();
}

/// Brings the voters' enrollment and first votes up to date, as a pass
/// does before it counts.
async fn project_activity(client: &mut Client, event: &Event) {
    let tx = client.transaction().await.unwrap();
    refresh_voter_activity(&tx, event.event).await.unwrap();
    tx.commit().await.unwrap();
}

/// Rewrites what the run at `revision` records it was counted from.
async fn edit_inputs(client: &Client, event: &Event, revision: i64, inputs: &str) {
    client
        .execute(
            &format!(
                "UPDATE sequent_backend.monitoring_snapshot_run SET counted_inputs = {inputs}
                 WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3"
            ),
            &[
                &event.event.tenant_id,
                &event.event.election_event_id,
                &revision,
            ],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn a_pass_counts_again_whenever_what_it_counts_from_moved() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = settings();
    let event = seed(&mut client).await;
    let area = Uuid::new_v4();
    change(
        &client,
        &event,
        &format!(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             VALUES ('{area}', $1, $2, 'area')"
        ),
    )
    .await;
    voter(&client, &event, "ana", event.madrid, "Europe", false).await;
    voter(&client, &event, "ben", event.tokyo, "Asia", false).await;
    let PassOutcome::Completed { revision, .. } = pass(&mut client, &event, &settings).await else {
        panic!();
    };
    assert_eq!(
        pass(&mut client, &event, &settings).await,
        unchanged(revision, Recount::Skipped)
    );
    let mut shown = revision;

    change(
        &client,
        &event,
        &format!(
            "INSERT INTO sequent_backend.cast_vote
                 (tenant_id, election_event_id, election_id, area_id, voter_id_string, status,
                  created_at)
             VALUES ($1, $2, '{}', '{area}', 'ana', 'valid', now())",
            event.madrid
        ),
    )
    .await;
    project_activity(&mut client, &event).await;
    shown = counted_again(&mut client, &event, &settings, shown, "a vote").await;

    change(
        &client,
        &event,
        &format!(
            "INSERT INTO sequent_backend.applications
                 (tenant_id, election_event_id, area_id, applicant_id, status,
                  verification_type, applicant_data, annotations, created_at, updated_at)
             VALUES ($1, $2, '{area}', 'ben', 'PENDING', 'MANUAL', '{{}}', '{{}}', now(),
                     now())"
        ),
    )
    .await;
    project_activity(&mut client, &event).await;
    shown = counted_again(&mut client, &event, &settings, shown, "an application").await;

    change(
        &client,
        &event,
        "INSERT INTO sequent_backend.monitoring_login_counter
             (tenant_id, election_event_id, bucket_start, event_type, registration, attempts)
         VALUES ($1, $2, date_bin('15 minutes', now(), '2000-01-01'), 'LOGIN', 'REGISTERED', 1)",
    )
    .await;
    shown = counted_again(&mut client, &event, &settings, shown, "a sign-in").await;
    change(
        &client,
        &event,
        "UPDATE sequent_backend.monitoring_login_counter SET attempts = attempts + 1
         WHERE tenant_id = $1 AND election_event_id = $2",
    )
    .await;
    shown = counted_again(&mut client, &event, &settings, shown, "another sign-in").await;

    let ceremony = Uuid::new_v4();
    change(
        &client,
        &event,
        &format!(
            "INSERT INTO sequent_backend.keys_ceremony
                 (id, tenant_id, election_event_id, trustee_ids, threshold)
             VALUES ('{ceremony}', $1, $2, '{{}}', 1)"
        ),
    )
    .await;
    change(
        &client,
        &event,
        &format!(
            "INSERT INTO sequent_backend.tally_session
                 (id, tenant_id, election_event_id, keys_ceremony_id, threshold, election_ids,
                  execution_status, is_execution_completed)
             VALUES ('{}', $1, $2, '{ceremony}', 1, ARRAY['{}'::uuid], 'SUCCESS', true)",
            Uuid::new_v4(),
            event.madrid
        ),
    )
    .await;
    shown = counted_again(&mut client, &event, &settings, shown, "a tally").await;

    change(
        &client,
        &event,
        &format!(
            "UPDATE sequent_backend.election SET status = '{{\"voting_status\": \"CLOSED\"}}'
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = '{}'",
            event.madrid
        ),
    )
    .await;
    shown = counted_again(&mut client, &event, &settings, shown, "a poll closing").await;

    shown = counted_again_under(&mut client, &event, &settings, 2, 1, shown, "new settings").await;
    shown = counted_again_under(
        &mut client,
        &event,
        &settings,
        2,
        2,
        shown,
        "a new configuration",
    )
    .await;

    voter(&client, &event, "cy", event.tokyo, "Asia", false).await;
    shown = counted_again_under(&mut client, &event, &settings, 2, 2, shown, "a new voter").await;
    change(
        &client,
        &event,
        "DELETE FROM sequent_backend.monitoring_voter
         WHERE tenant_id = $1 AND election_event_id = $2 AND voter_id = 'cy'",
    )
    .await;
    shown = counted_again_under(
        &mut client,
        &event,
        &settings,
        2,
        2,
        shown,
        "a voter removed",
    )
    .await;

    // Credentials issued: Voting credentials counts them.
    change(
        &client,
        &event,
        "UPDATE sequent_backend.monitoring_voter SET credentials_at = now()
         WHERE tenant_id = $1 AND election_event_id = $2",
    )
    .await;
    shown = counted_again_under(
        &mut client,
        &event,
        &settings,
        2,
        2,
        shown,
        "credentials issued",
    )
    .await;

    // Moved, but not in anything a figure shows.
    change(
        &client,
        &event,
        "UPDATE sequent_backend.monitoring_voter SET attributes_hash = 'moved'
         WHERE tenant_id = $1 AND election_event_id = $2",
    )
    .await;
    for recount in [Recount::Done, Recount::Skipped] {
        assert_eq!(
            count_event(&mut client, event.event, &settings, 2, 2)
                .await
                .unwrap(),
            unchanged(shown, recount),
            "the same figures keep the shown run"
        );
    }
}

#[tokio::test]
async fn a_new_day_or_a_count_long_ago_is_counted_again() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = settings();
    let event = seed(&mut client).await;
    voter(&client, &event, "ana", event.madrid, "Europe", false).await;
    let PassOutcome::Completed { revision, .. } = pass(&mut client, &event, &settings).await else {
        panic!();
    };

    // Counted yesterday: ages are counted on the day of the pass.
    edit_inputs(
        &client,
        &event,
        revision,
        "jsonb_set(counted_inputs, '{day}',
                   to_jsonb(to_char((now() AT TIME ZONE 'UTC') - interval '1 day', 'YYYY-MM-DD')))",
    )
    .await;
    for recount in [Recount::Done, Recount::Skipped] {
        assert_eq!(
            pass(&mut client, &event, &settings).await,
            unchanged(revision, recount)
        );
    }

    // Counted longer ago than a pass trusts what it counted from.
    edit_inputs(
        &client,
        &event,
        revision,
        &format!(
            "jsonb_set(counted_inputs, '{{counted_at}}',
                       to_jsonb(now() - make_interval(secs => {})))",
            RECOUNT_EVERY.num_seconds()
        ),
    )
    .await;
    for recount in [Recount::Done, Recount::Skipped] {
        assert_eq!(
            pass(&mut client, &event, &settings).await,
            unchanged(revision, recount)
        );
    }
}

#[tokio::test]
async fn a_run_recording_no_inputs_it_can_read_is_counted_again() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = settings();
    let event = seed(&mut client).await;
    voter(&client, &event, "ana", event.madrid, "Europe", false).await;
    let PassOutcome::Completed { revision, .. } = pass(&mut client, &event, &settings).await else {
        panic!();
    };
    // As a run completed before inputs were recorded, then as one whose
    // inputs a later reader records differently.
    for inputs in ["NULL", "'{\"version\": \"0\"}'::jsonb"] {
        edit_inputs(&client, &event, revision, inputs).await;
        for recount in [Recount::Done, Recount::Skipped] {
            assert_eq!(
                pass(&mut client, &event, &settings).await,
                unchanged(revision, recount),
                "{inputs}"
            );
        }
    }
}

/// Beat sends passes, which prune as they go, only for events on configured
/// dashboards. An event switched back to the legacy dashboard keeps the
/// runs it had, so beat lists it for pruning until only its live run is left.
#[tokio::test]
async fn an_event_off_configured_dashboards_is_pruned_until_only_its_live_run_is_left() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = settings();
    // Seeded on the legacy dashboard, the default.
    let event = seed(&mut client).await;
    voter(&client, &event, "ana", event.madrid, "Europe", false).await;
    let PassOutcome::Completed { .. } = pass(&mut client, &event, &settings).await else {
        panic!();
    };
    assert!(
        !events_to_prune(&client)
            .await
            .unwrap()
            .contains(&event.event),
        "its only run is the one shown"
    );
    vote(&client, &event, "ana").await;
    let PassOutcome::Completed { .. } = pass(&mut client, &event, &settings).await else {
        panic!();
    };
    assert!(events_to_prune(&client)
        .await
        .unwrap()
        .contains(&event.event));
    prune_snapshots(&mut client, event.event, Duration::zero())
        .await
        .unwrap();
    assert!(!events_to_prune(&client)
        .await
        .unwrap()
        .contains(&event.event));
}

/// A voter who votes without pre-enrolling is one who voted but not a
/// pre-enrolled voter who voted, so "Voted of pre-enrolled" stays within
/// the pre-enrolled.
#[tokio::test]
async fn a_voter_who_votes_without_pre_enrolling_counts_as_voted_only() {
    let pool = schema::pool().await;
    let mut client = client(&pool).await;
    let settings = settings();
    let event = seed(&mut client).await;
    for (voter, pre_enrolled, voted) in [
        ("ana", true, true),
        ("ben", false, true),
        ("cai", true, false),
    ] {
        client
            .execute(
                "INSERT INTO sequent_backend.monitoring_voter
                     (tenant_id, election_event_id, election_id, voter_id, region, country, dims,
                      pre_enrolled_at, first_voted_at, attributes_hash, settings_revision)
                 VALUES ($1, $2, $3, $4, 'Europe', 'Spain', '{\"sex\": \"F\"}',
                         CASE WHEN $5 THEN now() - interval '2 hours' END,
                         CASE WHEN $6 THEN now() - interval '1 hour' END, 'h', 1)",
                &[
                    &event.event.tenant_id,
                    &event.event.election_event_id,
                    &event.madrid,
                    &voter,
                    &pre_enrolled,
                    &voted,
                ],
            )
            .await
            .unwrap();
    }
    let PassOutcome::Completed { revision, .. } = pass(&mut client, &event, &settings).await else {
        panic!("the pass completes");
    };
    let set = full_set(&mut client, &event).await;
    for scope in ["event".to_string(), format!("post={}", event.madrid)] {
        let turnout = payload(
            read(
                &mut client,
                &event,
                revision,
                DataSourceId::VoterTurnout,
                &set,
                &scope,
            )
            .await,
        );
        assert_eq!(turnout.totals[&Measure::PreEnrolled], 2, "{scope}");
        assert_eq!(turnout.totals[&Measure::Voted], 2, "{scope}");
        assert_eq!(
            turnout.totals[&Measure::VotedPreEnrolled],
            1,
            "{scope}: ana only"
        );
    }
}
