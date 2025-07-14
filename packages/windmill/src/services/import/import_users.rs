// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_areas_by_name;
use crate::postgres::keycloak_realm;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use base64::prelude::*;
use csv::StringRecord;
use deadpool_postgres::Transaction;
use futures::pin_mut;
use rand::prelude::*;
use rand::{thread_rng, Rng};
use regex::Regex;
use ring::{digest, pbkdf2};
use sequent_core::services::keycloak::{
    get_event_realm, get_tenant_realm, MULTIVALUE_USER_ATTRIBUTE_SEPARATOR,
};
use sequent_core::types::keycloak::{AREA_ID_ATTR_NAME, TENANT_ID_ATTR_NAME};
use std::num::NonZeroU32;
use tempfile::NamedTempFile;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::{ToSql, Type};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

lazy_static! {
    pub static ref HEADER_RE: Regex = Regex::new(r"^[a-zA-Z0-9._-]+$").unwrap();
    static ref PBKDF2_ITERATIONS: NonZeroU32 = NonZeroU32::new(27_500).unwrap();
    static ref NUMBER_OF_ITERATIONS_COL_NAME: String = String::from("num_of_iterations");
    static ref SALT_COL_NAME: String = String::from("password_salt");
    static ref HASHED_PASSWORD_COL_NAME: String = String::from("hashed_password");
    static ref PASSWORD_COL_NAME: String = String::from("password");
    static ref USERNAME_COL_NAME: String = String::from("username");
    static ref EMAIL_COL_NAME: String = String::from("email");
    static ref EMAIL_VERIFIED_COL_NAME: String = String::from("email_verified");
    static ref GROUP_COL_NAME: String = String::from("group_name");
    static ref AREA_NAME_COL_NAME: String = String::from("area_name");
    static ref ELECTION_COL_PREFIX: String = String::from("election__");
    static ref RESERVED_COL_NAMES: Vec<String> = vec![
        HASHED_PASSWORD_COL_NAME.clone(),
        SALT_COL_NAME.clone(),
        PASSWORD_COL_NAME.clone(),
        GROUP_COL_NAME.clone(),
        NUMBER_OF_ITERATIONS_COL_NAME.clone(),
        EMAIL_VERIFIED_COL_NAME.clone()
    ];
}

static PBKDF2_ALGORITHM: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;
const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
pub type Credential = [u8; CREDENTIAL_LEN];

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

