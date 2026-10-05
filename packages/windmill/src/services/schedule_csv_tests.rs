// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use chrono::TimeZone as _;
use sequent_core::ballot::LogTimeZonePolicy;

const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
const EVENT: &str = "e0000000-0000-4000-8000-000000000000";

/// The Posts' zones of the overseas replay, cycled over the Posts: whole
/// hours, the fractional offsets (+05:30, +05:45, +03:30, +06:30), Cairo's
/// DST and both hemispheres.
const POST_ZONES: [&str; 20] = [
    "Asia/Dubai",
    "Asia/Kolkata",
    "Asia/Kathmandu",
    "Asia/Tehran",
    "Asia/Yangon",
    "Africa/Cairo",
    "Asia/Tokyo",
    "Europe/Rome",
    "America/Los_Angeles",
    "Africa/Nairobi",
    "Asia/Bangkok",
    "Asia/Shanghai",
    "Asia/Brunei",
    "Asia/Seoul",
    "Australia/Sydney",
    "America/Toronto",
    "Asia/Manila",
    "Asia/Riyadh",
    "Europe/London",
    "America/New_York",
];

/// An event configuration: its primary zone and its Posts' zones.
struct Config {
    primary: &'static str,
    post_zones: Vec<&'static str>,
    posts: usize,
}

/// Manila primary, 104 Posts around the world.
fn overseas() -> Config {
    Config {
        primary: "Asia/Manila",
        post_zones: POST_ZONES.to_vec(),
        posts: 104,
    }
}

/// Madrid primary, with an office in the Canary Islands.
fn association() -> Config {
    Config {
        primary: "Europe/Madrid",
        post_zones: vec!["Europe/Madrid", "Atlantic/Canary"],
        posts: 4,
    }
}

impl Config {
    fn event(&self) -> ScheduleEvent {
        let mut configured: Vec<String> = vec![self.primary.to_string()];
        for zone in &self.post_zones {
            if !configured.iter().any(|known| known == zone) {
                configured.push(zone.to_string());
            }
        }
        ScheduleEvent {
            tenant_id: TENANT.to_string(),
            election_event_id: EVENT.to_string(),
            time_zones: Some(ElectionEventTimeZones {
                configured,
                primary: self.primary.to_string(),
                logs: LogTimeZonePolicy::ELECTION,
            }),
            scheduled_events: vec![],
        }
    }

    fn post_zone(&self, index: usize) -> &'static str {
        self.post_zones[index % self.post_zones.len()]
    }

    fn elections(&self) -> Vec<ScheduleElection> {
        (0..self.posts)
            .map(|index| ScheduleElection {
                id: format!("00000000-0000-4000-8000-{index:012}"),
                alias: Some(alias(index)),
                name: Some(format!("Post {index:03}")),
                time_zone: Some(self.post_zone(index).to_string()),
            })
            .collect()
    }

    fn primary(&self) -> Tz {
        self.primary.parse().unwrap()
    }
}

fn alias(index: usize) -> String {
    format!("post-{index:03}")
}

fn election_id(index: usize) -> String {
    format!("00000000-0000-4000-8000-{index:012}")
}

const HEADER: &str = "election_alias,event_type,local_date_time,timezone,voting_channels";

/// Every Post opens enrollment and voting at midnight local time; the event
/// closes both at once, in the primary zone.
fn replay_csv(config: &Config) -> String {
    let mut lines = vec![HEADER.to_string()];
    for index in 0..config.posts {
        lines.push(format!(
            "{},START_ENROLLMENT_PERIOD,2028-02-09T00:00,,",
            alias(index)
        ));
    }
    for index in 0..config.posts {
        // A space, as spreadsheets write it.
        lines.push(format!(
            "{},START_VOTING_PERIOD,2028-04-09 00:00,,ONLINE|KIOSK",
            alias(index)
        ));
    }
    lines.push("ALL,END_ENROLLMENT_PERIOD,2028-03-09T00:00,,".to_string());
    lines.push("ALL,END_VOTING_PERIOD,2028-05-08T19:00,,ONLINE|KIOSK".to_string());
    lines.join("\n") + "\n"
}

fn parse(config: &Config, csv: &str) -> ParsedSchedule {
    parse_schedule(csv.as_bytes(), &config.event(), &config.elections()).unwrap()
}

fn naive(local: &str) -> NaiveDateTime {
    parse_local(local).unwrap()
}

