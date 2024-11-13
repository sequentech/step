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
use tracing::{event, info, instrument, Level};
use uuid::Uuid;

#[instrument(skip(hasura_transaction), err)]
async fn get_area_ids(
    hasura_transaction: &Transaction<'_>,
    election_id: Option<String>,
    area_id: Option<String>,
) -> Result<(Option<Vec<String>>, String, String)> {
    let res = match election_id {
        Some(ref election_id) => {
            let election_uuid: uuid::Uuid = Uuid::parse_str(&election_id)
                .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;

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
                            Ok(row.try_get::<&str, String>("id").map_err(|err| {
                                anyhow!("Error getting the area id of a row: {}", err)
                            })?)
                        })
                        .collect::<Result<Vec<String>>>()
                        .map_err(|err| anyhow!("Error getting the areas ids: {}", err))?;
                    area_ids
                }
            };

            (
                Some(area_ids),
                String::from(
                    r#"
                INNER JOIN 
                    user_attribute AS area_attr ON u.id = area_attr.user_id
                "#,
                ),
                format!(
                    r#"
                AND (
                    area_attr.name = '{AREA_ID_ATTR_NAME}' AND
                    area_attr.value = ANY($5)
                )
                "#
                ),
            )
        }
        None => (None, String::from(""), String::from("")),
    };
    Ok(res)
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
                area_attr.name = '{AREA_ID_ATTR_NAME}' AND
                area_attr.value = '{area_id}'
            )
        GROUP BY
            u.id;
    "#
            )
            .as_str(),
        )
        .await?;
    let params: Vec<&(dyn ToSql + Sync)> = vec![&realm];
    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let found_user_ids: Vec<String> = rows.into_iter().map(|row| row.get("id")).collect();
    Ok(found_user_ids.into_iter().collect())
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display)]
pub enum FilterOption {
    IsLike(String),     // Those elements that contain the string are returned
    IsNotLike(String),  // Those elements that do not contain the string are returned
    IsEqual(String),    // Those elements that match precisely the string are returned
    IsNotEqual(String), // Those elements that do not match precisely the string are returned
    IsEmpty(bool), // When it is true, those elements that are null or empty are returned. When it is false they are discarded
    InvalidOrNull, // Option not valid or set to null instead of an object, then it should not filter anything, display all.
}

