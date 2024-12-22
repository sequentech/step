// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_areas;
use crate::services::cast_votes::get_users_with_vote_info;
use crate::services::database::PgConfig;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::keycloak::*;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::str::FromStr;
use std::{
    collections::{HashMap, HashSet},
    convert::From,
};
use strum_macros::{Display, EnumString};
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::{debug, info, instrument};
use uuid::Uuid;

#[instrument(skip(hasura_transaction), err)]
async fn get_area_ids(
    hasura_transaction: &Transaction<'_>,
    election_id: Option<String>,
    area_id: Option<String>,
    param_number: i32,
) -> Result<(Option<Vec<String>>, String, String)> {
    let election_uuid: uuid::Uuid = match election_id {
        Some(election_id) => Uuid::parse_str(&election_id)
            .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?,
        None => return Ok((None, String::from(""), String::from(""))),
    };

    let area_ids: Vec<String> = match area_id {
        Some(area_id_value) => vec![area_id_value],
        None => {
            let areas_statement = hasura_transaction
                .prepare(
                    r#"
                SELECT DISTINCT
                    a.id::VARCHAR
                FROM
                    sequent_backend.area a
                JOIN
                    sequent_backend.area_contest ac ON a.id = ac.area_id
                JOIN
                    sequent_backend.contest c ON ac.contest_id = c.id
                WHERE c.election_id = $1;
            "#,
                )
                .await?;
            let rows: Vec<Row> = hasura_transaction
                .query(&areas_statement, &[&election_uuid])
                .await
                .map_err(|err| anyhow!("Error running the areas query: {}", err))?;
            let area_ids: Vec<String> = rows
                .into_iter()
                .map(|row| -> Result<String> {
                    Ok(row
                        .try_get::<&str, String>("id")
                        .map_err(|err| anyhow!("Error getting the area id of a row: {}", err))?)
                })
                .collect::<Result<Vec<String>>>()
                .map_err(|err| anyhow!("Error getting the areas ids: {}", err))?;
            area_ids
        }
    };

    debug!("area_ids: {area_ids:?}");
    let area_ids_join_clause = String::from(
        r#"
    INNER JOIN 
        user_attribute AS area_attr ON u.id = area_attr.user_id
    "#,
    );
    let area_ids_where_clause = format!(
        r#"
    AND (
        area_attr.name = '{AREA_ID_ATTR_NAME}' AND
        area_attr.value = ANY(${})
    )
    "#,
        param_number,
    );

    Ok((Some(area_ids), area_ids_join_clause, area_ids_where_clause))
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn list_keycloak_enabled_users_by_area_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
) -> Result<HashSet<String>> {
    let statement = keycloak_transaction
        .prepare(
            format!(
                r#"
        SELECT
            u.id,
            u.enabled,
            u.realm_id,
            u.username
        FROM
            user_entity AS u
        INNER JOIN
            realm AS ra ON ra.id = u.realm_id
        INNER JOIN 
            user_attribute AS area_attr ON u.id = area_attr.user_id
        WHERE
            ra.name = $1 AND 
            u.enabled IS TRUE AND
            (
                area_attr.name = $2 AND
                area_attr.value = $3
            )
        GROUP BY
            u.id;
    "#
            )
            .as_str(),
        )
        .await?;
    let params: Vec<&(dyn ToSql + Sync)> = vec![&realm, &AREA_ID_ATTR_NAME, &area_id];
    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let found_user_ids: Vec<String> = rows.into_iter().map(|row| row.get("id")).collect();
    Ok(found_user_ids.into_iter().collect())
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display)]
pub enum FilterOption {
    /// Those elements that contain the string are returned.
    IsLike(String),
    /// ILIKE but with unaccent and replacing blanks by single wildcards to detect hyphens.
    IsLikeUnaccentHyphens(String),
    /// Those elements that do not contain the string are returned.
    IsNotLike(String),
    /// Those elements that match precisely the string are returned.
    IsEqual(String),
    /// Those elements that do not match precisely the string are returned.
    IsNotEqual(String),
    /// When it is true, those elements that are null or empty are returned. When it is false they are discarded.
    IsEmpty(bool),
    /// Option not valid or set to null instead of an object, then it should not filter anything, display all.
    InvalidOrNull,
}

