// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura;
use crate::postgres::area::get_areas_by_id;
use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::services::election::{get_election_event_elections, ElectionHead};
use crate::services::s3;
use crate::services::tasks_execution::*;
use crate::services::temp_path::generate_temp_file;
use crate::services::users::ListUsersFilter;
use crate::services::users::{list_users, list_users_with_vote_info};
use crate::types::error::{Error, Result};
use crate::util::aws::get_max_upload_size;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use sequent_core::services::keycloak;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::types::keycloak::{User, UserProfileAttribute};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use tempfile::NamedTempFile;
use tracing::{debug, info, instrument};

pub const USER_FIELDS: [&str; 8] = [
    "id",
    "email",
    "first_name",
    "last_name",
    "username",
    "enabled",
    "email_verified",
    "area-id",
];

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ExportUsersBody {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub election_id: Option<String>,
}
#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ExportTenantUsersBody {
    pub tenant_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportUsersOutput {
    pub document_id: String,
    pub error_msg: Option<String>,
    pub task_execution: Option<TasksExecution>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ExportBody {
    Users {
        tenant_id: String,
        election_event_id: Option<String>,
        election_id: Option<String>,
    },
    TenantUsers {
        tenant_id: String,
    },
}

fn get_headers(
    elections: &Option<Vec<ElectionHead>>,
    user_attributes: &Vec<UserProfileAttribute>,
) -> Vec<String> {
    let mut user_headers: Vec<String> = vec![
        "id".to_string(),
        "email".to_string(),
        "email_verified".to_string(),
        "enabled".to_string(),
        "first_name".to_string(),
        "last_name".to_string(),
        "username".to_string(),
        "area".to_string(),
    ];
    for attr in user_attributes {
        match (&attr.name, &attr.display_name) {
            (Some(name), Some(display_name)) => {
                if (!USER_FIELDS.contains(&name.as_str())) {
                    user_headers.push(display_name.clone())
                }
            }
            _ => (),
        }
    }
    vec![
        user_headers,
        match elections {
            Some(ref some_elections) => some_elections
                .iter()
                .map(|election| match election.alias {
                    Some(ref election_alias) => format!("election: {}", election_alias.clone()),
                    None => format!("election: {}", election.name.clone()),
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
    user_attributes: &Vec<UserProfileAttribute>,
) -> Vec<String> {
    let votes_info_map_opt = user.get_votes_info_by_election_id();

    let mut user_info: Vec<String> = vec![
        user.id.clone().unwrap_or("-".to_string()),
        user.email.clone().unwrap_or("-".to_string()),
        format!("{}", user.email_verified.unwrap_or_default()),
        format!("{}", user.enabled.unwrap_or_default()),
        user.first_name.clone().unwrap_or("-".to_string()),
        user.last_name.clone().unwrap_or("-".to_string()),
        user.username.clone().unwrap_or("-".to_string()),
        match user.get_area_id() {
            Some(ref area_id) => areas_by_id
                .as_ref()
                .unwrap_or(&HashMap::new())
                .get(area_id)
                .unwrap_or(area_id)
                .to_string(),
            None => "-".to_string(),
        },
    ];
    for attr in user_attributes {
        match &attr.name {
            Some(name) => {
                if (!USER_FIELDS.contains(&name.as_str())) {
                    user_info.push(user.get_attribute_val(name).unwrap_or("-".to_string()))
                }
            }
            _ => (),
        }
    }
    return vec![
        user_info,
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
pub async fn export_users(
    body: ExportBody,
    document_id: String,
    task_execution: Option<TasksExecution>,
) -> Result<()> {
    let realm = match &body {
        ExportBody::Users {
            tenant_id,
            election_event_id,
            ..
        } => get_event_realm(tenant_id, election_event_id.as_deref().unwrap_or("")),
        ExportBody::TenantUsers { tenant_id } => get_tenant_realm(tenant_id),
    };

    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(task_execution, "Failed to get Hasura DB pool").await;
            }
            return Err(Error::String(format!(
                "Error getting Hasura DB pool: {}",
                err
            )));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(&task_execution, "Failed to start Hasura transaction").await?;
            }
            return Err(Error::String(format!(
                "Error starting Hasura transaction: {err}"
            )));
        }
    };

    let mut keycloak_db_client = match get_keycloak_pool().await.get().await {
        Ok(keycloak_db_client) => keycloak_db_client,
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(&task_execution, "Error getting keycloak db pool").await?;
            }
            return Err(Error::String(format!(
                "Error getting keycloak db pool: {err}"
            )));
        }
    };

    let keycloak_transaction = match keycloak_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(&task_execution, "Failed to start Keycloak transaction").await?;
            }
            return Err(Error::String(format!(
                "Error starting Keycloak transaction: {err}"
            )));
        }
    };

    let (elections, areas_by_id) = match &body {
        ExportBody::Users {
            tenant_id,
            election_event_id,
            ..
        } => {
            let elections = Some(
                match get_election_event_elections(
                    &hasura_transaction,
                    tenant_id,
                    election_event_id.as_deref().unwrap_or(""),
                )
                .await
                {
                    Ok(elections) => elections,
                    Err(err) => {
                        if let Some(task_execution) = &task_execution {
                            update_fail(&task_execution, "Failed to get election event elections")
                                .await?;
                        }
                        return Err(Error::String(format!(
                            "Error getting election event elections: {err}"
                        )));
                    }
                },
            );

            let areas_by_id = Some(
                match get_areas_by_id(
                    &hasura_transaction,
                    tenant_id,
                    election_event_id.as_deref().unwrap_or(""),
                )
                .await
                {
                    Ok(areas_by_id) => areas_by_id,
                    Err(err) => {
                        if let Some(task_execution) = &task_execution {
                            update_fail(&task_execution, "Failed to get election event areas")
                                .await?;
                        }
                        return Err(Error::String(format!(
                            "Error getting election event areas: {err}"
                        )));
                    }
                },
            );

            (elections, areas_by_id)
        }
        ExportBody::TenantUsers { .. } => (None, None),
    };

    let client = KeycloakAdminClient::new().await?;
    let attributes = client.get_user_profile_attributes(&realm).await?;
    let headers = get_headers(&elections, &attributes);

    let batch_size = PgConfig::from_env()?.default_sql_batch_size;
    let mut offset: i32 = 0;
    let mut total_count: Option<i32> = None;
    let file =
        generate_temp_file("export-users-", ".csv").with_context(|| "Error creating temp file")?;
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
                ExportBody::TenantUsers { tenant_id } => tenant_id.to_string(),
            },
            election_event_id: match &body {
                ExportBody::Users {
                    election_event_id, ..
                } => election_event_id.clone(),
                ExportBody::TenantUsers { .. } => None,
            },
            election_id: match &body {
                ExportBody::Users { election_id, .. } => election_id.clone(),
                ExportBody::TenantUsers { .. } => None,
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
            attributes: None,
            enabled: None,
            email_verified: None,
            sort: None,
            has_voted: None,
        };

        let (users, count) = match &body {
            ExportBody::Users {
                election_event_id, ..
            } if election_event_id.is_some() => {
                match list_users_with_vote_info(
                    &hasura_transaction,
                    &keycloak_transaction,
                    filter.clone(),
                )
                .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        if let Some(task_execution) = &task_execution {
                            update_fail(&task_execution, "Error listing users with vote info")
                                .await?;
                        }
                        return Err(Error::String(format!(
                            "Error listing users with vote info: {error:?}"
                        )));
                    }
                }
            }
            _ => {
                match list_users(&hasura_transaction, &keycloak_transaction, filter.clone()).await {
                    Ok(result) => result,
                    Err(error) => {
                        if let Some(task_execution) = &task_execution {
                            update_fail(&task_execution, "Error listing users").await?;
                        }
                        return Err(Error::String(format!("Error listing users: {error:?}")));
                    }
                }
            }
        };

        offset += users.len() as i32;

        for user in users {
            let record = get_user_record(&elections, &areas_by_id, &user, &attributes);
            writer.write_record(&record)?;
        }

        if count == 0 || offset > total_count.unwrap_or_default() {
            break;
        }
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let size = file2.metadata()?.len();
    let temp_path = file.into_temp_path();
    let timestamp = match util::date::timestamp() {
        Ok(timestamp) => timestamp,
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(&task_execution, "Failed to obtain timestamp").await?;
            }
            return Err(Error::String(format!("Error obtaining timestamp: {err}")));
        }
    };
    let name = format!("users-export-{timestamp}.csv");

    let tenant_id = match &body {
        ExportBody::TenantUsers { tenant_id } => tenant_id.to_string(),
        ExportBody::Users { tenant_id, .. } => tenant_id.to_string(),
    };

    let election_event_id = match &body {
        ExportBody::Users {
            election_event_id, ..
        } => election_event_id.clone().unwrap_or_else(|| "".to_string()),
        ExportBody::TenantUsers { .. } => "".to_string(),
    };

    let key = s3::get_document_key(&tenant_id, Some(&election_event_id), &document_id, &name);

    let media_type = "text/csv".to_string();

    match s3::upload_file_to_s3(
        key,
        false,
        s3::get_private_bucket()?,
        media_type.clone(),
        temp_path.to_string_lossy().to_string(),
        None,
    )
    .await
    {
        Ok(_) => (),
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(&task_execution, "Failed to upload file to s3").await?;
            }
            return Err(Error::String(format!("Error uploading file to s3: {err}")));
        }
    }

    temp_path
        .close()
        .with_context(|| "Error closing temp file path")?;

    if size > get_max_upload_size()? as u64 {
        if let Some(task_execution) = &task_execution {
            update_fail(
                &task_execution,
                "File is too big: file.metadata().len() [{}] > get_max_upload_size() [{}]",
            )
            .await?;
        }
        return Err(anyhow!(
            "File is too big: file.metadata().len() [{}] > get_max_upload_size() [{}]",
            size,
            get_max_upload_size()?
        )
        .into());
    }

    let auth_headers = match keycloak::get_client_credentials().await {
        Ok(auth_headers) => auth_headers,
        Err(err) => {
            if let Some(task_execution) = &task_execution {
                update_fail(&task_execution, "Error acquiring client credentials").await?;
            }
            return Err(Error::String(format!(
                "Error acquiring client credentials: {err:?}"
            )));
        }
    };

    let _document = &hasura::document::insert_document(
        auth_headers,
        tenant_id,
        match &body {
            ExportBody::Users {
                election_event_id, ..
            } => election_event_id.clone(),
            ExportBody::TenantUsers { .. } => None,
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

    if let Some(task_execution) = &task_execution {
        update_complete(&task_execution)
            .await
            .context("Failed to update task execution status to COMPLETED")?;
    }

    Ok(())
}
