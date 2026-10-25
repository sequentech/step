// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::{anyhow, ensure, Context, Result};
use deadpool_postgres::Transaction;
use protocol_board::BoardName;
use rocket::{http::Status, serde::json::Json, State};
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportVerificationTranscriptInput {
    election_event_id: String,
    protocol_board_id: String,
}

#[derive(Serialize)]
pub struct ExportVerificationTranscriptOutput {
    document_id: String,
    url: String,
}

fn failure(status: Status, message: &str) -> JsonError {
    ErrorResponse::new(
        status,
        message,
        if status == Status::Unauthorized {
            ErrorCode::Unauthorized
        } else if status == Status::NotFound {
            ErrorCode::DocumentNotFound
        } else {
            ErrorCode::InternalServerError
        },
    )
}

/// The caller chooses an existing board, never an address or object path.
/// Public evidence is returned as a new private event document. Its retrieval
/// is not a cryptographic verification verdict; independent anchors remain
/// necessary when the verifier consumes the downloaded transcript.
#[post("/export-verification-transcript", format = "json", data = "<body>")]
pub async fn export_verification_transcript(
    claims: JwtClaims,
    body: Json<ExportVerificationTranscriptInput>,
    services: &State<HarvestServices>,
) -> Result<Json<ExportVerificationTranscriptOutput>, JsonError> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_CEREMONY],
    )
    .map_err(|(status, message)| failure(status, &message))?;
    let tenant =
        parse_uuid_v4(&claims.hasura_claims.tenant_id).map_err(|_| {
            failure(Status::BadRequest, "Invalid tenant identifier")
        })?;
    let event = parse_uuid_v4(&body.election_event_id).map_err(|_| {
        failure(Status::BadRequest, "Invalid election event identifier")
    })?;
    let board = parse_uuid_v4(&body.protocol_board_id).map_err(|_| {
        failure(Status::BadRequest, "Invalid protocol board identifier")
    })?;
    let result = export(
        services,
        tenant,
        event,
        board,
        &decode_permission_labels(&claims),
    )
    .await;
    match result {
        Ok(Some(output)) => Ok(Json(output)),
        Ok(None) => Err(failure(Status::NotFound, "Protocol board not found")),
        Err(_) => {
            // Transport errors can contain presigned object URLs. Never log or
            // return their raw details, nor the transcript/document contents.
            tracing::warn!("Public transcript export failed");
            Err(failure(
                Status::InternalServerError,
                "Could not export the public transcript",
            ))
        }
    }
}

