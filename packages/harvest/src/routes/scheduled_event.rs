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
            let payload: SendTemplateBody = serde_json::from_value(
                input.event_payload.clone(),
            )
            .map_err(|error| {
                (
                    Status::BadRequest,
                    format!("Invalid send-template payload: {error}"),
                )
            })?;
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
        }
        EventProcessors::CREATE_REPORT => {
            authorize(
                &claims,
                true,
                Some(claims.hasura_claims.tenant_id.clone()),
                vec![], /* TODO: task not being used at the moment, and it
                         * has no specific perms yet */
            )?;
        }
        _ => {}
    };

    let element_id =
        services::worker::process_scheduled_event(input.clone(), claims)
            .await
            .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;

    Ok(Json(CreateEventOutput { id: element_id }))
}
