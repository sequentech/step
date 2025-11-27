// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use serde_json::value::Value;
use tokio_postgres::row::Row;
use tracing::{instrument};
use uuid::Uuid;

use crate::postgres::trustee::TrusteeWrapper;
use sequent_core::types::hasura::core::Trustee;

/// JSON path used inside trustee.annotations to store Braid WASM-related
/// metadata, including per-election key commitments.
const BRAID_WASM_KEY: &str = "braid_wasm";
const COMMITMENTS_KEY: &str = "key_commitments";

#[instrument(err, skip(hasura_transaction))]
pub async fn get_trustee(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    name: &str,
) -> Result<Trustee> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.trustee
                WHERE
                    tenant_id = $1 AND
                    name = $2;
            "#,
        )
        .await?;

    let tenant_uuid = Uuid::parse_str(tenant_id)
        .with_context(|| "Error parsing tenant_id as UUID")?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &name])
        .await?;

    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Trustee {name} not found"))?;

    let wrapper: TrusteeWrapper = row.try_into()?;
    Ok(wrapper.0)
}

#[instrument(err, skip(hasura_transaction))]
pub async fn update_trustee_annotations(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    trustee_id: &str,
    annotations: Value,
) -> Result<()> {
    let tenant_uuid: Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let trustee_uuid: Uuid =
        Uuid::parse_str(trustee_id).with_context(|| "Error parsing trustee_id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE
                "sequent_backend".trustee
            SET
                annotations = $3
            WHERE
                tenant_id = $1
                AND id = $2;
            "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &trustee_uuid, &annotations])
        .await
        .with_context(|| anyhow!("Error running the update_trustee_annotations query"))?;

    Ok(())
}

/// Record or update a PBKDF2 key commitment for the given election event and
/// trustee name inside trustee.annotations.
#[instrument(err, skip(hasura_transaction))]
pub async fn record_key_commitment(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    trustee_name: &str,
    election_event_id: &str,
    salt_b64: &str,
    iterations: i32,
    hash_b64: &str,
) -> Result<()> {
    let mut trustee = get_trustee(hasura_transaction, tenant_id, trustee_name).await?;

    let mut annotations = trustee.annotations.take().unwrap_or_else(|| Value::Object(Default::default()));

    // annotations.braid_wasm.key_commitments[election_event_id] = { salt_b64, iterations, hash_b64 }
    let obj = annotations.as_object_mut().ok_or_else(|| anyhow!("annotations is not a JSON object"))?;

    let braid_wasm = obj
        .entry(BRAID_WASM_KEY)
        .or_insert_with(|| Value::Object(Default::default()));

    let bw_obj = braid_wasm
        .as_object_mut()
        .ok_or_else(|| anyhow!("annotations.braid_wasm is not a JSON object"))?;

    let commitments = bw_obj
        .entry(COMMITMENTS_KEY)
        .or_insert_with(|| Value::Object(Default::default()));

    let commitments_obj = commitments
        .as_object_mut()
        .ok_or_else(|| anyhow!("annotations.braid_wasm.key_commitments is not a JSON object"))?;

    commitments_obj.insert(
        election_event_id.to_string(),
        serde_json::json!({
            "salt_b64": salt_b64,
            "iterations": iterations,
            "hash_b64": hash_b64,
        }),
    );

    update_trustee_annotations(
        hasura_transaction,
        tenant_id,
        &trustee.id,
        annotations,
    )
    .await
}

/// Verify a PBKDF2 key commitment for the given election event and trustee.
#[instrument(err, skip(hasura_transaction))]
pub async fn verify_key_commitment(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    trustee_name: &str,
    election_event_id: &str,
    salt_b64: &str,
    iterations: i32,
    hash_b64: &str,
) -> Result<bool> {
    let trustee = get_trustee(hasura_transaction, tenant_id, trustee_name).await?;

    let Some(mut annotations) = trustee.annotations else {
        return Ok(false);
    };

    let obj = match annotations.as_object_mut() {
        Some(o) => o,
        None => return Ok(false),
    };

    let Some(braid_wasm) = obj.get_mut(BRAID_WASM_KEY) else {
        return Ok(false);
    };

    let Some(bw_obj) = braid_wasm.as_object_mut() else {
        return Ok(false);
    };

    let Some(commitments) = bw_obj.get_mut(COMMITMENTS_KEY) else {
        return Ok(false);
    };

    let Some(commitments_obj) = commitments.as_object_mut() else {
        return Ok(false);
    };

    let Some(stored) = commitments_obj.get(election_event_id) else {
        return Ok(false);
    };

    let stored_salt = stored.get("salt_b64").and_then(|v| v.as_str()).unwrap_or("");
    let stored_hash = stored.get("hash_b64").and_then(|v| v.as_str()).unwrap_or("");
    let stored_iterations = stored
        .get("iterations")
        .and_then(|v| v.as_i64())
        .unwrap_or_default() as i32;

    Ok(stored_salt == salt_b64 && stored_hash == hash_b64 && stored_iterations == iterations)
}

