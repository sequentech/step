// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_areas_by_name;
use crate::postgres::keycloak_realm;
use crate::postgres::keycloak_realm::get_duplicate_emails_allowed;
use crate::services::authorized_elections::AuthorizedElectionIds;
use crate::services::csv_cell::unescape_formula;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::election::get_election_event_elections;
use crate::services::electoral_log::{
    post_voter_secret_attribute_audit_with_transaction, ElectoralLogAdminContext,
    VoterSecretAttributeAction, VoterSecretAttributeAudit,
};
use crate::services::sql_utils::{escape_sql_identifier, escape_sql_literal};
use crate::services::voter_secret_attributes::{
    encrypt_attribute_values, get_secret_attribute_config,
};
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
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::keycloak::{
    AREA_ID_ATTR_NAME, AUTHORIZED_ELECTION_IDS_NAME, DEFAULT_VOTE_WEIGHT, MAX_TOTAL_VOTE_WEIGHT,
    MAX_VOTE_WEIGHT, MIN_VOTE_WEIGHT, TENANT_ID_ATTR_NAME, VOTE_WEIGHT_ATTR_NAME,
};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::LazyLock;
use tempfile::NamedTempFile;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::{ToSql, Type};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

pub static HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9._-]+$").expect("Failed to build header regex"));
static PBKDF2_ITERATIONS: LazyLock<NonZeroU32> =
    LazyLock::new(|| NonZeroU32::new(27_500).expect("PBKDF2_ITERATIONS must be non-zero"));
const NUMBER_OF_ITERATIONS_COL_NAME: &str = "num_of_iterations";
const SALT_COL_NAME: &str = "password_salt";
const HASHED_PASSWORD_COL_NAME: &str = "hashed_password";
const PASSWORD_COL_NAME: &str = "password";
const USERNAME_COL_NAME: &str = "username";
const EMAIL_COL_NAME: &str = "email";
const EMAIL_VERIFIED_COL_NAME: &str = "email_verified";
const GROUP_COL_NAME: &str = "group_name";
const AREA_NAME_COL_NAME: &str = "area_name";
pub const ELECTION_COL_PREFIX: &str = "election__";
pub(crate) const INTERNAL_USER_ID_COL_NAME: &str = "sequent_internal_user_id";
/// Columns import reads as something other than the voter attribute of the same
/// name.
pub(crate) const RESERVED_COL_NAMES: [&str; 7] = [
    HASHED_PASSWORD_COL_NAME,
    SALT_COL_NAME,
    PASSWORD_COL_NAME,
    GROUP_COL_NAME,
    NUMBER_OF_ITERATIONS_COL_NAME,
    EMAIL_VERIFIED_COL_NAME,
    INTERNAL_USER_ID_COL_NAME,
];
/// The `user_entity` columns import fills. It does not read a column named like
/// one as a voter attribute.
pub(crate) const USER_ENTITY_COLUMNS: [&str; 8] = [
    "id",
    "email",
    "email_constraint",
    "enabled",
    "first_name",
    "last_name",
    "username",
    "not_before",
];
static PBKDF2_ALGORITHM: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;
const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
pub type Credential = [u8; CREDENTIAL_LEN];

/// Validates a non-empty `vote-weight` cell. The bulk import writes straight
/// into Keycloak's tables, bypassing the realm user profile validators, so this
/// is the only place a bad weight is caught before it reaches the tally.
fn validate_vote_weight(value: &str, row: usize) -> Result<u64> {
    let weight: u64 = value.parse().map_err(|_| {
        anyhow!(
            "Invalid `{VOTE_WEIGHT_ATTR_NAME}` value {value:?} on row {row}: \
             must be a whole number between 1 and {MAX_VOTE_WEIGHT}"
        )
    })?;
    if !(MIN_VOTE_WEIGHT..=MAX_VOTE_WEIGHT).contains(&weight) {
        return Err(anyhow!(
            "Invalid `{VOTE_WEIGHT_ATTR_NAME}` value {value:?} on row {row}: \
             must be between {MIN_VOTE_WEIGHT} and {MAX_VOTE_WEIGHT}"
        )
        .into());
    }
    Ok(weight)
}

