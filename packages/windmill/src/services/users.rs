// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_areas;
use crate::services::cast_votes::get_users_with_vote_info;
use crate::services::database::PgConfig;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use futures::stream::Filter;
use sequent_core::types::keycloak::*;
use std::{collections::HashSet, convert::From};
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::{event, instrument, Level};
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
                    area_attr.value = ANY($9)
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ListUsersFilter {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub realm: String,
    pub search: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub user_ids: Option<Vec<String>>,
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
    let email_pattern: Option<String> = if let Some(email_val) = filter.email {
        Some(format!("%{email_val}%"))
    } else {
        None
    };
    let first_name_pattern: Option<String> = if let Some(first_name_val) = filter.first_name {
        Some(format!("%{first_name_val}%"))
    } else {
        None
    };
    let last_name_pattern: Option<String> = if let Some(last_name_val) = filter.last_name {
        Some(format!("%{last_name_val}%"))
    } else {
        None
    };
    let username_pattern: Option<String> = if let Some(username_val) = filter.username {
        Some(format!("%{username_val}%"))
    } else {
        None
    };
    let (area_ids, area_ids_join_clause, area_ids_where_clause) = get_area_ids(
        hasura_transaction,
        filter.election_id.clone(),
        filter.area_id.clone(),
    )
    .await?;
    let statement = keycloak_transaction.prepare(format!(r#"
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
            COALESCE(json_object_agg(attr.name, attr.value) FILTER (WHERE attr.name IS NOT NULL), '{{}}'::json) AS attributes,
            COUNT(u.id) OVER() AS total_count
        FROM
            user_entity AS u
        INNER JOIN
            realm AS ra ON ra.id = u.realm_id
        {area_ids_join_clause}
        LEFT JOIN
            user_attribute AS attr ON u.id = attr.user_id
        WHERE
            ra.name = $1 AND
            ($4::VARCHAR IS NULL OR email ILIKE $4) AND
            ($5::VARCHAR IS NULL OR first_name ILIKE $5) AND
            ($6::VARCHAR IS NULL OR last_name ILIKE $6) AND
            ($7::VARCHAR IS NULL OR username ILIKE $7) AND
            (u.id = ANY($8) OR $8 IS NULL)
            {area_ids_where_clause}
        GROUP BY
            u.id
        LIMIT $2 OFFSET $3;
    "#).as_str()).await?;
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![
        &filter.realm,
        &query_limit,
        &query_offset,
        &email_pattern,
        &first_name_pattern,
        &last_name_pattern,
        &username_pattern,
        &filter.user_ids,
    ];
    if area_ids.is_some() {
        params.push(&area_ids);
    }
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
    let (users, users_count) = list_users(hasura_transaction, keycloak_transaction, filter)
        .await
        .with_context(|| "Error listing users")?;
    let users = get_users_with_vote_info(
        hasura_transaction,
        tenant_id.as_str(),
        election_event_id.as_str(),
        users,
    )
    .await
    .with_context(|| "Error listing users with vote info")?;

    Ok((users, users_count))
}