/// The instant of a wall time, straight from the tz database.
fn expected_instant(local: &str, zone: Tz) -> DateTime<Utc> {
    zone.from_local_datetime(&naive(local))
        .single()
        .unwrap()
        .with_timezone(&Utc)
}

fn row<'a>(preview: &'a SchedulePreview, line: u64) -> &'a PreviewRow {
    preview.rows.iter().find(|row| row.row == line).unwrap()
}

#[test]
fn the_overseas_replay_previews_every_row_in_local_time_and_in_the_primary() {
    for config in [overseas(), association()] {
        let parsed = parse(&config, &replay_csv(&config));
        let preview = &parsed.preview;
        let rows = 2 * config.posts + 2;
        assert_eq!(preview.rows.len(), rows);
        assert_eq!(preview.ok, rows as u64);
        assert_eq!(preview.errors, 0);
        assert_eq!(preview.posts, config.posts as u64);
        assert_eq!(preview.primary_time_zone, config.primary);
        assert_eq!(parsed.entries.len(), rows);

        let primary = config.primary();
        for (offset, local) in [(0, "2028-02-09T00:00"), (config.posts, "2028-04-09T00:00")] {
            for index in 0..config.posts {
                let line = (offset + index + 2) as u64;
                let row = row(preview, line);
                let zone: Tz = config.post_zone(index).parse().unwrap();
                let instant = expected_instant(local, zone);
                assert_eq!(row.election_alias, alias(index));
                assert_eq!(
                    row.election_id.as_deref(),
                    Some(election_id(index).as_str())
                );
                assert_eq!(row.local, local, "row {line}");
                assert_eq!(row.time_zone, zone.name(), "row {line}");
                assert_eq!(
                    row.instant.as_deref(),
                    Some(format_instant(instant, zone).as_str())
                );
                let shown = DateTime::parse_from_rfc3339(row.instant.as_deref().unwrap()).unwrap();
                assert_eq!(shown.with_timezone(&Utc), instant);
                assert_eq!(
                    shown.offset().local_minus_utc(),
                    crate::services::time_zones::offset_seconds_at(zone, instant)
                );
                assert_eq!(
                    row.primary_local.as_deref(),
                    Some(format_local(local_in(primary, instant)).as_str())
                );
                assert_eq!(row.error_code, None);
                assert_eq!(row.note_code, None);
            }
        }

        // The event-wide rows have no timezone: the primary's.
        for (line, local) in [
            (rows as u64, "2028-03-09T00:00"),
            (rows as u64 + 1, "2028-05-08T19:00"),
        ] {
            let row = row(preview, line);
            assert_eq!(row.election_alias, ALL_ELECTIONS);
            assert_eq!(row.election_id, None);
            assert_eq!(row.time_zone, config.primary);
            assert_eq!(row.primary_local.as_deref(), Some(local));
        }
    }
}

#[test]
fn fractional_offsets_show_in_the_instant() {
    let config = overseas();
    let preview = parse(&config, &replay_csv(&config)).preview;
    let voting_row = |zone: &str| {
        let index = config
            .post_zones
            .iter()
            .position(|known| *known == zone)
            .unwrap();
        row(&preview, (config.posts + index + 2) as u64).clone()
    };
    for (zone, offset) in [
        ("Asia/Kolkata", "+05:30"),
        ("Asia/Kathmandu", "+05:45"),
        ("Asia/Tehran", "+03:30"),
        ("Asia/Yangon", "+06:30"),
        ("Africa/Cairo", "+02:00"),
    ] {
        let row = voting_row(zone);
        assert_eq!(
            row.instant.as_deref(),
            Some(format!("2028-04-09T00:00:00{offset}").as_str()),
            "{zone}"
        );
    }
}

