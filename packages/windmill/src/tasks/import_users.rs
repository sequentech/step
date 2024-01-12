// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::election_event::get_election_event;
use crate::services::celery_app::get_celery_app;
use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use crate::services::s3;
use crate::types::error::{Error, Result};
use crate::util::aws::get_from_env_aws_config;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::services::{keycloak, reports};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::default::Default;
use std::fs::File;
use tokio_postgres::types::ToSql;
use tracing::{event, info, instrument, Level};

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ImportUsersBody {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub document_id: String,
}

impl ImportUsersBody {
    async fn get_s3_document_as_temp_file(&self) -> anyhow::Result<File> {
        let s3_bucket = s3::get_private_bucket()?;
        let document_s3_key = s3::get_document_key(
            self.tenant_id.clone(),
            self.election_event_id.clone().unwrap(),
            self.document_id.clone(),
        );
        s3::get_object_into_temp_file(s3_bucket, document_s3_key).await
    }

    /**
     * Creates a temp table and load voters from the voters_file with COPY FROM
     *
     * Notes:
     * - The temp table use the tenant_id and election_event_id in the name to
     *   avoid name clashes, and of course as it's temporal it's removed after
     *   the commit.
     *
     * The result is a tuple with:
     *  1. The name of the voters_table
     *  2. The SQL Query with COPY FROM
     *  3. The Vector with the name of the columns loaded. The columns being
     *     loaded come from the CSV file's first row.
     *
     *  Possible Table columns:
     *  - email: string. example: "somebody@example.com"
     *  - is_enabled: (TRUE or FALSE, ignore-case). Example: "TRUE"
     *  - first_name: string. Example: "John"
     *  - last_name: string. Example: "Doe"
     *  - username: string. Example: "johndoe"
     *  - attr/sequent.read-only.mobile-number: string. Example: "+34666777888"
     *  - attr/area: string. Example "Madrid"
     *  
     */
    fn get_copy_from_query(
        &self,
        voters_file: &File,
    ) -> anyhow::Result<(String, String, Vec<String>)> {
        todo!()
    }

    /**
     * Insert the voters from the temporal voters table into the user_element
     * and user_attribute tables. For each user, we enter in a single query
     * (using WITH statements or similar if need be) the user in the
     * "user_entity" table and multiple attributes (any column starting with
     * "attr/" as name) in "user_attribute" table.
     *
     * Table user_entity:
     * - id - character varying, primary key, unique
     * - email - character varying, nullable
     * - email_constraint - character varying, nullable
     * - email_verified - boolean, default: false
     * - enabled - boolean, default: false
     * - federation_link - character varying, nullable
     * - first_name - character varying, nullable
     * - last_name - character varying, nullable
     * - realm_id - character varying, nullable
     * - username - character varying, nullable
     * - created_timestamp - bigint, nullable
     *
     * Table user_attribute:
     * - name - character varying
     * - value - character varying, nullable
     * - user_id - character varying
     * - id - character varying, primary key, unique
     */
    fn get_insert_user_query(
        &self,
        voters_table: String,
        voters_table_columns: Vec<String>,
    ) -> anyhow::Result<String> {
        todo!()
    }

    async fn as_query(&self) -> anyhow::Result<(String, &[&(dyn ToSql + Sync)])> {
        let voters_file = self.get_s3_document_as_temp_file().await?;
        let (voters_table, copy_from_query, voters_table_columns) =
            self.get_copy_from_query(&voters_file)?;
        let sql_query = self.get_insert_user_query(voters_table, voters_table_columns);
        todo!()
    }
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn import_users(body: ImportUsersBody) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let _election_event = match body.election_event_id.clone() {
        None => None,
        Some(election_event_id) => {
            let event = get_election_event(
                auth_headers.clone(),
                body.tenant_id.clone(),
                election_event_id.clone(),
            )
            .await?
            .data
            .ok_or(anyhow!("Election event not found: {}", election_event_id))?
            .sequent_backend_election_event;
            if (event.is_empty()) {
                None
            } else {
                Some(event[0].clone())
            }
        }
    };

    let mut keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("{}", err))?;

    // we'll perform insert in a single transaction. It either works or it
    // doesn't
    info!("before transaction");
    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("{err}"))?;
    info!("before isolation");

    keycloak_transaction
        .simple_query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;")
        .await
        .with_context(|| "can't set transaction isolation level")?;
    info!("after isolation");

    keycloak_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;

    // TODO: generate and execute the import users sql in keycloak
    Ok(())
}
