// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura;
use crate::postgres::area::get_areas_by_id;
use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::services::election::{get_election_event_elections, ElectionHead};
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
use sequent_core::types::keycloak::User;
use sequent_core::util;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use tempfile::NamedTempFile;
use tracing::{debug, info, instrument};

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ExportUsersBody {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub election_id: Option<String>,
}
#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ExportAllUsersBody {
    pub tenant_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportUsersOutput {
    pub document_id: String,
    pub task_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ExportBody {
    Users {
        tenant_id: String,
        election_event_id: Option<String>,
        election_id: Option<String>,
    },
    AllUsers {
        tenant_id: String,
    },
}

fn get_headers(elections: &Option<Vec<ElectionHead>>) -> Vec<String> {
    vec![
        vec![
            "id".to_string(),
            "email".to_string(),
            "email_verified".to_string(),
            "enabled".to_string(),
            "first_name".to_string(),
            "last_name".to_string(),
            "username".to_string(),
            "telephone".to_string(),
            "area".to_string(),
        ],
        match elections {
            Some(ref some_elections) => some_elections
                .iter()
                .map(|election| match election.alias {
                    Some(ref election_alias) => election_alias.clone(),
                    None => election.name.clone(),
                })
                .collect::<Vec<String>>(),
            None => vec![],
        },
    ]
    .concat()
}

fn get_user_record(
    elections: &Option<Vec<ElectionHead>>,
    areas_by_id: &Option<HashMap<String, String>>,
    user: &User,
) -> Vec<String> {
    let votes_info_map_opt = user.get_votes_info_by_election_id();
    return vec![
        vec![
            user.id.clone().unwrap_or("-".to_string()),
            user.email.clone().unwrap_or("-".to_string()),
            format!("{}", user.email_verified.unwrap_or_default()),
            format!("{}", user.enabled.unwrap_or_default()),
            user.first_name.clone().unwrap_or("-".to_string()),
            user.last_name.clone().unwrap_or("-".to_string()),
            user.username.clone().unwrap_or("-".to_string()),
            user.get_mobile_phone().unwrap_or("-".to_string()),
            match user.get_area_id() {
                Some(ref area_id) => areas_by_id
                    .as_ref()
                    .unwrap_or(&HashMap::new())
                    .get(area_id)
                    .unwrap_or(area_id)
                    .to_string(),
                None => "-".to_string(),
            },
        ],
        match elections {
            Some(ref some_elections) => some_elections
                .iter()
                .map(|election: &ElectionHead| match votes_info_map_opt {
                    Some(ref votes_info_map) => match votes_info_map.get(&election.id) {
                        Some(ref votes_info) => votes_info.last_voted_at.clone(),
                        None => "-".to_string(),
                    },
                    None => "-".to_string(),
                })
                .collect::<Vec<String>>(),
            None => vec![],
        },
    ]
    .concat();
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_users(body: ExportBody, document_id: String) -> Result<()> {
    info!("export_users");
    let realm = match &body {
        ExportBody::Users { tenant_id, election_event_id, .. } => {
            get_event_realm(tenant_id, election_event_id.as_deref().unwrap_or(""))
        },
        ExportBody::AllUsers { tenant_id } => {
            get_tenant_realm(tenant_id)
        },
    };

    let mut hasura_db_client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    let mut keycloak_db_client = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting keycloak db pool: {err}"))?;

    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting keycloak transaction: {err}"))?;

    let (elections, areas_by_id) = match &body {
        ExportBody::Users { tenant_id, election_event_id, .. } => {
            let elections = Some(
                get_election_event_elections(
                    &hasura_transaction, 
                    tenant_id, 
                    election_event_id.as_deref().unwrap_or(""),
                )
                .await
                .with_context(|| "Error listing election event's elections")?,
            );

            let areas_by_id = Some(
                get_areas_by_id(
                    &hasura_transaction, 
                    tenant_id, 
                    election_event_id.as_deref().unwrap_or(""),
                )
                .await
                .with_context(|| "Error listing election event's areas")?,
            );

            (elections, areas_by_id)
        }
        ExportBody::AllUsers { .. } => (None, None),
    };
    info!("im here ");
    let headers = get_headers(&elections);

    let batch_size = PgConfig::from_env()?.default_sql_batch_size;
    let mut offset: i32 = 0;
    let mut total_count: Option<i32> = None;
    let file = generate_temp_file("export-users-", ".csv")
        .with_context(|| "Error creating temp file")?;
    let file2 = file
        .reopen()
        .with_context(|| "Couldn't reopen file for writing")?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b',')
        .from_writer(&file2);

    writer.write_record(&headers)?;

    loop {
        let filter = ListUsersFilter {
            tenant_id: match &body {
                ExportBody::Users { tenant_id, .. } => tenant_id.to_string(),
                ExportBody::AllUsers { tenant_id } => tenant_id.to_string(),
            },
            election_event_id: match &body {
                ExportBody::Users { election_event_id, .. } => election_event_id.clone(),
                ExportBody::AllUsers { .. } => None,
            },
            election_id: match &body {
                ExportBody::Users { election_id, .. } => election_id.clone(),
                ExportBody::AllUsers { .. } => None,
            },
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

        let (users, count) = match &body {
            ExportBody::Users { election_event_id, .. } if election_event_id.is_some() => {
                list_users_with_vote_info(
                    &hasura_transaction,
                    &keycloak_transaction,
                    filter.clone(),
                )
                .await
                .map_err(|error| anyhow!("Error listing users with vote info: {error:?}"))?
            },
            _ => {
                list_users(
                    &hasura_transaction, 
                    &keycloak_transaction, 
                    filter.clone()
                )
                .await
                .map_err(|error| anyhow!("Error listing users: {error:?}"))?
            }
        };

        if total_count.is_none() {
            total_count = Some(count);
        }

        offset += users.len() as i32;

        for user in users {
            let record = get_user_record(&elections, &areas_by_id, &user);
            writer.write_record(&record)?;
        }

        if count == 0 || offset > total_count.unwrap_or_default() {
            break;
        }
    }

    writer.flush().with_context(|| "Error flushing CSV writer")?;

    let size = file2.metadata()?.len();
    let temp_path = file.into_temp_path();
    let timestamp = util::date::timestamp().with_context(|| "Error obtaining timestamp")?;
    let name = format!("users-export-{timestamp}.csv");

    let tenant_id = match &body {
        ExportBody::AllUsers { tenant_id } => tenant_id.to_string(),
        ExportBody::Users { tenant_id, .. } => tenant_id.to_string(),
    };

    let election_event_id = match &body {
        ExportBody::Users { election_event_id, .. } => election_event_id.clone().unwrap_or_else(|| "".to_string()),
        ExportBody::AllUsers { .. } => "".to_string(),
    };

    let key = s3::get_document_key(
        &tenant_id,
        Some(&election_event_id),
        &document_id,
        &name,
    );

    let media_type = "text/csv".to_string();

    s3::upload_file_to_s3(
        key,
        false,
        s3::get_private_bucket()?,
        media_type.clone(),
        temp_path.to_string_lossy().to_string(),
        None,
    )
    .await
    .with_context(|| "Error uploading file to s3")?;

    temp_path.close().with_context(|| "Error closing temp file path")?;

    if size > get_max_upload_size()? as u64 {
        return Err(anyhow!(
            "File is too big: file.metadata().len() [{}] > get_max_upload_size() [{}]",
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
            tenant_id,
            match &body {
                ExportBody::Users { election_event_id, .. } => election_event_id.clone(),
                ExportBody::AllUsers { .. } => None,
            },
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
