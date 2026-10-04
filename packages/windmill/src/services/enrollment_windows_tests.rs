// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::services::time_zones::{parse_local, parse_zone, resolve_local};
use sequent_core::ballot::{ElectionEventTimeZones, LogTimeZonePolicy};
use sequent_core::types::scheduled_event::CronConfig;
use serde_json::json;

/// One configuration: the event zones, its Posts (election id, zone, the
/// `embassy` option and the area description it matches) and the schedule.
struct Config {
    configured: Vec<&'static str>,
    primary: &'static str,
    logs: LogTimeZonePolicy,
    posts: Vec<Post>,
    /// Event-wide START_ENROLLMENT_PERIOD, wall time in the primary zone.
    event_start: Option<&'static str>,
    /// Event-wide END_ENROLLMENT_PERIOD, wall time in the primary zone.
    event_end: &'static str,
}

struct Post {
    election_id: &'static str,
    zone: Option<&'static str>,
    option: &'static str,
    area_description: &'static str,
    /// The Post's own START_ENROLLMENT_PERIOD, wall time in its zone.
    start: Option<&'static str>,
}

/// The COMELEC preset: Manila primary, two Posts in other zones, each opening
/// at midnight local time, one close for every Post at 18:00 PhST.
fn comelec() -> Config {
    Config {
        configured: vec!["Asia/Manila", "Asia/Dubai", "America/Toronto"],
        primary: "Asia/Manila",
        logs: LogTimeZonePolicy::PRIMARY,
        posts: vec![
            Post {
                election_id: "11111111-1111-4111-8111-111111111111",
                zone: Some("Asia/Dubai"),
                option: "Dubai PCG",
                area_description: "Dubai PCG (United Arab Emirates)",
                start: Some("2028-02-09T00:00"),
            },
            Post {
                election_id: "22222222-2222-4222-8222-222222222222",
                zone: Some("America/Toronto"),
                option: "Toronto PCG",
                area_description: "Toronto PCG",
                start: Some("2028-02-09T00:00"),
            },
        ],
        event_start: None,
        event_end: "2028-05-08T18:00",
    }
}

/// The Madrid association: Madrid primary; the Canary office opens on its own
/// schedule, the Madrid office follows the event-wide opening.
fn madrid() -> Config {
    Config {
        configured: vec!["Europe/Madrid", "Atlantic/Canary"],
        primary: "Europe/Madrid",
        logs: LogTimeZonePolicy::ELECTION,
        posts: vec![
            Post {
                election_id: "33333333-3333-4333-8333-333333333333",
                zone: None,
                option: "Madrid office",
                area_description: "Madrid office",
                start: None,
            },
            Post {
                election_id: "44444444-4444-4444-8444-444444444444",
                zone: Some("Atlantic/Canary"),
                option: "Canary office",
                area_description: "Canary office (Las Palmas)",
                start: Some("2028-03-01T09:00"),
            },
        ],
        event_start: Some("2028-03-01T09:00"),
        event_end: "2028-03-31T20:00",
    }
}

