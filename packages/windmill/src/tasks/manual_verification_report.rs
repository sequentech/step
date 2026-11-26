// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::{Report, ReportType};
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::reports::manual_verification::ManualVerificationTemplate;
use crate::services::reports::template_renderer::{
    get_declared_report_secret_attribute_names, GenerateReportMode, ReportOriginatedFrom,
    ReportOrigins, TemplateRenderer,
};
use crate::services::signing::actions::manual_verification::{
    claim_render, finish_render, RenderPermit,
};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, ensure, Context, Result as AnyhowResult};
use celery::error::TaskError;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip_all, err)]
pub async fn generate_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
    _mode: GenerateReportMode,
    report_clone: Option<Report>,
    may_read_secret_attributes: bool,
) -> AnyhowResult<()> {
    // This task never accepts an arbitrary report/template or recipients. The
    // private voter action is the only way to reserve a document id.
    ensure!(
        report_clone.is_none(),
        "Custom manual verification reports are not permitted"
    );
    let mut client = get_hasura_pool().await.get().await?;
    let tx = client.transaction().await?;
    let mut names: Vec<_> = get_declared_report_secret_attribute_names(
        &tx,
        tenant_id,
        election_event_id,
        &ReportType::MANUAL_VERIFICATION,
        None,
    )
    .await?
    .into_iter()
    .collect();
    names.sort();
    ensure!(
        may_read_secret_attributes == !names.is_empty(),
        "Verification template changed"
    );
    let Some(permit) = claim_render(
        &tx,
        Uuid::parse_str(tenant_id)?,
        Uuid::parse_str(election_event_id)?,
        voter_id,
        Uuid::parse_str(document_id)?,
        &names,
    )
    .await?
    else {
        // Duplicate deliveries cannot mint a fresh credential.
        return Ok(());
    };
    tx.commit().await?;
    let tx = client.transaction().await?;
    let result: AnyhowResult<()> = async {
        let mut kc = get_keycloak_pool().await.get().await?;
        let key_tx = kc.transaction().await?;
        // Read the same report definition throughout generation, including any
        // secret attribute grant; another template cannot replace it mid-render.
        tx.batch_execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .await?;
        let mut current: Vec<_> = get_declared_report_secret_attribute_names(
            &tx,
            tenant_id,
            election_event_id,
            &ReportType::MANUAL_VERIFICATION,
            None,
        )
        .await?
        .into_iter()
        .collect();
        current.sort();
        ensure!(current == names, "Verification template changed");
        let report = ManualVerificationTemplate::for_issuance(
            ReportOrigins {
                tenant_id: tenant_id.to_owned(),
                election_event_id: election_event_id.to_owned(),
                election_id: None,
                template_alias: None,
                voter_id: Some(voter_id.to_owned()),
                report_origin: ReportOriginatedFrom::ExportFunction,
                executer_username: None,
                tally_session_id: None,
            },
            RenderPermit {
                initiation: permit.initiation,
                proof: permit.proof,
                signed: permit.signed,
            },
        );
        report
            .execute_report(
                document_id,
                tenant_id,
                election_event_id,
                false,
                vec![],
                GenerateReportMode::REAL,
                None,
                &tx,
                &key_tx,
                None,
                may_read_secret_attributes,
            )
            .await?;
        tx.commit().await?;
        Ok(())
    }
    .await;
    // Keycloak's callback commits the mint while rendering on a separate
    // connection. A new transaction deliberately observes that commit.
    let final_tx = client.transaction().await?;
    finish_render(&final_tx, &permit, result.is_ok()).await?;
    final_tx.commit().await?;
    result
}

#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn generate_manual_verification_report(
    document_id: String,
    tenant_id: String,
    election_event_id: String,
    voter_id: String,
    report: Option<Report>,
    may_read_secret_attributes: bool,
) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(async move {
            generate_report(
                &document_id,
                &tenant_id,
                &election_event_id,
                &voter_id,
                GenerateReportMode::REAL,
                report,
                may_read_secret_attributes,
            )
            .await
        })
    })
    .await
    .map_err(|_| Error::from(anyhow!("Verification report task stopped")))?
    .map_err(|e| Error::from(e.context("Verification report failed")))
}
