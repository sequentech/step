// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Durable link between a single-use board approval and asynchronous work.
//! Authorization is loaded from PostgreSQL, never inferred from a queue payload.
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::election_lifecycle::ElectoralPhase;
use sequent_core::signing::{canonical_json, SigningAction};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub enum BindingKind {
    Import,
    Export,
    Deletion,
    Tally,
    VoterExport,
}
impl BindingKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Export => "export",
            Self::Deletion => "deletion",
            Self::Tally => "tally",
            Self::VoterExport => "voter_export",
        }
    }
    fn action(self) -> SigningAction {
        match self {
            Self::Import => SigningAction::ImportElectionData,
            Self::Export => SigningAction::ExportExecutionData,
            Self::Deletion => SigningAction::DeleteExecutionData,
            Self::Tally => SigningAction::StartTally,
            Self::VoterExport => SigningAction::ReadVotersRegister,
        }
    }
    fn phase(self) -> ElectoralPhase {
        match self {
            Self::Import => ElectoralPhase::Preparation,
            Self::Tally => ElectoralPhase::Evaluation,
            _ => ElectoralPhase::PostProcessing,
        }
    }
}

pub async fn source_revision(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<i64> {
    Ok(tx.query_one("SELECT revision FROM sequent_backend.election_execution_revision WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid", &[&tenant,&event]).await?.try_get(0)?)
}

/// Consumed requests cannot be rebound to another task or document. Recounts
/// replace a session's binding only with a newly consumed request.
pub async fn bind(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    request: Uuid,
    kind: BindingKind,
    id: &str,
) -> Result<()> {
    let row=tx.query_opt("SELECT subject FROM sequent_backend.signing_request WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND id=$3 AND action=$4 AND status='executed' FOR UPDATE", &[&tenant,&event,&request,&kind.action().to_string()]).await?.context("No consumed board authorisation for this operation")?;
    let subject: Value = row.try_get(0)?;
    let current = crate::services::election_lifecycle::state(tx, tenant, event).await?;
    ensure!(
        subject["lifecycle_generation"].as_i64() == Some(current.generation),
        "Board authorisation belongs to an earlier lifecycle state"
    );
    crate::services::election_lifecycle::require_phase(tx, tenant, event, kind.phase()).await?;
    tx.execute("INSERT INTO sequent_backend.board_operation_binding(tenant_id,election_event_id,request_id,binding_kind,binding_id) VALUES($1::text::uuid,$2::text::uuid,$3,$4,$5::text::uuid) ON CONFLICT(tenant_id,election_event_id,binding_kind,binding_id) DO UPDATE SET request_id=EXCLUDED.request_id,created_at=clock_timestamp()", &[&tenant,&event,&request,&kind.as_str(),&id]).await?;
    Ok(())
}

/// Checks the current state and exact signed parameters before any data is read
/// or irreversible work is performed. A broker message contains only a locator.
pub async fn validate(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    kind: BindingKind,
    id: &str,
    parameters: Option<&Value>,
) -> Result<Value> {
    crate::services::election_lifecycle::require_phase(tx, tenant, event, kind.phase()).await?;
    let row=tx.query_opt("SELECT r.subject FROM sequent_backend.board_operation_binding b JOIN sequent_backend.signing_request r ON r.id=b.request_id AND r.tenant_id=b.tenant_id AND r.election_event_id=b.election_event_id WHERE b.tenant_id=$1::text::uuid AND b.election_event_id=$2::text::uuid AND b.binding_kind=$3 AND b.binding_id=$4::text::uuid AND r.action=$5 AND r.status='executed' AND EXISTS(SELECT 1 FROM sequent_backend.signing_rule rule WHERE rule.tenant_id=r.tenant_id AND rule.election_event_id=r.election_event_id AND rule.action=r.action AND rule.revision=r.rule_revision AND rule.requirement='required')", &[&tenant,&event,&kind.as_str(),&id,&kind.action().to_string()]).await?.context("A durable election-board authorisation is required")?;
    let subject: Value = row.try_get(0)?;
    let authorized:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.board_operation_binding b JOIN sequent_backend.signing_request r ON r.id=b.request_id WHERE b.tenant_id=$1::text::uuid AND b.election_event_id=$2::text::uuid AND b.binding_kind=$3 AND b.binding_id=$4::text::uuid AND r.required>0 AND (SELECT count(DISTINCT a.user_id) FROM sequent_backend.signing_approval a JOIN sequent_backend.staff_certificate c ON c.id=a.certificate_id AND c.tenant_id=a.tenant_id AND c.election_event_id=a.election_event_id WHERE a.request_id=r.id AND c.status='active' AND c.not_before<=clock_timestamp() AND c.not_after>clock_timestamp())>=r.required)", &[&tenant,&event,&kind.as_str(),&id]).await?.try_get(0)?;
    ensure!(
        authorized,
        "A required signing certificate expired or was revoked before execution"
    );
    let state = crate::services::election_lifecycle::state(tx, tenant, event).await?;
    ensure!(
        subject["lifecycle_generation"].as_i64() == Some(state.generation),
        "Board authorisation was invalidated by a lifecycle change"
    );
    if !matches!(kind, BindingKind::Tally) {
        ensure!(
            subject["source_revision"].as_i64() == Some(source_revision(tx, tenant, event).await?),
            "Election data changed after the board approved the operation"
        );
    }
    if let Some(parameters) = parameters {
        ensure!(
            canonical_json(&subject["parameters"])? == canonical_json(parameters)?,
            "Queued operation differs from the parameters approved by the board"
        );
    }
    Ok(subject)
}

