// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! `monitoring_voter`: what a pass over the voters' accounts writes, what it
//! leaves alone and removes, and how each voter's enrollment and first vote
//! follow their applications and votes.

#[path = "support/schema.rs"]
mod schema;

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box_reads::{FirstValidVote, FirstValidVotes};
use sequent_core::monitoring::config::Settings;
use sequent_core::monitoring::presets;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Mutex;
use uuid::Uuid;
use windmill::postgres::application::update_application_status;
use windmill::postgres::monitoring_config::EventRef;
use windmill::services::application::{
    store_verified_application, ApplicationAnnotations, ApplicationVerificationResult,
};
use windmill::services::monitoring::projection::{
    event_first_day, full_pass_due, load_event_places, project_accounts, read_first_votes,
    refresh_voter_activity, FirstVoteSource, FirstVotes, LastPass, ProjectionContext, VoterAccount,
};
use windmill::services::time_zones::event_time_zone;
use windmill::types::application::{ApplicationRejectReason, ApplicationStatus, ApplicationType};

fn comelec_settings() -> Settings {
    presets::load("comelec")
        .unwrap()
        .unwrap()
        .set
        .settings
        .clone()
        .unwrap()
}

fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, day, hour, 0, 0).unwrap()
}

struct Seed<'t, 'c> {
    tx: &'t Transaction<'c>,
    event: EventRef,
}

impl<'t, 'c> Seed<'t, 'c> {
    async fn new(tx: &'t Transaction<'c>) -> Seed<'t, 'c> {
        let event = EventRef {
            tenant_id: Uuid::new_v4(),
            election_event_id: Uuid::new_v4(),
        };
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
        Seed { tx, event }
    }

    /// An election (a Post) of `region`, with unlimited revotes.
    async fn election(&self, region: &str) -> Uuid {
        let id = Uuid::new_v4();
        self.tx
            .execute(
                "INSERT INTO sequent_backend.election
                     (id, tenant_id, election_event_id, annotations, num_allowed_revotes)
                 VALUES ($1, $2, $3, $4, 0)",
                &[
                    &id,
                    &self.event.tenant_id,
                    &self.event.election_event_id,
                    &json!({"miru:geographical-region": region}),
                ],
            )
            .await
            .unwrap();
        id
    }

    /// An area that votes in `elections`.
    async fn area(&self, elections: &[Uuid]) -> Uuid {
        let area = Uuid::new_v4();
        self.tx
            .execute(
                "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
                 VALUES ($1, $2, $3, 'area')",
                &[&area, &self.event.tenant_id, &self.event.election_event_id],
            )
            .await
            .unwrap();
        for election in elections {
            let contest = Uuid::new_v4();
            self.tx
                .execute(
                    "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
                     VALUES ($1, $2, $3, $4)",
                    &[&contest, &self.event.tenant_id, &self.event.election_event_id, election],
                )
                .await
                .unwrap();
            self.tx
                .execute(
                    "INSERT INTO sequent_backend.area_contest
                         (id, tenant_id, election_event_id, area_id, contest_id)
                     VALUES ($1, $2, $3, $4, $5)",
                    &[
                        &Uuid::new_v4(),
                        &self.event.tenant_id,
                        &self.event.election_event_id,
                        &area,
                        &contest,
                    ],
                )
                .await
                .unwrap();
        }
        area
    }

    async fn application(
        &self,
        area: Uuid,
        voter: &str,
        status: &str,
        annotations: Value,
        when: DateTime<Utc>,
    ) {
        self.stored_application(area, voter, status, "MANUAL", json!({}), annotations, when)
            .await;
    }