fn instant(local: &str, zone: &str) -> String {
    resolve_local(parse_local(local).unwrap(), parse_zone(zone).unwrap())
        .instant()
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn row(
    id: &str,
    processor: EventProcessors,
    election_id: Option<&str>,
    local: &str,
    zone: &str,
) -> ScheduledEvent {
    // The stored instant carries the zone's offset, as the scheduler saves it.
    let tz = parse_zone(zone).unwrap();
    let at = resolve_local(parse_local(local).unwrap(), tz)
        .instant()
        .with_timezone(&tz)
        .to_rfc3339();
    ScheduledEvent {
        id: id.to_string(),
        tenant_id: None,
        election_event_id: None,
        created_at: None,
        stopped_at: None,
        archived_at: None,
        labels: None,
        annotations: None,
        event_processor: Some(processor),
        cron_config: Some(CronConfig {
            cron: None,
            scheduled_date: Some(at),
            local: Some(local.to_string()),
            timezone: Some(zone.to_string()),
        }),
        event_payload: Some(json!({ "election_id": election_id })),
        task_id: None,
    }
}

impl Config {
    fn event(&self) -> ElectionEventPresentation {
        ElectionEventPresentation {
            timezones: Some(ElectionEventTimeZones {
                configured: self
                    .configured
                    .iter()
                    .map(|zone| zone.to_string())
                    .collect(),
                primary: self.primary.to_string(),
                logs: self.logs.clone(),
            }),
            ..Default::default()
        }
    }

    fn zone_of(&self, post: &Post) -> &'static str {
        post.zone.unwrap_or(self.primary)
    }

    fn elections(&self) -> Vec<PostElection> {
        self.posts
            .iter()
            .map(|post| PostElection {
                id: post.election_id.to_string(),
                presentation: Some(ElectionPresentation {
                    timezone: post.zone.map(str::to_string),
                    ..Default::default()
                }),
            })
            .collect()
    }

    fn schedule(&self) -> Vec<ScheduledEvent> {
        let mut rows = vec![row(
            "end",
            EventProcessors::END_ENROLLMENT_PERIOD,
            None,
            self.event_end,
            self.primary,
        )];
        if let Some(start) = self.event_start {
            rows.push(row(
                "start",
                EventProcessors::START_ENROLLMENT_PERIOD,
                None,
                start,
                self.primary,
            ));
        }
        for post in &self.posts {
            if let Some(start) = post.start {
                rows.push(row(
                    post.election_id,
                    EventProcessors::START_ENROLLMENT_PERIOD,
                    Some(post.election_id),
                    start,
                    self.zone_of(post),
                ));
            }
        }
        rows
    }

    fn options(&self) -> Vec<String> {
        self.posts
            .iter()
            .map(|post| post.option.to_string())
            .collect()
    }

    fn areas(&self) -> Vec<PostArea> {
        self.posts
            .iter()
            .map(|post| PostArea {
                name: format!("{} country", post.option),
                description: post.area_description.to_string(),
                election_id: post.election_id.to_string(),
            })
            .collect()
    }

    fn computed(&self) -> ComputedWindows {
        compute_windows(
            Some(&self.event()),
            &self.elections(),
            &self.schedule(),
            &self.options(),
            &self.areas(),
        )
    }

    fn windows(&self) -> BTreeMap<String, EnrollmentWindow> {
        let computed = self.computed();
        assert!(computed.problems.is_empty(), "{:?}", computed.problems);
        computed
            .entries
            .into_iter()
            .map(|(option, entry)| match entry {
                WindowEntry::Window(window) => (option, window),
                WindowEntry::Problem { problem } => panic!("{option}: {problem:?}"),
            })
            .collect()
    }

    /// What the attribute must hold, derived from the configuration.
    fn expected(&self) -> serde_json::Value {
        let mut expected = serde_json::Map::new();
        for post in &self.posts {
            let opens = post
                .start
                .map(|start| instant(start, self.zone_of(post)))
                .or_else(|| self.event_start.map(|start| instant(start, self.primary)));
            expected.insert(
                post.option.to_string(),
                json!({
                    "election_id": post.election_id,
                    "opens_at": opens,
                    "closes_at": instant(self.event_end, self.primary),
                    "time_zone": self.zone_of(post),
                    "close_time_zone": self.primary,
                }),
            );
        }
        serde_json::Value::Object(expected)
    }
}

#[test]
fn the_attribute_holds_each_post_window_in_its_zone() {
    for config in [comelec(), madrid()] {
        let windows = config.windows();
        assert_eq!(
            serde_json::to_value(&config.computed().entries).unwrap(),
            config.expected()
        );

        // Two Posts in different zones open at the same wall time but at
        // different instants, and close at the one event-wide instant.
        let zones: BTreeSet<&str> = windows.values().map(|w| w.time_zone.as_str()).collect();
        assert_eq!(zones.len(), 2);
        let closes: BTreeSet<_> = windows.values().map(|w| w.closes_at.clone()).collect();
        assert_eq!(closes.len(), 1);
    }
}

