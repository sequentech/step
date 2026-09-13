// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Scheduled dates must stay inside their tenant/event/election scope. These
//! tests use literal task identifiers, independently of the name generator.

use sequent_core::types::scheduled_event::*;
use serde_json::{json, Value};

fn scheduled(
    processor: EventProcessors,
    election: Option<&str>,
    date: Option<&str>,
) -> ScheduledEvent {
    serde_json::from_value(json!({
        "id": "schedule-1", "tenant_id": "north", "election_event_id": "mayor",
        "event_processor": processor, "event_payload": {"election_id": election},
        "cron_config": {"scheduled_date": date},
        "task_id": "tenant_north_event_mayor_election_city_START_VOTING_PERIOD"
    })).unwrap()
}

#[test]
fn task_names_distinguish_the_event_from_an_individual_election() {
    assert_eq!(
        generate_manage_date_task_name(
            "north",
            "mayor",
            None,
            &EventProcessors::END_VOTING_PERIOD
        ),
        "tenant_north_event_mayor_END_VOTING_PERIOD"
    );
    assert_eq!(
        generate_manage_date_task_name(
            "north",
            "mayor",
            Some("city"),
            &EventProcessors::START_VOTING_PERIOD
        ),
        "tenant_north_event_mayor_election_city_START_VOTING_PERIOD"
    );
}

#[test]
fn voting_dates_require_matching_tenant_event_task_and_payload() {
    let start = scheduled(
        EventProcessors::START_VOTING_PERIOD,
        Some("city"),
        Some("2026-10-01T09:00:00Z"),
    );
    let mut end = start.clone();
    end.task_id =
        Some("tenant_north_event_mayor_election_city_END_VOTING_PERIOD".into());
    end.event_processor = Some(EventProcessors::END_VOTING_PERIOD);
    end.cron_config.as_mut().unwrap().scheduled_date =
        Some("2026-10-01T18:00:00Z".into());
    let dates = generate_voting_period_dates(
        vec![start.clone(), end],
        "north",
        "mayor",
        Some("city"),
    )
    .unwrap();
    assert_eq!(dates.start_date.as_deref(), Some("2026-10-01T09:00:00Z"));
    assert_eq!(dates.end_date.as_deref(), Some("2026-10-01T18:00:00Z"));

    // Change one boundary at a time: a matching task name alone is insufficient.
    for field in ["tenant_id", "election_event_id", "task_id", "event_payload"]
    {
        let mut unrelated = serde_json::to_value(&start).unwrap();
        unrelated[field] = if field == "event_payload" {
            json!({"election_id": "district"})
        } else {
            json!("unrelated")
        };
        let dates = generate_voting_period_dates(
            vec![serde_json::from_value(unrelated).unwrap()],
            "north",
            "mayor",
            Some("city"),
        )
        .unwrap();
        assert!(dates.start_date.is_none(), "must reject mismatched {field}");
        assert!(dates.end_date.is_none());
    }
}

#[test]
fn missing_cron_dates_stay_absent_and_event_wide_tasks_use_a_null_election() {
    let mut event = scheduled(EventProcessors::START_VOTING_PERIOD, None, None);
    event.task_id = Some("tenant_north_event_mayor_START_VOTING_PERIOD".into());
    for cron in [event.cron_config.clone(), None] {
        event.cron_config = cron;
        let dates = generate_voting_period_dates(
            vec![event.clone()],
            "north",
            "mayor",
            None,
        )
        .unwrap();
        assert!(dates.start_date.is_none());
        assert!(dates.end_date.is_none());
    }
}

#[test]
fn event_overview_does_not_show_dates_from_an_individual_election() {
    // The event overview has no election filter: that means event-wide dates,
    // not all elections. Otherwise one election can overwrite the event's date.
    let event = scheduled(
        EventProcessors::START_VOTING_PERIOD,
        None,
        Some("event date"),
    );
    let election = scheduled(
        EventProcessors::END_VOTING_PERIOD,
        Some("city"),
        Some("election date"),
    );
    let dates = prepare_scheduled_dates(vec![event, election], None).unwrap();
    assert_eq!(dates.len(), 1);
    assert_eq!(
        dates["START_VOTING_PERIOD"].scheduled_at.as_deref(),
        Some("event date")
    );
}

#[test]
fn election_dates_include_event_defaults_but_discard_other_elections_and_malformed_tasks(
) {
    let mut tasks = vec![
        scheduled(
            EventProcessors::START_VOTING_PERIOD,
            None,
            Some("event start"),
        ),
        scheduled(
            EventProcessors::END_VOTING_PERIOD,
            Some("city"),
            Some("city end"),
        ),
        scheduled(
            EventProcessors::ALLOW_INIT_REPORT,
            Some("district"),
            Some("wrong district"),
        ),
        scheduled(
            EventProcessors::SEND_TEMPLATE,
            None,
            Some("not a date task"),
        ),
    ];
    for payload in [None, Some(Value::Null), Some(json!({"election_id": 17}))] {
        let mut invalid = tasks[0].clone();
        invalid.event_payload = payload;
        tasks.push(invalid);
    }
    let mut missing_processor = tasks[0].clone();
    missing_processor.event_processor = None;
    tasks.push(missing_processor);

    let dates = prepare_scheduled_dates(tasks, Some("city")).unwrap();
    assert_eq!(dates.len(), 2);
    assert_eq!(
        dates["START_VOTING_PERIOD"].scheduled_at.as_deref(),
        Some("event start")
    );
    assert_eq!(
        dates["END_VOTING_PERIOD"].scheduled_at.as_deref(),
        Some("city end")
    );
    assert_eq!(dates["END_VOTING_PERIOD"].stopped_at.as_deref(), Some("-"));
}

#[test]
fn omitted_allow_init_defaults_to_true_but_explicit_false_is_preserved() {
    let default: ManageAllowInitPayload =
        serde_json::from_value(json!({})).unwrap();
    let disabled: ManageAllowInitPayload =
        serde_json::from_value(json!({"allow_init": false})).unwrap();
    assert_eq!(default.allow_init, Some(true));
    assert_eq!(disabled.allow_init, Some(false));
}
