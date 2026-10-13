// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Public ceremony files request server attestations; only the existing,
//! human-authenticated custody routes may complete the ceremony.

use anyhow::{anyhow, ensure, Context, Result};
use deadpool_postgres::Transaction;
use protocol_board::{
    CustodyRequest, TrusteeAction, TrusteeActionKind, TrusteeActionOutcome, TrusteeActionResult,
    TrusteeEventUpdates,
};
use sequent_core::services::jwt::JwtClaims;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::keys_ceremony::validate_custody_confirmation;
use crate::domain::tally_ceremony::{
    authorizing_trustee, check_custody_authorization_status, tally_execution_status,
};
use crate::postgres::{keys_ceremony, tally_session, tally_session_execution, trustee};
use crate::services::ceremonies::{
    keys_ceremony::custody_request, tally_ceremony::get_tally_ceremony_status,
};
use crate::services::signing::custody_proofs::tally_trustee_signatures;

const ACTION_PREFIX: &str = "trustee:server-custody:";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RetryPolicy {
    #[default]
    Keep,
    RetryFailed,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerCustodyInput {
    pub election_event_id: String,
    pub keys_ceremony_id: Option<String>,
    pub tally_session_id: Option<String>,
    pub custody_request_json: String,
    #[serde(default)]
    pub retry_policy: RetryPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredAction {
    id: String,
    request: CustodyRequest,
    requested_by: String,
    outcome: Option<TrusteeActionOutcome>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ServerCustodyReply {
    Pending,
    Ready { custody_proof_json: String },
    Failed { detail: String },
}

impl StoredAction {
    fn reply(&self) -> ServerCustodyReply {
        match &self.outcome {
            None => ServerCustodyReply::Pending,
            Some(TrusteeActionOutcome::Ready { custody_proof_json }) => ServerCustodyReply::Ready {
                custody_proof_json: custody_proof_json.clone(),
            },
            Some(TrusteeActionOutcome::Failed { detail }) => ServerCustodyReply::Failed {
                detail: detail.clone(),
            },
        }
    }
}

fn validate_file(file: &CustodyRequest, trusted: &CustodyRequest) -> Result<()> {
    let mut expected = trusted.clone();
    expected.tally_session_id = None;
    ensure!(
        file == &expected,
        "The ceremony file does not match this trustee and key ceremony"
    );
    Ok(())
}

fn validate_result(
    pending: &StoredAction,
    trusted: &CustodyRequest,
    result: &TrusteeActionResult,
) -> Result<()> {
    ensure!(
        pending.id == result.id && &pending.request == trusted && &result.request == trusted,
        "The result does not match a requested trustee action"
    );
    if let TrusteeActionOutcome::Ready { custody_proof_json } = &result.outcome {
        ensure!(
            trusted.verifies(custody_proof_json)?,
            "Invalid server custody proof"
        );
    }
    if let Some(outcome) = &pending.outcome {
        ensure!(
            outcome == &result.outcome,
            "A completed action cannot be replaced"
        );
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Target<'a> {
    Keys(&'a str),
    Tally(&'a str),
}

impl Target<'_> {
    fn id(&self) -> &str {
        match self {
            Self::Keys(id) | Self::Tally(id) => id,
        }
    }
    fn table(&self) -> &'static str {
        match self {
            Self::Keys(_) => "keys_ceremony",
            Self::Tally(_) => "tally_session",
        }
    }
}

fn target(input: &ServerCustodyInput) -> Result<Target<'_>> {
    match (&input.keys_ceremony_id, &input.tally_session_id) {
        (Some(id), None) => Ok(Target::Keys(id)),
        (None, Some(id)) => Ok(Target::Tally(id)),
        _ => Err(anyhow!("Name either a keys ceremony or a tally session")),
    }
}

fn action_key(trustee_id: &str) -> String {
    format!("{ACTION_PREFIX}{trustee_id}")
}

fn stored(annotations: &Option<Value>, key: &str) -> Result<Option<StoredAction>> {
    annotations
        .as_ref()
        .and_then(|v| v.get(key))
        .map(|value| {
            let text = value
                .as_str()
                .ok_or_else(|| anyhow!("Invalid server custody annotation"))?;
            serde_json::from_str(text).context("Invalid server custody action")
        })
        .transpose()
}

/// Load only a registered committee member's canonical request, rechecking
/// lifecycle status each time. Callers lock the target before writing.
async fn scope(
    tx: &Transaction<'_>,
    claims: &JwtClaims,
    event: &str,
    target: Target<'_>,
) -> Result<(CustodyRequest, Option<Value>)> {
    let tenant = &claims.hasura_claims.tenant_id;
    let name = claims
        .trustee
        .as_deref()
        .ok_or_else(|| anyhow!("The token carries no trustee claim"))?;
    let (ceremony, tally_id, annotations) = match target {
        Target::Keys(id) => {
            let ceremony = keys_ceremony::get_keys_ceremony_by_id(tx, tenant, event, id).await?;
            validate_custody_confirmation(&ceremony, name)?;
            let annotations = ceremony.annotations.clone();
            (ceremony, None, annotations)
        }
        Target::Tally(id) => {
            let session = tally_session::get_tally_session_by_id(tx, tenant, event, id).await?;
            check_custody_authorization_status(&tally_execution_status(
                session.execution_status.as_deref(),
            ))?;
            let head =
                tally_session_execution::get_last_tally_session_execution(tx, tenant, event, id)
                    .await?
                    .ok_or_else(|| anyhow!("The tally has no execution"))?;
            let status = get_tally_ceremony_status(head.status)?;
            let signatures = tally_trustee_signatures(tx, tenant, event, id).await?;
            authorizing_trustee(&status, name, &signatures)?;
            let ceremony = keys_ceremony::get_keys_ceremony_by_id(
                tx,
                tenant,
                event,
                &session.keys_ceremony_id,
            )
            .await?;
            (ceremony, Some(id), session.annotations)
        }
    };
    Ok((
        custody_request(tx, tenant, event, name, &ceremony, tally_id).await?,
        annotations,
    ))
}

async fn lock(tx: &Transaction<'_>, tenant: &str, event: &str, target: Target<'_>) -> Result<()> {
    let row = tx.query_opt(&format!("SELECT id FROM sequent_backend.{} WHERE tenant_id=$1 AND election_event_id=$2 AND id=$3 FOR UPDATE", target.table()),
        &[&Uuid::parse_str(tenant)?, &Uuid::parse_str(event)?, &Uuid::parse_str(target.id())?]).await?;
    ensure!(row.is_some(), "No such ceremony");
    Ok(())
}

async fn save(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    target: Target<'_>,
    key: &str,
    action: &StoredAction,
) -> Result<()> {
    // String values preserve the existing annotations format for export/import.
    let value = json!({key: serde_json::to_string(action)?});
    let count = tx.execute(&format!("UPDATE sequent_backend.{} SET annotations=COALESCE(annotations,'{{}}'::jsonb) || $4 WHERE tenant_id=$1 AND election_event_id=$2 AND id=$3", target.table()),
        &[&Uuid::parse_str(tenant)?, &Uuid::parse_str(event)?, &Uuid::parse_str(target.id())?, &value]).await?;
    ensure!(count == 1, "No such ceremony");
    Ok(())
}

/// Queue a proof, or return its result. This never advances the ceremony:
/// the operator must submit the result through the existing approval gate.
pub async fn request_server_custody(
    tx: &Transaction<'_>,
    claims: &JwtClaims,
    input: &ServerCustodyInput,
) -> Result<ServerCustodyReply> {
    let tenant = &claims.hasura_claims.tenant_id;
    let target = target(input)?;
    lock(tx, tenant, &input.election_event_id, target).await?;
    let (trusted, annotations) = scope(tx, claims, &input.election_event_id, target).await?;
    let file: CustodyRequest =
        serde_json::from_str(&input.custody_request_json).context("Invalid ceremony file")?;
    validate_file(&file, &trusted)?;
    let trustee = trustee::get_trustee_by_name(tx, tenant, &trusted.trustee_name).await?;
    let key = action_key(&trustee.id);
    if let Some(action) = stored(&annotations, &key)? {
        // An imported event may retain work scoped to its original UUIDs.
        // Only a validated current file can replace that stale request.
        if action.request == trusted
            && !(input.retry_policy == RetryPolicy::RetryFailed
                && matches!(action.outcome, Some(TrusteeActionOutcome::Failed { .. })))
        {
            return Ok(action.reply());
        }
    }
    let action = StoredAction {
        id: Uuid::new_v4().to_string(),
        request: trusted,
        requested_by: claims.hasura_claims.user_id.clone(),
        outcome: None,
    };
    save(tx, tenant, &input.election_event_id, target, &key, &action).await?;
    Ok(ServerCustodyReply::Pending)
}

/// Snapshot discovery plus pending work. Actions are replayed until their
/// result is acknowledged. A result and a human approval are separate steps.
pub async fn event_updates(
    tx: &Transaction<'_>,
    claims: &JwtClaims,
) -> Result<TrusteeEventUpdates> {
    let tenant = &claims.hasura_claims.tenant_id;
    let name = claims
        .trustee
        .as_deref()
        .ok_or_else(|| anyhow!("The token carries no trustee claim"))?;
    let trustee = trustee::get_trustee_by_name(tx, tenant, name).await?;
    let key = action_key(&trustee.id);
    let rows = tx.query("SELECT election_event_id, id, annotations, 'KEYS' AS kind FROM sequent_backend.keys_ceremony WHERE tenant_id=$1 AND annotations ? $2 AND execution_status IN ('IN_PROGRESS','SUCCESS') UNION ALL SELECT election_event_id, id, annotations, 'TALLY' AS kind FROM sequent_backend.tally_session WHERE tenant_id=$1 AND annotations ? $2 AND execution_status IN ('STARTED','CONNECTED') ORDER BY election_event_id,id",
        &[&Uuid::parse_str(tenant)?, &key]).await?;
    let mut actions = Vec::new();
    for row in rows {
        let annotations: Option<Value> = row.try_get("annotations")?;
        let Some(action) = stored(&annotations, &key)? else {
            continue;
        };
        if action.outcome.is_some() {
            continue;
        }
        let event = row.try_get::<_, Uuid>("election_event_id")?.to_string();
        let id = row.try_get::<_, Uuid>("id")?.to_string();
        let kind: String = row.try_get("kind")?;
        let target = if kind == "KEYS" {
            Target::Keys(&id)
        } else {
            Target::Tally(&id)
        };
        match scope(tx, claims, &event, target).await {
            Ok((trusted, _)) if trusted == action.request => actions.push(TrusteeAction {
                id: action.id,
                action: TrusteeActionKind::CreateCustodyProof { request: trusted },
            }),
            _ => tracing::debug!(action = %action.id, "Skipping an inactive trustee action"),
        }
    }
    let boards = crate::services::trustee_boards::list_boards(tx, tenant, Some(name))
        .await?
        .boards;
    Ok(TrusteeEventUpdates { boards, actions })
}

pub async fn submit_action_result(
    tx: &Transaction<'_>,
    claims: &JwtClaims,
    result: &TrusteeActionResult,
) -> Result<()> {
    let tenant = &claims.hasura_claims.tenant_id;
    ensure!(
        &result.request.tenant_id == tenant
            && result.request.trustee_name.as_str()
                == claims
                    .trustee
                    .as_deref()
                    .ok_or_else(|| anyhow!("The token carries no trustee claim"))?,
        "The result belongs to another trustee or tenant"
    );
    let event = &result.request.election_event_id;
    let target = match &result.request.tally_session_id {
        Some(id) => Target::Tally(id),
        None => Target::Keys(&result.request.keys_ceremony_id),
    };
    lock(tx, tenant, event, target).await?;
    let (trusted, annotations) = scope(tx, claims, event, target).await?;
    let trustee = trustee::get_trustee_by_name(tx, tenant, &trusted.trustee_name).await?;
    let key = action_key(&trustee.id);
    let mut action =
        stored(&annotations, &key)?.ok_or_else(|| anyhow!("No requested trustee action"))?;
    validate_result(&action, &trusted, result)?;
    action.outcome = Some(result.outcome.clone());
    save(tx, tenant, event, target, &key, &action).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_board::TrusteeSecrets;

    fn request() -> protocol_board::CustodyRequest {
        protocol_board::CustodyRequest {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            keys_ceremony_id: "ceremony".into(),
            tally_session_id: None,
            trustee_name: "alice".into(),
            signing_public_key: "signing".into(),
            share_encryption_public_key: "encryption".into(),
        }
    }

    #[test]
    fn the_saved_file_is_reused_but_tally_scope_comes_from_the_server() {
        let file = request();
        let mut tally = file.clone();
        tally.tally_session_id = Some("current-tally".into());
        assert!(validate_file(&file, &tally).is_ok());
        for field in [
            "tenant_id",
            "election_event_id",
            "keys_ceremony_id",
            "trustee_name",
            "signing_public_key",
            "share_encryption_public_key",
            "tally_session_id",
        ] {
            let mut changed = serde_json::to_value(&file).unwrap();
            changed[field] = serde_json::json!("wrong");
            let changed = serde_json::from_value(changed).unwrap();
            assert!(
                validate_file(&changed, &tally).is_err(),
                "accepted changed {field}"
            );
        }
    }

    #[test]
    fn results_require_the_pending_action_and_its_exact_signed_scope() {
        let secrets = TrusteeSecrets::generate();
        let keys = secrets.public_keys().unwrap();
        let mut request = request();
        request.signing_public_key = keys.signing_public_key;
        request.share_encryption_public_key = keys.share_encryption_public_key;
        request.tally_session_id = Some("current-tally".into());
        let action = StoredAction {
            id: "pending".into(),
            request: request.clone(),
            requested_by: "operator".into(),
            outcome: None,
        };
        let mut result = protocol_board::TrusteeActionResult {
            id: "pending".into(),
            request: request.clone(),
            outcome: protocol_board::TrusteeActionOutcome::Ready {
                custody_proof_json: request.attest(&secrets).unwrap(),
            },
        };
        assert!(validate_result(&action, &request, &result).is_ok());
        result.id = "unsolicited".into();
        assert!(validate_result(&action, &request, &result).is_err());
        result.id = "pending".into();
        let mut other = request.clone();
        other.tally_session_id = Some("other-tally".into());
        result.outcome = protocol_board::TrusteeActionOutcome::Ready {
            custody_proof_json: other.attest(&secrets).unwrap(),
        };
        assert!(validate_result(&action, &request, &result).is_err());
    }
}
