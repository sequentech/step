// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::ballot::{Enrollment, Otp};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{error, info, instrument};
use windmill::postgres::election_event::get_election_event_by_id;

#[derive(Serialize, Deserialize, Debug)]
pub struct SetVoterAuthentication {
    pub election_event_id: String,
    pub enrollment: String,
    pub otp: String,
}

#[derive(Serialize)]
struct SetVoterAuthenticationOutput {
    success: bool,
    message: String,
}

fn authorize_set_voter_authentication(
    claims: &JwtClaims,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_WRITE],
    )
    .map_err(|err| {
        error!("Authorization failed: {:?}", err);
        (Status::Forbidden, "Authorization failed".to_string())
    })
}

fn parse_requested<T: FromStr>(
    value: &str,
    field: &str,
) -> Result<Option<T>, (Status, String)> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    value.parse::<T>().map(Some).map_err(|_| {
        (
            Status::BadRequest,
            format!("Invalid {field} value: {value:?}"),
        )
    })
}

/// Whether the stored setting `current` differs from the `requested` one. A
/// stored value that does not parse counts as different.
fn differs_from_current<T: FromStr + PartialEq>(
    current: &str,
    requested: &T,
) -> bool {
    !current
        .parse::<T>()
        .is_ok_and(|current| &current == requested)
}

#[instrument(skip(claims, services))]
#[post("/set-voter-authentication", format = "json", data = "<input>")]
pub async fn set_voter_authentication(
    claims: JwtClaims,
    input: Json<SetVoterAuthentication>,
    services: &State<HarvestServices>,
) -> Result<Json<SetVoterAuthenticationOutput>, (Status, String)> {
    let body = input.into_inner();

    authorize_set_voter_authentication(&claims)?;
    let enrollment =
        parse_requested::<Enrollment>(&body.enrollment, "enrollment")?;
    let otp = parse_requested::<Otp>(&body.otp, "otp")?;

    let mut hasura_db_client =
        services.databases.hasura().await.get().await.map_err(|e| {
            error!("Failed to get DB pool: {:?}", e);
            (Status::InternalServerError, format!("{:?}", e))
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            error!("Failed to start transaction: {:?}", e);
            (Status::InternalServerError, format!("{:?}", e))
        })?;

    // Serialize the full realm update with guard repair and enrollment window
    // refresh; the identity helper is also used by already-locked cron tasks.
    windmill::postgres::scheduled_event::lock_scheduling_event(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|error| (Status::InternalServerError, error.to_string()))?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|e| {
        error!("Failed to fetch election event: {:?}", e);
        (Status::InternalServerError, format!("{:?}", e))
    })?;

    // Extract or set default enrollment and OTP values
    let (prev_enrollment, prev_otp) =
        election_event.presentation.as_ref().map_or(
            (Enrollment::ENABLED.to_string(), Otp::ENABLED.to_string()),
            |presentation| {
                let enrollment = presentation
                    .get("enrollment")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&Enrollment::ENABLED.to_string())
                    .to_string();

                let otp = presentation
                    .get("otp")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&Otp::ENABLED.to_string())
                    .to_string();

                (enrollment, otp)
            },
        );

    // Update enrollment if it has changed
    if let Some(enrollment) =
        enrollment.filter(|value| differs_from_current(&prev_enrollment, value))
    {
        let enable_enrollment = enrollment == Enrollment::ENABLED;
        info!("Updating enrollment to: {}", enable_enrollment);

        services
            .identity
            .update_voter_enrollment(
                Some(claims.hasura_claims.tenant_id.clone()),
                Some(body.election_event_id.clone()),
                enable_enrollment,
            )
            .await
            .map_err(|error| {
                error!("Failed to update enrollment: {:?}", error);
                (
                    Status::InternalServerError,
                    format!("Error updating enrollment: {error:?}"),
                )
            })?;
    }

    if let Some(otp) =
        otp.filter(|value| differs_from_current(&prev_otp, value))
    {
        let new_otp_state = match otp {
            Otp::ENABLED => "REQUIRED".to_string(),
            Otp::DISABLED => "DISABLED".to_string(),
        };

        info!("Updating OTP to: {}", new_otp_state);

        services
            .identity
            .update_voter_otp(
                Some(claims.hasura_claims.tenant_id.clone()),
                Some(body.election_event_id.clone()),
                new_otp_state,
            )
            .await
            .map_err(|error| {
                error!("Failed to update OTP: {:?}", error);
                (
                    Status::InternalServerError,
                    format!("Error updating OTP: {error:?}"),
                )
            })?;
    }

    // Commit transaction
    hasura_transaction.commit().await.map_err(|e| {
        error!("Transaction commit failed: {:?}", e);
        (Status::InternalServerError, format!("{:?}", e))
    })?;

    Ok(Json(SetVoterAuthenticationOutput {
        success: true,
        message: "Authentication updated successfully".to_string(),
    }))
}