#[test]
fn comelec_posts_open_at_local_midnight() {
    let windows = comelec().windows();
    assert_eq!(
        windows["Dubai PCG"].opens_at.as_deref(),
        Some("2028-02-08T20:00:00Z")
    );
    // Toronto is on EST (UTC-5) on 9 February.
    assert_eq!(
        windows["Toronto PCG"].opens_at.as_deref(),
        Some("2028-02-09T05:00:00Z")
    );
    // 18:00 PhST (UTC+8).
    assert_eq!(
        windows["Dubai PCG"].closes_at.as_deref(),
        Some("2028-05-08T10:00:00Z")
    );
}

#[test]
fn a_post_without_its_own_start_uses_the_event_wide_one() {
    let windows = madrid().windows();
    let madrid_office = &windows["Madrid office"];
    assert_eq!(madrid_office.time_zone, "Europe/Madrid");
    // 09:00 CET (UTC+1) on 1 March.
    assert_eq!(
        madrid_office.opens_at.as_deref(),
        Some("2028-03-01T08:00:00Z")
    );
    // The Canary office's own start, 09:00 WET (UTC+0).
    assert_eq!(
        windows["Canary office"].opens_at.as_deref(),
        Some("2028-03-01T09:00:00Z")
    );
}

fn area(name: &str, description: &str, election_id: &str) -> PostArea {
    PostArea {
        name: name.to_string(),
        description: description.to_string(),
        election_id: election_id.to_string(),
    }
}

