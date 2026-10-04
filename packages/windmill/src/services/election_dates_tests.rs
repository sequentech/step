// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::json;

const TENANT: &str = "tenant";
const EVENT: &str = "event";

fn utc(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

fn local(local: &str, zone: Option<&str>) -> ScheduleInput {
    ScheduleInput {
        local_date_time: Some(local.into()),
        time_zone: zone.map(str::to_owned),
        scheduled_date: None,
    }
}

fn resolved(input: &ScheduleInput, default_zone: &str) -> (CronConfig, Vec<ScheduleWarning>) {
    resolve_schedule(input, default_zone, Some("post"))
        .unwrap()
        .expect("a date")
}

fn refusal(result: Result<Option<(CronConfig, Vec<ScheduleWarning>)>>) -> String {
    let err = result.expect_err("refused");
    err.downcast_ref::<InvalidSchedule>()
        .expect("an InvalidSchedule")
        .to_string()
}

#[test]
fn a_wall_time_and_zone_store_the_instant_the_wall_time_and_the_zone() {
    for (wall, zone, instant) in [
        ("2028-04-09T00:00", "Asia/Dubai", "2028-04-08T20:00:00Z"),
        ("2028-04-09T00:00", "Asia/Kathmandu", "2028-04-08T18:15:00Z"),
        ("2028-04-09T00:00", "Asia/Tehran", "2028-04-08T20:30:00Z"),
        ("2028-05-08T19:00", "Asia/Manila", "2028-05-08T11:00:00Z"),
    ] {
        let (cron, warnings) = resolved(&local(wall, Some(zone)), "UTC");
        assert_eq!(cron.scheduled_date.as_deref(), Some(instant), "{zone}");
        assert_eq!(cron.local.as_deref(), Some(wall));
        assert_eq!(cron.timezone.as_deref(), Some(zone));
        assert_eq!(cron.cron, None);
        assert!(warnings.is_empty());
    }
}

#[test]
fn without_a_zone_the_rows_zone_applies_and_aliases_become_canonical() {
    let (cron, _) = resolved(&local("2028-04-09T00:00", None), "Europe/Madrid");
    assert_eq!(cron.timezone.as_deref(), Some("Europe/Madrid"));
    assert_eq!(cron.scheduled_date.as_deref(), Some("2028-04-08T22:00:00Z"));
    let (cron, _) = resolved(&local("2028-04-09T00:00", Some("Asia/Calcutta")), "UTC");
    assert_eq!(cron.timezone.as_deref(), Some("Asia/Kolkata"));
    assert_eq!(cron.scheduled_date.as_deref(), Some("2028-04-08T18:30:00Z"));
}

#[test]
fn a_dst_gap_saves_the_shifted_time_and_warns() {
    let (cron, warnings) = resolved(&local("2028-03-12T02:30", Some("America/Toronto")), "UTC");
    assert_eq!(cron.scheduled_date.as_deref(), Some("2028-03-12T07:30:00Z"));
    // Saved as the shifted time, so an export and re-import round-trip.
    assert_eq!(cron.local.as_deref(), Some("2028-03-12T03:30"));
    assert_eq!(warnings.len(), 1);
    let warning = &warnings[0];
    assert_eq!(warning.code, WARNING_DST_GAP);
    assert_eq!(warning.message_key, "timezones.gap");
    assert_eq!(warning.election_id.as_deref(), Some("post"));
    assert_eq!(warning.params["shifted_local"], json!("2028-03-12T03:30"));
    assert_eq!(warning.params["time_zone"], json!("America/Toronto"));
}

#[test]
fn a_dst_overlap_uses_the_first_occurrence_and_warns() {
    let (cron, warnings) = resolved(&local("2028-11-05T01:30", Some("America/Toronto")), "UTC");
    assert_eq!(cron.scheduled_date.as_deref(), Some("2028-11-05T05:30:00Z"));
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, WARNING_DST_OVERLAP);
    assert_eq!(warnings[0].message_key, "timezones.overlap");
    assert_eq!(
        warnings[0].params["second_instant"],
        json!("2028-11-05T06:30:00Z")
    );
}

