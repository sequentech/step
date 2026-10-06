// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::json;
use std::cell::RefCell;

fn scheduled(id: &str, processor: EventProcessors, date: Option<&str>) -> ScheduledEvent {
    serde_json::from_value(json!({
        "id": id, "tenant_id": "tenant", "election_event_id": "event",
        "event_processor": processor.to_string(),
        "event_payload": {"election_id": null},
        "cron_config": {"scheduled_date": date},
    }))
    .unwrap()
}

#[tokio::test]
async fn one_failing_event_does_not_stop_the_rest_of_the_tick() {
    let events = vec![
        scheduled("first", EventProcessors::ALLOW_INIT_REPORT, None),
        scheduled("failing", EventProcessors::START_ENROLLMENT_PERIOD, None),
        scheduled("third", EventProcessors::ALLOW_TALLY, None),
        scheduled("also-failing", EventProcessors::START_LOCKDOWN_PERIOD, None),
        scheduled("last", EventProcessors::END_VOTING_PERIOD, None),
    ];
    let seen = RefCell::new(vec![]);
    let failed = run_due(&events, |event| {
        seen.borrow_mut().push(event.id.clone());
        let fails = event.id.contains("failing");
        async move {
            if fails {
                Err(Error::from(anyhow!("broker unavailable")))
            } else {
                Ok(())
            }
        }
    })
    .await;
    assert_eq!(failed, 2);
    assert_eq!(
        seen.into_inner(),
        ["first", "failing", "third", "also-failing", "last"]
    );
}

#[test]
fn due_events_are_the_ones_before_the_end_of_the_tick_with_a_date_that_has_an_offset() {
    let until = ISO8601::to_date("2028-04-09T00:00:10Z").unwrap();
    let events = vec![
        scheduled(
            "past",
            EventProcessors::START_VOTING_PERIOD,
            Some("2028-04-08T00:00:00Z"),
        ),
        scheduled(
            "in-tick",
            EventProcessors::START_VOTING_PERIOD,
            Some("2028-04-09T08:00:05+08:00"),
        ),
        scheduled(
            "later",
            EventProcessors::START_VOTING_PERIOD,
            Some("2028-04-09T00:00:10Z"),
        ),
        scheduled(
            "no-offset",
            EventProcessors::START_VOTING_PERIOD,
            Some("2028-04-08T00:00:00"),
        ),
        scheduled("no-date", EventProcessors::START_VOTING_PERIOD, None),
    ];
    let due: Vec<&str> = due_events(&events, until)
        .into_iter()
        .map(|event| event.id.as_str())
        .collect();
    assert_eq!(due, ["past", "in-tick"]);
}

#[test]
fn a_posts_own_row_for_the_same_change_takes_it_out_of_the_event_wide_one() {
    let row = |election: &str,
               processor: EventProcessors,
               channels: serde_json::Value|
     -> ScheduledEvent {
        serde_json::from_value(json!({
            "id": format!("{election}-{processor}"), "tenant_id": "tenant", "election_event_id": "event",
            "event_processor": processor.to_string(),
            "task_id": generate_manage_date_task_name("tenant", "event", Some(election), &processor),
            "event_payload": {"election_id": election, "voting_channels": channels},
        }))
        .unwrap()
    };
    let mut archived = row("archived", EventProcessors::END_VOTING_PERIOD, json!(null));
    archived.archived_at = Some(Utc::now());
    let events = vec![
        row("own", EventProcessors::END_VOTING_PERIOD, json!(null)),
        row(
            "kiosk-only",
            EventProcessors::END_VOTING_PERIOD,
            json!(["KIOSK"]),
        ),
        row(
            "opening-only",
            EventProcessors::START_VOTING_PERIOD,
            json!(null),
        ),
        row("tally", EventProcessors::ALLOW_TALLY, json!(null)),
        archived,
    ];
    let own = |processor| {
        let mut ids: Vec<String> = elections_with_own_row(&events, "tenant", "event", &processor)
            .into_iter()
            .collect();
        ids.sort();
        ids
    };
    assert_eq!(own(EventProcessors::END_VOTING_PERIOD), ["own"]);
    assert_eq!(own(EventProcessors::START_VOTING_PERIOD), ["opening-only"]);
    assert_eq!(own(EventProcessors::ALLOW_TALLY), ["tally"]);
}

#[test]
fn a_row_moved_beyond_the_slack_fires_later() {
    let now = ISO8601::to_date("2028-04-09T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let at = |date| scheduled("row", EventProcessors::END_VOTING_PERIOD, Some(date));
    assert!(!fires_later(&at("2028-04-09T00:00:30Z"), now));
    assert!(!fires_later(&at("2028-04-08T00:00:00Z"), now));
    assert!(fires_later(&at("2028-04-09T00:01:01Z"), now));
}