/// Rewrites an `authorized-election-ids` cell to store, for each election it
/// names by external ID or ID, the value that authorizes a voter for it. The
/// bulk import writes straight into Keycloak's tables, and the token mapper
/// silently drops a value it cannot resolve, so this is the only place a bad
/// one is caught before it locks the voter out of the election. A blank cell
/// stays blank, leaving the voter unrestricted.
pub(crate) fn resolve_authorized_election_ids(
    cell: &str,
    row: usize,
    elections: &AuthorizedElectionIds,
) -> Result<String> {
    if cell.trim().is_empty() {
        return Ok(String::new());
    }
    let mut values: Vec<&str> = Vec::new();
    for reference in cell.split(MULTIVALUE_USER_ATTRIBUTE_SEPARATOR) {
        let reference = reference.trim();
        let value = elections
            .resolve_imported(reference)
            .map_err(|unresolved| {
                anyhow!(
                    "Invalid `{AUTHORIZED_ELECTION_IDS_NAME}` value {reference:?} on row {row}: \
                 {unresolved}"
                )
            })?;
        if !values.contains(&value) {
            values.push(value);
        }
    }
    Ok(values.join(MULTIVALUE_USER_ATTRIBUTE_SEPARATOR))
}

/// Exports add a column per election saying when each voter last voted in it.
/// Nothing is imported from them, so they are dropped before the headers are
/// checked: they are named after the elections' external IDs, which may hold
/// characters a header may not, and older exports named them after display
/// names, which repeat.
pub(crate) fn is_election_column(header: &str) -> bool {
    header.starts_with(ELECTION_COL_PREFIX)
}

/// The fields of `record` in the columns that are imported.
pub(crate) fn imported_fields(record: &StringRecord, imported_columns: &[bool]) -> StringRecord {
    record
        .iter()
        .zip(imported_columns)
        .filter_map(|(field, imported)| imported.then_some(field))
        .collect()
}

/// The value `cell` holds in `column`, without the `'` export puts before a
/// formula. Export never writes passwords, so one is imported as written.
pub(crate) fn imported_value<'a>(column: &str, cell: &'a str) -> &'a str {
    if column == PASSWORD_COL_NAME {
        cell
    } else {
        unescape_formula(cell)
    }
}

fn sanitize_db_key(key: &String) -> String {
    key.replace(".", "_").replace("-", "_")
}

/// The column of the temp table that import stores the column `header` in. It
/// reads `area_name` as `area-id`. Postgres folds unquoted names to lower case,
/// so headers that differ only in case, or in the characters `sanitize_db_key`
/// rewrites, share one.
pub(crate) fn temp_table_column(header: &str) -> String {
    let field = if header == AREA_NAME_COL_NAME {
        AREA_ID_ATTR_NAME
    } else {
        header
    };
    sanitize_db_key(&field.to_string()).to_lowercase()
}