#[test]
fn each_problem_is_reported_by_row_and_nothing_with_one_is_written() {
    let config = overseas();
    let toronto = config
        .post_zones
        .iter()
        .position(|zone| *zone == "America/Toronto")
        .unwrap();
    let cairo = config
        .post_zones
        .iter()
        .position(|zone| *zone == "Africa/Cairo")
        .unwrap();
    let csv = [
        HEADER.to_string(),
        "post-000,START_VOTING_PERIOD,2028-04-09T00:00,,".to_string(), // 2 ok
        "post-999,START_VOTING_PERIOD,2028-04-09T00:00,,".to_string(), // 3
        "post-001,OPEN_THE_DOORS,2028-04-09T00:00,,".to_string(),      // 4
        "post-001,SEND_TEMPLATE,2028-04-09T00:00,,".to_string(),       // 5
        "post-002,START_VOTING_PERIOD,09/04/2028,,".to_string(),       // 6
        "post-003,START_VOTING_PERIOD,2028-04-09T00:00,Mars/Olympus,".to_string(), // 7
        "post-004,START_VOTING_PERIOD,2028-04-09T00:00,,ONLINE|FAX".to_string(), // 8
        "post-005,START_VOTING_PERIOD,2028-04-09T00:00,,ONLINE|EARLY_VOTING".to_string(), // 9
        format!("{},START_TEST_VOTING,2028-03-12 02:30,,", alias(toronto)), // 10
        format!("{},START_FINAL_TESTING,2028-04-28T00:30,,", alias(cairo)), // 11
        "post-000,START_VOTING_PERIOD,2028-04-10T00:00,,".to_string(), // 12
        format!("{},END_TEST_VOTING,2028-11-05T01:30,,", alias(toronto)), // 13 ok, overlap
    ]
    .join("\n");
    let parsed = parse(&config, &csv);
    let preview = &parsed.preview;
    let code = |line: u64| row(preview, line).error_code;
    assert_eq!(code(2), None);
    assert_eq!(code(3), Some(ScheduleRowError::UnknownElection));
    assert_eq!(code(4), Some(ScheduleRowError::UnknownEventType));
    assert_eq!(code(5), Some(ScheduleRowError::UnknownEventType));
    assert_eq!(code(6), Some(ScheduleRowError::InvalidDateTime));
    assert_eq!(code(7), Some(ScheduleRowError::InvalidTimeZone));
    assert_eq!(code(8), Some(ScheduleRowError::InvalidVotingChannels));
    assert_eq!(code(9), Some(ScheduleRowError::InvalidVotingChannels));
    assert_eq!(code(10), Some(ScheduleRowError::DstGap));
    assert_eq!(code(11), Some(ScheduleRowError::DstGap));
    assert_eq!(code(12), Some(ScheduleRowError::Duplicate));
    assert_eq!(code(13), None);
    assert_eq!(
        row(preview, 13).note_code,
        Some(ScheduleRowNote::DstOverlap)
    );
    assert_eq!(preview.errors, 10);
    assert_eq!(preview.ok, 2);
    assert!(!preview.is_clean());

    // The gap row says when it would run: the time after the change.
    let gap = row(preview, 10);
    assert_eq!(gap.time_zone, "America/Toronto");
    assert_eq!(gap.local, "2028-03-12T02:30");
    let toronto_zone: Tz = "America/Toronto".parse().unwrap();
    let shifted = resolve_local(naive("2028-03-12T02:30"), toronto_zone).instant();
    assert_eq!(
        gap.instant.as_deref(),
        Some(format_instant(shifted, toronto_zone).as_str())
    );
    assert_eq!(local_in(toronto_zone, shifted), naive("2028-03-12T03:30"));

    // The unreadable cells come back as written.
    assert_eq!(row(preview, 6).local, "09/04/2028");
    assert_eq!(row(preview, 7).time_zone, "Mars/Olympus");

    // Only error-free rows are entries; the refusal names the rows.
    assert_eq!(
        parsed
            .entries
            .iter()
            .map(|entry| entry.row)
            .collect::<Vec<_>>(),
        vec![2, 13]
    );
    let refusal = RowsWithErrors(preview.clone()).to_string();
    assert!(
        refusal.contains("rows 3, 4, 5, 6, 7, 8, 9, 10, 11, 12"),
        "{refusal}"
    );
}

#[test]
fn the_row_zone_wins_over_the_election_and_aliases_are_canonical() {
    for config in [overseas(), association()] {
        let csv = [
            HEADER,
            "post-000,START_VOTING_PERIOD,2028-04-09T00:00,Asia/Calcutta,",
            "post-001,START_VOTING_PERIOD,2028-04-09T00:00,,",
        ]
        .join("\n");
        let parsed = parse(&config, &csv);
        assert_eq!(row(&parsed.preview, 2).time_zone, "Asia/Kolkata");
        assert_eq!(parsed.entries[0].time_zone, "Asia/Kolkata");
        assert_eq!(row(&parsed.preview, 3).time_zone, config.post_zone(1));
    }
}