    /// An application as a writer stores it; its id.
    #[allow(clippy::too_many_arguments)]
    async fn stored_application(
        &self,
        area: Uuid,
        applicant_id: &str,
        status: &str,
        verification_type: &str,
        applicant_data: Value,
        annotations: Value,
        when: DateTime<Utc>,
    ) -> Uuid {
        self.tx
            .query_one(
                "INSERT INTO sequent_backend.applications
                     (tenant_id, election_event_id, area_id, applicant_id, status,
                      verification_type, applicant_data, annotations, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
                 RETURNING id",
                &[
                    &self.event.tenant_id,
                    &self.event.election_event_id,
                    &area,
                    &applicant_id,
                    &status,
                    &verification_type,
                    &applicant_data,
                    &annotations,
                    &when,
                ],
            )
            .await
            .unwrap()
            .get(0)
    }

    /// Gives the application `id` the id `new`, to order applications
    /// updated at the same time.
    async fn renumber(&self, id: Uuid, new: &str) {
        self.tx
            .execute(
                "UPDATE sequent_backend.applications SET id = $2::text::uuid WHERE id = $1",
                &[&id, &new],
            )
            .await
            .unwrap();
    }

    /// Each voter's enrollment state and reason.
    async fn states(&self) -> BTreeMap<String, (Option<String>, Option<String>)> {
        self.rows()
            .await
            .iter()
            .map(|row| {
                (
                    row["voter_id"].as_str().unwrap().to_string(),
                    (
                        row["enrollment_state"].as_str().map(str::to_string),
                        row["enrollment_reason"].as_str().map(str::to_string),
                    ),
                )
            })
            .collect()
    }

    async fn rows(&self) -> Vec<Value> {
        self.tx
            .query(
                "SELECT to_jsonb(v) - 'tenant_id' - 'election_event_id' - 'updated_at'
                 FROM sequent_backend.monitoring_voter v
                 WHERE tenant_id = $1 AND election_event_id = $2
                 ORDER BY voter_id, election_id",
                &[&self.event.tenant_id, &self.event.election_event_id],
            )
            .await
            .unwrap()
            .iter()
            .map(|row| row.get(0))
            .collect()
    }

    async fn row(&self, election: Uuid, voter: &str) -> Value {
        self.rows()
            .await
            .into_iter()
            .find(|row| row["voter_id"] == voter && row["election_id"] == election.to_string())
            .unwrap_or_else(|| panic!("no row for {voter}"))
    }
}

/// A voter's first valid ballot in an election, as the ballot box reads it.
fn first_vote(election: Uuid, voter: &str, when: DateTime<Utc>) -> FirstValidVote {
    FirstValidVote {
        election_id: election.to_string(),
        voter_id: voter.to_string(),
        accepted_at: when.timestamp_micros(),
    }
}

fn account(id: &str, area: Option<Uuid>, attributes: &[(&str, &str)]) -> VoterAccount {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Some(area) = area {
        map.insert("area-id".into(), vec![area.to_string()]);
    }
    for (name, value) in attributes {
        map.entry(name.to_string())
            .or_default()
            .push(value.to_string());
    }
    VoterAccount {
        voter_id: id.to_string(),
        attributes: map,
        credentials_at: None,
    }
}

async fn pass(
    seed: &Seed<'_, '_>,
    settings: &Settings,
    settings_revision: i32,
    accounts: &[VoterAccount],
) -> (u64, u64) {
    let places = load_event_places(seed.tx, seed.event).await.unwrap();
    let context = ProjectionContext {
        event: seed.event,
        settings,
        settings_revision,
        places: &places,
        on: NaiveDate::from_ymd_opt(2026, 5, 11).unwrap(),
    };
    project_accounts(seed.tx, &context, accounts).await.unwrap()
}

#[tokio::test]
async fn a_voter_is_projected_into_each_election_their_area_votes_in() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let settings = comelec_settings();
    let madrid = seed.election("Europe").await;
    let manila = seed.election("Asia").await;
    let both = seed.area(&[madrid, manila]).await;
    let voter = account(
        "voter-1",
        Some(both),
        &[
            ("country", "Spain/Madrid"),
            ("sex", "F"),
            ("dateOfBirth", "1990-01-01"),
            ("landBasedOrSeafarer", "Seafarer"),
            ("sequent.read-only.id-card-number-validated", "VERIFIED"),
        ],
    );
    let nowhere = account("voter-2", None, &[("sex", "M")]);
    let unknown_area = account("voter-3", Some(Uuid::new_v4()), &[]);
    let bare = account("voter-4", Some(both), &[]);
    assert_eq!(
        pass(
            &seed,
            &settings,
            1,
            &[voter.clone(), nowhere, unknown_area, bare]
        )
        .await,
        (4, 0)
    );

    let row = seed.row(madrid, "voter-1").await;
    assert_eq!(row["region"], "Europe", "the region is the election's");
    assert_eq!(
        row["country"], "Spain",
        "the country part of Country/Embassy"
    );
    assert_eq!(
        row["dims"],
        json!({"age_band": "25-39", "sex": "F", "status": "Seafarer"}),
        "derived values, never the date of birth"
    );
    assert!(row["pre_enrolled_at"].is_string());
    assert_eq!(row["area_id"], both.to_string());
    assert_eq!(row["settings_revision"], 1);
    assert_eq!(seed.row(manila, "voter-1").await["region"], "Asia");

    let bare = seed.row(madrid, "voter-4").await;
    assert_eq!(bare["dims"], json!({}), "every dimension Unknown");
    assert_eq!(bare["country"], Value::Null);
    assert_eq!(bare["pre_enrolled_at"], Value::Null);
    assert_eq!(
        seed.rows().await.len(),
        4,
        "a voter with no area or an area of no election has no row"
    );
}