fn options(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn options_match_area_descriptions_ignoring_case() {
    let areas = vec![area("UAE", "Philippine Consulate General in Dubai", "e1")];
    assert_eq!(
        post_elections(&options(&["consulate general in DUBAI"]), &areas),
        BTreeMap::from([(
            "consulate general in DUBAI".to_string(),
            PostMatch::Election("e1".to_string())
        )])
    );
}

#[test]
fn an_exact_name_or_description_wins_over_substrings() {
    let areas = vec![
        area("Italy", "Rome PE", "e1"),
        area("Italy", "Rome PE annex", "e2"),
        area("Canary office", "Las Palmas", "e3"),
    ];
    let mapped = post_elections(&options(&["Rome PE", "canary OFFICE"]), &areas);
    assert_eq!(mapped["Rome PE"], PostMatch::Election("e1".to_string()));
    assert_eq!(
        mapped["canary OFFICE"],
        PostMatch::Election("e3".to_string())
    );
}

#[test]
fn an_ambiguous_option_or_one_without_a_post_is_a_problem() {
    let areas = vec![
        area("Italy", "Rome PE (Rome)", "e1"),
        area("Malta", "Rome PE (Valletta)", "e2"),
    ];
    let mapped = post_elections(&options(&["Rome PE", "Oslo PE"]), &areas);
    assert_eq!(
        mapped["Rome PE"],
        PostMatch::Problem(PostProblem {
            option: "Rome PE".to_string(),
            kind: PostProblemKind::AmbiguousPost,
            election_ids: vec!["e1".to_string(), "e2".to_string()],
        })
    );
    assert_eq!(
        mapped["Oslo PE"],
        PostMatch::Problem(PostProblem {
            option: "Oslo PE".to_string(),
            kind: PostProblemKind::NoPost,
            election_ids: vec![],
        })
    );
}

#[test]
fn problem_options_are_written_so_the_form_action_refuses_them() {
    for config in [comelec(), madrid()] {
        // The Post whose area description is exactly its option.
        let exact = config
            .posts
            .iter()
            .find(|post| post.area_description == post.option)
            .unwrap();
        let mut areas = config.areas();
        // A second election's area also contains that option.
        areas.push(area(
            "Other",
            &format!("{} (second)", exact.option),
            "55555555-5555-4555-8555-555555555555",
        ));
        let mut options = config.options();
        options.push("Unknown Post".to_string());
        let computed = compute_windows(
            Some(&config.event()),
            &config.elections(),
            &config.schedule(),
            &options,
            &areas,
        );
        // The exact description still decides that Post.
        assert!(matches!(
            computed.entries[exact.option],
            WindowEntry::Window(_)
        ));
        assert_eq!(
            serde_json::to_value(&computed.entries["Unknown Post"]).unwrap(),
            json!({"problem": "no-post"})
        );
        assert_eq!(computed.problems.len(), 1);
    }

    // Without an exact description both elections match: refused.
    let config = madrid();
    let areas = vec![
        area("ES", "Canary office (Las Palmas)", "e1"),
        area("ES", "Canary office (Tenerife)", "e2"),
    ];
    let computed = compute_windows(
        Some(&config.event()),
        &config.elections(),
        &config.schedule(),
        &options(&["Canary office"]),
        &areas,
    );
    assert_eq!(
        serde_json::to_value(&computed.entries).unwrap(),
        json!({"Canary office": {"problem": "ambiguous-post"}})
    );
    assert_eq!(computed.problems[0].kind, PostProblemKind::AmbiguousPost);
}

#[test]
fn without_an_enrollment_schedule_nothing_is_written_but_problems_are_reported() {
    let config = comelec();
    let computed = compute_windows(
        Some(&config.event()),
        &config.elections(),
        &[],
        &options(&["Dubai PCG", "Unknown Post"]),
        &config.areas(),
    );
    assert!(computed.entries.is_empty());
    assert_eq!(computed.problems.len(), 1);
}

#[test]
fn the_common_close_is_in_the_primary_zone() {
    for config in [comelec(), madrid()] {
        for window in config.windows().values() {
            assert_eq!(window.close_time_zone, config.primary);
        }
    }
}

#[test]
fn several_areas_of_one_post_map_to_its_election() {
    let areas = vec![
        area("UAE", "Dubai PCG", "e1"),
        area("Oman", "Dubai PCG (Oman)", "e1"),
    ];
    assert_eq!(
        post_elections(&options(&["Dubai PCG"]), &areas)["Dubai PCG"],
        PostMatch::Election("e1".to_string())
    );
}

#[test]
fn archived_rows_and_rows_without_an_instant_set_nothing() {
    let config = comelec();
    let mut schedule = config.schedule();
    for row in schedule.iter_mut() {
        if row.event_processor == Some(EventProcessors::END_ENROLLMENT_PERIOD) {
            row.archived_at = Some(Utc::now());
        } else if let Some(cron) = row.cron_config.as_mut() {
            cron.scheduled_date = Some("2028-02-09 00:00".to_string());
        }
    }
    let windows = election_windows(Some(&config.event()), &config.elections(), &schedule);
    assert!(windows.is_empty());
}

#[test]
fn the_newest_row_of_a_kind_wins() {
    let config = madrid();
    let mut older = row(
        "old-end",
        EventProcessors::END_ENROLLMENT_PERIOD,
        None,
        "2028-03-20T20:00",
        config.primary,
    );
    older.created_at = Some(
        DateTime::parse_from_rfc3339("2027-01-01T00:00:00Z")
            .unwrap()
            .into(),
    );
    let mut schedule = config.schedule();
    for row in schedule.iter_mut() {
        row.created_at = Some(
            DateTime::parse_from_rfc3339("2027-06-01T00:00:00Z")
                .unwrap()
                .into(),
        );
    }
    schedule.insert(0, older);
    let windows = election_windows(Some(&config.event()), &config.elections(), &schedule);
    assert_eq!(
        windows["33333333-3333-4333-8333-333333333333"].closes_at,
        Some(instant(config.event_end, config.primary))
    );
}

#[test]
fn the_post_options_come_from_the_embassy_attribute() {
    let attributes: Vec<UserProfileAttribute> = serde_json::from_value(json!([
        {"name": "country", "validations": {"options": {"options": ["UAE/Dubai PCG"]}}},
        {"name": "embassy", "validations": {"options": {"options": ["Dubai PCG", "Toronto PCG"]}}}
    ]))
    .unwrap();
    assert_eq!(post_options(&attributes), vec!["Dubai PCG", "Toronto PCG"]);
    assert!(post_options(&attributes[..1]).is_empty());
}

#[test]
fn synchronization_marker_is_nonempty_and_not_a_windows_object() {
    // Keycloak treats an absent/blank attribute as unrestricted, and a valid
    // object with missing Post keys permits those Posts. A non-object instead
    // sets Windows.unreadable, whose lookup refuses every registration.
    assert!(!ENROLLMENT_SYNC_PAUSED.trim().is_empty());
    let marker: serde_json::Value = serde_json::from_str(ENROLLMENT_SYNC_PAUSED).unwrap();
    assert!(marker.is_null());
    assert!(!marker.is_object());
}