#[test]
fn an_election_zone_outside_the_configured_ones_falls_back_to_the_primary() {
    for config in [overseas(), association()] {
        let mut elections = config.elections();
        elections[0].time_zone = Some("Pacific/Honolulu".to_string());
        elections[1].time_zone = None;
        let csv = [
            HEADER,
            "post-000,START_VOTING_PERIOD,2028-04-09T00:00,,",
            "post-001,START_VOTING_PERIOD,2028-04-09T00:00,,",
        ]
        .join("\n");
        let parsed = parse_schedule(csv.as_bytes(), &config.event(), &elections).unwrap();
        for line in [2, 3] {
            assert_eq!(row(&parsed.preview, line).time_zone, config.primary);
        }
    }
}

#[test]
fn an_event_without_timezones_reads_utc() {
    let config = association();
    let csv = [HEADER, "ALL,END_VOTING_PERIOD,2028-05-08T19:00,,"].join("\n");
    let parsed = parse_schedule(
        csv.as_bytes(),
        &ScheduleEvent {
            tenant_id: TENANT.to_string(),
            election_event_id: EVENT.to_string(),
            ..Default::default()
        },
        &config.elections(),
    )
    .unwrap();
    assert_eq!(parsed.preview.primary_time_zone, "UTC");
    assert_eq!(
        parsed.entries[0].scheduled_date,
        "2028-05-08T19:00:00Z".to_string()
    );
}

#[test]
fn columns_are_read_by_name_and_the_header_is_checked() {
    let config = association();
    let reordered = "\u{feff}timezone,local_date_time,event_type,election_alias\n\
                     ,2028-04-09T00:00,START_VOTING_PERIOD,post-001\n";
    let parsed = parse(&config, reordered);
    assert_eq!(parsed.preview.errors, 0);
    assert_eq!(
        parsed.entries[0].election_id.as_deref(),
        Some(election_id(1).as_str())
    );
    assert_eq!(
        parsed.entries[0].voting_channels,
        ScheduleChannels::Unchanged
    );

    let events = config.event();
    let elections = config.elections();
    let missing = "election_alias,event_type\npost-001,START_VOTING_PERIOD\n";
    let error = parse_schedule(missing.as_bytes(), &events, &elections).unwrap_err();
    assert!(error.to_string().contains("local_date_time"), "{error}");
    let unknown = format!("{HEADER},date\n");
    let error = parse_schedule(unknown.as_bytes(), &events, &elections).unwrap_err();
    assert!(error.to_string().contains("\"date\""), "{error}");

    // Blank lines are skipped; the line numbers still follow the file.
    let blank = format!("{HEADER}\n,,,,\npost-001,START_VOTING_PERIOD,bad,,\n");
    let parsed = parse(&config, &blank);
    assert_eq!(parsed.preview.rows.len(), 1);
    assert_eq!(parsed.preview.rows[0].row, 3);
}

/// The scheduled events an import of `entries` leaves, as the database
/// would hold them.
fn stored(entries: &[ScheduleEntry]) -> Vec<ScheduledEvent> {
    plan_import(entries, &association().event())
        .into_iter()
        .enumerate()
        .map(|(index, (entry, action))| {
            assert_eq!(action, Upsert::Create);
            ScheduledEvent {
                id: format!("se-{index}"),
                tenant_id: Some(TENANT.to_string()),
                election_event_id: Some(EVENT.to_string()),
                created_at: None,
                stopped_at: None,
                archived_at: None,
                labels: None,
                annotations: None,
                event_processor: Some(entry.event_processor.clone()),
                cron_config: Some(entry.cron_config()),
                event_payload: Some(
                    serde_json::to_value(ManageElectionDatePayload {
                        election_id: entry.election_id.clone(),
                        voting_channels: entry.voting_channels.for_insert(),
                    })
                    .unwrap(),
                ),
                task_id: Some(entry.task_id(TENANT, EVENT)),
            }
        })
        .collect()
}

#[test]
fn a_second_import_updates_the_same_events() {
    let config = overseas();
    let entries = parse(&config, &replay_csv(&config)).entries;
    let existing = stored(&entries);

    let with = |scheduled_events: Vec<ScheduledEvent>| ScheduleEvent {
        scheduled_events,
        ..config.event()
    };
    let plan = plan_import(&entries, &with(existing.clone()));
    assert_eq!(plan.len(), entries.len());
    for ((_, action), scheduled) in plan.iter().zip(&existing) {
        assert_eq!(action, &Upsert::Update(scheduled.id.clone()));
    }

    // An archived event isn't updated: the row creates a new one.
    let mut archived = existing.clone();
    archived[0].archived_at = Some(Utc::now());
    let plan = plan_import(&entries, &with(archived));
    assert_eq!(plan[0].1, Upsert::Create);
    assert_eq!(plan[1].1, Upsert::Update(existing[1].id.clone()));
}