#[tokio::test]
async fn a_pass_rewrites_only_what_changed_and_removes_who_is_gone() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let settings = comelec_settings();
    let election = seed.election("Europe").await;
    let area = seed.area(&[election]).await;
    let verified = ("sequent.read-only.id-card-number-validated", "VERIFIED");
    let one = account("voter-1", Some(area), &[("sex", "F"), verified]);
    let two = account("voter-2", Some(area), &[("sex", "M")]);
    assert_eq!(
        pass(&seed, &settings, 1, &[one.clone(), two.clone()]).await,
        (2, 0)
    );
    let first = seed.row(election, "voter-1").await;

    assert_eq!(
        pass(&seed, &settings, 1, &[one.clone(), two.clone()]).await,
        (0, 0),
        "an unchanged voter is not rewritten"
    );

    let changed = account("voter-1", Some(area), &[("sex", "M"), verified]);
    assert_eq!(pass(&seed, &settings, 1, &[changed.clone()]).await, (1, 1));
    let row = seed.row(election, "voter-1").await;
    assert_eq!(row["dims"], json!({"sex": "M"}));
    assert_eq!(
        row["pre_enrolled_at"], first["pre_enrolled_at"],
        "pre-enrolled since it was first seen so"
    );
    assert_eq!(seed.rows().await.len(), 1, "a deleted account is removed");

    assert_eq!(
        pass(&seed, &settings, 2, &[changed.clone()]).await,
        (1, 0),
        "new settings rewrite every voter"
    );
    assert_eq!(seed.row(election, "voter-1").await["settings_revision"], 2);

    let unverified = account("voter-1", Some(area), &[("sex", "M")]);
    pass(&seed, &settings, 2, &[unverified]).await;
    assert_eq!(
        seed.row(election, "voter-1").await["pre_enrolled_at"],
        Value::Null
    );
}

#[tokio::test]
async fn enrollment_and_the_first_vote_follow_applications_and_votes() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let settings = comelec_settings();
    let election = seed.election("Europe").await;
    let other = seed.election("Asia").await;
    let area = seed.area(&[election, other]).await;
    let verified = ("sequent.read-only.id-card-number-validated", "VERIFIED");
    let accounts = [
        account("accepted", Some(area), &[verified]),
        account("rejected", Some(area), &[]),
        account("pending", Some(area), &[]),
        account("none", Some(area), &[]),
    ];
    pass(&seed, &settings, 1, &accounts).await;

    seed.application(
        area,
        "accepted",
        "REJECTED",
        json!({"rejection_reason": "blurry"}),
        at(1, 9),
    )
    .await;
    seed.application(area, "accepted", "ACCEPTED", json!({}), at(2, 9))
        .await;
    seed.application(
        area,
        "rejected",
        "REJECTED",
        json!({"rejection_reason": " no match "}),
        at(3, 9),
    )
    .await;
    seed.application(area, "pending", "PENDING", json!({}), at(3, 9))
        .await;

    let all = FirstVotes::from_read(&[first_vote(election, "accepted", at(5, 10))], true).unwrap();
    assert!(refresh_voter_activity(&tx, seed.event, &all).await.unwrap() > 0);
    let accepted = seed.row(election, "accepted").await;
    assert_eq!(
        accepted["enrollment_state"], "ACCEPTED",
        "the latest application"
    );
    assert_eq!(accepted["enrollment_reason"], Value::Null);
    let decided: DateTime<Utc> =
        serde_json::from_value(accepted["enrollment_decided_at"].clone()).unwrap();
    assert_eq!(decided, at(2, 9));
    let pre_enrolled: DateTime<Utc> =
        serde_json::from_value(accepted["pre_enrolled_at"].clone()).unwrap();
    assert_eq!(pre_enrolled, at(2, 9), "pre-enrolled from the approval on");
    let voted: DateTime<Utc> = serde_json::from_value(accepted["first_voted_at"].clone()).unwrap();
    assert_eq!(voted, at(5, 10), "the voter's first valid vote");
    assert_eq!(
        seed.row(other, "accepted").await["first_voted_at"],
        Value::Null,
        "a vote counts in its own election"
    );

    let rejected = seed.row(election, "rejected").await;
    assert_eq!(rejected["enrollment_state"], "REJECTED");
    assert_eq!(rejected["enrollment_reason"], "no match");
    assert_eq!(
        rejected["first_voted_at"],
        Value::Null,
        "a voter with no valid vote"
    );
    let pending = seed.row(election, "pending").await;
    assert_eq!(pending["enrollment_state"], "PENDING");
    assert_eq!(pending["enrollment_decided_at"], Value::Null);
    assert_eq!(
        seed.row(election, "none").await["enrollment_state"],
        Value::Null
    );

    assert_eq!(
        refresh_voter_activity(&tx, seed.event, &all).await.unwrap(),
        0,
        "nothing changed, nothing is written"
    );

    // A read of the ballots since: a later valid revote keeps the first
    // vote, and a voter it leaves out keeps theirs.
    let since = FirstVotes::from_read(
        &[
            first_vote(election, "accepted", at(6, 10)),
            first_vote(election, "pending", at(6, 11)),
        ],
        false,
    )
    .unwrap();
    assert_eq!(
        refresh_voter_activity(&tx, seed.event, &since)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        seed.row(election, "accepted").await["first_voted_at"],
        accepted["first_voted_at"]
    );
    let pending_voted: DateTime<Utc> =
        serde_json::from_value(seed.row(election, "pending").await["first_voted_at"].clone())
            .unwrap();
    assert_eq!(pending_voted, at(6, 11));
    // A read of every ballot without one the voter had (its ballot was
    // rejected since) takes their first vote away.
    assert_eq!(
        refresh_voter_activity(&tx, seed.event, &all).await.unwrap(),
        1
    );
    assert_eq!(
        seed.row(election, "pending").await["first_voted_at"],
        Value::Null
    );

    // A pass that rewrites the voter keeps what the activity refresh set.
    pass(&seed, &settings, 2, &accounts).await;
    let kept = seed.row(election, "accepted").await;
    assert_eq!(kept["enrollment_state"], "ACCEPTED");
    assert_eq!(kept["first_voted_at"], accepted["first_voted_at"]);
    assert_eq!(kept["pre_enrolled_at"], accepted["pre_enrolled_at"]);
}