#[test]
fn a_legacy_instant_needs_an_offset_and_keeps_its_wall_time_in_the_rows_zone() {
    for date in ["2028-04-09T00:00:00", "2028-04-09 00:00", "tomorrow"] {
        let input = ScheduleInput {
            scheduled_date: Some(date.into()),
            ..Default::default()
        };
        let message = refusal(resolve_schedule(&input, "Asia/Dubai", None));
        assert!(message.contains("offset"), "{message}");
    }
    let input = ScheduleInput {
        scheduled_date: Some("2028-04-09T00:00:00+04:00".into()),
        ..Default::default()
    };
    let (cron, warnings) = resolved(&input, "Asia/Dubai");
    assert_eq!(cron.scheduled_date.as_deref(), Some("2028-04-08T20:00:00Z"));
    assert_eq!(cron.local.as_deref(), Some("2028-04-09T00:00"));
    assert_eq!(cron.timezone.as_deref(), Some("Asia/Dubai"));
    assert!(warnings.is_empty());
}

#[test]
fn a_wall_time_wins_over_a_legacy_instant_and_bad_input_is_refused() {
    let input = ScheduleInput {
        local_date_time: Some("2028-04-09T00:00".into()),
        time_zone: Some("Asia/Dubai".into()),
        scheduled_date: Some("2000-01-01T00:00:00Z".into()),
    };
    assert_eq!(
        resolved(&input, "UTC").0.scheduled_date.as_deref(),
        Some("2028-04-08T20:00:00Z")
    );
    assert!(refusal(resolve_schedule(
        &local("2028-04-09T00:00", Some("Mars/Olympus")),
        "UTC",
        None
    ))
    .contains("Mars/Olympus"));
    assert!(refusal(resolve_schedule(
        &local("09/04/2028", Some("UTC")),
        "UTC",
        None
    ))
    .contains("09/04/2028"));
}

#[test]
fn no_date_means_the_row_is_removed() {
    assert_eq!(
        resolve_schedule(&ScheduleInput::default(), "UTC", None).unwrap(),
        None
    );
    let blank = ScheduleInput {
        local_date_time: Some(" ".into()),
        time_zone: Some("Asia/Dubai".into()),
        scheduled_date: Some(String::new()),
    };
    assert_eq!(resolve_schedule(&blank, "UTC", None).unwrap(), None);
}

/// One event's configuration: its primary zone, each Post's zone, and the
/// Posts each check warns about for an opening at 00:00 local on 9 April
/// and the common close at 19:00 primary time on 8 May.
struct Configuration {
    primary: &'static str,
    posts: Vec<(&'static str, &'static str)>,
    window_days: Vec<&'static str>,
    short_last_day: Vec<&'static str>,
}

/// The COMELEC preset (Manila primary) and the Madrid association.
fn configurations() -> Vec<Configuration> {
    vec![
        Configuration {
            primary: "Asia/Manila",
            posts: vec![
                ("manila", "Asia/Manila"),
                ("dubai", "Asia/Dubai"),
                // Closes at 01:00 on 8 May: 30 dates, one hour on the last.
                ("honolulu", "Pacific/Honolulu"),
                // Closes at 00:00 on 8 May: 29 dates.
                ("pago-pago", "Pacific/Pago_Pago"),
            ],
            window_days: vec!["pago-pago"],
            short_last_day: vec!["honolulu"],
        },
        Configuration {
            primary: "Europe/Madrid",
            posts: vec![
                ("madrid", "Europe/Madrid"),
                ("canarias", "Atlantic/Canary"),
                // Closes at 07:00 on 9 May: 31 dates, seven hours on the last.
                ("kiritimati", "Pacific/Kiritimati"),
            ],
            window_days: vec!["kiritimati"],
            short_last_day: vec!["kiritimati"],
        },
    ]
}

