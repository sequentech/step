// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::str::FromStr;

use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::tally_sheet_import::{
    TallySheetImport, TallySheetImportChangeType, TallySheetImportItem, TallySheetImportItemStatus,
    TallySheetImportSourceFormat, TallySheetImportStatus, TallySheetImportSummary,
};
use sequent_core::types::tally_sheets::VotingChannel;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

pub struct TallySheetImportWrapper(pub TallySheetImport);
pub struct TallySheetImportItemWrapper(pub TallySheetImportItem);

impl TryFrom<Row> for TallySheetImportWrapper {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        let source_format: String = row.try_get("source_format")?;
        let selected_channel: String = row.try_get("selected_channel")?;
        let status: String = row.try_get("status")?;
        let summary_value: Value = row.try_get("summary")?;
        let summary: TallySheetImportSummary = serde_json::from_value(summary_value)?;

        Ok(TallySheetImportWrapper(TallySheetImport {
            id: row.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: row.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: row.try_get::<_, Uuid>("election_event_id")?.to_string(),
            source_document_id: row.try_get::<_, Uuid>("source_document_id")?.to_string(),
            source_file_name: row.try_get("source_file_name")?,
            source_sha256: row.try_get("source_sha256")?,
            source_format: TallySheetImportSourceFormat::from_str(&source_format)
                .map_err(|err| anyhow!("Invalid import source format: {err}"))?,
            selected_channel: VotingChannel::from_str(&selected_channel)
                .map_err(|err| anyhow!("Invalid import selected channel: {err}"))?,
            status: TallySheetImportStatus::from_str(&status)
                .map_err(|err| anyhow!("Invalid import status: {err}"))?,
            created_by_user_id: row.try_get("created_by_user_id")?,
            labels: row.try_get("labels")?,
            annotations: row.try_get("annotations")?,
            summary,
            validation_report: row.try_get("validation_report")?,
            canonical_csv_sha256: row.try_get("canonical_csv_sha256")?,
        }))
    }
}

impl TryFrom<Row> for TallySheetImportItemWrapper {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        let channel: String = row.try_get("channel")?;
        let change_type: String = row.try_get("change_type")?;
        let status: String = row.try_get("status")?;

        Ok(TallySheetImportItemWrapper(TallySheetImportItem {
            id: row.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: row.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: row.try_get::<_, Uuid>("election_event_id")?.to_string(),
            import_id: row.try_get::<_, Uuid>("import_id")?.to_string(),
            election_id: row.try_get::<_, Uuid>("election_id")?.to_string(),
            area_id: row.try_get::<_, Uuid>("area_id")?.to_string(),
            contest_id: row.try_get::<_, Uuid>("contest_id")?.to_string(),
            channel: VotingChannel::from_str(&channel)
                .map_err(|err| anyhow!("Invalid import item channel: {err}"))?,
            generated_tally_sheet_id: row
                .try_get::<_, Option<Uuid>>("generated_tally_sheet_id")?
                .map(|value| value.to_string()),
            baseline_approved_tally_sheet_id: row
                .try_get::<_, Option<Uuid>>("baseline_approved_tally_sheet_id")?
                .map(|value| value.to_string()),
            baseline_approved_version: row.try_get("baseline_approved_version")?,
            baseline_content_hash: row.try_get("baseline_content_hash")?,
            incoming_content_hash: row.try_get("incoming_content_hash")?,
            change_type: TallySheetImportChangeType::from_str(&change_type)
                .map_err(|err| anyhow!("Invalid import item change type: {err}"))?,
            status: TallySheetImportItemStatus::from_str(&status)
                .map_err(|err| anyhow!("Invalid import item status: {err}"))?,
            previous_csv: row.try_get("previous_csv")?,
            incoming_csv: row.try_get("incoming_csv")?,
            source_refs: row.try_get("source_refs")?,
            validation_warnings: row.try_get("validation_warnings")?,
            labels: row.try_get("labels")?,
            annotations: row.try_get("annotations")?,
        }))
    }
}

