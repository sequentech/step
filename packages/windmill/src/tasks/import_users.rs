// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::election_event::get_election_event;
use crate::services::database::get_keycloak_pool;
use crate::services::s3;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use csv::StringRecord;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use futures::pin_mut;
use rand::prelude::*;
use regex::Regex;
use rocket::futures::SinkExt as _;
use sequent_core::services::{keycloak, reports};
use serde::{Deserialize, Serialize};
use std::fs::File;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::{ToSql, Type};
use tracing::{info, instrument};

lazy_static! {
    static ref HEADER_RE: Regex = Regex::new(r"^[a-zA-Z0-9._-]+$").unwrap();
}

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
     *  2. The SQL Query with the CREATE TEMP TABLE statement
     *  3. The SQL Query with COPY FROM statement
     *  4. The Vector with the name of the columns loaded. The columns being
     *     loaded come from the CSV file's first row.
     *  5. The vector with the types of the columns.
     *
     *  Possible Table columns:
     *  - email: string. example: "somebody@example.com"
     *  - is_enabled: (TRUE or FALSE, ignore-case). Example: "TRUE"
     *  - first_name: string. Example: "John"
     *  - last_name: string. Example: "Doe"
     *  - username: string. Example: "johndoe"
     *  - sequent.read-only.mobile-number: string. Example: "+34666777888"
     *  - area-id: string. Example "7c16620c-42aa-4129-c834-1d43b5ee012a"
     *  
     */
    fn get_copy_from_query(
        &self,
        headers: &StringRecord,
    ) -> anyhow::Result<(String, String, String, Vec<String>, Vec<Type>)> {
        let random_number: u64 = rand::random();

        let temp_table_name = format!(
            "temp_voters_{}_{}_{}",
            self.tenant_id,
            self.election_event_id.as_ref().unwrap_or(&String::new()),
            random_number
        );

        // Create the table creation query
        let create_table_query = format!(
            "CREATE TEMP TABLE {} ({});",
            temp_table_name,
            headers
                .iter()
                .map(|name| format!("{} text", name))
                .collect::<Vec<String>>()
                .join(", ")
        );

        // Create the COPY FROM STDIN query
        let copy_from_query = format!("COPY {} FROM STDIN WITH CSV HEADER;", temp_table_name);

        let column_names = headers
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let column_types = headers
            .iter()
            .map(|column_name| Type::VARCHAR)
            .collect::<Vec<Type>>();

        Ok((
            temp_table_name,
            create_table_query,
            copy_from_query,
            column_names,
            column_types,
        ))
    }

    /**
     * Insert the voters from the temporal voters table into the user_element
     * and user_attribute tables. For each user, we enter in a single query
     * (using WITH statements or similar if need be) the user in the
     * "user_entity" table and multiple user attributesin "user_attribute"
     * table.
     */
    fn get_insert_user_query(
        &self,
        voters_table: String,
        voters_table_columns: Vec<String>,
    ) -> anyhow::Result<String> {
        // Build the INSERT query for user_entity
        let user_entity_columns = vec!["email", "is_enabled", "first_name", "last_name", "username"];
        let select_columns: Vec<String> = user_entity_columns.iter().map(|&column| {
            if column == "is_enabled" {
                format!("{}::boolean", column)  // Cast is_enabled to boolean
            } else {
                column.to_string()
            }
        }).collect();
        let user_entity_query = format!(
            "INSERT INTO user_entity ({}) SELECT {} FROM {};",
            user_entity_columns.join(", "),
            select_columns.join(", "),
            voters_table
        );

        // Assuming all other columns are user attributes
        let user_attributes = voters_table_columns
            .into_iter()
            .filter(|col| !user_entity_columns.contains(&col.as_str()))
            .collect::<Vec<String>>();

        // Build a single INSERT query for user_attribute
        let user_attribute_query = if !user_attributes.is_empty() {
            let values_subquery = user_attributes
                .iter()
                .map(|attr| format!("(SELECT id, '{attr}', {attr} FROM {})", voters_table))
                .collect::<Vec<String>>()
                .join(", ");

            format!(
                "INSERT INTO user_attribute (user_id, name, value) VALUES {};",
                values_subquery
            )
        } else {
            String::new()
        };

        Ok(format!("{user_entity_query}\n{user_attribute_query}"))
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

    let voters_file = body.get_s3_document_as_temp_file().await?;
    // Read the first line of the file to get the columns
    let mut rdr = csv::Reader::from_reader(voters_file);
    let headers = rdr.headers()?.clone();

    // Validate headers
    for header in headers.iter() {
        if !HEADER_RE.is_match(header) {
            return Err(Error::from("CSV Header contains characters not allowed"));
        }
    }

    // Obtain statements
    let (
        voters_table,
        create_table_query,
        copy_from_query,
        voters_table_columns_names,
        voters_table_columns_types,
    ) = body
        .get_copy_from_query(&headers)
        .with_context(|| "Error obtaining copy_from query")?;
    let insert_user_query = body
        .get_insert_user_query(voters_table, voters_table_columns_names)
        .with_context(|| "Error obtaining insert_user_query query")?;

    // Execute the create table query
    keycloak_transaction
        .execute(create_table_query.as_str(), &[])
        .await
        .with_context(|| "Error executing create_table_query statement")?;

    // Prepare for COPY FROM STDIN
    let sink = keycloak_transaction
        .copy_in(copy_from_query.as_str())
        .await?;
    let writer = BinaryCopyInWriter::new(sink, &voters_table_columns_types);
    pin_mut!(writer);

    // Stream data from CSV to sink
    let mut owned_data: Vec<String> = Vec::new(); // To store owned string data

    for result in rdr.records() {
        let record = result?;
        owned_data.clear(); // Clear previously stored data

        for data in record.iter() {
            owned_data.push(data.to_string()); // Store owned data
        }

        let row: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = owned_data.iter()
            .map(|data| data as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        writer.as_mut().write(row.as_slice()).await?;
    }

    writer.finish().await?;

    // Complete the copy process

    // Execute the insert users query
    keycloak_transaction
        .execute(insert_user_query.as_str(), &[])
        .await?;

    // Commit the transaction
    keycloak_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;

    Ok(())
}

