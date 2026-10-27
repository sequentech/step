// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Mandatory, single-use board authorisations for data-returning interfaces.
use rocket::http::Status;
use sequent_core::election_lifecycle::ElectoralPhase;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::signing::SigningAction;
use sequent_core::types::permissions::Permissions;
use serde_json::Value;
use uuid::Uuid;
use windmill::services::database::get_hasura_pool;
use windmill::services::election_lifecycle;
use windmill::services::signing::board_gate::{self, BoardGateOutcome};
use windmill::services::signing::SigningCaller;
use windmill::tasks::signing_log_outbox::kick_signing_log_outbox;

type RouteResult<T> = Result<T, (Status, String)>;

fn operation_prerequisite(message: String) -> String {
    // Hasura action errors must be JSON even when the HTTP status is a refusal.
    // Plain text would hide the operator's next step behind a webhook error.
    serde_json::json!({
        "message": message,
        "extensions": { "code": "operation-prerequisite" }
    })
    .to_string()
}

#[cfg(test)]
mod prerequisite_tests {
    use super::operation_prerequisite;

    #[test]
    fn missing_export_remains_a_readable_action_error() {
        let message = "Export the complete, current election execution data before requesting deletion";
        let body: serde_json::Value =
            serde_json::from_str(&operation_prerequisite(message.into())).unwrap();
        assert_eq!(body["message"], message);
        assert_eq!(body["extensions"]["code"], "operation-prerequisite");
        assert!(body["extensions"].get("signing_request").is_none());
    }

    #[test]
    fn action_error_escapes_quotes_and_preserves_its_reason() {
        let message = "The \"before-counting\" check is required.\nRun it again.";
        let body: serde_json::Value =
            serde_json::from_str(&operation_prerequisite(message.into())).unwrap();
        assert_eq!(body["message"], message);
    }
}