#[instrument(err, skip_all)]
pub async fn insert_tally_sheet_import(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    source_document_id: &str,
    source_file_name: Option<&str>,
    source_sha256: Option<&str>,
    source_format: &TallySheetImportSourceFormat,
    selected_channel: &VotingChannel,
    status: &TallySheetImportStatus,
    created_by_user_id: &str,
    summary: &TallySheetImportSummary,
    validation_report: Option<&Value>,
    canonical_csv_sha256: Option<&str>,
) -> Result<TallySheetImport> {
    let id = Uuid::new_v4();
    let summary_value = serde_json::to_value(summary)?;
    let statement = transaction
        .prepare(
            r#"
            INSERT INTO sequent_backend.tally_sheet_import (
                id, tenant_id, election_event_id, source_document_id, source_file_name,
                source_sha256, source_format, selected_channel, status, created_by_user_id,
                created_at, last_updated_at, annotations, labels, summary, validation_report,
                canonical_csv_sha256
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW(), NULL, NULL, $11, $12, $13
            ) RETURNING *;
            "#,
        )
        .await?;

    let rows = transaction
        .query(
            &statement,
            &[
                &id,
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(source_document_id)?,
                &source_file_name.map(str::to_string),
                &source_sha256.map(str::to_string),
                &source_format.to_string(),
                &selected_channel.to_string(),
                &status.to_string(),
                &created_by_user_id.to_string(),
                &summary_value,
                &validation_report.cloned(),
                &canonical_csv_sha256.map(str::to_string),
            ],
        )
        .await?;

    one_import(rows)
}

#[instrument(err, skip_all)]
pub async fn insert_tally_sheet_import_item(
    transaction: &Transaction<'_>,
    import_item: &TallySheetImportItem,
) -> Result<TallySheetImportItem> {
    let source_refs = import_item.source_refs.clone();
    let validation_warnings = import_item.validation_warnings.clone();
    let annotations = import_item.annotations.clone();
    let labels = import_item.labels.clone();
    let statement = transaction
        .prepare(
            r#"
            INSERT INTO sequent_backend.tally_sheet_import_item (
                id, tenant_id, election_event_id, import_id, election_id, area_id, contest_id,
                channel, generated_tally_sheet_id, baseline_approved_tally_sheet_id,
                baseline_approved_version, baseline_content_hash, incoming_content_hash,
                change_type, status, previous_csv, incoming_csv, source_refs, validation_warnings,
                annotations, labels, created_at, last_updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, NOW(), NOW()
            ) RETURNING *;
            "#,
        )
        .await?;

    let rows = transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(&import_item.id)?,
                &parse_uuid_v4(&import_item.tenant_id)?,
                &parse_uuid_v4(&import_item.election_event_id)?,
                &parse_uuid_v4(&import_item.import_id)?,
                &parse_uuid_v4(&import_item.election_id)?,
                &parse_uuid_v4(&import_item.area_id)?,
                &parse_uuid_v4(&import_item.contest_id)?,
                &import_item.channel.to_string(),
                &import_item
                    .generated_tally_sheet_id
                    .as_ref()
                    .map(|id| parse_uuid_v4(id))
                    .transpose()?,
                &import_item
                    .baseline_approved_tally_sheet_id
                    .as_ref()
                    .map(|id| parse_uuid_v4(id))
                    .transpose()?,
                &import_item.baseline_approved_version,
                &import_item.baseline_content_hash,
                &import_item.incoming_content_hash,
                &import_item.change_type.to_string(),
                &import_item.status.to_string(),
                &import_item.previous_csv,
                &import_item.incoming_csv,
                &source_refs,
                &validation_warnings,
                &annotations,
                &labels,
            ],
        )
        .await?;

    one_import_item(rows)
}

#[instrument(err, skip_all)]
pub async fn get_tally_sheet_import_by_id(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    import_id: &str,
) -> Result<Option<TallySheetImport>> {
    let statement = transaction
        .prepare(
            r#"
            SELECT * FROM sequent_backend.tally_sheet_import
            WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3;
            "#,
        )
        .await?;
    let rows = transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(import_id)?,
            ],
        )
        .await?;
    let imports = rows
        .into_iter()
        .map(|row| {
            row.try_into()
                .map(|wrapper: TallySheetImportWrapper| wrapper.0)
        })
        .collect::<Result<Vec<TallySheetImport>>>()?;
    Ok(imports.first().cloned())
}

#[instrument(err, skip_all)]
pub async fn get_tally_sheet_import_items(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    import_id: &str,
) -> Result<Vec<TallySheetImportItem>> {
    let statement = transaction
        .prepare(
            r#"
            SELECT * FROM sequent_backend.tally_sheet_import_item
            WHERE tenant_id = $1 AND election_event_id = $2 AND import_id = $3
            ORDER BY area_id, contest_id, channel;
            "#,
        )
        .await?;
    let rows = transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(import_id)?,
            ],
        )
        .await?;
    rows.into_iter()
        .map(|row| {
            row.try_into()
                .map(|wrapper: TallySheetImportItemWrapper| wrapper.0)
        })
        .collect::<Result<Vec<TallySheetImportItem>>>()
}