fn schedule(
    processor: EventProcessors,
    election: Option<&str>,
    instant: DateTime<Utc>,
) -> ScheduledEvent {
    serde_json::from_value(json!({
        "id": format!("{processor}-{election:?}"),
        "tenant_id": TENANT, "election_event_id": EVENT,
        "event_processor": processor.to_string(),
        "task_id": generate_manage_date_task_name(TENANT, EVENT, election, &processor),
        "event_payload": {"election_id": election},
        "cron_config": {"scheduled_date": instant_text(instant)},
    }))
    .unwrap()
}

fn at_local(wall: &str, zone: Tz) -> DateTime<Utc> {
    resolve_local(parse_local(wall).unwrap(), zone).instant()
}

#[test]
fn the_voting_window_counts_the_local_dates_with_voting_time() {
    let manila = parse_zone("Asia/Manila").unwrap();
    let close = at_local("2028-05-08T19:00", manila);
    for (zone, days) in [
        ("Asia/Manila", 30),
        ("Asia/Dubai", 30),
        // 01:00 on 8 May: the last day has one hour.
        ("Pacific/Honolulu", 30),
        // 00:00 on 8 May: the last day has none.
        ("Pacific/Pago_Pago", 29),
    ] {
        let zone = parse_zone(zone).unwrap();
        let start = at_local("2028-04-09T00:00", zone);
        assert_eq!(voting_window_days(zone, start, close), days, "{zone}");
    }
}

#[test]
fn every_post_opening_at_local_midnight_is_checked_against_the_event_wide_close() {
    for configuration in configurations() {
        let primary = parse_zone(configuration.primary).unwrap();
        let close = at_local("2028-05-08T19:00", primary);
        let posts: Vec<PostZone> = configuration
            .posts
            .iter()
            .map(|(id, zone)| PostZone {
                election_id: id.to_string(),
                zone: parse_zone(zone).unwrap(),
            })
            .collect();
        let mut events = vec![schedule(EventProcessors::END_VOTING_PERIOD, None, close)];
        events.extend(posts.iter().map(|post| {
            schedule(
                EventProcessors::START_VOTING_PERIOD,
                Some(&post.election_id),
                at_local("2028-04-09T00:00", post.zone),
            )
        }));
        let warnings = rule_warnings(TENANT, EVENT, &events, &posts);
        let warned = |code: &str| -> Vec<String> {
            warnings
                .iter()
                .filter(|warning| warning.code == code)
                .filter_map(|warning| warning.election_id.clone())
                .collect()
        };
        assert_eq!(
            warned(WARNING_VOTING_WINDOW_DAYS),
            configuration.window_days,
            "{}",
            configuration.primary
        );
        assert_eq!(
            warned(WARNING_SHORT_LAST_DAY),
            configuration.short_last_day,
            "{}",
            configuration.primary
        );
        assert_eq!(
            warnings.len(),
            configuration.window_days.len() + configuration.short_last_day.len()
        );
        for warning in &warnings {
            if warning.code == WARNING_VOTING_WINDOW_DAYS {
                assert_eq!(warning.message_key, "eventsScreen.warning.votingWindowDays");
                assert_eq!(warning.params["expected"], json!(VOTING_WINDOW_DAYS));
            } else {
                assert_eq!(warning.message_key, "eventsScreen.warning.shortLastDay");
            }
        }
    }
}

#[test]
fn a_close_at_or_before_the_opening_warns() {
    let zone = parse_zone("Europe/Madrid").unwrap();
    let post = PostZone {
        election_id: "madrid".into(),
        zone,
    };
    for close in ["2028-04-09T00:00", "2028-04-08T12:00"] {
        let events = vec![
            schedule(
                EventProcessors::START_VOTING_PERIOD,
                Some("madrid"),
                at_local("2028-04-09T00:00", zone),
            ),
            schedule(
                EventProcessors::END_VOTING_PERIOD,
                None,
                at_local(close, zone),
            ),
        ];
        let warnings = rule_warnings(TENANT, EVENT, &events, &[post.clone()]);
        assert_eq!(warnings.len(), 1, "{close}");
        assert_eq!(warnings[0].code, WARNING_CLOSE_BEFORE_OPEN);
        assert_eq!(
            warnings[0].message_key,
            "eventsScreen.warning.closeBeforeOpen"
        );
    }
}