#[tokio::test]
async fn ages_are_counted_as_of_the_event_first_day_in_its_time_zone() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let now = at(20, 0);
    let first_start = Utc.with_ymd_and_hms(2026, 5, 11, 16, 30, 0).unwrap();
    for (processor, date, archived) in [
        ("START_VOTING_PERIOD", "2026-05-11T16:30:00Z", false),
        ("START_VOTING_PERIOD", "2026-05-12T16:30:00Z", false),
        ("START_VOTING_PERIOD", "2026-05-01T00:00:00Z", true),
        ("START_ENROLLMENT_PERIOD", "2026-04-01T00:00:00Z", false),
    ] {
        tx.execute(
            "INSERT INTO sequent_backend.scheduled_event
                 (tenant_id, election_event_id, event_processor, cron_config, archived_at)
             VALUES ($1, $2, $3, $4, CASE WHEN $5 THEN now() END)",
            &[
                &seed.event.tenant_id,
                &seed.event.election_event_id,
                &processor,
                &json!({"scheduled_date": date}),
                &archived,
            ],
        )
        .await
        .unwrap();
    }
    // The two configurations, and an event that names no zone (UTC).
    for primary in [Some("Asia/Manila"), Some("Europe/Madrid"), None] {
        let timezones = primary.map(
            |primary| json!({"configured": [primary], "primary": primary, "logs": "election"}),
        );
        tx.execute(
            "UPDATE sequent_backend.election_event
             SET presentation = jsonb_set(COALESCE(presentation, '{}'::jsonb),
                                          '{timezones}', COALESCE($3::jsonb, 'null'))
             WHERE tenant_id = $1 AND id = $2",
            &[
                &seed.event.tenant_id,
                &seed.event.election_event_id,
                &timezones,
            ],
        )
        .await
        .unwrap();
        let zone = event_time_zone(&tx, seed.event.tenant_id, seed.event.election_event_id)
            .await
            .unwrap();
        let expected: Tz = primary.unwrap_or("UTC").parse().unwrap();
        assert_eq!(zone, expected);
        assert_eq!(
            event_first_day(&tx, seed.event, zone, now).await.unwrap(),
            first_start.with_timezone(&zone).date_naive(),
            "the first unarchived voting start, as a day in {zone}"
        );
    }
    tx.execute(
        "DELETE FROM sequent_backend.scheduled_event
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[&seed.event.tenant_id, &seed.event.election_event_id],
    )
    .await
    .unwrap();
    let manila = chrono_tz::Asia::Manila;
    assert_eq!(
        event_first_day(&tx, seed.event, manila, now).await.unwrap(),
        now.with_timezone(&manila).date_naive(),
        "no start scheduled: today, in the event's zone"
    );
}