pub async fn require_export(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<()> {
    let row=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.execution_export x JOIN sequent_backend.election_execution_revision v ON v.tenant_id=x.tenant_id AND v.election_event_id=x.election_event_id JOIN sequent_backend.election_lifecycle l ON l.tenant_id=x.tenant_id AND l.election_event_id=x.election_event_id WHERE x.tenant_id=$1::text::uuid AND x.election_event_id=$2::text::uuid AND l.phase='post_processing' AND l.generation=x.lifecycle_generation AND v.revision=x.source_revision)", &[&tenant,&event]).await?;
    ensure!(
        row.try_get::<_, bool>(0)?,
        "Export the complete, current election execution data before requesting deletion"
    );
    Ok(())
}

pub async fn authorize_deletion(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    task_id: &str,
) -> Result<Vec<String>> {
    let subject = validate(tx, tenant, event, BindingKind::Deletion, task_id, None).await?;
    ensure!(
        subject["source_revision"].as_i64() == Some(source_revision(tx, tenant, event).await?),
        "Election data changed after the board approved deletion"
    );
    require_export(tx, tenant, event).await?;
    let request:String=tx.query_one("SELECT request_id::text FROM sequent_backend.board_operation_binding WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND binding_kind='deletion' AND binding_id=$3::text::uuid", &[&tenant,&event,&task_id]).await?.try_get(0)?;
    crate::postgres::trusted_write::trusted_write(tx).await?;
    tx.execute(
        "SELECT set_config('sequent.authorized_deletion',$1,true)",
        &[&request],
    )
    .await?;
    let valid: bool = tx
        .query_one(
            "SELECT sequent_backend.authorized_execution_deletion($1::text::uuid,$2::text::uuid)",
            &[&tenant, &event],
        )
        .await?
        .try_get(0)?;
    ensure!(
        valid,
        "Deletion authorization or export receipt is no longer valid"
    );
    Ok(serde_json::from_value(
        subject["permission_labels"].clone(),
    )?)
}

pub async fn is_authorized_deletion(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
) -> Result<bool> {
    Ok(tx
        .query_one(
            "SELECT sequent_backend.authorized_execution_deletion($1::text::uuid,$2::text::uuid)",
            &[&tenant, &event],
        )
        .await?
        .try_get(0)?)
}

pub async fn validate_import_document(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    binding_id: &str,
    document_id: &str,
) -> Result<Value> {
    let approved = validate(tx, tenant, event, BindingKind::Import, binding_id, None).await?;
    ensure!(
        approved["parameters"]["document_id"] == document_id,
        "Import names a different document from the board-approved upload"
    );
    let document = crate::postgres::document::get_document(tx, tenant, None, document_id)
        .await?
        .context("Import document not found")?;
    let file = crate::services::documents::get_document_as_temp_file(tenant, &document).await?;
    let sha256 = hex::encode(voting_crypto::hash::hash_sha256_file(file.path())?);
    ensure!(
        approved["parameters"]["sha256"] == sha256,
        "Uploaded document changed after the board approved it"
    );
    Ok(approved)
}

pub fn verify_import_file(approved: &Value, path: &std::path::Path) -> Result<()> {
    let digest = hex::encode(voting_crypto::hash::hash_sha256_file(path)?);
    ensure!(
        approved["parameters"]["sha256"] == digest,
        "The bytes to import differ from the board-approved upload"
    );
    Ok(())
}
