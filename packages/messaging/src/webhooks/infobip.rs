// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Infobip Viber delivery reports. Incoming messages are dropped.
//!
//! Infobip does not sign callbacks. They are accepted only on the account's
//! unguessable webhook URL, and a report only changes an attempt this
//! account sent with the reported message ID.

use super::{StatusReport, WebhookEvent};
use anyhow::Result;
use chrono::{DateTime, Utc};
use sequent_core::types::messaging::MessageAttemptState;
use serde_json::{json, Value};

fn state(group: &str) -> Option<MessageAttemptState> {
    match group {
        "PENDING" | "ACCEPTED" => Some(MessageAttemptState::ACCEPTED),
        "DELIVERED" => Some(MessageAttemptState::DELIVERED),
        "UNDELIVERABLE" | "EXPIRED" | "REJECTED" => Some(MessageAttemptState::FAILED),
        _ => None,
    }
}

fn at(value: &Value) -> Option<DateTime<Utc>> {
    value
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
}

pub fn parse_events(body: &[u8]) -> Result<Vec<WebhookEvent>> {
    let payload: Value = serde_json::from_slice(body)?;
    Ok(payload["results"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|result| {
            let message_id = result["messageId"].as_str()?.to_string();
            // Incoming messages carry no status: they are not kept.
            let group = result["status"]["groupName"].as_str()?;
            let billing = result["price"]
                .is_object()
                .then(|| json!({"price": result["price"].clone()}));
            Some(WebhookEvent::Status(StatusReport {
                provider_message_id: message_id,
                state: state(group)?,
                error_code: result["error"]["name"]
                    .as_str()
                    .filter(|name| *name != "NO_ERROR")
                    .map(str::to_string),
                billing,
                at: at(&result["doneAt"]),
            }))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_reports_map_to_attempt_states() {
        let body = json!({"results": [
            {"messageId": "m-1", "to": "639171234567", "doneAt": "2026-10-02T10:00:00.000+0000",
             "status": {"groupName": "DELIVERED", "name": "DELIVERED_TO_HANDSET"},
             "error": {"name": "NO_ERROR"}, "price": {"pricePerMessage": 0.02, "currency": "EUR"}},
            {"messageId": "m-2", "status": {"groupName": "UNDELIVERABLE"},
             "error": {"name": "EC_UNKNOWN_SUBSCRIBER"}},
            {"messageId": "m-3", "status": {"groupName": "SOMETHING_NEW"}}
        ]});
        let events = parse_events(body.to_string().as_bytes()).unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            WebhookEvent::Status(StatusReport {
                state: MessageAttemptState::DELIVERED,
                error_code: None,
                billing: Some(_),
                ..
            })
        ));
        assert!(matches!(
            &events[1],
            WebhookEvent::Status(StatusReport { state: MessageAttemptState::FAILED, error_code: Some(code), .. })
                if code == "EC_UNKNOWN_SUBSCRIBER"
        ));
    }

    #[test]
    fn incoming_messages_are_dropped() {
        let body = json!({"results": [
            {"messageId": "in-1", "from": "639171234567", "to": "Elections",
             "receivedAt": "2026-10-02T10:00:00.000+00:00", "message": {"text": "help"}}
        ]});
        assert!(parse_events(body.to_string().as_bytes())
            .unwrap()
            .is_empty());
    }
}
