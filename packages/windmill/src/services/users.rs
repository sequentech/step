// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::area::get_areas_by_ids;
use anyhow::{anyhow, Context, Result};
use keycloak::types::{CredentialRepresentation, UserRepresentation};
use sequent_core::services::connection;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::keycloak::*;
use serde_json::Value;
use std::collections::HashMap;
use std::convert::From;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::services::database::{get_hasura_pool, get_keycloak_pool, PgConfig};
use deadpool_postgres::{Client as DbClient, Pool, PoolError, Runtime, Transaction};
use tokio_postgres::row::Row;
use tokio_postgres::types::{BorrowToSql, ToSql, Type as SqlType};

#[instrument(skip(auth_headers, admin), err)]
pub async fn list_users(
    auth_headers: connection::AuthHeaders,
    transaction: &Transaction<'_>,
    admin: &KeycloakAdminClient,
    tenant_id: String,
    election_event_id: Option<String>,
    election_id: Option<String>,
    realm: &str,
    search: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    username: Option<String>,
    email: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
    user_ids: Option<Vec<String>>,
) -> Result<(Vec<User>, i32)> {
    let low_sql_limit = PgConfig::from_env()?.low_sql_limit;
    let default_sql_limit = PgConfig::from_env()?.default_sql_limit;
    let query_limit: i64 = std::cmp::min(low_sql_limit, limit.unwrap_or(default_sql_limit)).into();
    let query_offset: i64 = if let Some(offset_val) = offset {
        offset_val.into()
    } else {
        0
    };
    let email_pattern: Option<String> = if let Some(email_val) = email {
        Some(format!("%{email_val}%"))
    } else {
        None
    };
    let first_name_pattern: Option<String> = if let Some(first_name_val) = first_name {
        Some(format!("%{first_name_val}%"))
    } else {
        None
    };
    let last_name_pattern: Option<String> = if let Some(last_name_val) = last_name {
        Some(format!("%{last_name_val}%"))
    } else {
        None
    };
    let username_pattern: Option<String> = if let Some(username_val) = username {
        Some(format!("%{username_val}%"))
    } else {
        None
    };
    event!(Level::INFO, "before area ids");
    let area_ids: Option<Vec<String>> = match election_id {
        Some(ref election_id) => {
            let mut hasura_db_client: DbClient = get_hasura_pool()
                .await
                .get()
                .await
                .with_context(|| "Error acquiring hasura db client")?;
            event!(Level::INFO, "generating prepared statement");
            let election_uuid: uuid::Uuid = Uuid::parse_str(&election_id)
                .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;

            let areas_statement = hasura_db_client
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
            event!(Level::INFO, "getting the area rows");
            let rows: Vec<Row> = hasura_db_client
                .query(&areas_statement, &[&election_uuid])
                .await
                .map_err(|err| anyhow!("Error running the areas query: {}", err))?;
            let len = rows.len();
            event!(Level::INFO, "rows.len() = {len} getting the area ids");
            let area_ids: Vec<String> = rows
                .into_iter()
                .map(|row| -> Result<String> {
                    Ok(row
                        .try_get::<&str, String>("id")
                        .map_err(|err| anyhow!("Error getting the area id of a row: {}", err))?)
                })
                .collect::<Result<Vec<String>>>()
                .map_err(|err| anyhow!("Error getting the areas ids: {}", err))?;
            event!(Level::INFO, "area ids = {area_ids:?}");

            Some(area_ids)
        }
        None => {
            event!(Level::INFO, "NO election_id");
            None
        }
    };
    let statement = transaction.prepare(format!(r#"
        WITH realm_cte AS (
            SELECT id FROM realm WHERE name = $1
        )
        SELECT
            sub.id,
            sub.email,
            sub.email_verified,
            sub.enabled,
            sub.first_name,
            sub.last_name,
            sub.realm_id,
            sub.username,
            sub.created_timestamp,
            sub.attributes,
            sub.total_count
        FROM (
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
                realm_cte ON realm_cte.id = u.realm_id
            INNER JOIN
                user_attribute AS area_attr ON u.id = area_attr.user_id
            LEFT JOIN
                user_attribute AS attr ON u.id = attr.user_id
            WHERE
                (
                    area_attr.name = '{AREA_ID_ATTR_NAME}' AND
                    (area_attr.value = ANY($4) OR $4 IS NULL)
                ) AND
                ($5::VARCHAR IS NULL OR email ILIKE $5) AND
                ($6::VARCHAR IS NULL OR first_name ILIKE $6) AND
                ($7::VARCHAR IS NULL OR last_name ILIKE $7) AND
                ($8::VARCHAR IS NULL OR username ILIKE $8) AND
                (u.id = ANY($9) OR $9 IS NULL)
            GROUP BY
                u.id
        ) sub
        LIMIT $2 OFFSET $3;
    "#).as_str()).await?;
    event!(Level::INFO, "generating keycloak prepared statement");
    let rows: Vec<Row> = transaction
        .query(
            &statement,
            &[
                &realm,
                &query_limit,
                &query_offset,
                &area_ids,
                &email_pattern,
                &first_name_pattern,
                &last_name_pattern,
                &username_pattern,
                &user_ids,
            ],
        )
        .await
        .map_err(|err| anyhow!("{}", err))?;
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

    if let Some(ref some_election_event_id) = election_event_id {
        let area_ids: Vec<String> = users.iter().filter_map(|user| user.get_area_id()).collect();
        let areas_by_ids = get_areas_by_ids(
            auth_headers.clone(),
            tenant_id,
            some_election_event_id.clone(),
            area_ids,
        )
        .await
        .map_err(|err| anyhow!("{:?}", err))?
        .data
        .with_context(|| "can't find areas by ids")?
        .sequent_backend_area;
        let get_area = |user: &User| {
            let area_id = user.get_area_id()?;
            return areas_by_ids.iter().find_map(|area| {
                if (area.id == area_id) {
                    Some(UserArea {
                        id: Some(area.id.clone()),
                        name: area.name.clone(),
                    })
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