impl FilterOption {
    /// Return the sql condition to filter at the given column, to be used in the WHERE clause
    fn get_sql_filter_clause(&self, col_name: &str, operator: &str) -> String {
        match self {
            Self::IsLike(pattern) => {
                format!(
                    r#"('{pattern}'::VARCHAR IS NULL OR {col_name} ILIKE '%{pattern}%') {operator}"#,
                )
            }
            Self::IsNotLike(pattern) => {
                format!(r#"({col_name} IS NULL OR {col_name} NOT ILIKE '%{pattern}%') {operator}"#,)
            }
            Self::IsEqual(pattern) => {
                format!(r#"({col_name} = '{pattern}') {operator}"#,)
            }
            Self::IsNotEqual(pattern) => {
                format!(r#"({col_name} <> '{pattern}') {operator}"#,)
            }
            Self::IsEmpty(true) => {
                format!(r#"({col_name} IS NULL OR {col_name} = '') {operator}"#,)
            }
            Self::IsEmpty(false) => {
                format!(r#"({col_name} IS NOT NULL AND {col_name} <> '') {operator}"#,)
            }
            Self::InvalidOrNull => {
                "".to_string() // no filtering
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

#[derive(Debug, PartialEq, Eq, Clone)]
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
        Some(true) => format!("AND u.{} = true", field),
        Some(false) => format!("AND u.{} = false", field),
        None => "".to_string(),
    }
}

fn get_sort_order_and_field(sort: Option<HashMap<String, String>>) -> (String, String) {
    fn sanitize_string(s: &str) -> String {
        s.trim_matches('\'').to_string()
    }
    match sort {
        Some(sort_fields) => {
            let field = sort_fields
                .get("'field'")
                .map(|f| sanitize_string(f))
                .unwrap_or_else(|| "id".to_string());

            let order = sort_fields
                .get("'order'")
                .map(|o| sanitize_string(o).to_uppercase())
                .unwrap_or_else(|| "ASC".to_string());

            (field, order)
        }
        None => ("id".to_string(), "ASC".to_string()),
    }
}

#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn list_users(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    filter: ListUsersFilter,
) -> Result<(Vec<User>, i32)> {
    let low_sql_limit = PgConfig::from_env()?.low_sql_limit;
    let default_sql_limit = PgConfig::from_env()?.default_sql_limit;
    let query_limit: i64 =
        std::cmp::min(low_sql_limit, filter.limit.unwrap_or(default_sql_limit)).into();
    let query_offset: i64 = if let Some(offset_val) = filter.offset {
        offset_val.into()
    } else {
        0
    };

    let email_filter_clause = if let Some(email_filter) = filter.email {
        email_filter.get_sql_filter_clause("email", "AND")
    } else {
        "".to_string()
    };

    let first_name_filter_clause = if let Some(first_name_filter) = filter.first_name {
        first_name_filter.get_sql_filter_clause("first_name", "AND")
    } else {
        "".to_string()
    };

    let last_name_filter_clause = if let Some(last_name_filter) = filter.last_name {
        last_name_filter.get_sql_filter_clause("last_name", "AND")
    } else {
        "".to_string()
    };

    let username_filter_clause = if let Some(username_filter) = filter.username {
        username_filter.get_sql_filter_clause("username", "AND")
    } else {
        "".to_string()
    };

    let (area_ids, area_ids_join_clause, area_ids_where_clause) = get_area_ids(
        hasura_transaction,
        filter.election_id.clone(),
        filter.area_id.clone(),
    )
    .await?;

    let mut params_count = 5;

    if area_ids.is_some() {
        params_count += 1;
    }

    let (election_alias, authorized_alias_join_clause, authorized_alias_where_clause) = match filter
        .authorized_to_election_alias
    {
        Some(election_alias) => (
            Some(election_alias),
            format!(
                r#"
            LEFT JOIN 
                user_attribute AS authorization_attr ON u.id = authorization_attr.user_id AND authorization_attr.name = '{AUTHORIZED_ELECTION_IDS_NAME}'
            "#,
            ),
            format!(
                r#"
            AND (
                authorization_attr.value = ${} OR authorization_attr.user_id IS NULL
            )
            "#,
                params_count
            ),
        ),
        None => (None, "".to_string(), "".to_string()),
    };

    if election_alias.is_some() {
        params_count += 1;
    }

    let enabled_condition = get_query_bool_condition("enabled", filter.enabled);
    let email_verified_condition =
        get_query_bool_condition("email_verified", filter.email_verified);

    let mut dynamic_attr_conditions: Vec<String> = Vec::new();
    let mut dynamic_attr_params: Vec<Option<String>> = vec![];

    if let Some(attributes) = &filter.attributes {
        let mut attr_placeholder_count = params_count;

        for (key, value) in attributes {
            dynamic_attr_conditions.push(format!(
                "EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value ILIKE ${})",
                attr_placeholder_count,
                attr_placeholder_count + 1
            ));
            let val = Some(format!("%{value}%"));
            let formatted_keyy = key.trim_matches('\'').to_string();
            dynamic_attr_params.push(Some(formatted_keyy.clone()));
            dynamic_attr_params.push(val.clone());
            attr_placeholder_count += 2;
        }
    }

    let dynamic_attr_clause = if !dynamic_attr_conditions.is_empty() {
        dynamic_attr_conditions.join(" OR ")
    } else {
        "1=1".to_string() // Always true if no dynamic attributes are specified
    };

    let (sort_field, sort_order) = get_sort_order_and_field(filter.sort);

    let sort_clause = if [
        "id",
        "email",
        "first_name",
        "last_name",
        "username",
        "enabled",
        "email_verified",
    ]
    .contains(&sort_field.as_str())
    {
        format!("{} {}", sort_field, sort_order)
    } else {
        format!(
            "(SELECT value FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = '{}') {}",
            sort_field, sort_order
        )
    };

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
        COALESCE(attr_json.attributes, '{{}}'::json) AS attributes,
        COUNT(u.id) OVER() AS total_count
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
        {email_filter_clause}
        {first_name_filter_clause}
        {last_name_filter_clause}
        {username_filter_clause}
        (u.id = ANY($4) OR $4 IS NULL)
        {area_ids_where_clause}
        {authorized_alias_where_clause}
        {enabled_condition}
        {email_verified_condition}
    AND ({dynamic_attr_clause})
    ORDER BY {sort_clause}
    LIMIT $2 OFFSET $3;
    "#
    );

    info!("statement: {}", statement_str);

    let statement = keycloak_transaction.prepare(statement_str.as_str()).await?;

    let mut params: Vec<&(dyn ToSql + Sync)> =
        vec![&filter.realm, &query_limit, &query_offset, &filter.user_ids];

    if area_ids.is_some() {
        params.push(&area_ids);
    }

    if election_alias.is_some() {
        params.push(&election_alias)
    }

    for value in &dynamic_attr_params {
        params.push(value);
    }

    info!("params {:?}", params);

    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;
    let realm: &str = &filter.realm;
    event!(
        Level::INFO,
        "Count rows {} for realm={realm}, query_limit={query_limit}",
        rows.len()
    );

    // all rows contain the count and if there's no rows well, count is clearly
    // zero
    let count: i32 = if rows.len() == 0 {
        0
    } else {
        rows[0].try_get::<&str, i64>("total_count")?.try_into()?
    };
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
    let users = get_users_with_vote_info(
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
) -> Result<(Vec<User>, i32)> {
    let low_sql_limit = PgConfig::from_env()?.low_sql_limit;
    let default_sql_limit = PgConfig::from_env()?.default_sql_limit;
    let query_limit: i64 =
        std::cmp::min(low_sql_limit, filter.limit.unwrap_or(default_sql_limit)).into();
    let query_offset: i64 = if let Some(offset_val) = filter.offset {
        offset_val.into()
    } else {
        0
    };

    let email_filter_clause = if let Some(email_filter) = filter.email {
        email_filter.get_sql_filter_clause("email", "OR")
    } else {
        "".to_string()
    };

    let first_name_filter_clause = if let Some(first_name_filter) = filter.first_name {
        first_name_filter.get_sql_filter_clause("first_name", "OR")
    } else {
        "".to_string()
    };

    let last_name_filter_clause = if let Some(last_name_filter) = filter.last_name {
        last_name_filter.get_sql_filter_clause("last_name", "OR")
    } else {
        "".to_string()
    };

    let username_filter_clause = if let Some(username_filter) = filter.username {
        username_filter.get_sql_filter_clause("username", "OR")
    } else {
        "".to_string()
    };

    let (area_ids, area_ids_join_clause, area_ids_where_clause) = get_area_ids(
        hasura_transaction,
        filter.election_id.clone(),
        filter.area_id.clone(),
    )
    .await?;

    let mut params_count = 5;

    if area_ids.is_some() {
        params_count += 1;
    }

    let (election_alias, authorized_alias_join_clause, authorized_alias_where_clause) = match filter
        .authorized_to_election_alias
    {
        Some(election_alias) => (
            Some(election_alias),
            format!(
                r#"
            LEFT JOIN 
                user_attribute AS authorization_attr ON u.id = authorization_attr.user_id AND authorization_attr.name = '{AUTHORIZED_ELECTION_IDS_NAME}'
            "#,
            ),
            format!(
                r#"
            AND (
                authorization_attr.value = ${} OR authorization_attr.user_id IS NULL
            )
            "#,
                params_count
            ),
        ),
        None => (None, "".to_string(), "".to_string()),
    };

    if election_alias.is_some() {
        params_count += 1;
    }

    let enabled_condition = get_query_bool_condition("enabled", filter.enabled);
    let email_verified_condition =
        get_query_bool_condition("email_verified", filter.email_verified);

    let mut dynamic_attr_conditions: Vec<String> = Vec::new();
    let mut dynamic_attr_params: Vec<Option<String>> = vec![];

    if let Some(attributes) = &filter.attributes {
        let mut attr_placeholder_count = params_count;

        for (key, value) in attributes {
            dynamic_attr_conditions.push(format!(
                "EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value ILIKE ${})",
                attr_placeholder_count,
                attr_placeholder_count + 1
            ));
            let val = Some(format!("%{value}%"));
            let formatted_keyy = key.trim_matches('\'').to_string();
            dynamic_attr_params.push(Some(formatted_keyy.clone()));
            dynamic_attr_params.push(val.clone());
            attr_placeholder_count += 2;
        }
    }

    let dynamic_attr_clause = if !dynamic_attr_conditions.is_empty() {
        dynamic_attr_conditions.join(" OR ")
    } else {
        "1=0".to_string() // Always true if no dynamic attributes are specified
    };

    let (sort_field, sort_order) = get_sort_order_and_field(filter.sort);

    let sort_clause = if [
        "id",
        "email",
        "first_name",
        "last_name",
        "username",
        "enabled",
        "email_verified",
    ]
    .contains(&sort_field.as_str())
    {
        format!("{} {}", sort_field, sort_order)
    } else {
        format!(
            "(SELECT value FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = '{}') {}",
            sort_field, sort_order
        )
    };

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
        COALESCE(attr_json.attributes, '{{}}'::json) AS attributes,
        COUNT(u.id) OVER() AS total_count
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
        ra.name = $1 AND (
            {email_filter_clause}
            {first_name_filter_clause}
            {last_name_filter_clause}
            {username_filter_clause}
            1=0 OR ({dynamic_attr_clause})
        ) AND
        (u.id = ANY($4) OR $4 IS NULL)
        {area_ids_where_clause}
        {authorized_alias_where_clause}
        {enabled_condition}
        {email_verified_condition}
    ORDER BY {sort_clause}
    LIMIT $2 OFFSET $3;
    "#
    );

    info!("statement: {}", statement_str);

    let statement = keycloak_transaction.prepare(statement_str.as_str()).await?;

    let mut params: Vec<&(dyn ToSql + Sync)> =
        vec![&filter.realm, &query_limit, &query_offset, &filter.user_ids];

    if area_ids.is_some() {
        params.push(&area_ids);
    }

    if election_alias.is_some() {
        params.push(&election_alias)
    }

    for value in &dynamic_attr_params {
        params.push(value);
    }

    info!("params {:?}", params);

    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;
    let realm: &str = &filter.realm;
    event!(
        Level::INFO,
        "Count rows {} for realm={realm}, query_limit={query_limit}",
        rows.len()
    );

    // all rows contain the count and if there's no rows well, count is clearly
    // zero
    let count: i32 = if rows.len() == 0 {
        0
    } else {
        rows[0].try_get::<&str, i64>("total_count")?.try_into()?
    };
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

#[instrument(skip(keycloak_transaction), err)]
pub async fn count_keycloak_enabled_users_by_attrs(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    attrs: Option<HashMap<String, String>>,
) -> Result<i64> {
    let mut attr_conditions = Vec::new();
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&realm];

    if let Some(attributes) = &attrs {
        for (attr_name, attr_value) in attributes.iter() {
            params.push(attr_name);
            params.push(attr_value);

            attr_conditions.push(format!(
                "EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value = ${})",
                params.len() - 1,
                params.len()
            ));
        }
    }

    let attr_conditions_sql = if attr_conditions.is_empty() {
        "TRUE".to_string()
    } else {
        attr_conditions.join(" AND ")
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
