// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{event, instrument, Level};
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::custom_url::{
    get_page_rule, set_custom_url, CustomUrlKind, DnsLabel,
};
use windmill::services::database::get_hasura_pool;

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateCustomUrlInput {
    pub dns_prefix: String,
    pub election_id: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetCustomUrlInput {
    pub redirect_to: String,
}

#[derive(Serialize)]
struct GetCustomUrlOutput {
    success: bool,
    message: String,
    origin: String,
}

#[derive(Serialize)]
struct UpdateCustomUrlOutput {
    success: bool,
    message: String,
}

/// Kind of custom URL named by the key of the request; an unknown key is a bad
/// request.
fn parse_custom_url_kind(key: &str) -> Result<CustomUrlKind, (Status, String)> {
    CustomUrlKind::from_str(key).map_err(|_| {
        (
            Status::BadRequest,
            format!("Invalid custom URL key: {key:?}"),
        )
    })
}

/// Prefix named by the request. An invalid prefix is reported in the output of
/// the update, like the other failures of the update, so that the admin portal
/// can show the message of each custom URL.
fn parse_custom_url_prefix(
    dns_prefix: &str,
) -> Result<DnsLabel, UpdateCustomUrlOutput> {
    DnsLabel::from_str(dns_prefix).map_err(|error| UpdateCustomUrlOutput {
        success: false,
        message: format!("Error updating custom URL: {error}"),
    })
}

#[instrument(skip(claims))]
#[post("/set-custom-url", format = "json", data = "<input>")]
pub async fn update_custom_url(
    claims: JwtClaims,
    input: Json<UpdateCustomUrlInput>,
) -> Result<Json<UpdateCustomUrlOutput>, (Status, String)> {
    let body: UpdateCustomUrlInput = input.into_inner();
    if let Err(err) = authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_WRITE],
    ) {
        error!("Authorization failed: {:?}", err);
        return Err((Status::Forbidden, "Authorization failed".to_string()));
    }

    let kind = parse_custom_url_kind(&body.key)?;
    let dns_prefix = match parse_custom_url_prefix(&body.dns_prefix) {
        Ok(dns_prefix) => dns_prefix,
        Err(output) => return Ok(Json(output)),
    };

    info!("Authorization succeeded, processing URL update");
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    match set_custom_url(
        &claims.hasura_claims.tenant_id,
        &election_event.id,
        kind,
        &dns_prefix,
    )
    .await
    {
        Ok(message) => {
            info!("Custom URL successfully updated");
            let success_message = format!("Success updating custom URL");
            Ok(Json(UpdateCustomUrlOutput {
                success: true,
                message: success_message,
            }))
        }
        Err(error) => {
            let error_message =
                format!("Error updating custom URL: {:?}", error);
            error!("{}", error_message);

            Ok(Json(UpdateCustomUrlOutput {
                success: false,
                message: error_message,
            }))
        }
    }
}

#[instrument(skip(claims))]
#[post("/get-custom-url", format = "json", data = "<input>")]
pub async fn get_custom_url(
    claims: JwtClaims,
    input: Json<GetCustomUrlInput>,
) -> Result<Json<GetCustomUrlOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_READ],
    )?;
    let rule = get_page_rule(&body.redirect_to).await.map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error reading custom url: {error:?}"),
        )
    })?;

    match rule {
        Some(r) => {
            let origin = r
                .targets
                .get(0)
                .map(|target| target.constraint.value.clone());

            match origin {
                Some(origin) => Ok(Json(GetCustomUrlOutput {
                    success: true,
                    message: "Page rule found".to_string(),
                    origin,
                })),
                None => Ok(Json(GetCustomUrlOutput {
                    success: false,
                    message: "Error extracting page rule".to_string(),
                    origin: "".to_string(),
                })),
            }
        }
        None => Ok(Json(GetCustomUrlOutput {
            success: false,
            message: "No matching page rule found".to_string(),
            origin: "".to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_keys_are_custom_url_kinds() {
        assert_eq!(
            parse_custom_url_kind("login").ok(),
            Some(CustomUrlKind::Login)
        );
        assert_eq!(
            parse_custom_url_kind("enrollment").ok(),
            Some(CustomUrlKind::Enrollment)
        );
        assert_eq!(
            parse_custom_url_kind("saml").ok(),
            Some(CustomUrlKind::Saml)
        );
    }

    #[test]
    fn unknown_key_is_a_bad_request() {
        let (status, message) = parse_custom_url_kind("other").unwrap_err();

        assert_eq!(status, Status::BadRequest);
        assert!(message.contains("other"));
    }

    #[test]
    fn prefix_must_be_a_single_label() {
        assert!(parse_custom_url_prefix("my-vote").is_ok());
        for prefix in ["", "my.vote", "*"] {
            let output = parse_custom_url_prefix(prefix).unwrap_err();

            assert!(!output.success, "{prefix:?}");
            assert!(output.message.starts_with("Error updating custom URL"));
        }
    }
}
