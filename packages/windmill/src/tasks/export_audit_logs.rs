// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Writes an export of the tenant's log, or a scoped export of an election
//! event's, stores its file as a document and records the export in the log
//! it exports.

use crate::services::audit_log_export::{
    record_export, sha256_hex, write_export, AuditExportFormat, AuditExportRequest,
};
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::tasks::audit_log_outbox::kick_audit_log_outbox;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use chrono::Utc;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::pdf::PdfRenderer;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::temp_path::generate_temp_file;
use std::io::Write;
use tracing::{error, instrument};

/// The name of an export's file: what it exports and when it was written.
pub fn export_file_name(request: &AuditExportRequest, written_at: chrono::DateTime<Utc>) -> String {
    let log = match request
        .election_event_id
        .as_deref()
        .filter(|event| !event.trim().is_empty())
    {
        Some(event) => format!("logs-{event}"),
        None => "tenant-logs".to_string(),
    };
    format!(
        "{log}-{}.{}",
        written_at.format("%Y%m%dT%H%M%SZ"),
        request.format.extension()
    )
}

async fn export(request: &AuditExportRequest, document_id: &str) -> anyhow::Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting hasura client")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .context("Error starting hasura transaction")?;
    let written = write_export(&hasura_transaction, request).await?;
    let contents = match request.format {
        AuditExportFormat::Pdf => {
            let html = String::from_utf8(written.contents)
                .context("The page of a PDF export is not UTF-8")?;
            PdfRenderer::render_pdf(html, None)
                .await
                .map_err(|error| anyhow!("Error rendering the PDF export: {error:?}"))?
        }
        AuditExportFormat::Csv | AuditExportFormat::Sql => written.contents,
    };
    let name = export_file_name(request, Utc::now());
    let mut file = generate_temp_file(
        "audit-log-export",
        &format!(".{}", request.format.extension()),
    )
    .context("Error creating the export's file")?;
    file.write_all(&contents)
        .and_then(|()| file.flush())
        .context("Error writing the export's file")?;
    let path = file
        .path()
        .to_str()
        .ok_or_else(|| anyhow!("The export's file has no path"))?;
    upload_and_return_document(
        &hasura_transaction,
        path,
        u64::try_from(contents.len())?,
        request.format.media_type(),
        &request.tenant_id,
        request
            .election_event_id
            .clone()
            .filter(|event| !event.trim().is_empty()),
        &name,
        Some(document_id.to_string()),
        false,
    )
    .await
    .context("Error storing the export's file")?;
    record_export(
        &hasura_transaction,
        request,
        &written.manifest,
        &sha256_hex(&contents),
        document_id,
    )
    .await?;
    hasura_transaction
        .commit()
        .await
        .context("Error committing the export")?;
    kick_audit_log_outbox();
    Ok(())
}

#[instrument(skip(task_execution), err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_audit_logs(
    request: AuditExportRequest,
    document_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    if let Err(err) = export(&request, &document_id).await {
        let reason = format!("Failed to export the logs: {err:#}");
        if let Err(err) = update_fail(&task_execution, &reason).await {
            error!("Failed to update task execution status to FAILED: {err:?}");
        }
        return Err(Error::String(reason));
    }
    update_complete(&task_execution, Some(document_id))
        .await
        .context("Failed to update task execution status to COMPLETED")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::audit_log::AuditActor;
    use crate::services::audit_log_export::AuditExportScope;
    use chrono::TimeZone;

    #[test]
    fn an_exports_file_is_named_after_its_log_format_and_time() {
        let written_at = Utc.with_ymd_and_hms(2028, 2, 3, 4, 5, 6).unwrap();
        let mut request = AuditExportRequest {
            tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
            election_event_id: None,
            format: AuditExportFormat::Sql,
            scope: AuditExportScope::TenantLog,
            username: None,
            election_id: None,
            area_id: None,
            created_from: None,
            created_to: None,
            time_zone: None,
            requested_by: AuditActor::default(),
        };
        assert_eq!(
            export_file_name(&request, written_at),
            "tenant-logs-20280203T040506Z.sql"
        );
        request.election_event_id = Some(" ".into());
        request.format = AuditExportFormat::Pdf;
        assert_eq!(
            export_file_name(&request, written_at),
            "tenant-logs-20280203T040506Z.pdf"
        );
        request.election_event_id = Some("0d1b2c3d-4e5f-4071-8293-a4b5c6d7e8f9".into());
        request.format = AuditExportFormat::Csv;
        assert_eq!(
            export_file_name(&request, written_at),
            "logs-0d1b2c3d-4e5f-4071-8293-a4b5c6d7e8f9-20280203T040506Z.csv"
        );
    }
}