#[test]
fn a_full_pass_is_due_first_on_new_settings_or_zone_and_then_every_interval() {
    let every = Duration::minutes(5);
    let now = at(11, 12);
    let (manila, madrid) = (chrono_tz::Asia::Manila, chrono_tz::Europe::Madrid);
    assert!(full_pass_due(None, 1, manila, now, every));
    let last = LastPass {
        at: now - Duration::minutes(4),
        settings_revision: 1,
        time_zone: Some(manila),
    };
    assert!(!full_pass_due(Some(last), 1, manila, now, every));
    assert!(full_pass_due(Some(last), 2, manila, now, every));
    assert!(
        full_pass_due(Some(last), 1, madrid, now, every),
        "the event's zone changed"
    );
    let unrecorded = LastPass {
        time_zone: None,
        ..last
    };
    assert!(full_pass_due(Some(unrecorded), 1, manila, now, every));
    assert!(full_pass_due(
        Some(last),
        1,
        manila,
        now + Duration::minutes(1),
        every
    ));
}

/// A ballot box that answers every read with the same votes, and keeps the
/// sequence number each read started after.
#[derive(Debug, Default)]
struct RecordingBallotBox {
    votes: Vec<FirstValidVote>,
    next_seq: i64,
    reads: Mutex<Vec<i64>>,
}

#[async_trait]
impl FirstVoteSource for RecordingBallotBox {
    async fn first_valid_votes(
        &self,
        _: EventRef,
        after_seq: i64,
    ) -> anyhow::Result<FirstValidVotes> {
        self.reads.lock().unwrap().push(after_seq);
        Ok(FirstValidVotes {
            votes: self.votes.clone(),
            next_seq: self.next_seq,
        })
    }
}

#[tokio::test]
async fn first_votes_are_read_since_the_last_read_and_in_full_on_a_full_pass() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let election = Uuid::new_v4();
    let ballot_box = RecordingBallotBox {
        votes: vec![first_vote(election, "voter", at(5, 10))],
        next_seq: 7,
        ..Default::default()
    };
    let votes = |all: bool| {
        let read = [((election, "voter".to_string()), at(5, 10))]
            .into_iter()
            .collect();
        if all {
            FirstVotes::All(read)
        } else {
            FirstVotes::Since(read)
        }
    };

    assert_eq!(
        read_first_votes(&tx, seed.event, &ballot_box, false)
            .await
            .unwrap(),
        votes(true),
        "nothing read yet: every ballot"
    );
    assert_eq!(
        read_first_votes(&tx, seed.event, &ballot_box, false)
            .await
            .unwrap(),
        votes(false),
        "then the ballots after the last read"
    );
    assert_eq!(
        read_first_votes(&tx, seed.event, &ballot_box, true)
            .await
            .unwrap(),
        votes(true),
        "a full pass reads every ballot again"
    );
    assert_eq!(*ballot_box.reads.lock().unwrap(), vec![0, 7, 0]);
}

#[tokio::test]
async fn private_participation_never_retains_a_vote_or_refresh_timestamp() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let election = seed.election("Europe").await;
    let area = seed.area(&[election]).await;
    pass(
        &seed,
        &comelec_settings(),
        1,
        &[account("private", Some(area), &[])],
    )
    .await;
    // Give the projection a distinct historical update timestamp, so a trigger
    // setting it to this transaction's now() cannot pass unnoticed.
    tx.batch_execute("ALTER TABLE sequent_backend.monitoring_voter DISABLE TRIGGER set_sequent_backend_monitoring_voter_updated_at").await.unwrap();
    tx.execute("UPDATE sequent_backend.monitoring_voter SET updated_at = '2020-01-01 00:00:00+00', first_voted_at = now() WHERE tenant_id=$1", &[&seed.event.tenant_id]).await.unwrap();
    tx.batch_execute("ALTER TABLE sequent_backend.monitoring_voter ENABLE TRIGGER set_sequent_backend_monitoring_voter_updated_at").await.unwrap();
    let before = seed.row(election, "private").await;
    let private = FirstVotes::Private([((election, "private".into()))].into());
    assert_eq!(
        refresh_voter_activity(&tx, seed.event, &private)
            .await
            .unwrap(),
        1
    );
    let after = seed.row(election, "private").await;
    assert_eq!(after["has_voted"], true);
    assert_eq!(after["first_voted_at"], Value::Null);
    assert_eq!(after["updated_at"], before["updated_at"]);
    assert_eq!(
        refresh_voter_activity(&tx, seed.event, &private)
            .await
            .unwrap(),
        0
    );
}

fn state(state: Option<&str>, reason: Option<&str>) -> (Option<String>, Option<String>) {
    (state.map(str::to_string), reason.map(str::to_string))
}