#[test]
fn event_wide_rows_use_the_event_task_id() {
    let config = association();
    let entries = parse(&config, &replay_csv(&config)).entries;
    let end = entries
        .iter()
        .find(|entry| entry.event_processor == EventProcessors::END_VOTING_PERIOD)
        .unwrap();
    assert_eq!(
        end.task_id(TENANT, EVENT),
        format!("tenant_{TENANT}_event_{EVENT}_END_VOTING_PERIOD")
    );
    let start = &entries[0];
    assert_eq!(
        start.task_id(TENANT, EVENT),
        format!(
            "tenant_{TENANT}_event_{EVENT}_election_{}_START_ENROLLMENT_PERIOD",
            election_id(0)
        )
    );
    let cron = start.cron_config();
    assert_eq!(cron.local.as_deref(), Some("2028-02-09T00:00"));
    assert_eq!(cron.timezone.as_deref(), Some(config.post_zone(0)));
    assert_eq!(cron.scheduled_date, Some(start.scheduled_date.clone()));
}

/// What an entry schedules, without its line number.
fn scheduled(entries: &[ScheduleEntry]) -> Vec<ScheduleEntry> {
    let mut entries: Vec<ScheduleEntry> = entries
        .iter()
        .cloned()
        .map(|entry| ScheduleEntry { row: 0, ..entry })
        .collect();
    entries.sort_by_key(|entry| (entry.election_id.clone(), entry.event_processor.to_string()));
    entries
}

#[test]
fn an_export_imports_back_to_the_same_schedule() {
    for config in [overseas(), association()] {
        let entries = parse(&config, &replay_csv(&config)).entries;
        let event = ScheduleEvent {
            scheduled_events: stored(&entries),
            ..config.event()
        };
        let csv = export_schedule_csv(&event, &config.elections()).unwrap();
        assert!(csv.starts_with(&format!("{HEADER}\n")));
        assert!(csv.lines().nth(1).unwrap().starts_with("ALL,"));
        let again = parse(&config, &csv);
        assert_eq!(again.preview.errors, 0);
        assert_eq!(scheduled(&again.entries), scheduled(&entries));
    }
}

#[test]
fn an_export_reads_older_rows_in_their_election_zone() {
    let config = overseas();
    let entries = parse(&config, &replay_csv(&config)).entries;
    let mut events = stored(&entries[..1]);
    let zone: Tz = config.post_zone(0).parse().unwrap();
    let instant = expected_instant("2028-02-09T00:00", zone);
    events[0].cron_config = Some(CronConfig {
        cron: None,
        scheduled_date: Some(instant.to_rfc3339()),
        local: None,
        timezone: None,
    });
    // Without an offset it can't be read as an instant: left out.
    let mut offsetless = events[0].clone();
    offsetless.id = "se-offsetless".to_string();
    offsetless.cron_config = Some(CronConfig {
        scheduled_date: Some("2028-02-09T00:00:00".to_string()),
        ..Default::default()
    });
    events.push(offsetless);
    let event = ScheduleEvent {
        scheduled_events: events,
        ..config.event()
    };
    let csv = export_schedule_csv(&event, &config.elections()).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    let only = format!(
        "{},START_ENROLLMENT_PERIOD,2028-02-09T00:00,{},",
        alias(0),
        zone.name()
    );
    assert_eq!(lines, vec![HEADER, only.as_str()]);
}