#[cfg(test)]
#[path = "../../tests/support/voter_authentication_routes.rs"]
mod route_tests;

#[cfg(test)]
mod voter_authentication_tests {
    use super::*;

    fn admin(roles: &[&str]) -> JwtClaims {
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test",
            "sub": "admin", "typ": "Bearer", "azp": "admin-portal",
            "acr": "1", "allowed-origins": [], "scope": "openid",
            "email_verified": false,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "admin-user",
                "x-hasura-tenant-id": "tenant",
                "x-hasura-user-id": "admin",
                "x-hasura-allowed-roles": roles
            }
        }))
        .unwrap()
    }

    #[test]
    fn requires_election_event_write() {
        for roles in [
            vec![
                "admin-user",
                "election-event-read",
                "election-event-keys-tab",
                "election-event-tally-tab",
            ],
            vec!["admin-user", "election-event-read", "publish-write"],
        ] {
            assert_eq!(
                authorize_set_voter_authentication(&admin(&roles))
                    .unwrap_err()
                    .0,
                Status::Forbidden
            );
        }
    }

    #[test]
    fn allows_election_event_write() {
        let claims = admin(&["admin-user", "election-event-write"]);
        assert!(authorize_set_voter_authentication(&claims).is_ok());
    }

    #[test]
    fn empty_values_leave_settings_unchanged() {
        assert_eq!(parse_requested::<Enrollment>("", "enrollment"), Ok(None));
        assert_eq!(parse_requested::<Otp>(" ", "otp"), Ok(None));
    }

    #[test]
    fn parses_enabled_and_disabled() {
        assert_eq!(
            parse_requested::<Enrollment>("enabled", "enrollment"),
            Ok(Some(Enrollment::ENABLED))
        );
        assert_eq!(
            parse_requested::<Enrollment>("disabled", "enrollment"),
            Ok(Some(Enrollment::DISABLED))
        );
        assert_eq!(
            parse_requested::<Otp>("enabled", "otp"),
            Ok(Some(Otp::ENABLED))
        );
        assert_eq!(
            parse_requested::<Otp>("disabled", "otp"),
            Ok(Some(Otp::DISABLED))
        );
    }

    #[test]
    fn unchanged_settings_are_detected_by_value() {
        assert!(!differs_from_current("enabled", &Enrollment::ENABLED));
        assert!(!differs_from_current("disabled", &Otp::DISABLED));
        assert!(differs_from_current("disabled", &Enrollment::ENABLED));
        assert!(differs_from_current("enabled", &Otp::DISABLED));
    }

    #[test]
    fn unparsable_stored_settings_count_as_changed() {
        assert!(differs_from_current("REQUIRED", &Otp::ENABLED));
        assert!(differs_from_current("", &Enrollment::DISABLED));
    }

    #[test]
    fn rejects_unknown_values() {
        for value in ["FOO", "ENABLED", "disabled ", "required"] {
            assert_eq!(
                parse_requested::<Otp>(value, "otp").unwrap_err().0,
                Status::BadRequest
            );
            assert_eq!(
                parse_requested::<Enrollment>(value, "enrollment")
                    .unwrap_err()
                    .0,
                Status::BadRequest
            );
        }
    }
}