impl FilterOption {
    /// Get the parametrized sql clause which is a condition to filter at the given column, to be used in the WHERE clause.
    /// This function returns a tuple with the clause and the optional param, for which the param number must be provided.
    ///
    ///
    /// It is recommended to pass as param_number the current count of parameters in the dynamic sql query.
    /// If the returned parameter is Some, then the param count must be incremented by 1.
    fn get_sql_filter_clause(
        &self,
        col_name: &str,
        param_number: i32,
        operator: &str,
    ) -> (String, Option<String>) {
        match self {
            Self::IsLike(pattern) => (
                format!(
                    r#"(${param_number}::VARCHAR IS NULL OR {col_name} ILIKE ${param_number}){operator}"#,
                ),
                Some(format!("%{}%", pattern)),
            ),
            Self::IsLikeUnaccentHyphens(pattern) => {
                let pattern = pattern.replace(" ", "_"); // replace blanks by single wildcards to detect hyphens
                (
                    format!(
                        r#"('{pattern}'::VARCHAR IS NULL OR UNACCENT({col_name}) ILIKE ${param_number}){operator} "#,
                    ),
                    Some(format!("%{}%", pattern)),
                )
            }
            Self::IsNotLike(pattern) => (
                format!(
                    r#"({col_name} IS NULL OR {col_name} NOT ILIKE ${param_number}){operator} "#,
                ),
                Some(format!("%{}%", pattern)),
            ),
            Self::IsEqual(pattern) => (
                format!(r#"({col_name} = ${param_number}){operator} "#,),
                Some(pattern.into()),
            ),
            Self::IsNotEqual(pattern) => (
                format!(r#"({col_name} <> ${param_number}){operator} "#,),
                Some(pattern.into()),
            ),
            Self::IsEmpty(true) => (
                format!(r#"({col_name} IS NULL OR {col_name} = ''){operator} "#,),
                None,
            ),
            Self::IsEmpty(false) => (
                format!(r#"({col_name} IS NOT NULL AND {col_name} <> ''){operator} "#,),
                None,
            ),
            Self::InvalidOrNull => {
                ("".to_string(), None) // no filtering
            }
        }
    }
}

impl<'de> Deserialize<'de> for FilterOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        let map: HashMap<String, Value> = deserialize_value(value).map_err(|e| {
            serde::de::Error::custom(format!("Error parsing FilterOption o HMap: {e:?}"))
        })?;
        // Get the first key and value
        let (op, pattern_val) = map
            .iter()
            .next()
            .ok_or_else(|| serde::de::Error::custom("Error parsing FilterOption from map"))?;

        let filter: FilterOption = FilterOption::from_str(op).map_err(|e| {
            serde::de::Error::custom(format!("Error parsing FilterOption from str: {e:?}"))
        })?;

        let filter = match filter {
            FilterOption::InvalidOrNull => FilterOption::InvalidOrNull,
            FilterOption::IsEmpty(_) => {
                FilterOption::IsEmpty(pattern_val.as_bool().ok_or_else(|| {
                    serde::de::Error::custom(format!(
                        "Expected boolean value for IsEmpty Value: {pattern_val:?}"
                    ))
                })?)
            }
            FilterOption::IsLike(_) => {
                FilterOption::IsLike(deserialize_value(pattern_val.clone()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error parsing String value {pattern_val:?} for pattern: {e:?}"
                    ))
                })?)
            }
            FilterOption::IsLikeUnaccentHyphens(_) => FilterOption::IsLikeUnaccentHyphens(
                deserialize_value(pattern_val.clone()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error parsing String value {pattern_val:?} for pattern: {e:?}"
                    ))
                })?,
            ),
            FilterOption::IsNotLike(_) => {
                FilterOption::IsNotLike(deserialize_value(pattern_val.clone()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error parsing String value {pattern_val:?} for pattern: {e:?}"
                    ))
                })?)
            }
            FilterOption::IsEqual(_) => {
                FilterOption::IsEqual(deserialize_value(pattern_val.clone()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error parsing String value {pattern_val:?} for pattern: {e:?}"
                    ))
                })?)
            }
            FilterOption::IsNotEqual(_) => {
                FilterOption::IsNotEqual(deserialize_value(pattern_val.clone()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error parsing String value {pattern_val:?} for pattern: {e:?}"
                    ))
                })?)
            }
        };

        Ok(filter)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct ListUsersFilter {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub realm: String,
    pub search: Option<String>,
    pub first_name: Option<FilterOption>,
    pub last_name: Option<FilterOption>,
    pub username: Option<FilterOption>,
    pub email: Option<FilterOption>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub user_ids: Option<Vec<String>>,
    pub attributes: Option<HashMap<String, String>>,
    pub email_verified: Option<bool>,
    pub enabled: Option<bool>,
    pub sort: Option<HashMap<String, String>>,
    pub has_voted: Option<bool>,
    pub authorized_to_election_alias: Option<String>,
}

