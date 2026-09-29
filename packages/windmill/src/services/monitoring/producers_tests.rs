// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use chrono::TimeZone;
use sequent_core::monitoring::presets;

fn settings() -> Settings {
    presets::load("comelec")
        .unwrap()
        .unwrap()
        .set
        .settings
        .clone()
        .unwrap()
}

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn post(n: u128, name: &str, region: Option<&str>, poll: PostState) -> Post {
    Post {
        id: id(n),
        name: name.to_string(),
        region: region.map(str::to_string),
        poll,
        counting: PostState::NotTallied,
    }
}

fn voter(voter_id: &str, election: u128, region: &str, country: Option<&str>) -> VoterRow {
    VoterRow {
        voter_id: voter_id.to_string(),
        election_id: id(election),
        region: Some(region.to_string()),
        country: country.map(str::to_string),
        dims: [("sex".to_string(), "F".to_string())].into(),
        pre_enrolled_at: None,
        first_voted_at: None,
        enrollment: None,
        enrollment_reason: None,
        enrollment_decided_at: None,
    }
}

fn set(elections: &[u128]) -> ElectionSet {
    ElectionSet {
        key: format!("set-{}", elections.len()),
        elections: elections.iter().map(|n| id(*n)).collect(),
    }
}

fn facts(settings: &Settings, voters: Vec<VoterRow>) -> EventFacts<'_> {
    EventFacts {
        settings,
        zone: Tz::UTC,
        posts: vec![
            post(1, "Madrid", Some("Europe"), PostState::Opened),
            post(2, "Rome", Some("Europe"), PostState::Closed),
            post(3, "Tokyo", Some("Asia"), PostState::NotInitialized),
        ],
        voters,
        area_elections: HashMap::new(),
        logins: Vec::new(),
    }
}

fn scopes(figures: &[SourceFigures], source: DataSourceId) -> &BTreeMap<String, ScopePayload> {
    &figures
        .iter()
        .find(|figure| figure.source == source)
        .unwrap()
        .scopes
}

fn at(day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, hour, minute, 0)
        .unwrap()
}

#[test]
fn a_voter_of_two_posts_is_one_voter_of_the_event_and_the_region() {
    let settings = settings();
    let mut madrid = voter("ana", 1, "Europe", Some("Spain"));
    madrid.first_voted_at = Some(at(1, 10, 0));
    let rome = voter("ana", 2, "Europe", Some("Spain"));
    let tokyo = voter("ben", 3, "Asia", None);
    let facts = facts(&settings, vec![madrid, rome, tokyo]);
    let figures = produce(&facts, &[set(&[1, 2, 3])]);
    let turnout = scopes(&figures, DataSourceId::VoterTurnout);

    let event = &turnout["event"];
    assert_eq!(event.totals[&Measure::Registered], 2);
    assert_eq!(event.totals[&Measure::Voted], 1);
    let europe = &turnout["region=Europe"];
    assert_eq!(europe.totals[&Measure::Registered], 1, "once in the region");
    assert_eq!(europe.totals[&Measure::Voted], 1);
    let posts = &europe.groups["post"];
    assert_eq!(posts.len(), 2);
    let rome = posts
        .iter()
        .find(|row| row.key == id(2).to_string())
        .unwrap();
    assert_eq!(rome.label.as_deref(), Some("Rome"));
    assert_eq!(
        rome.counts[&Measure::Voted],
        0,
        "she voted in Madrid, not in Rome"
    );
    assert_eq!(
        turnout[&format!("post={}", id(2))].totals[&Measure::Voted],
        0
    );
    assert_eq!(
        turnout[&format!("region=Europe&country=Spain")].totals[&Measure::Registered],
        1
    );
    let countries = &event.groups["country"];
    assert_eq!(
        countries
            .iter()
            .map(|row| (row.key.as_str(), row.counts[&Measure::Registered]))
            .collect::<Vec<_>>(),
        vec![("Spain", 1), (UNKNOWN_KEY, 1)],
    );
    let cube = event.cube.as_ref().unwrap();
    assert_eq!(
        cube.dimensions,
        vec!["sex", "age_band", "status"],
        "in the settings' order"
    );
    assert_eq!(
        cube.cells
            .iter()
            .map(|cell| cell.counts[&Measure::Registered])
            .sum::<u64>(),
        2,
        "each voter in one cell"
    );
    assert!(!turnout.contains_key("country=__unknown__"));
}

