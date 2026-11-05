// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The saved versions of an election event's approval matrix. Versions are
//! never changed: saving adds one, and the latest decides new enrollments.

use super::evaluate::{MatrixSource, MatrixVersion};
use super::{ApprovalMatrix, BUILT_IN_VERSION};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio_postgres::row::Row;
use tokio_postgres::types::Json;
use tracing::instrument;

/// A saved version and who saved it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SavedMatrix {
    pub version: i32,
    pub matrix: ApprovalMatrix,
    pub sha256: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub created_by_username: Option<String>,
}

impl SavedMatrix {
    pub fn matrix_version(&self) -> MatrixVersion {
        MatrixVersion {
            version: self.version,
            source: MatrixSource::SAVED,
            matrix: self.matrix.clone(),
        }
    }
}

impl TryFrom<Row> for SavedMatrix {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(SavedMatrix {
            version: row.try_get("version")?,
            matrix: ApprovalMatrix {
                compared_fields: serde_json::from_value(row.try_get("compared_fields")?)
                    .context("Error reading the compared fields of an approval matrix")?,
                rules: serde_json::from_value(row.try_get("rules")?)
                    .context("Error reading the rules of an approval matrix")?,
                otherwise: serde_json::from_value(row.try_get("otherwise")?)
                    .context("Error reading the last rule of an approval matrix")?,
            },
            sha256: row.try_get("sha256")?,
            created_at: row.try_get("created_at")?,
            created_by: row.try_get("created_by")?,
            created_by_username: row.try_get("created_by_username")?,
        })
    }
}

/// Lowercase hex SHA-256 of the matrix's JSON, which names the saved
/// version in the electoral log.
pub fn matrix_sha256(matrix: &ApprovalMatrix) -> Result<String> {
    let document = serde_json::to_vec(matrix).context("Error serializing the approval matrix")?;
    Ok(hex::encode(Sha256::digest(document)))
}

/// The latest saved version of the election event's matrix.
#[instrument(err, skip(hasura_transaction))]
pub async fn get_latest_approval_matrix(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<SavedMatrix>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT version, compared_fields, rules, otherwise, sha256, created_at,
                   created_by, created_by_username
            FROM sequent_backend.approval_matrix
            WHERE tenant_id = $1 AND election_event_id = $2
            ORDER BY version DESC
            LIMIT 1
            "#,
        )
        .await?;
    hasura_transaction
        .query_opt(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await
        .context("Error reading the approval matrix")?
        .map(SavedMatrix::try_from)
        .transpose()
}

/// The matrix that decides the election event's new enrollments: the latest
/// saved version, or the built-in one comparing `fallback_fields`.
pub async fn get_current_approval_matrix(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    fallback_fields: Vec<String>,
) -> Result<MatrixVersion> {
    Ok(
        get_latest_approval_matrix(hasura_transaction, tenant_id, election_event_id)
            .await?
            .map(|saved| saved.matrix_version())
            .unwrap_or_else(|| MatrixVersion::built_in(fallback_fields)),
    )
}

/// Saves the matrix as the election event's next version. The built-in
/// matrix counts as the first version, so the first save of an event that
/// has none is the one after it. Concurrent saves can't take the same
/// version: the second fails on the unique key.
#[instrument(err, skip(hasura_transaction, matrix))]
pub async fn insert_approval_matrix(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    matrix: &ApprovalMatrix,
    created_by: Option<&str>,
    created_by_username: Option<&str>,
) -> Result<SavedMatrix> {
    let sha256 = matrix_sha256(matrix)?;
    let statement = hasura_transaction
        .prepare(
            r#"
            INSERT INTO sequent_backend.approval_matrix
            (tenant_id, election_event_id, version, compared_fields, rules, otherwise,
             sha256, created_by, created_by_username)
            SELECT $1, $2, COALESCE(MAX(version), $3) + 1, $4, $5, $6, $7, $8, $9
            FROM sequent_backend.approval_matrix
            WHERE tenant_id = $1 AND election_event_id = $2
            RETURNING version, compared_fields, rules, otherwise, sha256, created_at,
                      created_by, created_by_username
            "#,
        )
        .await?;
    let row = hasura_transaction
        .query_one(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &BUILT_IN_VERSION,
                &Json(&matrix.compared_fields),
                &Json(&matrix.rules),
                &Json(&matrix.otherwise),
                &sha256,
                &created_by,
                &created_by_username,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error saving the approval matrix: {err}"))?;
    SavedMatrix::try_from(row)
}

/// Saves the matrix an election event bundle carries as the event's first
/// version, refusing one that breaks an invariant.
#[instrument(err, skip_all)]
pub async fn import_approval_matrix(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    matrix: &serde_json::Value,
) -> Result<SavedMatrix> {
    let matrix: ApprovalMatrix = serde_json::from_value(matrix.clone())
        .context("Error reading the bundle's approval matrix")?;
    let errors = matrix.validate();
    if !errors.is_empty() {
        let listed: Vec<String> = errors.iter().map(|error| error.to_string()).collect();
        return Err(anyhow!(
            "The bundle's approval matrix is not valid: {}",
            listed.join("; ")
        ));
    }
    let statement = hasura_transaction
        .prepare(
            r#"
            INSERT INTO sequent_backend.approval_matrix
            (tenant_id, election_event_id, version, compared_fields, rules, otherwise, sha256)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING version, compared_fields, rules, otherwise, sha256, created_at,
                      created_by, created_by_username
            "#,
        )
        .await?;
    let row = hasura_transaction
        .query_one(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &BUILT_IN_VERSION,
                &Json(&matrix.compared_fields),
                &Json(&matrix.rules),
                &Json(&matrix.otherwise),
                &matrix_sha256(&matrix)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error importing the approval matrix: {err}"))?;
    SavedMatrix::try_from(row)
}