/// No new votes, for refreshes that only follow applications.
fn no_votes() -> FirstVotes {
    FirstVotes::Since(Default::default())
}

/// The approval matrix's recorded decision, as `verify_application` stores
/// it in the annotations.
fn matrix_decision(decision: &str, accepted_candidates: usize) -> Value {
    json!({"decision": decision, "candidates": 1, "accepted_candidates": accepted_candidates})
}

/// meta#14674: each voter's enrollment is the latest application stored
/// under their id, by update time and then id. Applications stored without
/// a voter id (`null` or blank, as the Keycloak enrollment flow stored
/// automatic decisions) count for no voter, even when their recorded
/// username is a voter's now: usernames can pass from voter to voter.
#[tokio::test]
async fn applications_count_for_the_voter_they_are_stored_under_and_never_by_username() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let other = Seed::new(&tx).await;
    let settings = comelec_settings();
    let election = seed.election("Europe").await;
    let area = seed.area(&[election]).await;
    let other_election = other.election("Europe").await;
    let other_area = other.area(&[other_election]).await;
    let voters = [
        "id-accepted",
        "id-email",
        "id-alice-now",
        "id-tie-later-id-accepts",
        "id-tie-later-id-rejects",
        "id-imported-email",
        "id-malformed",
        "id-not-an-object",
    ];
    let accounts: Vec<VoterAccount> = voters
        .iter()
        .map(|voter| account(voter, Some(area), &[]))
        .collect();
    pass(&seed, &settings, 1, &accounts).await;
    pass(
        &other,
        &settings,
        1,
        &[account("id-accepted", Some(other_area), &[])],
    )
    .await;

    let email_in_use =
        json!({"rejection_reason": "EMAIL_IN_USE", "decision": matrix_decision("ACCEPTED", 1)});
    let automatic = |voter: &'static str, status: &'static str, annotations: Value, when| {
        seed.stored_application(
            area,
            voter,
            status,
            "AUTOMATIC",
            json!({"username": "synthetic"}),
            annotations,
            when,
        )
    };
    automatic(
        "id-accepted",
        "ACCEPTED",
        json!({"decision": matrix_decision("ACCEPTED", 1)}),
        at(1, 9),
    )
    .await;
    automatic("id-email", "REJECTED", email_in_use.clone(), at(2, 9)).await;

    // Voter A enrolled automatically as `alice`, stored without a voter id.
    // A was removed and voter B, rejected by a reviewer, is `alice` now: A's
    // acceptance is not B's.
    seed.stored_application(
        area,
        "id-alice-now",
        "REJECTED",
        "MANUAL",
        json!({"username": "alice"}),
        json!({"rejection_reason": "blurry"}),
        at(1, 9),
    )
    .await;
    for (placeholder, status) in [
        ("null", "ACCEPTED"),
        ("", "ACCEPTED"),
        (" NULL ", "REJECTED"),
    ] {
        seed.stored_application(
            area,
            placeholder,
            status,
            "AUTOMATIC",
            json!({"username": "alice"}),
            json!({"decision": matrix_decision("ACCEPTED", 1)}),
            at(3, 9),
        )
        .await;
    }

    // Updated at the same time: the greater id decides.
    for (voter, first, second) in [
        (
            "id-tie-later-id-accepts",
            ("REJECTED", "1"),
            ("ACCEPTED", "2"),
        ),
        (
            "id-tie-later-id-rejects",
            ("REJECTED", "4"),
            ("ACCEPTED", "3"),
        ),
    ] {
        for (status, last) in [first, second] {
            let id = seed
                .stored_application(
                    area,
                    voter,
                    status,
                    "MANUAL",
                    json!({}),
                    json!({"rejection_reason": "blurry"}),
                    at(2, 9),
                )
                .await;
            seed.renumber(id, &format!("00000000-0000-4000-8000-00000000000{last}"))
                .await;
        }
    }

    // The CSV import stores annotations as a JSON string holding them.
    automatic(
        "id-imported-email",
        "REJECTED",
        Value::String(email_in_use.to_string()),
        at(2, 9),
    )
    .await;
    automatic(
        "id-malformed",
        "REJECTED",
        Value::String("{\"rejection_reason\": ".to_string()),
        at(2, 9),
    )
    .await;
    automatic(
        "id-not-an-object",
        "REJECTED",
        Value::String("\"EMAIL_IN_USE\"".to_string()),
        at(2, 9),
    )
    .await;

    // Another event's application under a voter id this event also has.
    other
        .stored_application(
            other_area,
            "id-email",
            "ACCEPTED",
            "AUTOMATIC",
            json!({}),
            json!({}),
            at(5, 9),
        )
        .await;

    assert!(
        refresh_voter_activity(&tx, seed.event, &no_votes())
            .await
            .unwrap()
            > 0
    );
    refresh_voter_activity(&tx, other.event, &no_votes())
        .await
        .unwrap();

    let accepted = state(Some("ACCEPTED"), None);
    let email = state(Some("REJECTED"), Some("EMAIL_IN_USE"));
    let blurry = state(Some("REJECTED"), Some("blurry"));
    let no_reason = state(Some("REJECTED"), None);
    assert_eq!(
        seed.states().await,
        BTreeMap::from([
            ("id-accepted".to_string(), accepted.clone()),
            ("id-email".to_string(), email.clone()),
            ("id-alice-now".to_string(), blurry.clone()),
            ("id-tie-later-id-accepts".to_string(), accepted.clone()),
            ("id-tie-later-id-rejects".to_string(), blurry.clone()),
            ("id-imported-email".to_string(), email.clone()),
            ("id-malformed".to_string(), no_reason.clone()),
            ("id-not-an-object".to_string(), no_reason.clone()),
        ])
    );
    let decided_at: DateTime<Utc> = serde_json::from_value(
        seed.row(election, "id-accepted").await["enrollment_decided_at"].clone(),
    )
    .unwrap();
    assert_eq!(decided_at, at(1, 9));
    assert_eq!(
        other.states().await,
        BTreeMap::from([("id-accepted".to_string(), state(None, None))]),
        "another event's applications decide nothing here, nor this one's there"
    );
    assert_eq!(
        refresh_voter_activity(&tx, seed.event, &no_votes())
            .await
            .unwrap(),
        0,
        "counted once: nothing changes on the next refresh"
    );
}