#[instrument(err, skip_all)]
pub async fn update_tally_sheet_import_status(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    import_id: &str,
    status: &TallySheetImportStatus,
) -> Result<TallySheetImport> {
    let statement = transaction
        .prepare(
            r#"
            UPDATE sequent_backend.tally_sheet_import
            SET status = $4, last_updated_at = NOW()
            WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3
            RETURNING *;
            "#,
        )
        .await?;
    let rows = transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(import_id)?,
                &status.to_string(),
            ],
        )
        .await?;
    one_import(rows)
}

#[instrument(err, skip_all)]
pub async fn update_tally_sheet_import_items_status(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    import_id: &str,
    status: &TallySheetImportItemStatus,
) -> Result<Vec<TallySheetImportItem>> {
    let statement = transaction
        .prepare(
            r#"
            UPDATE sequent_backend.tally_sheet_import_item
            SET status = $4, last_updated_at = NOW()
            WHERE tenant_id = $1 AND election_event_id = $2 AND import_id = $3
            RETURNING *;
            "#,
        )
        .await?;
    let rows = transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(import_id)?,
                &status.to_string(),
            ],
        )
        .await?;

    rows.into_iter()
        .map(|row| {
            row.try_into()
                .map(|wrapper: TallySheetImportItemWrapper| wrapper.0)
        })
        .collect::<Result<Vec<TallySheetImportItem>>>()
}

#[instrument(err, skip_all)]
pub async fn update_tally_sheet_import_status_with_conflict_count(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    import_id: &str,
    status: &TallySheetImportStatus,
    conflicted_ballot_box_count: usize,
) -> Result<TallySheetImport> {
    let conflict_count = conflicted_ballot_box_count as i32;
    let statement = transaction
        .prepare(
            r#"
            UPDATE sequent_backend.tally_sheet_import
            SET
                status = $4,
                summary = jsonb_set(summary, '{conflicted_ballot_box_count}', to_jsonb($5::int), true),
                last_updated_at = NOW()
            WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3
            RETURNING *;
            "#,
        )
        .await?;
    let rows = transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(import_id)?,
                &status.to_string(),
                &conflict_count,
            ],
        )
        .await?;
    one_import(rows)
}

#[instrument(err, skip_all)]
pub async fn update_tally_sheet_import_items_status_by_ids(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    import_id: &str,
    item_ids: &[String],
    status: &TallySheetImportItemStatus,
) -> Result<Vec<TallySheetImportItem>> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }

    let parsed_item_ids = item_ids
        .iter()
        .map(|id| parse_uuid_v4(id))
        .collect::<Result<Vec<Uuid>, _>>()?;
    let statement = transaction
        .prepare(
            r#"
            UPDATE sequent_backend.tally_sheet_import_item
            SET status = $5, last_updated_at = NOW()
            WHERE tenant_id = $1 AND election_event_id = $2 AND import_id = $3 AND id = ANY($4)
            RETURNING *;
            "#,
        )
        .await?;
    let rows = transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(import_id)?,
                &parsed_item_ids,
                &status.to_string(),
            ],
        )
        .await?;

    rows.into_iter()
        .map(|row| {
            row.try_into()
                .map(|wrapper: TallySheetImportItemWrapper| wrapper.0)
        })
        .collect::<Result<Vec<TallySheetImportItem>>>()
}

fn one_import(rows: Vec<Row>) -> Result<TallySheetImport> {
    let imports = rows
        .into_iter()
        .map(|row| {
            row.try_into()
                .map(|wrapper: TallySheetImportWrapper| wrapper.0)
        })
        .collect::<Result<Vec<TallySheetImport>>>()?;

    match imports.len() {
        1 => Ok(imports[0].clone()),
        _ => Err(anyhow!(
            "Unexpected tally_sheet_import rows affected {}",
            imports.len()
        )),
    }
}

fn one_import_item(rows: Vec<Row>) -> Result<TallySheetImportItem> {
    let import_items = rows
        .into_iter()
        .map(|row| {
            row.try_into()
                .map(|wrapper: TallySheetImportItemWrapper| wrapper.0)
        })
        .collect::<Result<Vec<TallySheetImportItem>>>()?;

    match import_items.len() {
        1 => Ok(import_items[0].clone()),
        _ => Err(anyhow!(
            "Unexpected tally_sheet_import_item rows affected {}",
            import_items.len()
        )),
    }
}