/// Resolve both names through tenant/event scoped rows, including the parent's
/// ceremony and the tally session's permission-label restriction.
async fn selected_boards(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    board: Uuid,
    labels: &[String],
) -> Result<Option<(BoardName, Option<BoardName>)>> {
    let row = tx.query_opt(
        "SELECT b.name, b.parent_id, b.tally_session_id, b.batch, b.keys_ceremony_id,
                p.name AS parent_name, t.election_ids
         FROM sequent_backend.protocol_board b
         JOIN sequent_backend.tally_session t
           ON t.id=b.tally_session_id AND t.tenant_id=b.tenant_id
          AND t.election_event_id=b.election_event_id
          AND t.keys_ceremony_id=b.keys_ceremony_id
         LEFT JOIN sequent_backend.protocol_board p
           ON p.id=b.parent_id AND p.tenant_id=b.tenant_id
          AND p.election_event_id=b.election_event_id
          AND p.keys_ceremony_id=b.keys_ceremony_id
          AND p.parent_id IS NULL AND p.tally_session_id IS NULL
         WHERE b.id=$3 AND b.tenant_id=$1 AND b.election_event_id=$2
           AND EXISTS (SELECT 1 FROM sequent_backend.keys_ceremony k
               WHERE k.id=b.keys_ceremony_id AND k.tenant_id=b.tenant_id
                 AND k.election_event_id=b.election_event_id
                 AND (k.permission_label IS NULL OR k.permission_label <@ $4))
           AND b.parent_id IS NOT NULL
           AND t.tally_type='ELECTORAL_RESULTS'
           AND t.execution_status='SUCCESS' AND t.is_execution_completed IS TRUE
           AND (t.permission_label IS NULL OR t.permission_label <@ $4)",
        &[&tenant, &event, &board, &labels],
    ).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let name = BoardName::parse(row.get::<_, &str>("name"))?;
    let parent_id: Option<Uuid> = row.get("parent_id");
    let tally_id: Option<Uuid> = row.get("tally_session_id");
    let batch: Option<i32> = row.get("batch");
    let ceremony: Uuid = row.get("keys_ceremony_id");
    ensure!(parent_id.is_some(), "tally parent missing");
    ensure!(
        tally_id.is_some() && batch.is_some(),
        "invalid tally binding"
    );
    let parent = row
        .get::<_, Option<&str>>("parent_name")
        .ok_or_else(|| anyhow!("invalid parent binding"))?;
    let parent = BoardName::parse(parent)?;
    ensure!(
        parent == BoardName::for_dkg(&ceremony),
        "invalid DKG parent name"
    );
    ensure!(
        name == BoardName::for_tally(
            &tally_id.unwrap(),
            i64::from(batch.unwrap())
        ),
        "invalid tally name"
    );
    let ids: Vec<String> = row
        .get::<_, Option<Vec<Uuid>>>("election_ids")
        .unwrap_or_default()
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    if ids.is_empty() {
        return Ok(None);
    }
    let elections = windmill::postgres::election::get_elections_by_ids(
        tx,
        &tenant.to_string(),
        &event.to_string(),
        &ids,
    )
    .await?;
    if elections.len() != ids.len() {
        return Ok(None);
    }
    for election in elections {
        if election
            .permission_label
            .as_ref()
            .is_some_and(|label| !labels.contains(label))
        {
            return Ok(None);
        }
        let Some(status) = election.status.as_ref().and_then(|value| {
            serde_json::from_value::<sequent_core::ballot::ElectionStatus>(
                value.clone(),
            )
            .ok()
        }) else {
            return Ok(None);
        };
        if !windmill::services::ceremonies::tally_validation::voting_channels_closed(&election, &status)
            .map_err(|_| anyhow!("invalid voting channel status"))? { return Ok(None); }
    }
    Ok(Some((parent, Some(name))))
}

async fn export(
    services: &HarvestServices,
    tenant: Uuid,
    event: Uuid,
    board: Uuid,
    labels: &[String],
) -> Result<Option<ExportVerificationTranscriptOutput>> {
    let mut client = services.databases.hasura().await.get().await?;
    let tx = client.transaction().await?;
    let Some((dkg, tally)) =
        selected_boards(&tx, tenant, event, board, labels).await?
    else {
        return Ok(None);
    };
    // Read transaction ends before network I/O; board rows cannot be edited by
    // the board role. Re-check exactly the selected names before saving.
    tx.commit().await?;
    let transcript = services.transcripts.fetch(&dkg, tally.as_ref()).await?;
    let bytes = serde_json::to_vec(&transcript)?;
    let tx = client.transaction().await?;
    ensure!(
        selected_boards(&tx, tenant, event, board, labels).await?
            == Some((dkg, tally)),
        "protocol board binding changed"
    );
    let name = format!("verification-{board}.json");
    let tenant = tenant.to_string();
    let event = event.to_string();
    let output = tokio::time::timeout(Duration::from_secs(30), async {
        let document = services
            .documents
            .save_json(&tx, &tenant, &event, &name, &bytes)
            .await?;
        let url = services
            .documents
            .url(&tx, &tenant, Some(&event), &document.id)
            .await?
            .context("missing export URL")?;
        Ok::<_, anyhow::Error>(ExportVerificationTranscriptOutput {
            document_id: document.id,
            url,
        })
    })
    .await
    .context("export storage timed out")??;
    tx.commit().await?;
    Ok(Some(output))
}

#[cfg(test)]
#[path = "../../tests/support/transcript_export.rs"]
mod tests;