#[test]
fn the_last_day_counts_the_hours_before_the_local_close() {
    let honolulu = parse_zone("Pacific/Honolulu").unwrap();
    let start = at_local("2028-04-09T00:00", honolulu);
    for (close, hours) in [
        ("2028-05-08T01:00", 1),
        ("2028-05-08T00:00", 24),
        ("2028-05-08T19:00", 19),
        // A one-day window counts from the opening.
        ("2028-04-09T10:00", 10),
    ] {
        assert_eq!(
            last_day_hours(honolulu, start, at_local(close, honolulu)),
            hours,
            "{close}"
        );
    }
}

#[test]
fn a_posts_own_close_wins_over_the_event_wide_close() {
    let dubai = parse_zone("Asia/Dubai").unwrap();
    let post = PostZone {
        election_id: "dubai".into(),
        zone: dubai,
    };
    let events = vec![
        schedule(
            EventProcessors::END_VOTING_PERIOD,
            None,
            at_local("2028-05-08T19:00", dubai),
        ),
        schedule(
            EventProcessors::START_VOTING_PERIOD,
            Some("dubai"),
            at_local("2028-04-09T00:00", dubai),
        ),
        schedule(
            EventProcessors::END_VOTING_PERIOD,
            Some("dubai"),
            at_local("2028-04-20T00:00", dubai),
        ),
    ];
    let warnings = rule_warnings(TENANT, EVENT, &events, &[post]);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].params["days"], json!(11));
    assert_eq!(warnings[0].params["end_local"], json!("2028-04-20T00:00"));
}

#[test]
fn final_testing_starts_at_least_seven_days_before_the_posts_opening() {
    for configuration in configurations() {
        let (id, zone) = configuration.posts[1];
        let zone = parse_zone(zone).unwrap();
        let post = PostZone {
            election_id: id.into(),
            zone,
        };
        let start = schedule(
            EventProcessors::START_VOTING_PERIOD,
            Some(id),
            at_local("2028-04-09T00:00", zone),
        );
        for (final_testing, warns) in [
            ("2028-04-02T00:00", false),
            ("2028-03-25T09:00", false),
            ("2028-04-02T00:01", true),
            ("2028-04-08T00:00", true),
        ] {
            let events = vec![
                start.clone(),
                schedule(
                    EventProcessors::START_FINAL_TESTING,
                    Some(id),
                    at_local(final_testing, zone),
                ),
            ];
            let warnings: Vec<ScheduleWarning> =
                rule_warnings(TENANT, EVENT, &events, &[post.clone()])
                    .into_iter()
                    .filter(|warning| warning.code == WARNING_FINAL_TESTING_LEAD_TIME)
                    .collect();
            assert_eq!(
                !warnings.is_empty(),
                warns,
                "{} {final_testing}",
                configuration.primary
            );
            if warns {
                assert_eq!(
                    warnings[0].message_key,
                    "eventsScreen.warning.finalTestingLeadTime"
                );
                assert_eq!(
                    warnings[0].params["final_testing_local"],
                    json!(final_testing)
                );
                assert_eq!(warnings[0].params["time_zone"], json!(zone.name()));
            }
        }
    }
}

#[test]
fn archived_rows_and_other_posts_raise_no_warnings() {
    let zone = parse_zone("Europe/Madrid").unwrap();
    let mut archived = schedule(
        EventProcessors::END_VOTING_PERIOD,
        Some("madrid"),
        at_local("2028-04-10T00:00", zone),
    );
    archived.archived_at = Some(utc("2028-01-01T00:00:00Z"));
    let events = vec![
        schedule(
            EventProcessors::START_VOTING_PERIOD,
            Some("madrid"),
            at_local("2028-04-09T00:00", zone),
        ),
        archived,
        schedule(
            EventProcessors::START_FINAL_TESTING,
            Some("canarias"),
            at_local("2028-04-08T00:00", zone),
        ),
    ];
    let post = PostZone {
        election_id: "madrid".into(),
        zone,
    };
    assert!(rule_warnings(TENANT, EVENT, &events, &[post]).is_empty());
}

