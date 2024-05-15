// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::election_event::get_election_event;
use crate::postgres::area::get_areas_by_name;
use crate::postgres::document::get_document;
use crate::postgres::keycloak_realm;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::s3;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use base64::prelude::*;
use celery::error::TaskError;
use csv::StringRecord;
use deadpool_postgres::Transaction;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use futures::pin_mut;
use rand::prelude::*;
use rand::{thread_rng, Rng};
use regex::Regex;
use ring::{digest, pbkdf2};
use rocket::futures::SinkExt as _;
use sequent_core::services::connection::AuthHeaders;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::services::{keycloak, reports};
use sequent_core::types::keycloak::{AREA_ID_ATTR_NAME, TENANT_ID_ATTR_NAME};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Seek;
use std::num::NonZeroU32;
use tempfile::NamedTempFile;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::{ToSql, Type};
use tracing::{debug, info, instrument};

lazy_static! {
    static ref HEADER_RE: Regex = Regex::new(r"^[a-zA-Z0-9._-]+$").unwrap();
    static ref PBKDF2_ITERATIONS: NonZeroU32 = NonZeroU32::new(27_500).unwrap();
    static ref SALT_COL_NAME: String = String::from("password_salt");
    static ref HASHED_PASSWORD_COL_NAME: String = String::from("hashed_password");
    static ref PASSWORD_COL_NAME: String = String::from("password");
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
    pub id: String,
}

fn sanitize_db_key(key: &String) -> String {
    key.replace(".", "_").replace("-", "_")
}

