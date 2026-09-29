// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Snapshot passes: a pass shows what it counted, an unchanged pass writes
//! nothing, a change writes only the scopes it changed while earlier
//! revisions keep what they showed, and pruning keeps the shown run.

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
use windmill::services::monitoring::snapshot::{
    complete_snapshot, count_event, live_snapshot, prune_snapshots, read_scope,
    request_election_set, scope_catalogue, PassOutcome, ScopeRead,
};

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
            shown: Some(revision)
        }
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