/// An authenticated caller repeats the same request after the board approves
/// it. A waiting request commits its audit record but releases no protected
/// data. The returned id is persisted with asynchronous work for worker checks.
pub async fn authorize_operation(
    claims: &JwtClaims,
    event: &str,
    action: SigningAction,
    operation: &str,
    subject: Value,
) -> RouteResult<Option<Uuid>> {
    let permission = match action {
        SigningAction::ImportElectionData => match operation {
            "import-voters" => Permissions::VOTER_CREATE,
            "configure-area" => Permissions::AREA_CREATE,
            "import-election-event" => Permissions::ELECTION_EVENT_CREATE,
            "import-schedule" => Permissions::SCHEDULED_EVENT_WRITE,
            "import-candidates" => Permissions::CANDIDATE_CREATE,
            _ => Permissions::AREA_WRITE,
        },
        SigningAction::StartTally if operation == "recount-tally" => {
            Permissions::TALLY_RECOUNT_EXECUTE
        }
        SigningAction::StartTally => Permissions::ADMIN_CEREMONY,
        SigningAction::ExportExecutionData => {
            Permissions::ELECTION_EXECUTION_EXPORT
        }
        SigningAction::DeleteExecutionData => {
            Permissions::ELECTION_EVENT_DELETE
        }
        SigningAction::ReadVotersRegister => Permissions::VOTER_READ,
        _ => {
            return Err((
                Status::BadRequest,
                "Unsupported board operation".into(),
            ))
        }
    };
    crate::services::authorization::authorize(
        claims,
        false,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![permission],
    )?;
    let caller = SigningCaller::from_claims(claims);
    board_gate::require_board(&caller)
        .map_err(|error| (Status::Forbidden, error.to_string()))?;
    let tenant = &claims.hasura_claims.tenant_id;
    let tenant_id = Uuid::parse_str(tenant).map_err(|_| {
        (Status::BadRequest, "Invalid tenant identifier".into())
    })?;
    let event_id = Uuid::parse_str(event).map_err(|_| {
        (
            Status::BadRequest,
            "Invalid election event identifier".into(),
        )
    })?;
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    let tx = client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    windmill::postgres::signing::lock_signing_event(&tx, tenant_id, event_id)
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    crate::services::authorization::authorize_election_permission_labels(
        &tx, claims, event, None,
    )
    .await?;
    let state = election_lifecycle::state(&tx, tenant, event)
        .await
        .map_err(|e| (Status::Conflict, e.to_string()))?;
    if action == SigningAction::ReadVotersRegister
        && state.phase != ElectoralPhase::PostProcessing
    {
        return Ok(None);
    }
    let required_phase = if action == SigningAction::ImportElectionData {
        ElectoralPhase::Preparation
    } else if action == SigningAction::StartTally {
        ElectoralPhase::Evaluation
    } else {
        ElectoralPhase::PostProcessing
    };
    election_lifecycle::require_phase(&tx, tenant, event, required_phase)
        .await
        .map_err(|e| (Status::Conflict, operation_prerequisite(e.to_string())))?;
    if action == SigningAction::DeleteExecutionData {
        windmill::services::board_operations::require_export(
            &tx, tenant, event,
        )
        .await
        .map_err(|e| (Status::Conflict, operation_prerequisite(e.to_string())))?;
    }
    if action == SigningAction::StartTally {
        election_lifecycle::require_counting_test(&tx, tenant, event)
            .await
            .map_err(|e| (Status::Conflict, operation_prerequisite(e.to_string())))?;
    }
    let source_revision =
        windmill::services::board_operations::source_revision(
            &tx, tenant, event,
        )
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    // One board-approved viewing session covers the register table, its count,
    // pages and voter details. Secret reveals and exports remain separate actions.
    let register_view = action == SigningAction::ReadVotersRegister
        && matches!(operation, "count-voters" | "list-voters" | "read-voter");
    if register_view {
        let existing=tx.query_opt("SELECT s.request_id FROM sequent_backend.register_read_session s JOIN sequent_backend.signing_request r ON r.id=s.request_id WHERE s.tenant_id=$1 AND s.election_event_id=$2 AND s.user_id=$3 AND s.lifecycle_generation=$4 AND s.source_revision=$5 AND s.expires_at>clock_timestamp() AND r.status='executed' AND r.required>0 AND EXISTS(SELECT 1 FROM sequent_backend.signing_rule rule WHERE rule.tenant_id=r.tenant_id AND rule.election_event_id=r.election_event_id AND rule.action=r.action AND rule.revision=r.rule_revision AND rule.requirement='required') AND (SELECT count(DISTINCT a.user_id) FROM sequent_backend.signing_approval a JOIN sequent_backend.staff_certificate c ON c.id=a.certificate_id AND c.tenant_id=a.tenant_id AND c.election_event_id=a.election_event_id WHERE a.request_id=r.id AND c.status='active' AND c.not_before<=clock_timestamp() AND c.not_after>clock_timestamp())>=r.required", &[&tenant_id,&event_id,&caller.user_id,&state.generation,&source_revision]).await
            .map_err(|e|(Status::InternalServerError,e.to_string()))?;
        if let Some(row) = existing {
            return Ok(Some(row.get(0)));
        }
    }
    let operation = if register_view {
        "view-voters-register"
    } else {
        operation
    };
    let subject = if register_view {
        serde_json::json!({"access":"register table and voter details","duration_seconds":300,"secret_values":"separate permission and approval"})
    } else {
        subject
    };
    let subject = serde_json::json!({"operation": operation, "parameters": subject,
        "lifecycle_generation": state.generation, "source_revision": source_revision,
        "permission_labels": sequent_core::services::jwt::decode_permission_labels(claims)});
    let outcome = board_gate::gate(
        &tx, &caller, tenant_id, event_id, action, operation, subject,
    )
    .await
    .map_err(|e| (Status::Forbidden, e.to_string()))?;
    if let BoardGateOutcome::Authorized(request_id) = &outcome {
        if register_view {
            tx.execute("INSERT INTO sequent_backend.register_read_session(tenant_id,election_event_id,user_id,request_id,lifecycle_generation,source_revision,expires_at) VALUES($1,$2,$3,$4,$5,$6,clock_timestamp()+interval '5 minutes') ON CONFLICT(tenant_id,election_event_id,user_id) DO UPDATE SET request_id=EXCLUDED.request_id,lifecycle_generation=EXCLUDED.lifecycle_generation,source_revision=EXCLUDED.source_revision,expires_at=EXCLUDED.expires_at", &[&tenant_id,&event_id,&caller.user_id,request_id,&state.generation,&source_revision]).await
                .map_err(|e|(Status::InternalServerError,e.to_string()))?;
        }
    }
    tx.commit()
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    kick_signing_log_outbox();
    match outcome {
        BoardGateOutcome::Authorized(request_id) => Ok(Some(request_id)),
        BoardGateOutcome::Waiting(request) => Err((Status::Conflict, serde_json::json!({
            "message": format!("Board authorisation required: {} distinct signatures for request {}.",request.required,request.code),
            "extensions": {"code":"signing-required","signing_request":request}
        }).to_string())),
    }
}

/// Persist the server-created work locator before its queue message is sent.
pub async fn bind_operation(
    claims: &JwtClaims,
    event: &str,
    request: Uuid,
    kind: windmill::services::board_operations::BindingKind,
    id: &str,
) -> RouteResult<()> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    let tx = client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    windmill::services::board_operations::bind(
        &tx,
        &claims.hasura_claims.tenant_id,
        event,
        request,
        kind,
        id,
    )
    .await
    .map_err(|e| (Status::Conflict, e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    Ok(())
}

/// Hash the bytes actually stored by the server. Neither a filename nor a
/// caller-provided checksum authorizes importing a different upload later.
pub async fn authorize_import_document(
    claims: &JwtClaims,
    event: &str,
    operation: &str,
    document_id: &str,
    parameters: Value,
) -> RouteResult<Uuid> {
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    let tx = client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    let tenant = &claims.hasura_claims.tenant_id;
    let document = windmill::postgres::document::get_document(
        &tx,
        tenant,
        None,
        document_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, e.to_string()))?
    .ok_or_else(|| {
        (Status::NotFound, "Import document not found".to_string())
    })?;
    let file = windmill::services::documents::get_document_as_temp_file(
        tenant, &document,
    )
    .await
    .map_err(|e| (Status::InternalServerError, e.to_string()))?;
    let sha256 = hex::encode(
        voting_crypto::hash::hash_sha256_file(file.path())
            .map_err(|e| (Status::InternalServerError, e.to_string()))?,
    );
    drop(tx);
    authorize_operation(claims,event,SigningAction::ImportElectionData,operation,
        serde_json::json!({"document_id":document_id,"sha256":sha256,"input":parameters})).await?
        .ok_or_else(||(Status::Forbidden,"Board authorization is required for importing election data".to_string()))
}
