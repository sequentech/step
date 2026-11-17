// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::json;
use std::cell::RefCell;

fn scheduled(id: &str, processor: EventProcessors) -> ScheduledEvent {
    serde_json::from_value(json!({
        "id": id, "tenant_id": "tenant", "election_event_id": "event",
        "event_processor": processor.to_string(),
        "event_payload": {"election_id": null},
        "cron_config": {"scheduled_date": null},
    }))
    .unwrap()
}

#[tokio::test]
async fn one_failing_event_does_not_stop_the_rest_of_the_tick() {
    let events = vec![
        scheduled("first", EventProcessors::ALLOW_INIT_REPORT),
        scheduled("failing", EventProcessors::START_ENROLLMENT_PERIOD),
        scheduled("third", EventProcessors::ALLOW_TALLY),
        scheduled("also-failing", EventProcessors::ALLOW_VOTING_PERIOD_END),
        scheduled("last", EventProcessors::END_VOTING_PERIOD),
    ];
    let seen = RefCell::new(vec![]);
    let failed = run_due(&events, |event| {
        seen.borrow_mut().push(event.id.clone());
        let fails = event.id.contains("failing");
        async move {
            if fails {
                Err(Error::from(anyhow!("Error deserializing payload")))
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
