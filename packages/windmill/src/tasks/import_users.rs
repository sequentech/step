// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_areas_by_name;
use crate::postgres::document::get_document;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::documents::get_document_as_temp_file;
use crate::services::import_users::import_users_file;
use crate::services::s3;
use crate::services::tasks_execution::*;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use regex::Regex;
use ring::{digest, pbkdf2};
use rocket::futures::SinkExt as _;
use sequent_core::services::connection::AuthHeaders;
use sequent_core::services::keycloak::get_client_credentials;
use sequent_core::services::keycloak::{
    get_event_realm, get_tenant_realm, MULTIVALUE_USER_ATTRIBUTE_SEPARATOR,
};
use sequent_core::services::{keycloak, reports};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::keycloak::{
    AREA_ID_ATTR_NAME, AUTHORIZED_ELECTION_IDS_NAME, TENANT_ID_ATTR_NAME,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Seek;
use std::num::NonZeroU32;
use tempfile::NamedTempFile;
use tracing::{debug, info, instrument};

lazy_static! {
    static ref HEADER_RE: Regex = Regex::new(r"^[a-zA-Z0-9._-]+$").unwrap();
    static ref PBKDF2_ITERATIONS: NonZeroU32 = NonZeroU32::new(27_500).unwrap();
    static ref SALT_COL_NAME: String = String::from("password_salt");
    static ref HASHED_PASSWORD_COL_NAME: String = String::from("hashed_password");
    static ref PASSWORD_COL_NAME: String = String::from("password");
    static ref USERNAME_COL_NAME: String = String::from("username");
    static ref EMAIL_COL_NAME: String = String::from("email");
    static ref GROUP_COL_NAME: String = String::from("group_name");
    static ref AREA_NAME_COL_NAME: String = String::from("area_name");
    static ref RESERVED_COL_NAMES: Vec<String> = vec![
        HASHED_PASSWORD_COL_NAME.clone(),
        SALT_COL_NAME.clone(),
        PASSWORD_COL_NAME.clone(),
        GROUP_COL_NAME.clone(),
    ];
}

static PBKDF2_ALGORITHM: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;
const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
pub type Credential = [u8; CREDENTIAL_LEN];

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ImportUsersBody {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub document_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportUsersOutput {
    pub task_execution: TasksExecution,
}

fn sanitize_db_key(key: &String) -> String {
    key.replace(".", "_").replace("-", "_")
}

impl ImportUsersBody {
    #[instrument(ret)]
    async fn get_s3_document_as_temp_file(
        &self,
        hasura_transaction: &Transaction<'_>,
    ) -> anyhow::Result<(NamedTempFile, u8)> {
        let document = get_document(
            hasura_transaction,
            self.tenant_id.as_str(),
            None,
            self.document_id.as_str(),
        )
        .await
        .with_context(|| "Error obtaining the document")?
        .ok_or(anyhow!("document not found"))?;

        let s3_bucket = s3::get_private_bucket()?;
        let document_name = document.name.clone().unwrap_or_default();

        // Determine file type and set the appropriate separator
        let (postfix, separator) = if document_name.ends_with(".tsv") {
            (".tsv", b'\t')
        } else {
            (".csv", b',')
        };
        info!("postfix={postfix:?} separator={separator:?}");

        let temp_file = get_document_as_temp_file(self.tenant_id.as_str(), &document).await?;

        // Return the temporary file and the separator as a tuple
        Ok((temp_file, separator))
    }
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2)]
pub async fn import_users(body: ImportUsersBody, task_execution: TasksExecution) -> Result<()> {
    let auth_headers = get_client_credentials()
        .await
        .with_context(|| "Error obtaining keycloak client credentials")?;

    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            update_fail(&task_execution, "Failed to get Hasura DB pool").await?;
            return Err(Error::String(format!(
                "Error getting Hasura DB pool: {}",
                err
            )));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            update_fail(&task_execution, "Failed to start Hasura transaction").await?;
            return Err(Error::String(format!(
                "Error starting Hasura transaction: {err}"
            )));
        }
    };

    let (mut voters_file, separator) =
        match body.get_s3_document_as_temp_file(&hasura_transaction).await {
            Ok(result) => result,
            Err(err) => {
                update_fail(
                    &task_execution,
                    "Error obtaining voters file from S3 as temp file",
                )
                .await?;
                return Err(Error::String(format!(
                    "Error obtaining voters file from S3: {err}"
                )));
            }
        };
    voters_file.rewind()?;

    match import_users_file(
        &hasura_transaction,
        &voters_file,
        separator,
        body.election_event_id,
        body.tenant_id,
    )
    .await
    {
        Ok(_) => (),
        Err(err) => {
            update_fail(&task_execution, &err.to_string()).await?;
            return Err(Error::String(format!("Error importing users file: {err}")));
        }
    }

    update_complete(&task_execution)
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}
