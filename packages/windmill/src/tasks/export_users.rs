// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura;
use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::services::s3;
use crate::services::temp_path::generate_temp_file;
use crate::services::users::ListUsersFilter;
use crate::services::users::{list_users, list_users_with_vote_info};
use crate::types::error::{Error, Result};
use crate::util::aws::get_max_upload_size;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use sequent_core::services::keycloak;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use serde::{Deserialize, Serialize};
use std::io::{BufWriter, Write};
use tempfile::NamedTempFile;
use tracing::{debug, info, instrument};

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ExportUsersBody {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub election_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportUsersOutput {
    pub document_id: String,
    pub task_id: String,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_users(body: ExportUsersBody, document_id: String) -> Result<()> {
    let realm = match body.election_event_id {
        Some(ref election_event_id) => get_event_realm(&body.tenant_id, &election_event_id),
        None => get_tenant_realm(&body.tenant_id),
    };

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    let mut keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting keycloak db pool: {err}"))?;

    // we'll perform insert in a single keycloaktransaction. It either works or
    // it doesn't
    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting keycloak transaction: {err}"))?;

    let batch_size = PgConfig::from_env()?.default_sql_batch_size;

    let mut offset: i32 = 0;
    let mut total_count: Option<i32> = None;
    let file =
        generate_temp_file("export-users-", ".tsv").with_context(|| "Error creating temp file")?;
    let file2 = file
        .reopen()
        .with_context(|| "Couldn't reopen file for writing")?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_writer(&file2);

    // Define the headers for the TSV file
    let headers = if body.election_event_id.is_some() {
        vec![
            "id",
            "attributes",
            "email",
            "email_verified",
            "enabled",
            "first_name",
            "last_name",
            "username",
            "area",
            "votes_info",
        ]
    } else {
        vec![
            "id",
            "attributes",
            "email",
            "email_verified",
            "enabled",
            "first_name",
            "last_name",
            "username",
            "area",
        ]
    };
    writer.write_record(&headers)?;
    loop {
        let filter = ListUsersFilter {
            tenant_id: body.tenant_id.clone(),
            election_event_id: body.election_event_id.clone(),
            election_id: body.election_id.clone(),
            area_id: None,
            realm: realm.clone(),
            search: None,
            first_name: None,
            last_name: None,
            username: None,
            email: None,
            limit: Some(batch_size),
            offset: Some(offset),
            user_ids: None,
        };
        let (users, count) = match body.election_event_id.is_some() {
            true => list_users_with_vote_info(
                &hasura_transaction,
                &keycloak_transaction,
                filter.clone(),
            )
            .await
            .map_err(|error| anyhow!("Error listing users with vote info {error:?}"))?,
            false => list_users(&hasura_transaction, &keycloak_transaction, filter.clone())
                .await
                .map_err(|error| anyhow!("Error listing users {error:?}"))?,
        };

        if total_count.is_none() {
            total_count = Some(count);
        }
        offset += count;

        for user in users {
            // Serialize user data to TSV format and write it
            let record = if body.election_event_id.is_some() {
                vec![
                    user.id.unwrap_or_default(),
                    format!("{:?}", user.attributes),
                    user.email.unwrap_or_default(),
                    format!("{}", user.email_verified.unwrap_or_default()),
                    format!("{}", user.enabled.unwrap_or_default()),
                    user.first_name.unwrap_or_default(),
                    user.last_name.unwrap_or_default(),
                    user.username.unwrap_or_default(),
                    format!("{:?}", user.area),
                    format!("{:?}", user.votes_info),
                ]
            } else {
                vec![
                    user.id.unwrap_or_default(),
                    format!("{:?}", user.attributes),
                    user.email.unwrap_or_default(),
                    format!("{}", user.email_verified.unwrap_or_default()),
                    format!("{}", user.enabled.unwrap_or_default()),
                    user.first_name.unwrap_or_default(),
                    user.last_name.unwrap_or_default(),
                    user.username.unwrap_or_default(),
                    format!("{:?}", user.area),
                ]
            };
            writer.write_record(&record)?;
        }

        if count == 0 || offset > total_count.unwrap_or_default() {
            break;
        }
    }
    writer
        .flush()
        .with_context(|| "Error flushing CSV writter")?;

    let size = file2.metadata()?.len();
    let temp_path = file.into_temp_path();
    let name = "users-export.tsv".to_string();
    let key = s3::get_document_key(
        body.tenant_id.to_string(),
        body.election_event_id.clone().unwrap_or("".to_string()),
        document_id.clone(),
    );
    let media_type = "text/tsv".to_string();
    s3::upload_file_to_s3(
        /* key */ key,
        /* is_public */ false,
        /* s3_bucket */ s3::get_private_bucket()?,
        /* media_type */ media_type.clone(),
        /* file_path */ temp_path.to_string_lossy().to_string(),
    )
    .await
    .with_context(|| "Error uploading file to s3")?;
    temp_path
        .close()
        .with_context(|| "Error closing temp file path")?;
    if size > get_max_upload_size()? as u64 {
        return Err(anyhow!(
            "File is too big: file.metada().len() [{}] > get_max_upload_size() [{}]",
            size,
            get_max_upload_size()?
        )
        .into());
    }

    let auth_headers = keycloak::get_client_credentials()
        .await
        .map_err(|error| anyhow!("Error acquiring client credentials: {error:?}"))?;

    let _document = &hasura::document::insert_document(
        auth_headers,
        body.tenant_id.to_string(),
        body.election_event_id.clone(),
        name.clone(),
        media_type.clone(),
        size as i64,
        false,
        Some(document_id),
    )
    .await?
    .data
    .ok_or(anyhow!("expected data"))?
    .insert_sequent_backend_document
    .ok_or(anyhow!("expected document"))?
    .returning[0];
    Ok(())
}
