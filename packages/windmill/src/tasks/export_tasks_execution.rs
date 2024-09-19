// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura;
use crate::postgres::area::get_areas_by_id;
use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::services::election::{get_election_event_elections, ElectionHead};
use crate::services::export_tasks_execution::process_export;
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
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::keycloak::{User, UserProfileAttribute};
use sequent_core::util;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use tempfile::NamedTempFile;
use tracing::{debug, info, instrument};

// pub const USER_FIELDS: [&str; 8] = [
//     "id",
//     "email",
//     "first_name",
//     "last_name",
//     "username",
//     "enabled",
//     "email_verified",
//     "area-id",
// ];

// #[derive(Deserialize, Debug, Clone, Serialize)]
// pub struct ExportUsersBody {
//     pub tenant_id: String,
//     pub election_event_id: Option<String>,
//     pub election_id: Option<String>,
// }
// #[derive(Deserialize, Debug, Clone, Serialize)]
// pub struct ExportTenantUsersBody {
//     pub tenant_id: String,
// }

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct ExportUsersOutput {
//     pub document_id: String,
//     pub error_msg: Option<String>,
//     pub task_execution: Option<TasksExecution>,
// }

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub enum ExportBody {
//     Users {
//         tenant_id: String,
//         election_event_id: Option<String>,
//         election_id: Option<String>,
//     },
//     TenantUsers {
//         tenant_id: String,
//     },
// }

// fn get_headers(
//     elections: &Option<Vec<ElectionHead>>,
//     user_attributes: &Vec<UserProfileAttribute>,
// ) -> Vec<String> {
//     let mut user_headers: Vec<String> = vec![
//         "id".to_string(),
//         "email".to_string(),
//         "email_verified".to_string(),
//         "enabled".to_string(),
//         "first_name".to_string(),
//         "last_name".to_string(),
//         "username".to_string(),
//         "area".to_string(),
//     ];
//     for attr in user_attributes {
//         match (&attr.name, &attr.display_name) {
//             (Some(name), Some(display_name)) => {
//                 if (!USER_FIELDS.contains(&name.as_str())) {
//                     user_headers.push(display_name.clone())
//                 }
//             }
//             _ => (),
//         }
//     }
//     vec![
//         user_headers,
//         match elections {
//             Some(ref some_elections) => some_elections
//                 .iter()
//                 .map(|election| match election.alias {
//                     Some(ref election_alias) => format!("election: {}", election_alias.clone()),
//                     None => format!("election: {}", election.name.clone()),
//                 })
//                 .collect::<Vec<String>>(),
//             None => vec![],
//         },
//     ]
//     .concat()
// }

// fn get_user_record(
//     elections: &Option<Vec<ElectionHead>>,
//     areas_by_id: &Option<HashMap<String, String>>,
//     user: &User,
//     user_attributes: &Vec<UserProfileAttribute>,
// ) -> Vec<String> {
//     let votes_info_map_opt = user.get_votes_info_by_election_id();

//     let mut user_info: Vec<String> = vec![
//         user.id.clone().unwrap_or("-".to_string()),
//         user.email.clone().unwrap_or("-".to_string()),
//         format!("{}", user.email_verified.unwrap_or_default()),
//         format!("{}", user.enabled.unwrap_or_default()),
//         user.first_name.clone().unwrap_or("-".to_string()),
//         user.last_name.clone().unwrap_or("-".to_string()),
//         user.username.clone().unwrap_or("-".to_string()),
//         match user.get_area_id() {
//             Some(ref area_id) => areas_by_id
//                 .as_ref()
//                 .unwrap_or(&HashMap::new())
//                 .get(area_id)
//                 .unwrap_or(area_id)
//                 .to_string(),
//             None => "-".to_string(),
//         },
//     ];
//     for attr in user_attributes {
//         match &attr.name {
//             Some(name) => {
//                 if (!USER_FIELDS.contains(&name.as_str())) {
//                     user_info.push(user.get_attribute_val(name).unwrap_or("-".to_string()))
//                 }
//             }
//             _ => (),
//         }
//     }
//     return vec![
//         user_info,
//         match elections {
//             Some(ref some_elections) => some_elections
//                 .iter()
//                 .map(|election: &ElectionHead| match votes_info_map_opt {
//                     Some(ref votes_info_map) => match votes_info_map.get(&election.id) {
//                         Some(ref votes_info) => votes_info.last_voted_at.clone(),
//                         None => "-".to_string(),
//                     },
//                     None => "-".to_string(),
//                 })
//                 .collect::<Vec<String>>(),
//             None => vec![],
//         },
//     ]
//     .concat();
// }

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_tasks_execution(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
) -> Result<()> {
    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            return Err(Error::String(format!(
                "Error getting Hasura DB pool: {}",
                err
            )));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            return Err(Error::String(format!(
                "Error starting Hasura transaction: {err}"
            )));
        }
    };

    // TODO: Process the export
    match process_export(&tenant_id, &election_event_id, &document_id).await {
        Ok(_) => (),
        Err(err) => {
            return Err(Error::String(format!(
                "Failed to export election event data: {}",
                err
            )));
        }
    }

    match hasura_transaction.commit().await {
        Ok(_) => (),
        Err(err) => {
            return Err(Error::String(format!("Commit failed: {}", err)));
        }
    };

    Ok(())
}
