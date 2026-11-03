// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;

use sequent_core::services::connection;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use sequent_core::types::scheduled_event::*;

use anyhow::Result;
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;

use crate::services;

#[derive(Deserialize, Debug, Clone)]
pub struct CreateEventBody {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub event_processor: EventProcessors,
    pub cron_config: Option<String>,
    pub event_payload: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateEventOutput {
    pub id: String,
}

fn authorize_create_report(
    claims: &JwtClaims,
    input: &CreateEventBody,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::REPORT_GENERATE],
    )
}

#[instrument(skip(claims, body))]
#[post("/scheduled-event", format = "json", data = "<body>")]
pub async fn create_scheduled_event(
    body: Json<CreateEventBody>,
    claims: JwtClaims,
) -> Result<Json<CreateEventOutput>, (Status, String)> {
    let input = body.into_inner();
    match input.event_processor.clone() {
        EventProcessors::SEND_TEMPLATE => {
            authorize(
                &claims,
                true,
                Some(input.tenant_id.clone()),
                vec![Permissions::NOTIFICATION_SEND],
            )?;
        }
        EventProcessors::CREATE_REPORT => {
            authorize_create_report(&claims, &input)?;
        }
        _ => {}
    };

    let element_id =
        services::worker::process_scheduled_event(input.clone(), claims)
            .await
            .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;

    Ok(Json(CreateEventOutput { id: element_id }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::test_claims::{
        admin_claims, CALLER_TENANT_ID, OTHER_TENANT_ID,
    };

    fn create_report(tenant_id: &str) -> CreateEventBody {
        CreateEventBody {
            tenant_id: tenant_id.to_string(),
            election_event_id: Some("event".to_string()),
            event_processor: EventProcessors::CREATE_REPORT,
            cron_config: None,
            event_payload: Value::Null,
        }
    }

    #[test]
    fn create_report_rejects_tenant_other_than_callers() {
        let claims = admin_claims(CALLER_TENANT_ID, &["report-generate"]);
        let result =
            authorize_create_report(&claims, &create_report(OTHER_TENANT_ID));
        assert_eq!(result.unwrap_err().0, Status::Unauthorized);
    }

    #[test]
    fn create_report_requires_report_generate() {
        let claims = admin_claims(CALLER_TENANT_ID, &["admin-user"]);
        let result =
            authorize_create_report(&claims, &create_report(CALLER_TENANT_ID));
        assert_eq!(result.unwrap_err().0, Status::Unauthorized);
    }

    #[test]
    fn create_report_accepts_callers_tenant_with_report_generate() {
        let claims = admin_claims(CALLER_TENANT_ID, &["report-generate"]);
        assert!(authorize_create_report(
            &claims,
            &create_report(CALLER_TENANT_ID)
        )
        .is_ok());
    }
}
