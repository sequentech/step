// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Setting a tenant's trust in configuration packages. The setting is
//! checked before it is saved; a setting with errors is returned with them
//! and nothing is written.

use crate::routes::apply_configuration_package::PACKAGE_PERMISSIONS;
use crate::services::audit;
use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::election_config::package_verify::{TrustAnchor, TrustCheck};
use sequent_core::election_config::problem::{Code, Problem, Report};
use sequent_core::services::jwt::JwtClaims;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::instrument;
use windmill::services::audit_log::{
    self, AdminAction, AuditObject, FieldChange,
};
use windmill::services::configuration_signing::{
    read_input, save, SaveOutcome,
};
use windmill::services::database::get_hasura_pool;

/// Where the setting lives in the tenant's settings, as the audit log names it.
const SETTING_PATH: &str = "settings.configuration_signing";

#[derive(Deserialize, Debug)]
pub struct SetConfigurationSigningInput {
    tenant_id: String,
    configuration_signing: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetConfigurationSigningOutput {
    /// Whether the setting was saved.
    saved: bool,
    /// The roots it trusts and its problems: errors when it was not saved,
    /// warnings when it was.
    check: TrustCheck,
}

/// The answer while a package application or import runs on the tenant.
fn busy() -> TrustCheck {
    TrustCheck {
        anchors: Vec::new(),
        report: Report::from_problem(
            Problem::error(
                Code::InvalidValue,
                "configuration_signing",
                "a configuration package is being applied or imported on this \
                 tenant; save again when it has finished",
            )
            .id("configuration-signing.busy"),
        ),
        policy_sha256: None,
    }
}

/// Maps a failure of the database or the audit outbox to a 500 that says
/// which step failed, with the error's causes (`{:#}`), never a backtrace.
fn internal<E: std::fmt::Display>(
    context: &'static str,
) -> impl FnOnce(E) -> (Status, String) {
    move |error| (Status::InternalServerError, format!("{context}: {error:#}"))
}

/// One trusted root as the audit entry records it: public certificate data.
fn anchor_summary(anchor: &TrustAnchor) -> Value {
    json!({
        "setting": anchor.setting,
        "subject": anchor.subject,
        "fingerprint_sha256": anchor.fingerprint_sha256,
    })
}

/// Whoever may apply configuration packages to a tenant may set what those
/// packages must be signed by: the same six permissions, on that tenant.
fn authorize_setting(
    claims: &JwtClaims,
    tenant_id: &str,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(tenant_id.to_string()),
        PACKAGE_PERMISSIONS.to_vec(),
    )
}