#[test]
fn an_election_is_named_by_its_alias_in_its_default_language() {
    let presentation = serde_json::json!({
        "language_conf": {"default_language_code": "es"},
        "i18n": {
            "en": {"alias": "Dubai PCG", "name": "Dubai Philippine Consulate General"},
            "es": {"alias": "CG Dubái", "name": "Consulado General en Dubái"}
        },
        "timezone": "Asia/Dubai"
    });
    let election = schedule_election("el".to_string(), Some(&presentation), "en");
    assert_eq!(election.alias.as_deref(), Some("CG Dubái"));
    assert_eq!(election.name.as_deref(), Some("Consulado General en Dubái"));
    assert_eq!(election.time_zone.as_deref(), Some("Asia/Dubai"));

    // Without its own language: the event's. As the Admin Portal shows it:
    // the alias, else the name, in that language, before English.
    let presentation = serde_json::json!({
        "i18n": {"en": {"alias": "Madrid", "name": "Madrid office"}, "es": {"name": "Sede de Madrid"}}
    });
    let election = schedule_election("el".to_string(), Some(&presentation), "es");
    assert_eq!(election.alias.as_deref(), Some("Sede de Madrid"));
    assert_eq!(election.name.as_deref(), Some("Sede de Madrid"));
    let election = schedule_election("el".to_string(), Some(&presentation), "fr");
    assert_eq!(election.alias.as_deref(), Some("Madrid"));
    assert_eq!(election.name.as_deref(), Some("Madrid office"));
    assert_eq!(election.time_zone, None);
    assert_eq!(schedule_election("el".to_string(), None, "en").alias, None);
}

#[test]
fn an_alias_two_elections_share_is_refused() {
    let config = association();
    let mut elections = config.elections();
    elections[1].alias = elections[0].alias.clone();
    let csv = [HEADER, "post-000,START_VOTING_PERIOD,2028-04-09T00:00,,"].join("\n");
    let parsed = parse_schedule(csv.as_bytes(), &config.event(), &elections).unwrap();
    assert_eq!(
        row(&parsed.preview, 2).error_code,
        Some(ScheduleRowError::AmbiguousElection)
    );
}

#[test]
fn row_numbers_follow_the_file_with_crlf_and_empty_lines() {
    let config = association();
    for newline in ["\n", "\r\n"] {
        let csv = [
            HEADER,
            "post-000,START_VOTING_PERIOD,2028-04-09T00:00,,",
            "",
            "",
            "post-001,START_VOTING_PERIOD,bad,,",
            ",,,,",
            "post-002,START_VOTING_PERIOD,2028-04-09T00:00,,",
        ]
        .join(newline);
        let preview = parse(&config, &csv).preview;
        let lines: Vec<u64> = preview.rows.iter().map(|row| row.row).collect();
        assert_eq!(lines, vec![2, 5, 7], "{newline:?}");
        assert_eq!(
            row(&preview, 5).error_code,
            Some(ScheduleRowError::InvalidDateTime)
        );
    }
}

#[test]
fn extra_cells_are_an_error_and_a_trailing_header_comma_is_not() {
    let config = association();
    let csv = [
        &format!("{HEADER},")[..],
        "post-000,START_VOTING_PERIOD,2028-04-09T00:00,,,",
        "post-001,START_VOTING_PERIOD,2028-04-09T00:00,,,surprise",
    ]
    .join("\n");
    let preview = parse(&config, &csv).preview;
    assert_eq!(row(&preview, 2).error_code, None);
    assert_eq!(
        row(&preview, 3).error_code,
        Some(ScheduleRowError::ExtraCells)
    );

    let short_header = "election_alias,event_type,local_date_time\n\
                        post-000,START_VOTING_PERIOD,2028-04-09T00:00,Asia/Tokyo\n";
    let preview = parse(&config, short_header).preview;
    assert_eq!(
        row(&preview, 2).error_code,
        Some(ScheduleRowError::ExtraCells)
    );
}

#[test]
fn an_election_without_an_alias_is_named_by_its_name() {
    let presentation = serde_json::json!({
        "i18n": {"en": {"name": "Canary Islands office"}},
        "timezone": "Atlantic/Canary"
    });
    let election = schedule_election(election_id(9), Some(&presentation), "en");
    assert_eq!(election.alias.as_deref(), Some("Canary Islands office"));

    let config = association();
    let mut elections = config.elections();
    elections.push(election);
    let event = ScheduleEvent {
        scheduled_events: vec![],
        ..config.event()
    };
    let csv = [
        HEADER,
        "Canary Islands office,START_VOTING_PERIOD,2028-04-09T00:00,,",
    ]
    .join("\n");
    let parsed = parse_schedule(csv.as_bytes(), &event, &elections).unwrap();
    assert_eq!(parsed.preview.errors, 0);
    assert_eq!(parsed.entries[0].election_id, Some(election_id(9)));
    assert_eq!(parsed.entries[0].time_zone, "Atlantic/Canary");

    // And the export names it the same way.
    let exported = export_schedule_csv(
        &ScheduleEvent {
            scheduled_events: stored(&parsed.entries),
            ..config.event()
        },
        &elections,
    )
    .unwrap();
    assert!(
        exported.contains(
            "Canary Islands office,START_VOTING_PERIOD,2028-04-09T00:00,Atlantic/Canary,"
        ),
        "{exported}"
    );
}