/// Whether `header` is a misspelling of `vote-weight`, which import refuses.
pub(crate) fn is_misspelled_vote_weight(header: &str) -> bool {
    header != VOTE_WEIGHT_ATTR_NAME
        && header
            .replace(['_', '.', '-'], "")
            .eq_ignore_ascii_case(&VOTE_WEIGHT_ATTR_NAME.replace('-', ""))
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
 *  - vote-weight: positive whole number. Example "3". Only meaningful when
 *    the election event uses the voters-weighted voting policy.
 *  - group_name: string. Example "voter"
 *  - password: string: Example "secret-password"
 */
#[instrument(ret)]
pub(crate) fn get_copy_from_query(
    headers: &StringRecord,
) -> anyhow::Result<(String, String, String, Vec<String>, Vec<String>, Vec<Type>)> {
    let random_number: u64 = rand::random();

    let temp_table_name = format!("temp_voters_{}", random_number);
    let headers_vec = headers.iter().map(String::from).collect::<Vec<String>>();

    let input_column_names = headers_vec
        .iter()
        .map(|column_name| match column_name.as_str() {
            column_name if column_name == AREA_NAME_COL_NAME => AREA_ID_ATTR_NAME.to_string(),
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
            column_name if column_name == AREA_NAME_COL_NAME => Some(AREA_ID_ATTR_NAME.to_string()),
            column_name if column_name == PASSWORD_COL_NAME => None,
            column_name if column_name == SALT_COL_NAME => None,
            column_name if column_name == HASHED_PASSWORD_COL_NAME => None,
            column_name if column_name == NUMBER_OF_ITERATIONS_COL_NAME => None,
            _ => Some(column_name.clone()),
        })
        .chain(if headers_vec.iter().any(|s| s == PASSWORD_COL_NAME) {
            vec![
                SALT_COL_NAME.to_string(),
                HASHED_PASSWORD_COL_NAME.to_string(),
                NUMBER_OF_ITERATIONS_COL_NAME.to_string(),
            ]
            .into_iter()
        } else if headers_vec.iter().any(|s| s == HASHED_PASSWORD_COL_NAME) {
            vec![
                SALT_COL_NAME.to_string(),
                HASHED_PASSWORD_COL_NAME.to_string(),
                NUMBER_OF_ITERATIONS_COL_NAME.to_string(),
            ]
            .into_iter()
        } else {
            Vec::new().into_iter()
        })
        // note that in this case, username is at the end
        .chain(if !headers_vec.iter().any(|s| s == USERNAME_COL_NAME) {
            vec![USERNAME_COL_NAME.to_string()].into_iter()
        } else {
            Vec::new().into_iter()
        })
        .chain(std::iter::once(INTERNAL_USER_ID_COL_NAME.to_string()))
        .collect::<Vec<String>>();

    // Two headers can map to the same field (`area_name` and `area-id`), or to
    // the same temp table column once sanitized. Left alone that surfaces as a
    // duplicate-column error from Postgres that names neither header.
    // `vote_weight` is the spelling an operator is most likely to reach for, and
    // it would be accepted as an ordinary attribute, stored under a name the
    // ballot dump never reads, and tallied as weight 1 for every voter. Refuse
    // it by name rather than let that happen silently.
    for header in headers_vec.iter() {
        if is_misspelled_vote_weight(header) {
            return Err(anyhow!(
                "Column `{header}` is not recognised. The per-voter vote weight \
                 column is spelled exactly `{VOTE_WEIGHT_ATTR_NAME}`, in lower \
                 case and hyphenated"
            ));
        }
    }

    // Compare sanitized, case-folded names, since that is what becomes the temp
    // table column: `area-id` and `area_id`, or `Email` and `email`, are
    // distinct headers that collide there.
    let mut seen: Vec<String> = Vec::with_capacity(processed_column_names.len());
    for column_name in &processed_column_names {
        let sanitized = temp_table_column(column_name);
        if seen.contains(&sanitized) {
            return Err(anyhow!(
                "Duplicate column `{column_name}` in the import file: two headers \
                 map to the same field"
            ));
        }
        seen.push(sanitized);
    }

    // Create the table creation query
    let quoted_table_name = escape_sql_identifier(&temp_table_name);
    let create_table_query = format!(
        "CREATE TEMP TABLE {} ({});",
        quoted_table_name,
        processed_column_names
            .iter()
            .map(|name| format!("{} VARCHAR", sanitize_db_key(&name.to_string())))
            .collect::<Vec<String>>()
            .join(", ")
    );

    // Create the COPY FROM STDIN query
    let copy_from_query = format!("COPY {} FROM STDIN BINARY;", quoted_table_name);

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
    duplicate_emails_allowed: bool,
) -> anyhow::Result<String> {
    parse_uuid_v4(&tenant_id)
        .with_context(|| format!("invalid v4 UUID for tenant_id: {}", tenant_id))?;
    parse_uuid_v4(&realm_id)
        .with_context(|| format!("invalid v4 UUID for realm_id: {}", realm_id))?;
    let realm_id = escape_sql_literal(&realm_id);
    let tenant_id = escape_sql_literal(&tenant_id);
    let voters_table = escape_sql_identifier(&voters_table);

    // Build the INSERT query for user_entity
    let select_columns: Vec<String> = USER_ENTITY_COLUMNS
        .iter()
        .map(|&column| {
            let col_name = column.to_string();
            match column {
                "id" => INTERNAL_USER_ID_COL_NAME.to_string(),
                // Cast enabled to boolean, with TRUE as default
                "enabled" => {
                    if voters_table_columns.contains(&col_name) {
                        "COALESCE(NULLIF(enabled, ''), 'TRUE')::boolean".to_string()
                    } else {
                        "'TRUE'::boolean".to_string()
                    }
                }
                // empty as default, lowercase required in keycloak
                "email" => {
                    if voters_table_columns.contains(&col_name) {
                        "NULLIF(LOWER(email), '')".to_string()
                    } else {
                        "NULL".to_string()
                    }
                }
                "email_constraint" => {
                    if duplicate_emails_allowed {
                        "gen_random_uuid()::text".to_string()
                    } else if voters_table_columns.contains(&"email".to_string()) {
                        "NULLIF(LOWER(email), '')".to_string()
                    } else {
                        "NULL".to_string()
                    }
                }
                "not_before" => "0".to_string(),
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

    // Conditionally create the SELECT statement for email_verified.
    // If the column doesn't exist in the source table, default to true.
    let email_verified_select = if voters_table_columns.contains(&"email_verified".to_string()) {
        "COALESCE(NULLIF(email_verified, '')::boolean, true)".to_string()
    } else {
        "true".to_string()
    };

    let user_entity_query = format!(
        r#"INSERT INTO user_entity (
                    realm_id,
                    email_verified,
                    created_timestamp,
                    {}
                )
                SELECT
                    '{realm_id}',
                    {email_verified_select},
                    (extract(epoch from now()) * 1000)::bigint,
                    {}
                FROM
                    {}
                RETURNING *"#,
        USER_ENTITY_COLUMNS.join(", "),
        select_columns.join(", "),
        voters_table,
    );

    // Assume all other columns are user attributes
    let user_attributes = voters_table_columns
        .clone()
        .into_iter()
        .filter(|col| {
            !USER_ENTITY_COLUMNS.contains(&col.as_str())
                && !RESERVED_COL_NAMES.iter().any(|&s| s == col)
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
                                nu.id = v.{INTERNAL_USER_ID_COL_NAME}
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

    let group_name = if voters_table_columns.iter().any(|s| s == GROUP_COL_NAME) {
        format!("v.{GROUP_COL_NAME}")
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
                        nu.id = v.{INTERNAL_USER_ID_COL_NAME}
                JOIN
                    keycloak_group kg ON
                        kg.name = {group_name}
                        AND kg.realm_id = '{realm_id}'
            ),
            user_group AS (
                INSERT 
                INTO user_group_membership (
                    group_id,
                    user_id,
                    membership_type
                )
                SELECT
                    pug.group_id,
                    pug.user_id,
                    'UNMANAGED'
                FROM pre_user_group pug
            )
            "#
    );

    // Inserts password credentials if need be
    let salt_col_name = SALT_COL_NAME;
    let hashed_password_col_name = HASHED_PASSWORD_COL_NAME;
    let credentials_query = if voters_table_columns
        .iter()
        .any(|s| s == hashed_password_col_name)
    {
        let num_iterations = if voters_table_columns
            .iter()
            .any(|s| s == NUMBER_OF_ITERATIONS_COL_NAME)
        {
            format!("v.{NUMBER_OF_ITERATIONS_COL_NAME}")
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
                    nu.id = v.{INTERNAL_USER_ID_COL_NAME}
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

/// `replaced_ids` maps the IDs in an exported election event to those its
/// import gave them, so that voters keep elections the file names by ID.
#[instrument(err, skip(hasura_transaction, replaced_ids))]
pub async fn import_users_file(
    hasura_transaction: &Transaction<'_>,
    voters_file: &NamedTempFile,
    separator: u8,
    election_event_id: Option<String>,
    tenant_id: String,
    is_admin: bool,
    may_write_secret_attributes: bool,
    secret_write_initiator: Option<&ElectoralLogAdminContext>,
    replaced_ids: Option<&HashMap<String, String>>,
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
        .context("Error starting Keycloak transaction")?;

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

    let all_headers = match rdr.headers() {
        Ok(headers) => headers.clone(),
        Err(err) => {
            return Err(Error::String(format!(
                "Error reading CSV headers from voters file: {err}"
            )));
        }
    };
    let imported_columns = all_headers
        .iter()
        .map(|header| !is_election_column(header))
        .collect::<Vec<bool>>();
    let headers = imported_fields(&all_headers, &imported_columns);

    info!("headers: {headers:?}");
    for header in headers.iter() {
        if !HEADER_RE.is_match(header) {
            return Err(Error::String(format!(
                "CSV Header contains characters not allowed: {header}"
            )));
        }
        if header == INTERNAL_USER_ID_COL_NAME {
            return Err(Error::String(format!(
                "CSV header `{INTERNAL_USER_ID_COL_NAME}` is reserved"
            )));
        }
    }

    let authorized_elections = match election_event_id.as_deref() {
        Some(event_id)
            if headers
                .iter()
                .any(|header| header == AUTHORIZED_ELECTION_IDS_NAME) =>
        {
            let elections = get_election_event_elections(hasura_transaction, &tenant_id, event_id)
                .await
                .map_err(|err| Error::String(format!("Error retrieving elections: {err:#}")))?;
            let authorized_elections = AuthorizedElectionIds::new(&elections);
            Some(match replaced_ids {
                Some(replaced_ids) => authorized_elections.with_replaced_ids(replaced_ids),
                None => authorized_elections,
            })
        }
        _ => None,
    };

    let secret_names = if let Some(event_id) = election_event_id.as_deref() {
        get_secret_attribute_config(&tenant_id, event_id)
            .await
            .map_err(|err| Error::String(format!("{err:#}")))?
            .validated_names()
            .map_err(|err| Error::String(err.to_string()))?
    } else {
        Default::default()
    };
    let imported_secret_names = headers
        .iter()
        .filter(|header| secret_names.contains(*header))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !imported_secret_names.is_empty() && !may_write_secret_attributes {
        return Err(Error::String(format!(
            "Importing encrypted voter attributes requires voter-secret-attribute-write: {}",
            imported_secret_names.join(", ")
        )));
    }
    if let (false, Some(event_id)) = (
        imported_secret_names.is_empty(),
        election_event_id.as_deref(),
    ) {
        let initiator = secret_write_initiator.ok_or_else(|| {
            Error::String(
                "Importing encrypted voter attributes requires an identified initiator".to_string(),
            )
        })?;
        post_voter_secret_attribute_audit_with_transaction(
            hasura_transaction,
            &tenant_id,
            event_id,
            initiator,
            VoterSecretAttributeAction::Import,
            VoterSecretAttributeAudit {
                voter_id: None,
                voter_username: None,
                attribute_names: &imported_secret_names,
                document_id: None,
            },
        )
        .await
        .map_err(|err| {
            Error::String(format!(
                "Failed to record the secret-attribute electoral-log entry: {err:#}"
            ))
        })?;
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

    let duplicate_emails_allowed = get_duplicate_emails_allowed(&keycloak_transaction, &realm_id)
        .await
        .map_err(|err| Error::String(format!("Error obtaining duplicate_emails_allowed: {err}")))?;

    let insert_user_query = match get_insert_user_query(
        tenant_id.clone(),
        realm_id,
        voters_table,
        &voters_table_processed_columns_names,
        duplicate_emails_allowed,
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
    // A lower bound on the ciphertexts this file's voters will contribute: it
    // cannot see voters imported earlier, does not know which of them will
    // actually vote, and pools areas that the tally keeps separate. It only
    // ever warns, so none of that can reject a valid import.
    let mut imported_weight_total: u64 = 0;
    let mut owned_data: Vec<String> = Vec::new();
    // 1-based and counting the header, so it matches what a spreadsheet shows.
    let mut row_number: usize = 1;
    for result in rdr.records() {
        row_number += 1;
        let record = match result {
            Ok(record) => imported_fields(&record, &imported_columns),
            Err(err) => {
                return Err(Error::String(format!("Error reading CSV record: {err}")));
            }
        };
        owned_data.clear();
        let user_id = Uuid::new_v4().to_string();

        let mut password_salt: Option<String> = None;
        let mut hashed_password: Option<String> = None;
        let mut num_of_iterations = *PBKDF2_ITERATIONS;
        let mut password: Option<String> = None;
        for (data, column_name) in record.iter().zip(voters_table_input_columns_names.iter()) {
            let data = imported_value(column_name, data);
            let mut processed_data = match column_name.as_str() {
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
                    column_name if column_name == VOTE_WEIGHT_ATTR_NAME => {
                        let trimmed = data.trim();
                        // A blank cell votes with the default weight, so it
                        // counts as one ciphertext here too.
                        imported_weight_total = imported_weight_total.saturating_add(
                            trimmed.parse::<u64>().unwrap_or(DEFAULT_VOTE_WEIGHT),
                        );
                        // A blank cell means "no weight for this voter", which
                        // the ballot dump resolves to the default. Requiring a
                        // value would reject the usual way of authoring the
                        // file: weights for some voters, blanks for the rest.
                        if trimmed.is_empty() {
                            trimmed.to_string()
                        } else {
                            // Store the canonical number: `+5` and `007` parse
                            // here but would be rejected by the realm's integer
                            // validator on any later edit.
                            validate_vote_weight(trimmed, row_number)?.to_string()
                        }
                    }
                    column_name if column_name == AUTHORIZED_ELECTION_IDS_NAME => {
                        match &authorized_elections {
                            Some(authorized_elections) => resolve_authorized_election_ids(
                                data,
                                row_number,
                                authorized_elections,
                            )?,
                            None => data.to_string(),
                        }
                    }
                    column_name if column_name == USERNAME_COL_NAME => data.to_lowercase(),
                    column_name if column_name == EMAIL_COL_NAME => data.to_lowercase(),
                    column_name if column_name == EMAIL_VERIFIED_COL_NAME => data.to_lowercase(),
                    _ => data.to_string(),
                };

            if secret_names.contains(column_name) && !processed_data.is_empty() {
                let event_id = election_event_id.as_deref().ok_or_else(|| {
                    anyhow!("Encrypted voter attributes require an election event")
                })?;
                let values = processed_data
                    .split(MULTIVALUE_USER_ATTRIBUTE_SEPARATOR)
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                processed_data =
                    encrypt_attribute_values(&tenant_id, event_id, &user_id, column_name, &values)
                        .await?
                        .join(MULTIVALUE_USER_ATTRIBUTE_SEPARATOR);
            }

            if column_name == PASSWORD_COL_NAME {
                password = Some(data.to_string());
            } else if column_name == NUMBER_OF_ITERATIONS_COL_NAME {
                num_of_iterations = match data.parse::<u32>() {
                    Ok(value) => NonZeroU32::new(value).unwrap_or(*PBKDF2_ITERATIONS),
                    Err(_) => *PBKDF2_ITERATIONS,
                };
            } else if column_name == SALT_COL_NAME {
                password_salt = Some(data.to_string());
            } else if column_name == HASHED_PASSWORD_COL_NAME {
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

        if voters_table_processed_columns_names
            .iter()
            .any(|s| s == HASHED_PASSWORD_COL_NAME)
        {
            owned_data.push(password_salt.ok_or_else(|| anyhow!("Password salt empty"))?);
            owned_data.push(hashed_password.ok_or_else(|| anyhow!("Hashed password empty"))?);
            owned_data.push(num_of_iterations.get().to_string());
        }

        if !voters_table_input_columns_names
            .iter()
            .any(|s| s == USERNAME_COL_NAME)
        {
            let username = Uuid::new_v4().to_string();
            owned_data.push(username);
        }

        owned_data.push(user_id);

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

    // Warn rather than refuse: the limit applies per contest area, and this file
    // may spread its voters across several, so exceeding it here does not prove
    // any one area will. The exact check runs where the ballots are extracted.
    if imported_weight_total > MAX_TOTAL_VOTE_WEIGHT {
        warn!(
            "Imported vote weights total {imported_weight_total}, above the \
             per-area maximum of {MAX_TOTAL_VOTE_WEIGHT}. If these voters share \
             a contest area the tally will refuse to count them."
        );
    }

    let num_rows = keycloak_transaction
        .execute(insert_user_query.as_str(), &[])
        .await
        .context("Error executing INSERT USER transaction")?;

    info!("num_rows = {num_rows}");

    if let Err(err) = keycloak_transaction.commit().await {
        return Err(Error::String(format!(
            "Error committing transaction: {err}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::election::ElectionHead;
    use csv::StringRecord;

    /// The import writes `user_attribute.name` straight from these lists and the
    /// ballot dump reads the attribute by name, so the column has to survive
    /// under exactly the name the dump looks for. If it does not, every weight
    /// silently resolves to the default and the election tallies unweighted.
    /// Sharing the name also keeps an export -> edit -> re-import round trip
    /// working, since the export names columns after the realm attributes.
    #[test]
    fn vote_weight_column_survives_as_the_attribute_the_dump_reads() {
        let headers = StringRecord::from(vec!["username", VOTE_WEIGHT_ATTR_NAME]);
        let (_, _, _, input_columns, processed_columns, _) =
            get_copy_from_query(&headers).expect("query builds");

        assert!(
            input_columns.contains(&VOTE_WEIGHT_ATTR_NAME.to_string()),
            "input columns must carry the attribute name, got {input_columns:?}"
        );
        assert!(
            processed_columns.contains(&VOTE_WEIGHT_ATTR_NAME.to_string()),
            "processed columns must carry the attribute name, got {processed_columns:?}"
        );
    }

    /// Headers that differ only by the characters `sanitize_db_key` rewrites,
    /// or by case, become one temp table column. Postgres would reject that with
    /// an error naming neither header.
    #[test]
    fn colliding_headers_are_rejected_with_both_names_visible() {
        for headers in [
            vec!["username", "area-id", "area_id"],
            vec!["username", "Email", "email"],
        ] {
            let record = StringRecord::from(headers.clone());
            let error =
                get_copy_from_query(&record).expect_err(&format!("{headers:?} must be rejected"));
            assert!(
                error.to_string().contains("Duplicate column"),
                "unexpected error for {headers:?}: {error}"
            );
        }
    }

    const ELECTION_A: &str = "6f1c2d3e-4a5b-4c6d-8e7f-0a1b2c3d4e5f";
    const ELECTION_B: &str = "7a2b3c4d-5e6f-4a7b-9c8d-1e2f3a4b5c6d";
    const ELECTION_C: &str = "8b3c4d5e-6f7a-4b8c-ad9e-2f3a4b5c6d7e";
    const EXTERNAL_ID: &str = "GIAMBI30-3-31";
    const OTHER_EXTERNAL_ID: &str = "GTELEC31+GCIBER30-1-01";

    /// An election with what resolving values reads from it: its ID and
    /// external ID.
    fn election(id: &str, external_id: Option<&str>) -> ElectionHead {
        ElectionHead {
            id: id.to_string(),
            name: "-".to_string(),
            alias: None,
            external_id: external_id.map(str::to_string),
        }
    }

    /// Resolves values among two elections with external IDs and one without.
    fn authorized_elections() -> AuthorizedElectionIds {
        AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some(OTHER_EXTERNAL_ID)),
            election(ELECTION_B, Some(EXTERNAL_ID)),
            election(ELECTION_C, None),
        ])
    }

    /// The headers of a file exported before election columns were named
    /// after external IDs, and after: neither is a valid or unique header.
    #[test]
    fn election_columns_are_not_imported() {
        let election_column = format!("election__{OTHER_EXTERNAL_ID}");
        let headers = StringRecord::from(vec![
            "username",
            "election__-",
            AUTHORIZED_ELECTION_IDS_NAME,
            "election__-",
            election_column.as_str(),
        ]);
        let record = StringRecord::from(vec!["voter", "2025-01-01T00:00:00Z", EXTERNAL_ID, "", ""]);
        let imported_columns = headers
            .iter()
            .map(|header| !is_election_column(header))
            .collect::<Vec<bool>>();

        let headers = imported_fields(&headers, &imported_columns);
        assert_eq!(
            headers,
            StringRecord::from(vec!["username", AUTHORIZED_ELECTION_IDS_NAME])
        );
        assert!(headers.iter().all(|header| HEADER_RE.is_match(header)));
        get_copy_from_query(&headers).expect("the remaining headers import");
        assert_eq!(
            imported_fields(&record, &imported_columns),
            StringRecord::from(vec!["voter", EXTERNAL_ID])
        );
    }

    /// Export never writes passwords, so one is imported as written.
    #[test]
    fn formulas_are_read_back_in_every_column_but_password() {
        assert_eq!(imported_value("first_name", "'=1+1"), "=1+1");
        assert_eq!(
            imported_value(AUTHORIZED_ELECTION_IDS_NAME, "'-1|=1+1"),
            "-1|=1+1"
        );
        assert_eq!(imported_value(PASSWORD_COL_NAME, "'=1+1"), "'=1+1");
    }

    /// Values are trimmed, and an election named by its ID is stored by its
    /// external ID.
    #[test]
    fn authorized_elections_are_stored_by_external_id_or_id_without_one() {
        let stored = resolve_authorized_election_ids(
            &format!(" {EXTERNAL_ID} |{ELECTION_A}|{ELECTION_C}"),
            2,
            &authorized_elections(),
        )
        .expect("every value names an election");

        assert_eq!(
            stored,
            format!("{EXTERNAL_ID}|{OTHER_EXTERNAL_ID}|{ELECTION_C}")
        );
    }

    /// Once by its external ID and once by its ID.
    #[test]
    fn an_election_named_twice_is_stored_once() {
        let stored = resolve_authorized_election_ids(
            &format!("{EXTERNAL_ID}|{ELECTION_B}"),
            2,
            &authorized_elections(),
        )
        .expect("every value names an election");

        assert_eq!(stored, EXTERNAL_ID);
    }

    /// A blank cell leaves the voter unrestricted, so it must not turn into a
    /// value that restricts them, nor fail.
    #[test]
    fn blank_authorized_elections_stay_blank() {
        for cell in ["", "  "] {
            assert_eq!(
                resolve_authorized_election_ids(cell, 2, &authorized_elections())
                    .expect("a blank cell imports"),
                ""
            );
        }
    }

    /// A value that differs only in case, an empty one and an unknown ID. The
    /// message quotes the value, so that an empty one shows.
    #[test]
    fn authorized_elections_matching_no_election_are_rejected_naming_row_and_value() {
        let lowercase = EXTERNAL_ID.to_lowercase();
        let unknown_id = "1d2e3f4a-5b6c-4d7e-8f9a-0b1c2d3e4f5a";
        for (cell, value) in [
            (
                format!("{EXTERNAL_ID}|{lowercase}"),
                format!("\"{lowercase}\""),
            ),
            (format!("{EXTERNAL_ID}|"), "\"\"".to_string()),
            (unknown_id.to_string(), format!("\"{unknown_id}\"")),
        ] {
            let message = resolve_authorized_election_ids(&cell, 4, &authorized_elections())
                .expect_err(&format!("{cell:?} must be rejected"))
                .to_string();
            assert!(
                message.contains(AUTHORIZED_ELECTION_IDS_NAME),
                "error must name the column, got: {message}"
            );
            assert!(
                message.contains("row 4"),
                "error must name the row, got: {message}"
            );
            assert!(
                message.contains(&value),
                "error must name the value {value}, got: {message}"
            );
        }
    }

    /// The token mapper resolves the shared external ID to none of them, so
    /// they can only be named by ID.
    #[test]
    fn authorized_elections_naming_several_elections_are_rejected() {
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some(EXTERNAL_ID)),
            election(ELECTION_B, Some(EXTERNAL_ID)),
        ]);

        let message = resolve_authorized_election_ids(EXTERNAL_ID, 3, &elections)
            .expect_err("the external ID names two elections")
            .to_string();
        assert!(
            message.contains(&format!(
                "\"{EXTERNAL_ID}\" on row 3: more than one election"
            )),
            "error must name the value, the row and the reason, got: {message}"
        );
        assert_eq!(
            resolve_authorized_election_ids(&format!("{ELECTION_A}|{ELECTION_B}"), 3, &elections)
                .expect("each ID names one election"),
            format!("{ELECTION_A}|{ELECTION_B}")
        );
    }

    /// Export writes values that do not name a single election in double
    /// quotes, so they must not import, even where elections have external IDs
    /// with double quotes.
    #[test]
    fn authorized_elections_starting_with_a_double_quote_are_rejected() {
        let trailing_quote = format!("{OTHER_EXTERNAL_ID}\"");
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some(&format!("\"{EXTERNAL_ID}"))),
            election(ELECTION_B, Some(&trailing_quote)),
        ]);

        let message = resolve_authorized_election_ids(
            &format!("\"{EXTERNAL_ID}|{trailing_quote}"),
            5,
            &elections,
        )
        .expect_err("a value starting with a double quote names no election")
        .to_string();
        assert!(
            message.contains(&format!(r#""\"{EXTERNAL_ID}" on row 5: "#))
                && message.contains("double quote"),
            "error must name the value, the row and the reason, got: {message}"
        );
        assert_eq!(
            resolve_authorized_election_ids(
                &format!("{ELECTION_A}|{trailing_quote}"),
                5,
                &elections
            )
            .expect("each value names one election"),
            format!("{ELECTION_A}|{trailing_quote}")
        );
    }

    #[test]
    fn valid_vote_weights_are_accepted() {
        for value in ["1", "5", "100000"] {
            assert!(
                validate_vote_weight(value, 2).is_ok(),
                "vote weight {value} must be accepted"
            );
        }
    }

    #[test]
    fn invalid_vote_weights_are_rejected_naming_row_and_column() {
        for value in ["0", "-1", "1.5", "abc", "100001", ""] {
            let error = validate_vote_weight(value, 7)
                .expect_err(&format!("vote weight {value:?} must be rejected"));
            let message = error.to_string();
            assert!(
                message.contains(VOTE_WEIGHT_ATTR_NAME),
                "error must name the column, got: {message}"
            );
            assert!(
                message.contains("row 7"),
                "error must name the row, got: {message}"
            );
        }
    }
}
