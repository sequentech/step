// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use chrono::TimeZone;
use indexmap::IndexMap;
use sequent_core::monitoring::compute::{evaluate, event_days};
use sequent_core::monitoring::config::{DynamicOptions, Widget};
use sequent_core::monitoring::presets;
use sequent_core::monitoring::resolve::{resolve_widget, DynamicOptionValues};
use serde_json::json;

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
        regions: region.map(str::to_string).into_iter().collect(),
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

fn facts(settings: &Settings, voters: Vec<VoterRow>) -> EventFacts {
    EventFacts {
        settings: settings.clone(),
        zone: Tz::UTC,
        posts: vec![
            post(1, "Madrid", Some("Europe"), PostState::Opened),
            post(2, "Rome", Some("Europe"), PostState::Closed),
            post(3, "Tokyo", Some("Asia"), PostState::NotInitialized),
        ],
        voters,
        area_elections: HashMap::new(),
        area_regions: HashMap::new(),
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
fn a_voter_who_votes_without_pre_enrolling_is_not_a_pre_enrolled_voter_who_voted() {
    let settings = settings();
    let mut ana = voter("ana", 1, "Europe", Some("Spain"));
    ana.pre_enrolled_at = Some(at(1, 9, 0));
    ana.first_voted_at = Some(at(1, 10, 0));
    let mut ben = voter("ben", 1, "Europe", Some("Spain"));
    ben.first_voted_at = Some(at(1, 11, 0));
    let mut cai = voter("cai", 1, "Europe", Some("Spain"));
    cai.pre_enrolled_at = Some(at(1, 9, 0));
    let facts = facts(&settings, vec![ana, ben, cai]);
    let figures = produce(&facts, &[set(&[1, 2, 3])]);
    let turnout = scopes(&figures, DataSourceId::VoterTurnout);

    for scope in [
        "event".to_string(),
        "region=Europe".to_string(),
        format!("post={}", id(1)),
    ] {
        let totals = &turnout[&scope].totals;
        assert_eq!(totals[&Measure::PreEnrolled], 2, "{scope}");
        assert_eq!(totals[&Measure::Voted], 2, "{scope}");
        assert_eq!(totals[&Measure::VotedPreEnrolled], 1, "{scope}: ana only");
    }
    let event = &turnout["event"];
    let madrid = event.groups["post"]
        .iter()
        .find(|row| row.key == id(1).to_string())
        .unwrap();
    assert_eq!(madrid.counts[&Measure::VotedPreEnrolled], 1);
    let cells = &event.cube.as_ref().unwrap().cells;
    assert_eq!(
        cells
            .iter()
            .map(|cell| cell.counts[&Measure::VotedPreEnrolled])
            .sum::<u64>(),
        1
    );
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

#[test]
fn a_post_is_open_while_any_of_its_channels_is() {
    let status = |value: serde_json::Value| voting_status(Some(&value));
    assert_eq!(
        status(json!({"voting_status": "CLOSED", "kiosk_voting_status": "OPEN"})),
        "OPEN"
    );
    assert_eq!(
        status(json!({"voting_status": "NOT_STARTED", "early_voting_status": "OPEN"})),
        "OPEN"
    );
    assert_eq!(
        status(json!({"voting_status": "CLOSED", "telephone_voting_status": "PAUSED"})),
        "PAUSED"
    );
    assert_eq!(
        status(json!({"voting_status": "NOT_STARTED", "kiosk_voting_status": "CLOSED"})),
        "CLOSED"
    );
    assert_eq!(
        status(json!({"voting_status": "NOT_STARTED", "kiosk_voting_status": "NOT_STARTED"})),
        "NOT_STARTED"
    );
    assert_eq!(voting_status(None), "NOT_STARTED");
}

#[test]
fn a_post_in_two_regions_is_counted_in_each() {
    let settings = settings();
    let mut facts = facts(&settings, Vec::new());
    facts.posts[0].regions = ["Europe", "Iberia"].map(String::from).into();
    let figures = produce(&facts, &[set(&[1, 2, 3])]);
    let poll = scopes(&figures, DataSourceId::PollStatus);
    assert_eq!(poll["region=Iberia"].totals[&Measure::Posts], 1);
    assert_eq!(poll["region=Europe"].totals[&Measure::Posts], 2);
    let regions = &poll["event"].groups["region"];
    assert_eq!(
        regions
            .iter()
            .map(|row| (row.key.as_str(), row.counts[&Measure::Posts]))
            .collect::<Vec<_>>(),
        vec![("Asia", 1), ("Europe", 2), ("Iberia", 1)],
        "Madrid in both of its regions, so the groups add up to more than the Posts"
    );
    assert_eq!(
        poll["region=Iberia"].groups["region"]
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        vec!["Iberia"],
        "a region's own group only"
    );
    assert_eq!(
        poll["region=Iberia"].posts[0].region.as_deref(),
        Some("Iberia")
    );
}

fn login(event_type: &str, registered: bool, area_id: Option<Uuid>, attempts: u64) -> LoginRow {
    LoginRow {
        bucket_start: at(1, 10, 15),
        event_type: event_type.into(),
        registered,
        area_id,
        attempts,
    }
}

#[test]
fn a_sign_in_is_in_its_area_s_region_when_regions_come_from_areas() {
    let settings = presets::load("campus")
        .unwrap()
        .unwrap()
        .set
        .settings
        .unwrap();
    assert!(settings.scope.region.area_annotation.is_some());
    let mut facts = facts(&settings, Vec::new());
    let (north, south) = (id(100), id(101));
    facts.posts[0].regions = ["North"].map(String::from).into();
    facts.posts[1].regions = ["North", "South"].map(String::from).into();
    facts.area_elections.insert(north, vec![id(1), id(2)]);
    facts.area_elections.insert(south, vec![id(2)]);
    facts.area_regions.insert(north, "North".into());
    facts.area_regions.insert(south, "South".into());
    facts.logins = vec![login("LOGIN", true, Some(north), 3)];
    let figures = produce(&facts, &[set(&[1, 2, 3])]);
    let access = scopes(&figures, DataSourceId::AccessSecurity);
    assert_eq!(access["region=North"].totals[&Measure::Logins], 3);
    assert!(
        !access.contains_key("region=South"),
        "Rome is also in the South, the voter's area is not"
    );
    assert_eq!(
        access["event"].groups["region"]
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        vec!["North"]
    );
}

#[test]
fn a_voter_s_sign_in_from_no_post_counts_for_the_full_set_only() {
    let settings = settings();
    let mut facts = facts(&settings, Vec::new());
    let nowhere = id(100);
    facts.area_elections.insert(nowhere, Vec::new());
    facts.logins = vec![
        login("LOGIN", true, None, 2),
        login("LOGIN", true, Some(nowhere), 4),
        login("LOGIN", true, Some(id(999)), 8),
    ];
    let full = produce(&facts, &[set(&[1, 2, 3])]);
    assert_eq!(
        scopes(&full, DataSourceId::AccessSecurity)["event"].totals[&Measure::Logins],
        14
    );
    let restricted = produce(&facts, &[set(&[3])]);
    assert_eq!(
        scopes(&restricted, DataSourceId::AccessSecurity)["event"].totals[&Measure::Logins],
        0,
        "a restricted set sees none of them"
    );
}

// -- every widget of every preset reads what the producers write -----------

/// Every set of selector values a viewer can pick, hidden selectors left out.
fn combinations(widget: &Widget, days: &[String]) -> Vec<IndexMap<String, String>> {
    let mut combinations = vec![IndexMap::new()];
    for (name, selector) in &widget.selectors {
        let options: Vec<String> = match selector.options_from {
            Some(DynamicOptions::EventDays) => days.to_vec(),
            None => selector.options.keys().cloned().collect(),
        };
        combinations = combinations
            .into_iter()
            .flat_map(|chosen: IndexMap<String, String>| {
                let shown = selector.when.as_ref().map_or(true, |when| {
                    chosen
                        .get(&when.selector)
                        .is_some_and(|value| when.one_of.contains(value))
                });
                if !shown || options.is_empty() {
                    return vec![chosen];
                }
                options
                    .iter()
                    .map(|option| {
                        let mut chosen = chosen.clone();
                        chosen.insert(name.clone(), option.clone());
                        chosen
                    })
                    .collect()
            })
            .collect();
    }
    combinations
}

/// A voter of `election` with a value of every dimension the settings name.
fn preset_voter(settings: &Settings, voter_id: &str, election: u128) -> VoterRow {
    let mut row = voter(voter_id, election, "Europe", Some("Spain"));
    row.dims = settings
        .dimensions
        .iter()
        .map(|(name, mapping)| {
            let value = mapping
                .age_bands
                .first()
                .map(|band| band.label.clone())
                .or_else(|| mapping.labels.keys().next().cloned())
                .unwrap_or_else(|| "some".to_string());
            (name.clone(), value)
        })
        .collect();
    row
}

/// Evaluates every query of every widget over the payload `payload_of`
/// gives its source; returns how many it evaluated.
fn evaluate_all(
    what: &str,
    settings: &Settings,
    widgets: &indexmap::IndexMap<String, Widget>,
    payload_of: &dyn Fn(DataSourceId) -> ScopePayload,
    failures: &mut Vec<String>,
) -> usize {
    let mut evaluated = 0;
    for (key, widget) in widgets {
        if widget.source.spec().producer != Producer::Available {
            continue;
        }
        let payload = payload_of(widget.source);
        let dynamic = DynamicOptionValues {
            event_days: event_days(&payload),
        };
        for requested in combinations(widget, &dynamic.event_days) {
            let resolved = match resolve_widget(widget, &IndexMap::new(), &requested, &dynamic) {
                Ok(resolved) => resolved,
                Err(report) => {
                    failures.push(format!("{what} {key} {requested:?}: resolve: {report}"));
                    continue;
                }
            };
            for (name, query) in &resolved.queries {
                match evaluate(widget.source, query, &payload, Some(settings)) {
                    Ok(_) => evaluated += 1,
                    Err(report) => {
                        failures.push(format!("{what} {key}.{name} {requested:?}: {report}"))
                    }
                }
            }
        }
    }
    evaluated
}

#[test]
fn every_query_of_every_preset_evaluates_over_what_the_producers_write() {
    let mut failures = Vec::new();
    let mut evaluated = 0;
    for shipped in presets::PRESETS {
        let preset = shipped.load().unwrap();
        let settings = preset.set.settings.clone().unwrap();
        let widgets = &preset.set.widgets;
        let preset_id = preset.manifest.id.clone();

        // No rejection and no sign-in: groups nobody is in are still counted.
        let mut ana = preset_voter(&settings, "ana", 1);
        ana.first_voted_at = Some(at(1, 10, 0));
        ana.enrollment = Some(Enrollment::Accepted);
        ana.enrollment_decided_at = Some(at(1, 9, 0));
        let mut ben = preset_voter(&settings, "ben", 3);
        ben.enrollment = Some(Enrollment::Pending);
        let counted = facts(&settings, vec![ana, ben]);
        let figures = produce(&counted, &[set(&[1, 2, 3])]);
        for scope in ["event".to_string(), format!("post={}", id(1))] {
            evaluated += evaluate_all(
                &format!("{preset_id} at {scope}"),
                &settings,
                widgets,
                &|source| {
                    scopes(&figures, source)
                        .get(&scope)
                        .cloned()
                        .unwrap_or_else(|| empty_payload(source, &settings))
                },
                &mut failures,
            );
        }

        // Nothing at all: what the reader builds for a scope nobody is in,
        // and the event scope of a set with nobody.
        evaluated += evaluate_all(
            &format!("{preset_id} empty"),
            &settings,
            widgets,
            &|source| empty_payload(source, &settings),
            &mut failures,
        );
        let mut nobody = facts(&settings, Vec::new());
        nobody.posts.clear();
        let figures = produce(&nobody, &[set(&[1, 2, 3])]);
        evaluated += evaluate_all(
            &format!("{preset_id} with nobody"),
            &settings,
            widgets,
            &|source| {
                scopes(&figures, source)
                    .get("event")
                    .cloned()
                    .unwrap_or_else(|| empty_payload(source, &settings))
            },
            &mut failures,
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(evaluated > 100, "only {evaluated} queries evaluated");
}

#[test]
fn an_empty_scope_has_every_group_and_the_cube() {
    let settings = settings();
    let turnout = empty_payload(DataSourceId::VoterTurnout, &settings);
    assert_eq!(
        turnout
            .groups
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["country", "post", "region"]
    );
    assert_eq!(
        turnout.cube.as_ref().unwrap().dimensions,
        vec!["sex", "age_band", "status"]
    );
    assert!(empty_payload(DataSourceId::EnrollmentDecisions, &settings)
        .groups
        .contains_key("reason"));
    assert_eq!(
        empty_payload(DataSourceId::AccessSecurity, &settings).notices,
        vec![Notice::UnregisteredAttemptsExcluded]
    );
}