#[test]
fn a_restricted_set_counts_only_its_posts() {
    let settings = settings();
    let facts = facts(
        &settings,
        vec![
            voter("ana", 1, "Europe", None),
            voter("ben", 3, "Asia", None),
        ],
    );
    let figures = produce(&facts, &[set(&[3])]);
    let turnout = scopes(&figures, DataSourceId::VoterTurnout);
    assert_eq!(turnout["event"].totals[&Measure::Registered], 1);
    assert!(!turnout.contains_key("region=Europe"));
    let poll = scopes(&figures, DataSourceId::PollStatus);
    assert_eq!(
        poll["event"].totals[&Measure::Posts],
        1,
        "the Posts in scope"
    );
    assert_eq!(poll["event"].posts.len(), 1);
}

#[test]
fn milestones_count_the_posts_that_reached_them() {
    let settings = settings();
    let facts = facts(&settings, Vec::new());
    let figures = produce(&facts, &[set(&[1, 2, 3])]);
    let poll = scopes(&figures, DataSourceId::PollStatus);
    let event = &poll["event"];
    assert_eq!(event.totals[&Measure::Posts], 3);
    assert_eq!(
        event.totals[&Measure::Opened],
        2,
        "a closed Post was opened"
    );
    assert_eq!(event.totals[&Measure::Closed], 1);
    assert_eq!(event.totals[&Measure::Initialized], 2);
    assert_eq!(poll["region=Europe"].totals[&Measure::Posts], 2);
    assert_eq!(
        event
            .posts
            .iter()
            .map(|row| row.post.as_str())
            .collect::<Vec<_>>(),
        vec!["Madrid", "Rome", "Tokyo"]
    );
    assert_eq!(event.groups["region"].len(), 2);
    assert!(scopes(&figures, DataSourceId::TestVoting).is_empty());
    let test_voting = figures
        .iter()
        .find(|figure| figure.source == DataSourceId::TestVoting)
        .unwrap();
    assert_eq!(
        test_voting.status,
        SourceStatus::NotConnected(PendingProducer::TestElectionDesignation)
    );
}

#[test]
fn enrollment_counts_each_voter_latest_decision_and_its_reason() {
    let settings = settings();
    let mut rejected = voter("ana", 1, "Europe", None);
    rejected.enrollment = Some(Enrollment::Rejected);
    rejected.enrollment_reason = Some("blurry".into());
    rejected.enrollment_decided_at = Some(at(1, 9, 0));
    let mut rejected_too = rejected.clone();
    rejected_too.election_id = id(2);
    let mut accepted = voter("ben", 1, "Europe", None);
    accepted.enrollment = Some(Enrollment::Accepted);
    accepted.enrollment_decided_at = Some(at(1, 11, 0));
    accepted.first_voted_at = Some(at(1, 12, 30));
    let facts = facts(&settings, vec![rejected, rejected_too, accepted]);
    let figures = produce(&facts, &[set(&[1, 2, 3])]);
    let decisions = &scopes(&figures, DataSourceId::EnrollmentDecisions)["event"];
    assert_eq!(decisions.totals[&Measure::Applications], 2);
    assert_eq!(decisions.totals[&Measure::Disapproved], 1);
    assert_eq!(decisions.groups["reason"][0].key, "blurry");
    assert_eq!(
        decisions.groups["reason"][0].counts[&Measure::Disapproved],
        1
    );
    let activity = &scopes(&figures, DataSourceId::VotingEnrollmentActivity)["event"];
    assert_eq!(activity.totals[&Measure::Approved], 1);
    assert_eq!(
        activity
            .series
            .iter()
            .map(|bucket| bucket.start.as_str())
            .collect::<Vec<_>>(),
        vec!["2026-10-01T11:00:00", "2026-10-01T12:00:00"]
    );
    assert_eq!(activity.series[1].counts[&Measure::Voted], 1);
    assert_eq!(activity.series[1].counts[&Measure::Approved], 0);
}

fn hourly_votes(zone: Tz, votes: &[DateTime<Utc>]) -> ScopePayload {
    let settings = settings();
    let voters = votes
        .iter()
        .enumerate()
        .map(|(n, when)| {
            let mut row = voter(&format!("v{n:03}"), 1, "Europe", None);
            row.first_voted_at = Some(*when);
            row
        })
        .collect();
    let mut facts = facts(&settings, voters);
    facts.zone = zone;
    let figures = produce(&facts, &[set(&[1])]);
    scopes(&figures, DataSourceId::VoterTurnout)["event"].clone()
}

