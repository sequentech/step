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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportUsersOutput {
    pub document_id: String,
    pub task_id: String,
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

    let elections = match body.election_event_id {
        Some(ref election_event_id) => Some(
            get_election_event_elections(&hasura_transaction, &body.tenant_id, &election_event_id)
                .await
                .with_context(|| "Error listing election event's elections")?,
        ),
        None => None,
    };
    let areas_by_id = match body.election_event_id {
        Some(ref election_event_id) => Some(
            get_areas_by_id(&hasura_transaction, &body.tenant_id, &election_event_id)
                .await
                .with_context(|| "Error listing election event's elections")?,
        ),
        None => None,
    };
    let headers = get_headers(&elections);

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
        offset += users.len() as i32;

        for user in users {
            // Serialize user data to TSV format and write it
            let record = get_user_record(&elections, &areas_by_id, &user);
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
    let timestamp = util::date::timestamp().with_context(|| "Error obtaining timestamp")?;
    let name = format!("users-export-{timestamp}.csv");
    let key = s3::get_document_key(
        &body.tenant_id,
        &body.election_event_id.clone().unwrap_or("".to_string()),
        &document_id,
        &name,
    );
    let media_type = "text/csv".to_string();
    s3::upload_file_to_s3(
        /* key */ key,
        /* is_public */ false,
        /* s3_bucket */ s3::get_private_bucket()?,
        /* media_type */ media_type.clone(),
        /* file_path */ temp_path.to_string_lossy().to_string(),
        /* cache_control_policy */ None,
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