#[test]
fn without_a_channels_column_an_event_keeps_its_channels() {
    let config = association();
    let with_channels = [
        HEADER,
        "post-000,START_VOTING_PERIOD,2028-04-09T00:00,,TELEPHONE",
    ]
    .join("\n");
    let existing = stored(&parse(&config, &with_channels).entries);
    let event = ScheduleEvent {
        scheduled_events: existing,
        ..config.event()
    };
    let elections = config.elections();

    let without = "election_alias,event_type,local_date_time\n\
                   post-000,START_VOTING_PERIOD,2028-04-10T00:00\n\
                   post-001,START_VOTING_PERIOD,2028-04-10T00:00\n";
    let parsed = parse_schedule(without.as_bytes(), &event, &elections).unwrap();
    assert_eq!(
        parsed.entries[0].voting_channels,
        ScheduleChannels::Unchanged
    );
    assert_eq!(parsed.entries[0].voting_channels.for_update(), None);
    // The preview says what will apply: the event's own, else the defaults.
    assert_eq!(
        row(&parsed.preview, 2).voting_channels,
        vec![VotingStatusChannel::TELEPHONE]
    );
    assert_eq!(
        row(&parsed.preview, 3).voting_channels,
        vec![VotingStatusChannel::ONLINE, VotingStatusChannel::KIOSK]
    );

    // An empty cell means the defaults, written as such.
    let empty = format!("{HEADER}\npost-000,START_VOTING_PERIOD,2028-04-10T00:00,,\n");
    let parsed = parse_schedule(empty.as_bytes(), &event, &elections).unwrap();
    assert_eq!(parsed.entries[0].voting_channels, ScheduleChannels::Default);
    assert_eq!(parsed.entries[0].voting_channels.for_update(), Some(vec![]));
    assert_eq!(
        row(&parsed.preview, 2).voting_channels,
        vec![VotingStatusChannel::ONLINE, VotingStatusChannel::KIOSK]
    );

    // Other types show none.
    let enrollment = format!("{HEADER}\npost-000,START_ENROLLMENT_PERIOD,2028-02-09T00:00,,\n");
    let parsed = parse_schedule(enrollment.as_bytes(), &event, &elections).unwrap();
    assert!(row(&parsed.preview, 2).voting_channels.is_empty());
}

#[test]
fn event_wide_types_need_all_and_window_types_need_an_election() {
    let config = association();
    let csv = [
        HEADER,
        "post-000,END_ENROLLMENT_PERIOD,2028-03-09T00:00,,",
        "ALL,END_ENROLLMENT_PERIOD,2028-03-09T00:00,,",
        "ALL,START_TEST_VOTING,2028-03-01T00:00,,",
        "post-000,START_TEST_VOTING,2028-03-01T00:00,,",
        "post-000,START_ENROLLMENT_PERIOD,2028-02-09T00:00,,",
        "post-000,END_VOTING_PERIOD,2028-05-08T19:00,,",
    ]
    .join("\n");
    let preview = parse(&config, &csv).preview;
    let code = |line: u64| row(&preview, line).error_code;
    assert_eq!(code(2), Some(ScheduleRowError::EventWideOnly));
    assert_eq!(code(3), None);
    assert_eq!(code(4), Some(ScheduleRowError::ElectionOnly));
    assert_eq!(code(5), None);
    assert_eq!(code(6), None);
    assert_eq!(code(7), None);
}

#[test]
fn row_zones_are_stored_canonical_and_abbreviations_refused() {
    let config = overseas();
    let csv = [
        HEADER,
        "post-000,START_VOTING_PERIOD,2028-04-09T00:00,US/Eastern,",
        "post-001,START_VOTING_PERIOD,2028-04-09T00:00,EST,",
        "post-002,START_VOTING_PERIOD,2028-04-09T00:00,Japan,",
    ]
    .join("\n");
    let parsed = parse(&config, &csv);
    assert_eq!(row(&parsed.preview, 2).time_zone, "America/New_York");
    assert_eq!(parsed.entries[0].time_zone, "America/New_York");
    for line in [3, 4] {
        assert_eq!(
            row(&parsed.preview, line).error_code,
            Some(ScheduleRowError::InvalidTimeZone)
        );
    }
}