/// meta#14674: a reviewer rejects without naming a voter. The application
/// keeps the voter it is stored under, so the rejection counts for them;
/// one stored under no voter stays so. Naming a voter, as an approval does,
/// stores it under that voter.
#[tokio::test]
async fn a_manual_rejection_keeps_the_voter_its_application_is_stored_under() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let settings = comelec_settings();
    let election = seed.election("Europe").await;
    let area = seed.area(&[election]).await;
    pass(
        &seed,
        &settings,
        1,
        &[
            account("voter-1", Some(area), &[]),
            account("voter-2", Some(area), &[]),
        ],
    )
    .await;
    let stored = seed
        .stored_application(
            area,
            "voter-1",
            "ACCEPTED",
            "AUTOMATIC",
            json!({}),
            json!({}),
            at(1, 9),
        )
        .await;
    let unmatched = seed
        .stored_application(
            area,
            "null",
            "PENDING",
            "MANUAL",
            json!({"username": "voter-2-username"}),
            json!({}),
            at(1, 9),
        )
        .await;
    let update =
        |id: Uuid, applicant: &'static str, status: ApplicationStatus, reason: Option<&str>| {
            let tx = &tx;
            let seed = &seed;
            let reason = reason.map(str::to_string);
            async move {
                let application = update_application_status(
                    tx,
                    &id.to_string(),
                    &seed.event.tenant_id.to_string(),
                    &seed.event.election_event_id.to_string(),
                    applicant,
                    status,
                    ApplicationType::MANUAL,
                    reason,
                    None,
                    "Synthetic reviewer",
                    &vec![],
                )
                .await
                .unwrap();
                (application.applicant_id, application.status)
            }
        };

    assert_eq!(
        update(stored, "", ApplicationStatus::REJECTED, Some("blurry")).await,
        ("voter-1".to_string(), "REJECTED".to_string())
    );
    assert_eq!(
        update(unmatched, " ", ApplicationStatus::REJECTED, Some("blurry")).await,
        ("null".to_string(), "REJECTED".to_string()),
        "an application of no voter stays of none"
    );
    refresh_voter_activity(&tx, seed.event, &no_votes())
        .await
        .unwrap();
    assert_eq!(
        seed.states().await,
        BTreeMap::from([
            (
                "voter-1".to_string(),
                state(Some("REJECTED"), Some("blurry"))
            ),
            ("voter-2".to_string(), state(None, None)),
        ])
    );

    assert_eq!(
        update(unmatched, "voter-2", ApplicationStatus::ACCEPTED, None).await,
        ("voter-2".to_string(), "ACCEPTED".to_string()),
        "an approval stores the application under the voter it names"
    );
    refresh_voter_activity(&tx, seed.event, &no_votes())
        .await
        .unwrap();
    assert_eq!(
        seed.states().await["voter-2"],
        state(Some("ACCEPTED"), None)
    );
}

