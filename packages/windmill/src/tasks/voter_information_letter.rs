// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::get_document;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::document_password::read_password;
use crate::services::documents::upload_and_return_document_with_annotations;
use crate::services::electoral_log::{
    post_voter_password_change, ElectoralLogAdminContext, VoterPasswordChangeSource,
};
use crate::services::pdf_encryption::encrypt_pdf;
use crate::services::reports::voter_information_letter::VoterInformationLetterTemplate;
use crate::services::tasks_execution::{update_complete_with_annotations, update_fail};
use crate::types::error::{Error as TaskWrapError, Result as TaskWrapResult};
use anyhow::{anyhow, Context, Result};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak::{
    get_event_realm, get_realm_password_policy, KeycloakAdminClient,
};
use sequent_core::types::hasura::core::{DocumentAnnotations, TasksExecution};
use sequent_core::util::temp_path::write_into_named_temp_file;
use serde::Serialize;
use tracing::{error, instrument};

const FAILURE_MESSAGE: &str = "Voter Information Letter generation failed";

#[derive(Serialize)]
struct VoterInformationLetterTaskAnnotations<'a> {
    document_id: &'a str,
    voter_id: &'a str,
}

#[instrument(skip_all, err)]
async fn generate(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
    document_id: &str,
    password_secret_id: &str,
    password_change_initiator: &ElectoralLogAdminContext,
    task_execution: &TasksExecution,
) -> Result<()> {
    let task_annotations = serde_json::to_value(VoterInformationLetterTaskAnnotations {
        document_id,
        voter_id,
    })?;

    let mut hasura_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to get Hasura DB client")?;
    let hasura_transaction = hasura_client
        .transaction()
        .await
        .context("Failed to start Hasura transaction")?;

    if get_document(
        &hasura_transaction,
        tenant_id,
        Some(election_event_id.to_string()),
        document_id,
    )
    .await?
    .is_some()
    {
        hasura_transaction.commit().await?;
        update_complete_with_annotations(task_execution, task_annotations).await?;
        return Ok(());
    }

    let document_password = read_password(
        &hasura_transaction,
        tenant_id,
        Some(election_event_id),
        document_id,
        password_secret_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Document password secret is not available"))?;
    let voter_password = get_realm_password_policy(tenant_id, election_event_id)
        .await
        .context("Failed to load the election event password policy")?
        .generate_password()
        .context("The election event password policy is not configured or valid")?;

    let mut keycloak_client = get_keycloak_pool()
        .await
        .get()
        .await
        .context("Failed to get Keycloak DB client")?;
    let keycloak_transaction = keycloak_client
        .transaction()
        .await
        .context("Failed to start Keycloak transaction")?;

    let report = VoterInformationLetterTemplate::new(
        tenant_id.to_string(),
        election_event_id.to_string(),
        voter_id.to_string(),
        voter_password.clone(),
    );
    let pdf = report
        .render_pdf(&hasura_transaction, &keycloak_transaction)
        .await?;
    let encrypted_pdf = encrypt_pdf(&pdf, &document_password.password)?;

    let (_temporary_file, path, file_size) =
        write_into_named_temp_file(&encrypted_pdf, "voter-information-letter-", ".pdf")
            .context("Failed to create encrypted PDF temporary file")?;
    let document_name = format!("voter-information-letter-{voter_id}.pdf");
    let document_annotations =
        DocumentAnnotations::password_protected(password_secret_id.to_string());
    upload_and_return_document_with_annotations(
        &hasura_transaction,
        &path,
        file_size,
        "application/pdf",
        tenant_id,
        Some(election_event_id.to_string()),
        &document_name,
        Some(document_id.to_string()),
        false,
        &document_annotations,
    )
    .await
    .context("Failed to store encrypted Voter Information Letter")?;

    let voter = KeycloakAdminClient::new()
        .await
        .context("Failed to initialize Keycloak admin client")?
        .edit_user(
            &get_event_realm(tenant_id, election_event_id),
            voter_id,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(voter_password),
            Some(false),
        )
        .await
        .context("Failed to assign the generated voter credential")?;

    post_voter_password_change(
        tenant_id,
        election_event_id,
        voter_id,
        voter.username,
        password_change_initiator,
        VoterPasswordChangeSource::VoterInformationLetter,
    )
    .await
    .context("Voter credential changed, but its electoral-log entry failed")?;

    hasura_transaction
        .commit()
        .await
        .context("Failed to commit Voter Information Letter document")?;
    update_complete_with_annotations(task_execution, task_annotations).await?;
    Ok(())
}

#[instrument(skip_all, err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2)]
pub async fn generate_voter_information_letter(
    tenant_id: String,
    election_event_id: String,
    voter_id: String,
    document_id: String,
    password_secret_id: String,
    password_change_initiator: ElectoralLogAdminContext,
    task_execution: TasksExecution,
) -> TaskWrapResult<()> {
    if let Err(generation_error) = generate(
        &tenant_id,
        &election_event_id,
        &voter_id,
        &document_id,
        &password_secret_id,
        &password_change_initiator,
        &task_execution,
    )
    .await
    {
        error!(
            task_id = %task_execution.id,
            "Voter Information Letter generation failed"
        );
        update_fail(&task_execution, FAILURE_MESSAGE).await.ok();
        return Err(TaskWrapError::from(anyhow!(
            "Voter Information Letter task failed: {generation_error:#}"
        )));
    }

    Ok(())
}
