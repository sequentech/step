// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Optional step-up when an officer approves an enrollment: with the event's
//! `presentation.approval_step_up_policy` set to `enabled`, approving needs a
//! fresh second factor (a `gold` token authenticated less than 60 seconds
//! ago, as [`has_gold_permission`] decides). Missing, the policy is
//! `disabled` and approving works as before. Rejecting is never gated.

use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use deadpool_postgres::Transaction;
use rocket::http::Status;
use sequent_core::services::jwt::{has_gold_permission, JwtClaims};
use serde::Deserialize;
use serde_json::Value;
use tracing::warn;
use windmill::postgres::election_event::get_election_event_by_id;

/// The key in the event's presentation.
pub const APPROVAL_STEP_UP_POLICY_KEY: &str = "approval_step_up_policy";

#[allow(non_camel_case_types)]
#[derive(Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStepUpPolicy {
    #[default]
    #[serde(rename = "disabled")]
    DISABLED,
    #[serde(rename = "enabled")]
    ENABLED,
}

/// The event's policy. A missing key (or presentation) is `disabled`; a
/// value that isn't one of the policy's is `enabled`, so a mistyped setting
/// asks for the second factor rather than silently dropping it.
pub fn policy_of(presentation: Option<&Value>) -> ApprovalStepUpPolicy {
    match presentation.and_then(|value| value.get(APPROVAL_STEP_UP_POLICY_KEY)) {
        None | Some(Value::Null) => ApprovalStepUpPolicy::DISABLED,
        Some(value) => serde_json::from_value(value.clone()).unwrap_or_else(|_| {
            warn!("Unknown {APPROVAL_STEP_UP_POLICY_KEY} {value}: requiring the step-up");
            ApprovalStepUpPolicy::ENABLED
        }),
    }
}

/// Whether approving may go ahead, given whether the caller's token is a
/// fresh `gold` one.
pub fn approval_allowed(
    policy: ApprovalStepUpPolicy,
    fresh_gold: bool,
) -> bool {
    policy == ApprovalStepUpPolicy::DISABLED || fresh_gold
}

/// 403 `ApprovalStepUpRequired`: sign in again with the second factor and
/// approve within 60 seconds.
pub fn step_up_required() -> JsonError {
    ErrorResponse::new(
        Status::Forbidden,
        "Approving this enrollment needs a fresh second factor: sign in again and approve.",
        ErrorCode::ApprovalStepUpRequired,
    )
}

/// Refuses the approval when the event requires the step-up and the token
/// isn't a fresh `gold` one. Reads the event only when that could matter.
pub async fn check_approval_step_up(
    hasura_transaction: &Transaction<'_>,
    claims: &JwtClaims,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(), JsonError> {
    let fresh_gold = has_gold_permission(claims);
    if fresh_gold {
        return Ok(());
    }
    let event = get_election_event_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await
    .map_err(|error| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Error reading the election event: {error:?}"),
            ErrorCode::ElectionEventNotFound,
        )
    })?;
    if approval_allowed(policy_of(event.presentation.as_ref()), fresh_gold) {
        Ok(())
    } else {
        Err(step_up_required())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_policy_is_disabled() {
        assert_eq!(policy_of(None), ApprovalStepUpPolicy::DISABLED);
        assert_eq!(policy_of(Some(&json!({}))), ApprovalStepUpPolicy::DISABLED);
        assert_eq!(
            policy_of(Some(&json!({ "approval_step_up_policy": null }))),
            ApprovalStepUpPolicy::DISABLED
        );
        assert_eq!(
            policy_of(Some(&json!({ "approval_step_up_policy": "disabled" }))),
            ApprovalStepUpPolicy::DISABLED
        );
    }

    #[test]
    fn enabled_and_unknown_values_require_the_step_up() {
        assert_eq!(
            policy_of(Some(&json!({ "approval_step_up_policy": "enabled" }))),
            ApprovalStepUpPolicy::ENABLED
        );
        assert_eq!(
            policy_of(Some(&json!({ "approval_step_up_policy": "Enabled" }))),
            ApprovalStepUpPolicy::ENABLED
        );
        assert_eq!(
            policy_of(Some(&json!({ "approval_step_up_policy": true }))),
            ApprovalStepUpPolicy::ENABLED
        );
    }

    #[test]
    fn disabled_never_refuses() {
        assert!(approval_allowed(ApprovalStepUpPolicy::DISABLED, false));
        assert!(approval_allowed(ApprovalStepUpPolicy::DISABLED, true));
    }

    #[test]
    fn enabled_needs_a_fresh_gold_token() {
        assert!(!approval_allowed(ApprovalStepUpPolicy::ENABLED, false));
        assert!(approval_allowed(ApprovalStepUpPolicy::ENABLED, true));
    }

    #[test]
    fn refusal_is_a_403_with_its_own_code() {
        let refusal = step_up_required();
        assert_eq!(refusal.0, Status::Forbidden);
        assert_eq!(refusal.1.extensions.code, "ApprovalStepUpRequired");
    }
}
