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

use deadpool_postgres::{Client as DbClient, Pool, PoolError, Runtime};
use tokio_postgres::types::{BorrowToSql, ToSql, Type as SqlType};
use tokio_postgres::row::Row;
use crate::services::database::{get_database_pool, PgConfig};

#[instrument(skip(auth_headers, admin), err)]
pub async fn list_users(
    auth_headers: connection::AuthHeaders,
    admin: &KeycloakAdminClient,
    tenant_id: String,
    election_event_id: Option<String>,
    realm: &str,
    search: Option<String>,
    email: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<(Vec<User>, i32)> {
    let db_client: DbClient = get_database_pool().await.get().await
        .map_err(|err| anyhow!("{}", err))?;

    let low_sql_limit = PgConfig::from_env()?.low_sql_limit;
    let query_limit: i64 = 
        std::cmp::min(
            low_sql_limit,
            limit.unwrap_or(low_sql_limit)
        ).into();

    let statement = db_client.prepare_cached(
            r#"
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
                    json_object_agg(attr.name, attr.value) AS attributes,
                    COUNT(u.id) OVER() AS total_count
                FROM
                    user_entity AS u
                JOIN
                    user_attribute AS attr ON u.id = attr.user_id
                WHERE 
                    u.realm_id = $1 AND
                    (u.email = $2 OR $2 IS NULL)
                GROUP BY
                    u.id
            ) sub
            LIMIT $3;
        "#,
    ).await?;
    let rows: Vec<Row> = db_client
        .query(
            &statement,
            &[
                &realm,
                &email,
                &query_limit,
            ]
        ).await
        .map_err(|err| anyhow!("{}", err))?;

    // all rows contain the count and if there's no rows well, count is clearly
    // zero
    let count: i32 = if rows.len() == 0 {
        0
    } else {
        rows[0].try_get::<&str, i32>("total_count")?
    };
    let users = rows
        .into_iter()
        .map(|row| -> Result<User> { row.try_into() })
        .collect::<Result<Vec<User>>>()?;

    if let Some(ref some_election_event_id) = election_event_id {
        let area_ids: Vec<String> = users
            .iter()
            .filter_map(|user| {
                Some(
                    user.attributes.as_ref()?.get("area-id")?.as_array()?[0]
                        .as_str()?
                        .to_string(),
                )
            })
            .collect();
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
            let area_id = user.attributes.as_ref()?.get("area-id")?.as_array()?[0]
                .as_str()?
                .to_string();
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
