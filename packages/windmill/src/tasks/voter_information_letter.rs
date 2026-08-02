// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::get_document;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::documents::upload_and_return_document;
use crate::services::pdf_encryption::encrypt_pdf;
use crate::services::reports::voter_information_letter::VoterInformationLetterTemplate;
use crate::services::tasks_execution::{update_complete_with_annotations, update_fail};
use crate::services::voter_information_letter::{
    read_secret, save_secret, VoterInformationLetterSecret,
};
use crate::types::error::{Error as TaskWrapError, Result as TaskWrapResult};
use anyhow::{anyhow, Context, Result};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak::{
    get_event_realm, get_realm_password_policy, KeycloakAdminClient,
};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::temp_path::write_into_named_temp_file;
use serde_json::json;
use tracing::{error, instrument};
use uuid::Uuid;

const FAILURE_MESSAGE: &str = "Voter Information Letter generation failed";

async fn get_or_create_secret(
    tenant_id: &str,
    election_event_id: &str,
    task_id: &str,
) -> Result<VoterInformationLetterSecret> {
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to get Hasura DB client")?;
    let transaction = client
        .transaction()
        .await
        .context("Failed to start secret transaction")?;

    if let Some(secret) = read_secret(&transaction, tenant_id, election_event_id, task_id).await? {
        transaction.commit().await?;
        return Ok(secret);
    }

    let policy = get_realm_password_policy(tenant_id, election_event_id)
        .await
        .context("Failed to load the election event password policy")?;
    let secret = VoterInformationLetterSecret {
        voter_password: policy
            .generate_password()
            .context("The election event password policy is not configured or valid")?,
        pdf_password: Uuid::new_v4().simple().to_string(),
    };
    save_secret(&transaction, tenant_id, election_event_id, task_id, &secret).await?;
    transaction
        .commit()
        .await
        .context("Failed to commit Voter Information Letter secret")?;
    Ok(secret)
}

#[instrument(skip_all, err)]
async fn generate(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
    document_id: &str,
    task_execution: &TasksExecution,
) -> Result<()> {
    let secret = get_or_create_secret(tenant_id, election_event_id, &task_execution.id).await?;
    let annotations = json!({
        "document_id": document_id,
        "voter_id": voter_id,
    });

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
        update_complete_with_annotations(task_execution, annotations).await?;
        return Ok(());
    }

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
        secret.voter_password.clone(),
    );
    let pdf = report
        .render_pdf(&hasura_transaction, &keycloak_transaction)
        .await?;
    let encrypted_pdf = encrypt_pdf(&pdf, &secret.pdf_password)?;

    KeycloakAdminClient::new()
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
            Some(secret.voter_password),
            Some(false),
        )
        .await
        .context("Failed to assign the generated voter credential")?;

    let (_temporary_file, path, file_size) =
        write_into_named_temp_file(&encrypted_pdf, "voter-information-letter-", ".pdf")
            .context("Failed to create encrypted PDF temporary file")?;
    let document_name = format!("voter-information-letter-{voter_id}.pdf");
    upload_and_return_document(
        &hasura_transaction,
        &path,
        file_size,
        "application/pdf",
        tenant_id,
        Some(election_event_id.to_string()),
        &document_name,
        Some(document_id.to_string()),
        false,
    )
    .await
    .context("Failed to store encrypted Voter Information Letter")?;

    hasura_transaction
        .commit()
        .await
        .context("Failed to commit Voter Information Letter document")?;
    update_complete_with_annotations(task_execution, annotations).await?;
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
    task_execution: TasksExecution,
) -> TaskWrapResult<()> {
    if let Err(generation_error) = generate(
        &tenant_id,
        &election_event_id,
        &voter_id,
        &document_id,
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