#[test]
fn imported_offset_dates_validate_supplied_wall_time_and_zone() {
    let valid = CronConfig {
        scheduled_date: Some("2028-04-09T00:00:00+05:30".into()),
        local: Some("2028-04-09T00:00".into()),
        timezone: Some("Asia/Kolkata".into()),
        cron: None,
    };
    assert_eq!(checked_import_cron_config(valid.clone()).unwrap(), valid);
    for (local, timezone, message) in [
        (
            Some("2028-04-09T00:00"),
            Some("Not/AZone"),
            "Unknown timezone",
        ),
        (
            Some("not-a-date"),
            Some("Asia/Kolkata"),
            "Invalid local time",
        ),
        (Some("2028-04-09T00:00"), None, "requires a timezone"),
    ] {
        let error = checked_import_cron_config(CronConfig {
            local: local.map(str::to_owned),
            timezone: timezone.map(str::to_owned),
            ..valid.clone()
        })
        .expect_err("Malformed supplied timezone metadata must not bypass validation");
        assert!(error.to_string().contains(message), "{error}");
    }
    let canonical = checked_import_cron_config(CronConfig {
        timezone: Some("Asia/Calcutta".into()),
        ..valid.clone()
    })
    .unwrap();
    assert_eq!(canonical.timezone.as_deref(), Some("Asia/Kolkata"));
    assert_eq!(canonical.scheduled_date, valid.scheduled_date);
    let trimmed = checked_import_cron_config(CronConfig {
        scheduled_date: Some(" 2028-04-09T00:00:00+05:30 ".into()),
        ..valid.clone()
    })
    .unwrap();
    assert_eq!(trimmed.scheduled_date, valid.scheduled_date);
    let normalized = checked_import_cron_config(CronConfig {
        local: Some(" 2028-04-09 00:00 ".into()),
        ..valid.clone()
    })
    .unwrap();
    assert_eq!(normalized.local, valid.local);
    assert!(checked_import_cron_config(CronConfig {
        timezone: Some("Not/AZone".into()),
        ..Default::default()
    })
    .unwrap_err()
    .to_string()
    .contains("Unknown timezone"));
}

#[test]
fn imported_offset_dates_preserve_selected_overlap_and_tzdata_drift() {
    for config in [
        CronConfig {
            scheduled_date: Some("2028-11-05T06:30:00Z".into()),
            local: Some("2028-11-05T01:30".into()),
            timezone: Some("America/New_York".into()),
            cron: None,
        },
        // A previous tzdata version may have resolved this wall time differently.
        // Recompute proposes the change for Apply; importing must retain its instant.
        CronConfig {
            scheduled_date: Some("2028-04-09T01:00:00Z".into()),
            local: Some("2028-04-09T00:00".into()),
            timezone: Some("Europe/Madrid".into()),
            cron: None,
        },
    ] {
        assert_eq!(checked_import_cron_config(config.clone()).unwrap(), config);
    }
}

#[test]
fn an_imported_date_without_an_offset_is_recomputed_or_refused() {
    let with_offset = CronConfig {
        scheduled_date: Some("2028-04-09T00:00:00+04:00".to_string()),
        ..Default::default()
    };
    assert_eq!(
        checked_import_cron_config(with_offset.clone()).unwrap(),
        with_offset
    );
    assert_eq!(
        checked_import_cron_config(CronConfig::default()).unwrap(),
        CronConfig::default()
    );

    let recomputed = checked_import_cron_config(CronConfig {
        scheduled_date: Some("2028-04-09T00:00:00".to_string()),
        local: Some("2028-04-09T00:00".to_string()),
        timezone: Some("Asia/Calcutta".to_string()),
        cron: None,
    })
    .unwrap();
    assert_eq!(
        recomputed.scheduled_date.as_deref(),
        Some("2028-04-09T00:00:00+05:30")
    );
    assert_eq!(recomputed.timezone.as_deref(), Some("Asia/Kolkata"));

    for refused in [
        CronConfig {
            scheduled_date: Some("2028-04-09T00:00:00".to_string()),
            ..Default::default()
        },
        CronConfig {
            scheduled_date: Some("2028-03-12T02:30:00".to_string()),
            local: Some("2028-03-12T02:30".to_string()),
            timezone: Some("America/Toronto".to_string()),
            cron: None,
        },
    ] {
        assert!(checked_import_cron_config(refused).is_err());
    }
}