/// meta#14674: `verify_application` stores an automatic decision under the
/// voter it decided and returns the stored application's id; the monitoring
/// then counts it for that voter. A decision about no voter, or naming the
/// voter already enrolled, keeps the applicant id the enrollment flow sent.
#[tokio::test]
async fn a_verified_application_is_stored_under_the_voter_it_decided() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let seed = Seed::new(&tx).await;
    let settings = comelec_settings();
    let election = seed.election("Europe").await;
    let area = seed.area(&[election]).await;
    let voters = ["voter-accepted", "voter-email", "voter-enrolled"];
    let accounts: Vec<VoterAccount> = voters
        .iter()
        .map(|voter| account(voter, Some(area), &[]))
        .collect();
    pass(&seed, &settings, 1, &accounts).await;
    seed.stored_application(
        area,
        "voter-enrolled",
        "ACCEPTED",
        "MANUAL",
        json!({}),
        json!({}),
        at(1, 9),
    )
    .await;

    let result =
        |voter: Option<&str>, status: ApplicationStatus, reason| ApplicationVerificationResult {
            application_id: None,
            user_id: voter.map(str::to_string),
            username: voter
                .map(|voter| format!("username-of-{voter}"))
                .unwrap_or_default(),
            application_status: status,
            application_type: ApplicationType::AUTOMATIC,
            mismatches: Some(0),
            fields_match: None,
            attributes_unset: None,
            rejection_reason: reason,
            rejection_message: None,
            manual_verify_reason: None,
        };
    // As verify_application annotates it: with the result's rejection reason.
    let store = |result: ApplicationVerificationResult| {
        let tx = &tx;
        let seed = &seed;
        let annotations: ApplicationAnnotations = serde_json::from_value(json!({
            "unset-attributes": "",
            "rejection_reason": result.rejection_reason,
        }))
        .unwrap();
        async move {
            store_verified_application(
                tx,
                &seed.event.tenant_id.to_string(),
                &seed.event.election_event_id.to_string(),
                &Some(area),
                "null",
                &std::collections::HashMap::from([(
                    "firstName".to_string(),
                    "Synthetic".to_string(),
                )]),
                &None,
                &annotations,
                &result,
                &None,
            )
            .await
            .unwrap()
        }
    };
    let stored = |id: Uuid| {
        let tx = &tx;
        async move {
            let row = tx
                .query_one(
                    "SELECT applicant_id, applicant_data->>'username', status
                     FROM sequent_backend.applications WHERE id = $1",
                    &[&id],
                )
                .await
                .unwrap();
            (
                row.get::<_, String>(0),
                row.get::<_, String>(1),
                row.get::<_, String>(2),
            )
        }
    };

    let accepted = store(result(
        Some("voter-accepted"),
        ApplicationStatus::ACCEPTED,
        None,
    ))
    .await;
    let email = || {
        result(
            Some("voter-email"),
            ApplicationStatus::REJECTED,
            Some(ApplicationRejectReason::EMAIL_IN_USE),
        )
    };
    // Two in one transaction under the same voter: each returns its own id,
    // not the voter's latest by creation time.
    let first = store(email()).await;
    let second = store(email()).await;
    let duplicate = store(result(
        Some("voter-enrolled"),
        ApplicationStatus::REJECTED,
        Some(ApplicationRejectReason::ALREADY_APPROVED),
    ))
    .await;
    let no_voter = store(result(
        None,
        ApplicationStatus::REJECTED,
        Some(ApplicationRejectReason::NO_VOTER),
    ))
    .await;
    assert_ne!(first, second);
    assert_eq!(
        stored(accepted).await,
        (
            "voter-accepted".to_string(),
            "username-of-voter-accepted".to_string(),
            "ACCEPTED".to_string()
        )
    );
    assert_eq!(stored(first).await.0, "voter-email");
    assert_eq!(stored(second).await.0, "voter-email");
    assert_eq!(
        stored(duplicate).await.0,
        "null",
        "a duplicate attempt is not stored under the enrolled voter"
    );
    assert_eq!(
        stored(no_voter).await,
        ("null".to_string(), String::new(), "REJECTED".to_string())
    );

    refresh_voter_activity(&tx, seed.event, &no_votes())
        .await
        .unwrap();
    assert_eq!(
        seed.states().await,
        BTreeMap::from([
            ("voter-accepted".to_string(), state(Some("ACCEPTED"), None)),
            (
                "voter-email".to_string(),
                state(Some("REJECTED"), Some("EMAIL_IN_USE"))
            ),
            ("voter-enrolled".to_string(), state(Some("ACCEPTED"), None)),
        ])
    );
}