#[test]
fn lifecycle_windows_are_election_scoped() {
    for processor in [
        EventProcessors::START_READINESS_TEST,
        EventProcessors::END_READINESS_TEST,
        EventProcessors::START_FINAL_TESTING,
        EventProcessors::END_FINAL_TESTING,
        EventProcessors::START_TEST_VOTING,
        EventProcessors::END_TEST_VOTING,
    ] {
        assert!(is_election_scoped_only(&processor));
    }
    assert!(!is_election_scoped_only(
        &EventProcessors::END_VOTING_PERIOD
    ));
    assert!(!is_election_scoped_only(
        &EventProcessors::START_ENROLLMENT_PERIOD
    ));
}

#[test]
fn the_dates_in_ballot_styles_use_the_event_wide_close_and_the_posts_own_opening() {
    let manila = parse_zone("Asia/Manila").unwrap();
    let dubai = parse_zone("Asia/Dubai").unwrap();
    let election: Election = serde_json::from_value(json!({
        "id": "dubai", "tenant_id": TENANT, "election_event_id": EVENT,
    }))
    .unwrap();
    let mut close = schedule(
        EventProcessors::END_VOTING_PERIOD,
        None,
        at_local("2028-05-08T19:00", manila),
    );
    close.cron_config.as_mut().unwrap().timezone = Some("Asia/Manila".into());
    let events = vec![
        close,
        schedule(
            EventProcessors::START_VOTING_PERIOD,
            Some("dubai"),
            at_local("2028-04-09T00:00", dubai),
        ),
        schedule(
            EventProcessors::START_VOTING_PERIOD,
            Some("other"),
            at_local("2028-04-01T00:00", dubai),
        ),
    ];
    let dates = get_election_dates(&election, events)
        .unwrap()
        .scheduled_event_dates
        .unwrap();
    assert_eq!(
        dates["START_VOTING_PERIOD"].scheduled_at.as_deref(),
        Some("2028-04-08T20:00:00Z")
    );
    assert_eq!(
        dates["END_VOTING_PERIOD"].scheduled_at.as_deref(),
        Some("2028-05-08T11:00:00Z")
    );
    assert_eq!(
        dates["END_VOTING_PERIOD"].timezone.as_deref(),
        Some("Asia/Manila")
    );
}

#[test]
fn display_deadline_override_uses_authority_and_known_null() {
    let mut dates = StringifiedPeriodDates::default();
    dates.scheduled_event_dates = Some(std::collections::HashMap::from([(
        EventProcessors::END_VOTING_PERIOD.to_string(),
        sequent_core::ballot::ScheduledEventDates {
            scheduled_at: Some("2030-01-01T09:00:00Z".into()),
            ..Default::default()
        },
    )]));
    let authoritative = DisplayVotingClose {
        scheduled_at: Some("2030-01-01T10:00:00Z".into()),
        timezone: Some("Asia/Manila".into()),
    };
    apply_display_voting_close(&mut dates, Some(&authoritative));
    let closing = &dates.scheduled_event_dates.as_ref().unwrap()
        [&EventProcessors::END_VOTING_PERIOD.to_string()];
    assert_eq!(closing.scheduled_at, authoritative.scheduled_at);
    assert_eq!(closing.timezone, authoritative.timezone);
    apply_display_voting_close(
        &mut dates,
        Some(&DisplayVotingClose {
            scheduled_at: None,
            timezone: None,
        }),
    );
    assert_eq!(
        dates.scheduled_event_dates.as_ref().unwrap()
            [&EventProcessors::END_VOTING_PERIOD.to_string()]
            .scheduled_at,
        None
    );
}
