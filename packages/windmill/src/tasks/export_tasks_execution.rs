// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::services::export::export_tasks_execution::process_export;
use crate::types::error::{Error, Result};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use serde::{Deserialize, Serialize};
use std::io::{BufWriter, Write};
use tracing::{debug, info, instrument};

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