fn hash_password(password: &String, salt: &[u8]) -> Result<String> {
    let mut output: Credential = [0u8; CREDENTIAL_LEN];
    pbkdf2::derive(
        PBKDF2_ALGORITHM,
        *PBKDF2_ITERATIONS,
        salt,
        password.as_bytes(),
        &mut output,
    );

    let generated_hash = BASE64_STANDARD.encode(&output);
    Ok(generated_hash)
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

        // Obtain the key for the document in S3
        let document_s3_key =
            s3::get_document_key(&self.tenant_id, "", &self.document_id, &document_name);

        // Retrieve the S3 object and save it to a temporary file
        let temp_file = s3::get_object_into_temp_file(
            s3_bucket.as_str(),
            document_s3_key.as_str(),
            "import-users-",
            postfix,
        )
        .await
        .with_context(|| "Failed to get S3 object into temporary file")?;

        // Return the temporary file and the separator as a tuple
        Ok((temp_file, separator))
    }

    /*
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
     *  4. The Vector with the name of the input columns loaded. The columns being
     *     loaded come from the CSV file's first row.
     *  5. The Vector with the name of the processed columns loaded. The columns being
     *     loaded come from the CSV file's first row changing the password for example.
     *  6. The vector with the types of the columns.
     *
     *  Possible Table columns:
     *  - email: string. example: "somebody@example.com"
     *  - enabled: (TRUE or FALSE, ignore-case). Example: "TRUE"
     *  - first_name: string. Example: "John"
     *  - last_name: string. Example: "Doe"
     *  - username: string. Example: "johndoe"
     *  - sequent.read-only.mobile-number: string. Example: "+34666777888"
     *  - area_name: string. Example "Area 52"
     *  - group_name: string. Example "voter"
     *  - password: string: Example "secret-password"
     */
    #[instrument(ret)]
    fn get_copy_from_query(
        &self,
        headers: &StringRecord,
    ) -> anyhow::Result<(String, String, String, Vec<String>, Vec<String>, Vec<Type>)> {
        let random_number: u64 = rand::random();

        let temp_table_name = format!("temp_voters_{}", random_number);
        let headers_vec = headers.iter().map(String::from).collect::<Vec<String>>();

        let input_column_names = headers_vec
            .iter()
            .map(|column_name| match column_name.as_str() {
                column_name if column_name == *AREA_NAME_COL_NAME => AREA_ID_ATTR_NAME.to_string(),
                _ => column_name.clone(),
            })
            .collect::<Vec<String>>();

        let processed_column_names = headers_vec
            .iter()
            .filter_map(|column_name| match column_name.as_str() {
                column_name if column_name == *AREA_NAME_COL_NAME => {
                    Some(AREA_ID_ATTR_NAME.to_string())
                }
                column_name if column_name == *PASSWORD_COL_NAME => None,
                _ => Some(column_name.clone()),
            })
            .chain(if headers_vec.contains(&PASSWORD_COL_NAME) {
                vec![SALT_COL_NAME.clone(), HASHED_PASSWORD_COL_NAME.clone()].into_iter()
            } else {
                Vec::new().into_iter()
            })
            .collect::<Vec<String>>();

        // Create the table creation query
        let create_table_query = format!(
            "CREATE TEMP TABLE {} ({});",
            temp_table_name,
            processed_column_names
                .iter()
                .map(|name| format!("{} VARCHAR", sanitize_db_key(&name.to_string())))
                .collect::<Vec<String>>()
                .join(", ")
        );

        // Create the COPY FROM STDIN query
        let copy_from_query = format!("COPY {} FROM STDIN BINARY;", temp_table_name);

        let processed_column_types = processed_column_names
            .iter()
            .map(|_column_name| Type::VARCHAR)
            .collect::<Vec<Type>>();

        Ok((
            temp_table_name,
            create_table_query,
            copy_from_query,
            input_column_names,
            processed_column_names,
            processed_column_types,
        ))
    }

    /*
     * Insert the voters from the temporal voters table into the user_element
     * and user_attribute tables. For each user, we enter in a single query
     * (using WITH statements or similar if need be) the user in the
     * "user_entity" table and multiple user attributesin "user_attribute"
     * table.
     */
    fn get_insert_user_query(
        &self,
        realm_id: String,
        voters_table: String,
        voters_table_columns: &Vec<String>,
    ) -> anyhow::Result<String> {
        // Build the INSERT query for user_entity
        let user_entity_columns = vec![
            "id",
            "email",
            "enabled",
            "first_name",
            "last_name",
            "username",
        ];
        let select_columns: Vec<String> = user_entity_columns
            .iter()
            .map(|&column| {
                match column {
                    // Cast enabled to boolean
                    "enabled" => "enabled::boolean".to_string(),
                    "id" => "gen_random_uuid()".to_string(),
                    _ => column.to_string(),
                }
            })
            .collect();
        let user_entity_query = format!(
            r#"INSERT INTO user_entity (
                    realm_id,
                    email_verified,
                    created_timestamp,
                    {}
                )
                SELECT
                    '{realm_id}',
                    true,
                    (extract(epoch from now()) * 1000)::bigint,
                    {}
                FROM
                    {}
                RETURNING *"#,
            user_entity_columns.join(", "),
            select_columns.join(", "),
            voters_table,
        );

        // Assume all other columns are user attributes
        let user_attributes = voters_table_columns
            .clone()
            .into_iter()
            .filter(|col| {
                !user_entity_columns.contains(&col.as_str()) && !RESERVED_COL_NAMES.contains(&col)
            })
            .collect::<Vec<String>>();

        // Build a single INSERT query for all user_attribute elements
        let user_attribute_query = if !user_attributes.is_empty() {
            let values_subquery = user_attributes
                .iter()
                .map(|attr| {
                    let sanitized_attr = sanitize_db_key(attr);
                    format!(
                        r#"
                        SELECT
                            gen_random_uuid(),
                            nu.id,
                            '{attr}',
                            v.{sanitized_attr}
                        FROM
                            {voters_table} v
                        JOIN
                            new_user nu ON
                                nu.username = v.username
                        "#
                    )
                })
                .collect::<Vec<String>>()
                .join(" UNION ALL ");

            format!(
                r#"
                INSERT
                INTO user_attribute (id, user_id, name, value)
                {values_subquery}
                UNION ALL
                SELECT
                    gen_random_uuid(),
                    nu.id,
                    '{TENANT_ID_ATTR_NAME}',
                    '{}'
                FROM
                    new_user nu
                "#,
                self.tenant_id,
            )
        } else {
            String::new()
        };

        let group_col_name = &*GROUP_COL_NAME;
        let group_query = if voters_table_columns.contains(group_col_name) {
            format!(
                r#",
                pre_user_group AS (
                    SELECT
                        kg.id AS group_id,
                        nu.id AS user_id
                    FROM
                        {voters_table} v
                    JOIN
                        new_user nu ON
                            nu.username = v.username
                    JOIN
                        keycloak_group kg ON
                            kg.name = v.{group_col_name}
                            AND kg.realm_id = '{realm_id}'
                ),
                user_group AS (
                    INSERT 
                    INTO user_group_membership (
                        group_id,
                        user_id
                    )
                    SELECT
                        pug.group_id,
                        pug.user_id
                    FROM pre_user_group pug
                )
                "#
            )
        } else {
            String::new()
        };

        // Inserts password credentials if need be
        let salt_col_name = &*SALT_COL_NAME;
        let hashed_password_col_name = &*HASHED_PASSWORD_COL_NAME;
        let num_iterations = &*PBKDF2_ITERATIONS;
        let credentials_query = if voters_table_columns.contains(hashed_password_col_name) {
            format!(
                r#",
                pre_credentials AS (
                    SELECT
                        v.{salt_col_name} AS salt,
                        v.{hashed_password_col_name} AS hashed_password,
                        nu.id AS id
                    FROM
                        {voters_table} v
                    JOIN
                        new_user nu ON
                            nu.username = v.username
                ),
                credentials AS (
                    INSERT 
                    INTO credential (
                        id,
                        type,
                        user_id,
                        created_date,
                        user_label,
                        secret_data,
                        credential_data,
                        priority
                    )
                    SELECT
                        gen_random_uuid(),
                        'password',
                        pc.id,
                        (extract(epoch from now()) * 1000)::bigint,
                        'My password',
                        json_build_object(
                            'value', pc.hashed_password,
                            'salt', pc.salt
                        )::text,
                        json_build_object(
                            'hashIterations', {num_iterations},
                            'algorithm', 'pbkdf2-sha256',
                            'additionalParameters', json_build_object()
                        )::text,
                        10
                    FROM pre_credentials pc
                )
                "#
            )
        } else {
            String::new()
        };

        let ret = format!(
            r#"
            WITH 
                new_user AS (
                    {user_entity_query}
                )
                {credentials_query}
                {group_query}
            {user_attribute_query};
            "#
        );
        info!("ret = {ret}");
        Ok(ret)
    }
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2)]
pub async fn import_users(body: ImportUsersBody) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials()
        .await
        .with_context(|| "Error obtaining keycloak client credentials")?;
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

    keycloak_transaction
        .simple_query(
            r#"
            SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;
            "#,
        )
        .await
        .with_context(|| "can't set transaction isolation level or encoding")?;

    let areas_map = match body.election_event_id {
        Some(ref election_event_id) => Some(
            get_areas_by_name(
                &hasura_transaction,
                body.tenant_id.as_str(),
                election_event_id.as_str(),
            )
            .await
            .with_context(|| "error retrieving areas")?,
        ),
        None => None,
    };

    let (mut voters_file, separator) = body
        .get_s3_document_as_temp_file(&hasura_transaction)
        .await
        .with_context(|| "Error obtaining voters file from S3 as temp file")?;
    voters_file.rewind()?;
    // Read the first line of the file to get the columns
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(separator)
        .from_reader(voters_file);
    let headers = rdr
        .headers()
        .with_context(|| "Error reading CSV headers from voters file")?
        .clone();

    // Validate headers
    info!("headers: {headers:?}");
    for header in headers.iter() {
        if !HEADER_RE.is_match(header) {
            return Err(Error::from(format!(
                "CSV Header contains characters not allowed: {header}"
            )));
        }
    }

    // Obtain statements
    let (
        voters_table,
        create_table_query,
        copy_from_query,
        voters_table_input_columns_names,
        voters_table_processed_columns_names,
        voters_table_processed_columns_types,
    ) = body
        .get_copy_from_query(&headers)
        .with_context(|| "Error obtaining copy_from query")?;

    let realm_name = match body.election_event_id {
        Some(ref event_id) => get_event_realm(body.tenant_id.as_str(), event_id.as_str()),
        None => get_tenant_realm(body.tenant_id.as_str()),
    };
    let realm_id = keycloak_realm::get_realm_id(&keycloak_transaction, realm_name)
        .await
        .with_context(|| "Error obtaining realm id")?;

    let insert_user_query = body
        .get_insert_user_query(
            realm_id,
            voters_table,
            &voters_table_processed_columns_names,
        )
        .with_context(|| "Error obtaining insert_user_query query")?;

    // Execute the create table query
    keycloak_transaction
        .execute(create_table_query.as_str(), &[])
        .await
        .with_context(|| "Error executing create_table_query statement")?;

    // Prepare for COPY FROM STDIN
    let sink = keycloak_transaction
        .copy_in(copy_from_query.as_str())
        .await
        .with_context(|| "Error preparing COPY transaction")?;
    let writer = BinaryCopyInWriter::new(sink, &voters_table_processed_columns_types);
    pin_mut!(writer);

    // Stream data from CSV to sink
    let mut owned_data: Vec<String> = Vec::new(); // To store owned string data

    for result in rdr.records() {
        let record = result.with_context(|| "Error reading CSV record")?;
        owned_data.clear(); // Clear previously stored data

        // Store owned data, and process it
        let mut password_salt: Option<String> = None;
        let mut hashed_password: Option<String> = None;
        for (data, column_name) in record.iter().zip(voters_table_input_columns_names.iter()) {
            let processed_data = match column_name.as_str() {
                column_name if column_name == AREA_ID_ATTR_NAME => areas_map
                    .as_ref()
                    .ok_or_else(|| anyhow!("Using area-id without providing election-event-id"))?
                    .get(data)
                    .ok_or_else(|| anyhow!("Area not found by name=`{data}`"))?
                    .to_string(),
                _ => data.to_string(),
            };
            if column_name == &*PASSWORD_COL_NAME {
                let mut salt_bytes: Credential = Default::default();
                thread_rng().fill(&mut salt_bytes);

                password_salt = Some(BASE64_STANDARD.encode(salt_bytes));
                hashed_password = Some(
                    hash_password(&processed_data, &salt_bytes)
                        .with_context(|| "Error generating hashed password")?,
                );
            } else {
                owned_data.push(processed_data);
            }
        }
        info!("owned_data: {owned_data:?}");
        if voters_table_processed_columns_names.contains(&*HASHED_PASSWORD_COL_NAME) {
            info!("password data: salt={password_salt:?}, hashed_password={hashed_password:?}");
            owned_data.push(password_salt.ok_or_else(|| anyhow!("password salt empty"))?);
            owned_data.push(hashed_password.ok_or_else(|| anyhow!("hashed password empty"))?);
        }

        let row: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = owned_data
            .iter()
            .map(|data| data as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        writer
            .as_mut()
            .write(row.as_slice())
            .await
            .with_context(|| "Error writing to COPY IN stdin transaction")?;
    }

    writer
        .finish()
        .await
        .with_context(|| "Error finishing COPY IN transaction")?;

    // Complete the copy process

    // Execute the insert users query from the temporal table
    let num_rows = keycloak_transaction
        .execute(insert_user_query.as_str(), &[])
        .await
        .with_context(|| "Error executing INSERT USER transaction")?;

    info!("num_rows = {num_rows}");

    // Commit the transaction
    keycloak_transaction
        .commit()
        .await
        .with_context(|| "error committing transaction")?;

    Ok(())
}
