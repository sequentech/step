// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;

use sequent_core::services::connection;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use sequent_core::types::scheduled_event::*;
use sequent_core::types::templates::SendTemplateBody;

use anyhow::Result;
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::messaging::record_bulk_send;
use windmill::services::database::get_hasura_pool;
use windmill::services::electoral_log::{
    post_voter_secret_attribute_audit, ElectoralLogAdminContext,
    VoterSecretAttributeAction, VoterSecretAttributeAudit,
};

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

/// Stores the send's payload as a stopped, archived `SEND_TEMPLATE`
/// scheduled event and returns the payload with its `send_id`.
async fn record_send(
    input: &CreateEventBody,
    claims: &JwtClaims,
) -> Result<Value, (Status, String)> {
    let internal = |error: anyhow::Error| {
        tracing::error!("could not record the send: {error:#}");
        (
            Status::InternalServerError,
            "could not record the send".to_string(),
        )
    };
    let tenant_id = Uuid::parse_str(&input.tenant_id)
        .map_err(|_| (Status::BadRequest, "invalid tenant_id".to_string()))?;
    let election_event_id = input
        .election_event_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| {
            (Status::BadRequest, "invalid election_event_id".to_string())
        })?;
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| internal(anyhow::anyhow!("{error}")))?;
    let tx = client
        .transaction()
        .await
        .map_err(|error| internal(error.into()))?;
    let send_id = record_bulk_send(
        &tx,
        &tenant_id,
        election_event_id.as_ref(),
        Some(&claims.hasura_claims.user_id),
        &input.event_payload,
    )
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(|error| internal(error.into()))?;
    let mut payload = input.event_payload.clone();
    if let Some(object) = payload.as_object_mut() {
        object
            .insert("send_id".to_string(), Value::String(send_id.to_string()));
    }
    Ok(payload)
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
    let mut input = body.into_inner();
    match input.event_processor.clone() {
        EventProcessors::SEND_TEMPLATE => {
            authorize(
                &claims,
                true,
                Some(input.tenant_id.clone()),
                vec![Permissions::NOTIFICATION_SEND],
            )?;
            let payload: SendTemplateBody = serde_json::from_value(
                input.event_payload.clone(),
            )
            .map_err(|error| {
                (
                    Status::BadRequest,
                    format!("Invalid send-template payload: {error}"),
                )
            })?;
            // Refused here, with a 400, rather than when the task runs.
            if let Some(reason) = payload.audience_refusal() {
                return Err((Status::BadRequest, reason.to_string()));
            }
            if !payload.secret_attribute_names.is_empty() {
                authorize(
                    &claims,
                    true,
                    Some(input.tenant_id.clone()),
                    vec![Permissions::VOTER_SECRET_ATTRIBUTE_READ],
                )?;
                let election_event_id =
                    input.election_event_id.as_deref().ok_or_else(|| {
                        (
                            Status::BadRequest,
                            "Encrypted voter attributes require an election event"
                                .to_string(),
                        )
                    })?;
                post_voter_secret_attribute_audit(
                    &input.tenant_id,
                    election_event_id,
                    &ElectoralLogAdminContext::from_claims(&claims),
                    VoterSecretAttributeAction::Communication,
                    VoterSecretAttributeAudit {
                        voter_id: None,
                        voter_username: None,
                        attribute_names: &payload.secret_attribute_names,
                        document_id: None,
                    },
                )
                .await
                .map_err(|error| {
                    (
                        Status::InternalServerError,
                        format!("Failed to record the secret-attribute electoral-log entry: {error:#}"),
                    )
                })?;
            }
            // What the administrator sent is kept, so a notice that fails
            // for a moment is sent again with that content, never with the
            // template as stored now. Its ID is the send's ID.
            input.event_payload = record_send(&input, &claims).await?;
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