#[instrument(skip_all)]
#[post("/set-configuration-signing", format = "json", data = "<body>")]
pub async fn set_configuration_signing(
    body: Json<SetConfigurationSigningInput>,
    claims: JwtClaims,
) -> Result<Json<SetConfigurationSigningOutput>, (Status, String)> {
    let input = body.into_inner();
    authorize_setting(&claims, &input.tenant_id)?;
    let setting = read_input(input.configuration_signing)
        .map_err(|error| (Status::BadRequest, error))?;

    let mut hasura_db_client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(internal("Error getting hasura db pool"))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(internal("Error starting hasura transaction"))?;

    let outcome = save(
        &hasura_transaction,
        &input.tenant_id,
        &setting,
        chrono::Utc::now(),
    )
    .await
    .map_err(internal("Error saving the configuration signing setting"))?;

    let (check, previous, saved) = match outcome {
        SaveOutcome::Busy => {
            return Ok(Json(SetConfigurationSigningOutput {
                saved: false,
                check: busy(),
            }));
        }
        SaveOutcome::Refused { check } => {
            // Nothing was written; the transaction is dropped.
            return Ok(Json(SetConfigurationSigningOutput {
                saved: false,
                check,
            }));
        }
        SaveOutcome::Saved {
            check,
            previous,
            saved,
        } => (check, previous, saved),
    };

    // Roots are public certificates: the entry keeps both values, and the
    // fingerprints the tenant now trusts.
    let change = audit::change(
        &claims,
        audit::target(&input.tenant_id, None)?,
        AuditObject::new("tenant", &input.tenant_id, None),
        AdminAction::Update,
    )
    .with_changes(vec![FieldChange::new(
        SETTING_PATH,
        previous.as_ref(),
        Some(&saved),
    )])
    .with_extra(json!({
        "policy": setting.policy,
        "required_approvals": setting.required_approvals,
        "anchors": check.anchors.iter().map(anchor_summary).collect::<Vec<_>>(),
    }))
    .with_description("Updated the tenant's configuration package signing");
    audit_log::stage(&hasura_transaction, &change)
        .await
        .map_err(internal("Error queueing the audit entry"))?;

    hasura_transaction
        .commit()
        .await
        .map_err(internal("Error committing hasura transaction"))?;
    windmill::tasks::audit_log_outbox::kick_audit_log_outbox();

    Ok(Json(SetConfigurationSigningOutput { saved: true, check }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::test_claims::{
        admin_claims, CALLER_TENANT_ID, OTHER_TENANT_ID,
    };
    use sequent_core::types::permissions::Permissions;

    fn claims_holding(held: &[&Permissions]) -> JwtClaims {
        let roles: Vec<String> = held.iter().map(|p| p.to_string()).collect();
        let roles: Vec<&str> = roles.iter().map(String::as_str).collect();
        admin_claims(CALLER_TENANT_ID, &roles)
    }

    #[test]
    fn only_a_package_administrator_of_that_tenant_may_set_it() {
        let all: Vec<&Permissions> = PACKAGE_PERMISSIONS.iter().collect();
        let claims = claims_holding(&all);
        assert!(authorize_setting(&claims, CALLER_TENANT_ID).is_ok());
        assert_eq!(
            authorize_setting(&claims, OTHER_TENANT_ID).unwrap_err().0,
            Status::Unauthorized
        );
        // Tenant write alone, which the other settings tabs need, is not
        // enough; nor is any five of the six.
        assert!(authorize_setting(
            &claims_holding(&[&Permissions::TENANT_WRITE]),
            CALLER_TENANT_ID
        )
        .is_err());
        for missing in &PACKAGE_PERMISSIONS {
            let held: Vec<&Permissions> =
                all.iter().copied().filter(|p| *p != missing).collect();
            assert!(
                authorize_setting(&claims_holding(&held), CALLER_TENANT_ID)
                    .is_err(),
                "{missing} is needed"
            );
        }
    }

    #[test]
    fn the_input_carries_the_setting_as_json() {
        let input: SetConfigurationSigningInput =
            serde_json::from_value(json!({
                "tenant_id": "t",
                "configuration_signing": {"policy": "required"}
            }))
            .unwrap();
        assert_eq!(input.configuration_signing["policy"], "required");
    }

    #[tokio::test]
    async fn a_setting_that_is_not_one_is_refused_before_the_database() {
        let all: Vec<&Permissions> = PACKAGE_PERMISSIONS.iter().collect();
        let refused = set_configuration_signing(
            Json(SetConfigurationSigningInput {
                tenant_id: CALLER_TENANT_ID.to_string(),
                configuration_signing: json!({"policy": 7}),
            }),
            claims_holding(&all),
        )
        .await
        .unwrap_err();
        assert_eq!(refused.0, Status::BadRequest);
    }

    #[test]
    fn a_busy_tenant_is_told_to_save_again_and_nothing_is_trusted() {
        let check = busy();
        assert!(check.anchors.is_empty());
        assert_eq!(check.policy_sha256, None);
        let ids: Vec<_> = check
            .report
            .errors()
            .map(|problem| problem.id.clone())
            .collect();
        assert_eq!(ids, vec![Some("configuration-signing.busy".to_string())]);
        assert!(check.report.has_errors());
    }

    #[test]
    fn internal_failures_name_the_step_and_the_causes() {
        let error = anyhow::anyhow!("connection reset")
            .context("Error queueing an administrative change");
        let (status, message) =
            internal("Error queueing the audit entry")(error);
        assert_eq!(status, Status::InternalServerError);
        assert_eq!(
            message,
            "Error queueing the audit entry: Error queueing an administrative \
             change: connection reset"
        );
    }

    #[test]
    fn the_audit_entry_keeps_each_roots_public_identity_only() {
        let anchor = TrustAnchor {
            setting: "package_roots".into(),
            subject: "CN=DEMO root".into(),
            not_before: "2026-01-01T00:00:00Z".into(),
            not_after: "2036-01-01T00:00:00Z".into(),
            fingerprint_sha256: "ab".repeat(32),
        };
        assert_eq!(
            anchor_summary(&anchor),
            json!({
                "setting": "package_roots",
                "subject": "CN=DEMO root",
                "fingerprint_sha256": "ab".repeat(32),
            })
        );
    }
}
