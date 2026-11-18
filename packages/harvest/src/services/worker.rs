// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::routes::scheduled_event;
use crate::services::worker::scheduled_event::CreateEventBody;
use anyhow::{anyhow, Result};
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::scheduled_event::*;
use sequent_core::types::templates::SendTemplateBody;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::tasks::render_report;
use windmill::tasks::send_template::*;

/// Deserializes a scheduled event's payload. The error keeps the field path
/// and the kind of error but not serde's message, which can quote payload
/// values (`invalid type: string "...", expected a map`). A missing field is
/// still named: that message holds only the field name.
fn deserialize_payload<T: DeserializeOwned>(payload: Value) -> Result<T> {
    deserialize_value(payload).map_err(|error| {
        let message = error.inner().to_string();
        let detail = if message.starts_with("missing field") {
            message
        } else {
            format!("{:?} error", error.inner().classify())
        };
        anyhow!("invalid event payload at `{}`: {detail}", error.path())
    })
}

#[instrument(skip_all, err)]
pub async fn process_scheduled_event(
    event: CreateEventBody,
    claims: JwtClaims,
) -> Result<String> {
    let celery_app = get_celery_app().await;
    let element_id: String = Uuid::new_v4().to_string();
    match event.event_processor.clone() {
        EventProcessors::CREATE_REPORT => {
            let body: render_report::RenderTemplateBody =
                deserialize_payload(event.event_payload.clone())?;
            let election_event_id = event
                .election_event_id
                .ok_or(anyhow!("empty election_event_id"))?;
            let task = celery_app
                .send_task(render_report::render_report::new(
                    body,
                    event.tenant_id,
                    election_event_id,
                ))
                .await?;
            event!(Level::INFO, "Sent CREATE_REPORT task {}", task.task_id);
        }
        EventProcessors::SEND_TEMPLATE => {
            let payload: SendTemplateBody =
                deserialize_payload(event.event_payload.clone())?;
            let user_id = claims.hasura_claims.user_id;
            let task = celery_app
                .send_task(send_template::new(
                    payload,
                    event.tenant_id,
                    user_id,
                    event.election_event_id.clone(),
                ))
                .await?;
            event!(Level::INFO, "Sent SEND_TEMPLATE task {}", task.task_id);
        }
        EventProcessors::ALLOW_INIT_REPORT => {}
        EventProcessors::START_VOTING_PERIOD => {}
        EventProcessors::END_VOTING_PERIOD => {}
        EventProcessors::ALLOW_VOTING_PERIOD_END => {}
        EventProcessors::START_ENROLLMENT_PERIOD => {}
        EventProcessors::END_ENROLLMENT_PERIOD => {}
        EventProcessors::START_LOCKDOWN_PERIOD => {}
        EventProcessors::END_LOCKDOWN_PERIOD => {}
        EventProcessors::ALLOW_TALLY => {}
    }
    Ok(element_id)
}

#[cfg(test)]
mod payload_tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize, Debug)]
    struct Payload {
        template: String,
        variables: HashMap<String, String>,
    }

    #[test]
    fn a_wrong_type_keeps_the_path_but_not_the_value() {
        let error = deserialize_payload::<Payload>(serde_json::json!({
            "template": "t",
            "variables": "C90PayloadValue",
        }))
        .unwrap_err()
        .to_string();
        assert!(!error.contains("C90PayloadValue"), "{error}");
        assert!(error.contains("`variables`"), "{error}");
    }

    #[test]
    fn a_missing_field_is_named() {
        let error = deserialize_payload::<Payload>(serde_json::json!({
            "variables": {},
        }))
        .unwrap_err()
        .to_string();
        assert!(error.contains("missing field `template`"), "{error}");
    }

    #[test]
    fn a_valid_payload_deserializes() {
        let payload = deserialize_payload::<Payload>(serde_json::json!({
            "template": "t",
            "variables": {"name": "value"},
        }))
        .unwrap();
        assert_eq!(payload.template, "t");
        assert_eq!(payload.variables["name"], "value");
    }
}