#[test]
fn hours_sum_to_the_totals_across_a_change_of_clocks() {
    // Clocks go back from 03:00 to 02:00 in Madrid on 25 October 2026.
    let votes = [
        at(24, 23, 30),
        at(25, 0, 10),
        at(25, 0, 50),
        at(25, 1, 20),
        at(25, 3, 5),
    ];
    let payload = hourly_votes(chrono_tz::Europe::Madrid, &votes);
    let hours: Vec<(&str, &str, u64)> = payload
        .series
        .iter()
        .map(|bucket| {
            (
                bucket.start.as_str(),
                bucket.utc_offset.as_str(),
                bucket.counts[&Measure::Voted],
            )
        })
        .collect();
    assert_eq!(
        hours,
        vec![
            ("2026-10-25T01:00:00", "+02:00", 1),
            ("2026-10-25T02:00:00", "+02:00", 2),
            ("2026-10-25T02:00:00", "+01:00", 1),
            ("2026-10-25T03:00:00", "+01:00", 0),
            ("2026-10-25T04:00:00", "+01:00", 1),
        ],
        "every hour once, the repeated hour twice, none missing"
    );
    assert_eq!(
        payload
            .series
            .iter()
            .map(|bucket| bucket.counts[&Measure::Voted])
            .sum::<u64>(),
        payload.totals[&Measure::Voted]
    );
}

#[test]
fn hours_start_on_the_local_hour_in_a_zone_off_the_utc_hour() {
    let zone: Tz = "Asia/Kolkata".parse().unwrap();
    let payload = hourly_votes(zone, &[at(1, 0, 29), at(1, 0, 31), at(1, 2, 45)]);
    assert_eq!(
        payload
            .series
            .iter()
            .map(|bucket| (bucket.start.as_str(), bucket.counts[&Measure::Voted]))
            .collect::<Vec<_>>(),
        vec![
            ("2026-10-01T05:00:00", 1),
            ("2026-10-01T06:00:00", 1),
            ("2026-10-01T07:00:00", 0),
            ("2026-10-01T08:00:00", 1),
        ]
    );
    let manila = hourly_votes(chrono_tz::Asia::Manila, &[at(1, 16, 0)]);
    assert_eq!(manila.series[0].day, "2026-10-02");
    assert_eq!(manila.series[0].utc_offset, "+08:00");
}

#[test]
fn sign_ins_of_unknown_users_count_for_the_whole_event_only() {
    let settings = settings();
    let mut facts = facts(&settings, Vec::new());
    let area = id(100);
    facts.area_elections.insert(area, vec![id(1), id(2)]);
    let login =
        |event_type: &str, registered: bool, area_id: Option<Uuid>, attempts: u64| LoginRow {
            bucket_start: at(1, 10, 15),
            event_type: event_type.into(),
            registered,
            area_id,
            attempts,
        };
    facts.logins = vec![
        login("LOGIN", true, Some(area), 3),
        login("LOGIN_ERROR", false, None, 5),
        login("CODE_TO_TOKEN", true, Some(area), 7),
    ];
    let figures = produce(&facts, &[set(&[1, 2, 3])]);
    let access = scopes(&figures, DataSourceId::AccessSecurity);
    let event = &access["event"];
    assert_eq!(event.totals[&Measure::Logins], 3);
    assert_eq!(event.totals[&Measure::LoginFailures], 5);
    assert_eq!(event.totals[&Measure::PasswordResets], 0);
    assert_eq!(
        event.notices,
        vec![Notice::UnregisteredAttemptsAtEventScopeOnly]
    );
    let europe = &access["region=Europe"];
    assert_eq!(
        europe.totals[&Measure::Logins],
        3,
        "once for the area, not per Post"
    );
    assert_eq!(europe.totals[&Measure::LoginFailures], 0);
    assert_eq!(europe.notices, vec![Notice::UnregisteredAttemptsExcluded]);
    assert_eq!(europe.groups["post"].len(), 2);
    assert_eq!(
        access[&format!("post={}", id(2))].totals[&Measure::Logins],
        3
    );
    assert!(!access.contains_key("region=Asia"));

    let restricted = produce(&facts, &[set(&[3])]);
    let access = scopes(&restricted, DataSourceId::AccessSecurity);
    assert_eq!(
        access["event"].totals[&Measure::Logins],
        0,
        "voters of Posts the set does not see"
    );
    assert_eq!(access["event"].totals[&Measure::LoginFailures], 5);
}

#[test]
fn a_poll_state_reads_the_status_and_the_initialization_report() {
    assert_eq!(poll_state(Some("CLOSED"), false), PostState::Closed);
    assert_eq!(poll_state(Some("PAUSED"), true), PostState::Paused);
    assert_eq!(poll_state(Some("OPEN"), false), PostState::Opened);
    assert_eq!(
        poll_state(Some("NOT_STARTED"), true),
        PostState::Initialized
    );
    assert_eq!(poll_state(None, false), PostState::NotInitialized);
}