//////////////////////////////////////////////////////////////////////
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

    // This is the list of columns, in order, of the temporal voters table.
    // It is important to match columns and order of elements with the data
    // being processed and sent through the COPY FROM STDIN below in the
    // variable `owned_data` in `import_users_file()`
    let processed_column_names = headers_vec
        .iter()
        .filter_map(|column_name| match column_name.as_str() {
            column_name if column_name == *AREA_NAME_COL_NAME => {
                Some(AREA_ID_ATTR_NAME.to_string())
            }
            column_name if column_name == *PASSWORD_COL_NAME => None,
            column_name if column_name == *SALT_COL_NAME => None,
            column_name if column_name == *HASHED_PASSWORD_COL_NAME => None,
            column_name if column_name == *NUMBER_OF_ITERATIONS_COL_NAME => None,
            _ => Some(column_name.clone()),
        })
        .chain(if headers_vec.contains(&PASSWORD_COL_NAME) {
            vec![
                SALT_COL_NAME.clone(),
                HASHED_PASSWORD_COL_NAME.clone(),
                NUMBER_OF_ITERATIONS_COL_NAME.clone(),
            ]
            .into_iter()
        } else if headers_vec.contains(&HASHED_PASSWORD_COL_NAME) {
            vec![
                SALT_COL_NAME.clone(),
                HASHED_PASSWORD_COL_NAME.clone(),
                NUMBER_OF_ITERATIONS_COL_NAME.clone(),
            ]
            .into_iter()
        } else {
            Vec::new().into_iter()
        })
        // note that in this case, username is at the end
        .chain(if !headers_vec.contains(&USERNAME_COL_NAME) {
            vec![USERNAME_COL_NAME.clone()].into_iter()
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
#[instrument(err)]
fn get_insert_user_query(
    tenant_id: String,
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
            let col_name = column.to_string();
            match column {
                "id" => "gen_random_uuid()".to_string(),
                // Cast enabled to boolean, with TRUE as default
                "enabled" => {
                    if voters_table_columns.contains(&col_name) {
                        "enabled::boolean".to_string()
                    } else {
                        "'TRUE'::boolean".to_string()
                    }
                }
                // empty as default, lowercase required in keycloak
                "email" => {
                    if voters_table_columns.contains(&col_name) {
                        "LOWER(email)".to_string()
                    } else {
                        "''".to_string()
                    }
                }
                // empty as default
                _ => {
                    if voters_table_columns.contains(&col_name) {
                        col_name
                    } else {
                        "''".to_string()
                    }
                }
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
                    COALESCE(email_verified::boolean, true),
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
            !user_entity_columns.contains(&col.as_str())
                && !RESERVED_COL_NAMES.contains(&col)
                && !col.starts_with(&*ELECTION_COL_PREFIX)
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
                            unnest(string_to_array(v.{sanitized_attr}, '{MULTIVALUE_USER_ATTRIBUTE_SEPARATOR}'))
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
                    '{tenant_id}'
                FROM
                    new_user nu
                "#,
        )
    } else {
        String::new()
    };

    let group_name = if voters_table_columns.contains(&*GROUP_COL_NAME) {
        format!("v.{}", &*GROUP_COL_NAME)
    } else {
        "'voter'".to_string()
    };

    let group_query = format!(
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
                        kg.name = {group_name}
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
    );

    // Inserts password credentials if need be
    let salt_col_name = &*SALT_COL_NAME;
    let hashed_password_col_name = &*HASHED_PASSWORD_COL_NAME;
    // let num_iterations = &*PBKDF2_ITERATIONS;
    let credentials_query = if voters_table_columns.contains(hashed_password_col_name) {
        let num_iterations = if voters_table_columns.contains(&*NUMBER_OF_ITERATIONS_COL_NAME) {
            format!("v.{}", &*NUMBER_OF_ITERATIONS_COL_NAME)
        } else {
            PBKDF2_ITERATIONS.to_string()
        };
        info!("num_iterations = {voters_table}");
        format!(
            r#",
                pre_credentials AS (
                SELECT
                    v.{salt_col_name} AS salt,
                    v.{hashed_password_col_name} AS hashed_password,
                    nu.id AS id,
                    {num_iterations} AS num_of_iterations
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
                    'hashIterations', pc.num_of_iterations,
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

#[instrument(err, skip(hasura_transaction))]
pub async fn import_users_file(
    hasura_transaction: &Transaction<'_>,
    voters_file: &NamedTempFile,
    separator: u8,
    election_event_id: Option<String>,
    tenant_id: String,
    is_admin: bool,
) -> Result<()> {
    let mut keycloak_db_client = match get_keycloak_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            return Err(Error::String(format!(
                "Error getting Keycloak DB pool: {err}"
            )));
        }
    };

    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting Keycloak transaction: {err}"))?;

    keycloak_transaction
        .simple_query(
            r#"
                SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;
                "#,
        )
        .await
        .with_context(|| "can't set transaction isolation level or encoding")?;

    // Only retrieve areas if not an admin and election_event_id is provided
    let areas_map = if !is_admin {
        match election_event_id {
            Some(ref event_id) => {
                match get_areas_by_name(&hasura_transaction, tenant_id.as_str(), event_id.as_str())
                    .await
                {
                    Ok(areas) => Some(areas),
                    Err(err) => {
                        return Err(Error::String(format!("Error retrieving areas: {err}")));
                    }
                }
            }
            None => {
                return Err(Error::String(format!(
                    "Using area-id without providing election-event-id (is_admin: {is_admin})"
                )));
            }
        }
    } else {
        None
    };

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(separator)
        .from_reader(voters_file);

    let headers = match rdr.headers() {
        Ok(headers) => headers.clone(),
        Err(err) => {
            return Err(Error::String(format!(
                "Error reading CSV headers from voters file: {err}"
            )));
        }
    };

    info!("headers: {headers:?}");
    for header in headers.iter() {
        if !HEADER_RE.is_match(header) {
            return Err(Error::String(format!(
                "CSV Header contains characters not allowed: {header}"
            )));
        }
    }

    let (
        voters_table,
        create_table_query,
        copy_from_query,
        voters_table_input_columns_names,
        voters_table_processed_columns_names,
        voters_table_processed_columns_types,
    ) = match get_copy_from_query(&headers) {
        Ok(result) => result,
        Err(err) => {
            return Err(Error::String(format!(
                "Error obtaining copy_from query: {err}"
            )));
        }
    };

    let realm_name = match election_event_id {
        Some(ref event_id) => get_event_realm(tenant_id.as_str(), event_id.as_str()),
        None => get_tenant_realm(tenant_id.as_str()),
    };

    let realm_id = match keycloak_realm::get_realm_id(&keycloak_transaction, realm_name).await {
        Ok(id) => id,
        Err(err) => {
            return Err(Error::String(format!("Error obtaining realm id: {err}")));
        }
    };

    let insert_user_query = match get_insert_user_query(
        tenant_id,
        realm_id,
        voters_table,
        &voters_table_processed_columns_names,
    ) {
        Ok(query) => query,
        Err(err) => {
            return Err(Error::String(format!(
                "Error obtaining insert_user_query: {err}"
            )));
        }
    };

    if let Err(err) = keycloak_transaction
        .execute(create_table_query.as_str(), &[])
        .await
    {
        return Err(Error::String(format!(
            "Error executing create table query: {err}"
        )));
    }

    let sink = match keycloak_transaction.copy_in(copy_from_query.as_str()).await {
        Ok(sink) => sink,
        Err(err) => {
            return Err(Error::String(format!(
                "Error preparing COPY transaction: {err}"
            )));
        }
    };
    let writer = BinaryCopyInWriter::new(sink, &voters_table_processed_columns_types);
    pin_mut!(writer);

    // 1. Owned data is defined outside the loop because otherwise the
    //    lifetime wouldn't match the one required in the write() function
    //    below, that writes into the COPY FROM STDIN sql sentence.
    // 2. `owned_data` is inserted into the temporal table in the order
    //    defined in `processed_column_names` of the function
    //    `get_copy_from_query()`. It's important to match these two
    //    together or else the temporal voters data table will be polluted
    //    with incorrectly assigned data.
    let mut owned_data: Vec<String> = Vec::new();
    for result in rdr.records() {
        let record = match result {
            Ok(record) => record,
            Err(err) => {
                return Err(Error::String(format!("Error reading CSV record: {err}")));
            }
        };
        owned_data.clear();

        let mut password_salt: Option<String> = None;
        let mut hashed_password: Option<String> = None;
        let mut num_of_iterations = *PBKDF2_ITERATIONS;
        let mut password: Option<String> = None;
        for (data, column_name) in record.iter().zip(voters_table_input_columns_names.iter()) {
            let processed_data = match column_name.as_str() {
                    column_name if column_name == AREA_ID_ATTR_NAME && !is_admin => {
                        match areas_map
                            .as_ref()
                            .ok_or_else(|| {
                                anyhow!("Using area-id without providing election-event-id (is_admin: {is_admin})")
                            })?
                            .get(data)
                        {
                            Some(area_id) => area_id.to_string(),
                            None => {
                                info!("Area not found by name `{data}`, setting area to NULL");
                                "".to_string()
                            }
                        }
                    }
                    column_name if column_name == &*USERNAME_COL_NAME => data.to_lowercase(),
                    column_name if column_name == &*EMAIL_COL_NAME => data.to_lowercase(),
                    column_name if column_name == &*EMAIL_VERIFIED_COL_NAME => data.to_lowercase(),
                    _ => data.to_string(),
                };

            if column_name == &*PASSWORD_COL_NAME {
                info!("password = {processed_data}");
                password = Some(data.to_string());
            } else if column_name == &*NUMBER_OF_ITERATIONS_COL_NAME {
                num_of_iterations = match data.parse::<u32>() {
                    Ok(value) => NonZeroU32::new(value).unwrap_or(*PBKDF2_ITERATIONS),
                    Err(_) => *PBKDF2_ITERATIONS,
                };
            } else if column_name == &*SALT_COL_NAME {
                password_salt = Some(data.to_string());
            } else if column_name == &*HASHED_PASSWORD_COL_NAME {
                hashed_password = Some(data.to_string());
            } else {
                owned_data.push(processed_data);
            }
        }

        if let Some(some_password) = password {
            let mut salt_bytes: Credential = Default::default();
            thread_rng().fill(&mut salt_bytes);

            password_salt = Some(BASE64_STANDARD.encode(salt_bytes));
            hashed_password = Some(
                hash_password(&some_password, &salt_bytes)
                    .with_context(|| "Error generating hashed password")?,
            );
        }

        if voters_table_processed_columns_names.contains(&*HASHED_PASSWORD_COL_NAME) {
            owned_data.push(password_salt.ok_or_else(|| anyhow!("Password salt empty"))?);
            owned_data.push(hashed_password.ok_or_else(|| anyhow!("Hashed password empty"))?);
            owned_data.push(num_of_iterations.get().to_string());
        }

        if !voters_table_input_columns_names.contains(&*USERNAME_COL_NAME) {
            let username = Uuid::new_v4().to_string();
            owned_data.push(username);
        }

        let row: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = owned_data
            .iter()
            .map(|data| data as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        if let Err(err) = writer.as_mut().write(row.as_slice()).await {
            return Err(Error::String(format!(
                "Error writing to COPY IN transaction: {err}"
            )));
        }
    }

    if let Err(err) = writer.finish().await {
        return Err(Error::String(format!(
            "Error finishing COPY IN transaction: {err}"
        )));
    }

    let num_rows = keycloak_transaction
        .execute(insert_user_query.as_str(), &[])
        .await
        .map_err(|err| anyhow!("Error executing INSERT USER transaction: {err}"))?;

    info!("num_rows = {num_rows}");

    if let Err(err) = keycloak_transaction.commit().await {
        return Err(Error::String(format!(
            "Error committing transaction: {err}"
        )));
    }

    Ok(())
}