fn get_query_bool_condition(field: &str, value: Option<bool>) -> String {
    match value {
        Some(true) => format!(r#"AND u.{} = true"#, field),
        Some(false) => format!(r#"AND u.{} = false"#, field),
        None => "".to_string(),
    }
}

/// Gets sort clause ORDER BY, and the field parameter (column name or configurable attribute).
/// Checks if the field is valid and return None otherwise.
///
/// Maps the order input from the user into one of the valid options (ASC or DESC) to avoid injection, since we cannot put them as an sql parameter.
fn get_sort_clause_and_field_param(
    sort: Option<HashMap<String, String>>,
    param_number: i32,
) -> (String, Option<String>) {
    const ASC: &str = "ASC";
    const DESC: &str = "DESC";
    fn sanitize_string(s: &str) -> String {
        s.trim_matches('\'').to_string()
    }
    let (sort_field, verified_order) = match sort {
        Some(sort_fields) => {
            let field = sort_fields
                .get("'field'")
                .map(|f| sanitize_string(f))
                .unwrap_or_else(|| "id".to_string());

            let order = sort_fields
                .get("'order'")
                .map(|o| match sanitize_string(o).to_uppercase().as_str() {
                    ASC => ASC,
                    DESC => DESC,
                    _ => ASC,
                })
                .unwrap_or_else(|| ASC);
            (field, order.to_string())
        }
        None => ("id".to_string(), ASC.to_string()),
    };

    match sort_field.as_str() {
        "id" | "email" | "first_name" | "last_name" | "username" | "enabled" | "email_verified" => {
            (format!(r#"ORDER BY {sort_field} {verified_order}"#), None)
        }
        "has_voted" | "actions" => ("".to_string(), None),
        _ => (
            format!(
                r#"ORDER BY (SELECT value FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${}) {}"#,
                param_number, verified_order
            ),
            Some(sort_field),
        ),
    }
}

#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn list_users(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    filter: ListUsersFilter,
) -> Result<(Vec<User>, i32)> {
    info!("filter: {filter:?}");
    let low_sql_limit = PgConfig::from_env()?.low_sql_limit;
    let default_sql_limit = PgConfig::from_env()?.default_sql_limit;
    let query_limit: i64 =
        std::cmp::min(low_sql_limit, filter.limit.unwrap_or(default_sql_limit)).into();
    let query_offset: i64 = if let Some(offset_val) = filter.offset {
        offset_val.into()
    } else {
        0
    };

    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&filter.realm, &filter.user_ids];
    let mut next_param_number = 3;

    let mut filters_clause = "".to_string();
    let mut filter_params: Vec<String> = vec![];

    for tuple in [
        ("email", &filter.email),
        ("first_name", &filter.first_name),
        ("last_name", &filter.last_name),
        ("username", &filter.username),
    ] {
        let (col_name, filter_option) = tuple;
        match filter_option {
            Some(filter_obj) => {
                let (clause, param) =
                    filter_obj.get_sql_filter_clause(col_name, next_param_number, " AND");
                filters_clause.push_str(&clause);
                if let Some(param) = param {
                    next_param_number += 1;
                    filter_params.push(param.to_string());
                }
            }
            None => {}
        }
    }
    for filt_param in filter_params.iter() {
        params.push(filt_param);
    }

    let (area_ids, area_ids_join_clause, area_ids_where_clause) = get_area_ids(
        hasura_transaction,
        filter.election_id.clone(),
        filter.area_id.clone(),
        next_param_number,
    )
    .await?;
    if let Some(area_ids) = &area_ids {
        params.push(area_ids);
        next_param_number += 1;
    }

    let (election_alias, authorized_alias_join_clause, authorized_alias_where_clause) = match filter
        .authorized_to_election_alias
    {
        Some(election_alias) => (
            Some(election_alias),
            format!(
                r#"
            LEFT JOIN 
                user_attribute AS authorization_attr ON u.id = authorization_attr.user_id AND authorization_attr.name = ${}
            "#,
                next_param_number
            ),
            format!(
                r#"
            AND (
                authorization_attr.value = ${} OR authorization_attr.user_id IS NULL
            )
            "#,
                next_param_number + 1
            ),
        ),
        None => (None, "".to_string(), "".to_string()),
    };

    if election_alias.is_some() {
        params.push(&AUTHORIZED_ELECTION_IDS_NAME);
        params.push(&election_alias);
        next_param_number += 2;
    }

    let enabled_condition = get_query_bool_condition("enabled", filter.enabled);
    let email_verified_condition =
        get_query_bool_condition("email_verified", filter.email_verified);

    let mut dynamic_attr_conditions: Vec<String> = Vec::new();
    let mut dynamic_attr_params: Vec<Option<String>> = vec![];

    if let Some(attributes) = &filter.attributes {
        for (key, value) in attributes {
            dynamic_attr_conditions.push(format!(
                 r#"EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND UNACCENT(ua.value) ILIKE ${})"#,
                next_param_number,
                next_param_number + 1
            ));
            let val = Some(format!("%{value}%"));
            let formatted_keyy = key.trim_matches('\'').to_string();
            dynamic_attr_params.push(Some(formatted_keyy.clone()));
            dynamic_attr_params.push(val.clone());
            next_param_number += 2;
        }
    }
    for value in &dynamic_attr_params {
        params.push(value);
    }

    let dynamic_attr_clause = match dynamic_attr_conditions.is_empty() {
        true => "".to_string(),
        false => {
            format!(r#"AND({})"#, dynamic_attr_conditions.join(" OR "))
        }
    };

    let mut sort_params: Vec<Option<String>> = vec![];
    let (sort_clause, field_param) =
        get_sort_clause_and_field_param(filter.sort, next_param_number);

    if field_param.is_some() {
        sort_params.push(field_param);
        next_param_number += 1;
    }
    for value in &sort_params {
        params.push(value);
    }

    debug!("parameters count: {}", next_param_number - 1);
    debug!("params {:?}", params);
    let statement_str = format!(
        r#"
    SELECT
        u.id,
        u.email,
        u.email_verified,
        u.enabled,
        u.first_name,
        u.last_name,
        u.realm_id,
        u.username,
        u.created_timestamp,
        COALESCE(attr_json.attributes, '{{}}'::json) AS attributes
    FROM
        user_entity AS u
    INNER JOIN
        realm AS ra ON ra.id = u.realm_id
    {area_ids_join_clause}
    {authorized_alias_join_clause}
    LEFT JOIN LATERAL (
        SELECT
            json_object_agg(attr.name, attr.values_array) AS attributes
        FROM (
            SELECT
                ua.name,
                json_agg(ua.value) AS values_array
            FROM user_attribute ua
            WHERE ua.user_id = u.id
            GROUP BY ua.name
        ) attr
    ) attr_json ON true
    WHERE
        ra.name = $1 AND
        {filters_clause}
        (u.id = ANY($2) OR $2 IS NULL)
        {area_ids_where_clause}
        {authorized_alias_where_clause}
        {enabled_condition}
        {email_verified_condition}
        {dynamic_attr_clause}
    {sort_clause}
    LIMIT {query_limit} OFFSET {query_offset};
    "#
    );
    debug!("statement_str {statement_str:?}");

    let statement = keycloak_transaction.prepare(statement_str.as_str()).await?;
    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;
    let realm: &str = &filter.realm;
    info!(
        "Count rows {} for realm={realm}, query_limit={query_limit}",
        rows.len()
    );

    // Count the amount of users for pagination
    let count_statement_str = format!(
        r#"
    SELECT
        COUNT(*) as total_count
    FROM
        user_entity AS u
    INNER JOIN
        realm AS ra ON ra.id = u.realm_id
    {area_ids_join_clause}
    {authorized_alias_join_clause}
    WHERE
        ra.name = $1 AND
        {filters_clause}
        (u.id = ANY($2) OR $2 IS NULL)
        {area_ids_where_clause}
        {authorized_alias_where_clause}
        {enabled_condition}
        {email_verified_condition}
        {dynamic_attr_clause}
    ;
    "#
    );
    debug!("statement_str {count_statement_str:?}");

    let count_statement = keycloak_transaction
        .prepare(count_statement_str.as_str())
        .await?;
    let count_row: Row = keycloak_transaction
        .query_one(&count_statement, &params)
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let count: i32 = count_row.try_get::<&str, i64>("total_count")?.try_into()?;

    // Process the users
    let users = rows
        .into_iter()
        .map(|row| -> Result<User> { row.try_into() })
        .collect::<Result<Vec<User>>>()?;
    if let Some(ref some_election_event_id) = filter.election_event_id {
        let area_ids: Vec<String> = users.iter().filter_map(|user| user.get_area_id()).collect();
        let areas_by_ids = get_areas(
            hasura_transaction,
            filter.tenant_id.as_str(),
            some_election_event_id.as_str(),
            &area_ids,
        )
        .await
        .with_context(|| "can't find areas by ids")?;
        let get_area = |user: &User| {
            let area_id = user.get_area_id()?;
            return areas_by_ids.iter().find_map(|area| {
                let Some(ref area_dot_id) = area.id else {
                    return None;
                };
                if area_dot_id == &area_id {
                    Some(area.clone())
                } else {
                    None
                }
            });
        };
        let users_with_area = users
            .into_iter()
            .map(|user| {
                let area = get_area(&user);
                User {
                    area: area,
                    ..user.clone()
                }
            })
            .collect();
        Ok((users_with_area, count))
    } else {
        Ok((users, count))
    }
}

#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn list_users_with_vote_info(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    filter: ListUsersFilter,
) -> Result<(Vec<User>, i32)> {
    let tenant_id = filter.tenant_id.clone();
    let election_event_id = filter
        .election_event_id
        .clone()
        .ok_or(anyhow!("Election event id is empty"))?;
    let filter_by_has_voted = filter.has_voted.clone();
    let (users, users_count) = list_users(hasura_transaction, keycloak_transaction, filter)
        .await
        .with_context(|| "Error listing users")?;
    let users: Vec<User> = get_users_with_vote_info(
        hasura_transaction,
        tenant_id.as_str(),
        election_event_id.as_str(),
        users,
        filter_by_has_voted,
    )
    .await
    .with_context(|| "Error listing users with vote info")?;

    Ok((users, users_count))
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn count_keycloak_enabled_users(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
) -> Result<i64> {
    let statement = keycloak_transaction
        .prepare(
            format!(
                r#"
                SELECT
                    COUNT(DISTINCT u.id) AS total_users
                FROM
                    user_entity AS u
                INNER JOIN
                    realm AS ra ON ra.id = u.realm_id
                WHERE
                    ra.name = $1 AND 
                    u.enabled IS TRUE
                "#
            )
            .as_str(),
        )
        .await?;

    let params: Vec<&(dyn ToSql + Sync)> = vec![&realm];

    let row = keycloak_transaction
        .query_one(&statement, &params)
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let user_count: i64 = row.get("total_users");
    Ok(user_count)
}

#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn lookup_users(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    filter: ListUsersFilter,
) -> Result<Vec<User>> {
    let low_sql_limit = PgConfig::from_env()?.low_sql_limit;
    let default_sql_limit = PgConfig::from_env()?.default_sql_limit;
    let query_limit: i64 =
        std::cmp::min(low_sql_limit, filter.limit.unwrap_or(default_sql_limit)).into();

    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&filter.realm];
    let mut next_param_number = 2;

    let mut filters_clause = "".to_string();
    let mut filter_params: Vec<String> = vec![];
    for tuple in [
        ("email", &filter.email),
        ("first_name", &filter.first_name),
        ("last_name", &filter.last_name),
        ("username", &filter.username),
    ] {
        let (col_name, filter_option) = tuple;
        match filter_option {
            Some(filter_obj) => {
                let (clause, param) =
                    filter_obj.get_sql_filter_clause(col_name, next_param_number, "::int +");
                filters_clause.push_str(&clause);
                if let Some(param) = param {
                    next_param_number += 1;
                    filter_params.push(param.to_string());
                }
            }
            None => {}
        }
    }
    for filt_param in filter_params.iter() {
        params.push(filt_param);
    }

    let enabled_condition = get_query_bool_condition("enabled", filter.enabled);

    let mut dynamic_attr_conditions: Vec<String> = Vec::new();
    let mut dynamic_attr_params: Vec<Option<String>> = vec![];
    if let Some(attributes) = &filter.attributes {
        for (key, value) in attributes {
            dynamic_attr_conditions.push(format!(
                // r#"(ua.name = ${} AND UNACCENT(ua.value) ILIKE ${})::int"#,
                r#"(ua.name = ${} AND UNACCENT(ua.value) ILIKE ${})"#,
                next_param_number,
                next_param_number + 1
            ));
            let value = value.replace(" ", "_"); // replace blanks by single wildcards to detect hyphens
            let val = Some(format!("%{value}%"));
            let formatted_keyy = key.trim_matches('\'').to_string();
            dynamic_attr_params.push(Some(formatted_keyy.clone()));
            dynamic_attr_params.push(val.clone());
            next_param_number += 2;
        }
    }
    for value in &dynamic_attr_params {
        params.push(value);
    }
    let dynamic_attr_clause = if !dynamic_attr_conditions.is_empty() {
        dynamic_attr_conditions.join(" OR ")
    } else {
        "0=1".to_string() // Always false if no dynamic attributes are specified
    };

    debug!("parameters count: {}", next_param_number - 1);
    debug!("params {:?}", params);
    let statement_str = format!(
        r#"
        WITH matching_users AS (
            WITH matching_user_attributes AS (
                SELECT 
                    ua.user_id,
                    count(*) as matched_user_attributes
                FROM user_attribute ua
                WHERE
                    {dynamic_attr_clause}
                GROUP BY ua.user_id
                ORDER BY matched_user_attributes DESC
            )
            SELECT
                u.id,
                ({filters_clause}
                COALESCE(mua.matched_user_attributes, 0)) AS match_score
            FROM
                user_entity u
            LEFT JOIN 
                matching_user_attributes mua ON u.id = mua.user_id
            INNER JOIN 
                realm ra ON ra.id = u.realm_id
            WHERE
                ra.name = $1
                {enabled_condition}
            ORDER BY match_score DESC
        )
        SELECT 
            u.id,
            u.email,
            u.email_verified,
            u.enabled,
            u.first_name,
            u.last_name,
            u.realm_id,
            u.username,
            u.created_timestamp,
            COALESCE(attr_json.attributes, '{{}}'::json) AS attributes
        FROM 
            matching_users mu
        INNER JOIN 
            user_entity u ON u.id = mu.id
        INNER JOIN 
            realm ra ON ra.id = u.realm_id
        LEFT JOIN LATERAL (
            SELECT
                json_object_agg(attr.name, attr.values_array) AS attributes
            FROM (
                SELECT
                    ua.name,
                    json_agg(ua.value) AS values_array
                FROM user_attribute ua
                WHERE ua.user_id = u.id
                GROUP BY ua.name
            ) attr
        ) attr_json ON true
        WHERE 
            match_score > 0 AND
            match_score = (SELECT MAX(match_score) FROM matching_users);
    "#
    );

    debug!("statement: {}", statement_str);

    let statement = keycloak_transaction.prepare(statement_str.as_str()).await?;
    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;
    let realm: &str = &filter.realm;
    debug!(
        "Count rows {} for realm={realm}, query_limit={query_limit}",
        rows.len()
    );

    let users = rows
        .into_iter()
        .map(|row| -> Result<User> { row.try_into() })
        .collect::<Result<Vec<User>>>()?;
    if let Some(ref some_election_event_id) = filter.election_event_id {
        let area_ids: Vec<String> = users.iter().filter_map(|user| user.get_area_id()).collect();
        let areas_by_ids = get_areas(
            hasura_transaction,
            filter.tenant_id.as_str(),
            some_election_event_id.as_str(),
            &area_ids,
        )
        .await
        .with_context(|| "can't find areas by ids")?;
        let get_area = |user: &User| {
            let area_id = user.get_area_id()?;
            return areas_by_ids.iter().find_map(|area| {
                let Some(ref area_dot_id) = area.id else {
                    return None;
                };
                if area_dot_id == &area_id {
                    Some(area.clone())
                } else {
                    None
                }
            });
        };
        let users_with_area = users
            .into_iter()
            .map(|user| {
                let area = get_area(&user);
                User {
                    area: area,
                    ..user.clone()
                }
            })
            .collect();
        Ok(users_with_area)
    } else {
        Ok(users)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display)]
pub enum AttributesFilterBy {
    IsLike,      // Those elements that contain the string are returned
    IsEqual,     // Those elements that match precisely the string are returned
    NotExist,    // Those elements that Not exist with givin value
    PartialLike, // Those elements that Not exist with givin value
}

#[derive(Debug, Clone)]
pub struct AttributesFilterOption {
    pub value: String,
    pub filter_by: AttributesFilterBy,
}

impl AttributesFilterOption {
    /// Return the sql condition to filter at the given column, to be used in the WHERE clause
    pub fn get_sql_filter_clause(&self, index: usize) -> String {
        let filter_option = self;
        match filter_option.filter_by {
            AttributesFilterBy::IsLike => {
                format!("EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value ILIKE ${})",index - 1,index)
            }
            AttributesFilterBy::IsEqual => {
                format!("EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value = ${})",index -1, index)
            }
            AttributesFilterBy::NotExist => {
                format!("NOT EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value = ${})",index -1, index)
            }
            AttributesFilterBy::PartialLike => {
                format!("EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value ILIKE '%' || ${} || '%')",index -1, index)
            }
        }
    }
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn count_keycloak_enabled_users_by_attrs(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    attrs: Option<HashMap<String, AttributesFilterOption>>, // bool : true = equal, false = isLike
) -> Result<i64> {
    let mut attr_conditions = Vec::new();
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&realm];

    if let Some(attributes) = &attrs {
        for (attr_name, attr_value) in attributes.iter() {
            let clause = attr_value.get_sql_filter_clause(params.len() + 2);
            params.push(attr_name);
            params.push(&attr_value.value);

            attr_conditions.push(clause);
        }
    }

    let attr_conditions_sql = if attr_conditions.is_empty() {
        r#"TRUE"#.to_string()
    } else {
        attr_conditions.join(r#" AND "#)
    };

    let statement = keycloak_transaction
        .prepare(
            format!(
                r#"
            SELECT
                COUNT(DISTINCT u.id) AS total_users
            FROM
                user_entity AS u
            INNER JOIN
                realm AS ra ON ra.id = u.realm_id
            WHERE
                ra.name = $1 
                AND u.enabled IS TRUE
                AND ({attr_conditions_sql})
            "#
            )
            .as_str(),
        )
        .await?;

    let row = keycloak_transaction
        .query_one(&statement, &params)
        .await
        .map_err(|err| anyhow!("Error executing the query: {}", err))?;

    let user_count: i64 = row.get("total_users");
    Ok(user_count)
}
