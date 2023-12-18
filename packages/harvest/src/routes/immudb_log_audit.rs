// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::resources::{
    Aggregate, DataList, OrderDirection, TotalAggregate,
};
use anyhow::{anyhow, Context, Result};
use immudb_rs::{sql_value::Value as SqlValue, Client, Row};
use regex::Regex;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use strum_macros::{Display, EnumString, ToString};
use tracing::instrument;
use windmill::services::database::PgConfig;

macro_rules! assign_value {
    ($enum_variant:path, $value:expr, $target:ident) => {
        match $value.value.as_ref() {
            Some($enum_variant(inner)) => {
                $target = inner.clone();
            }
            _ => {
                return Err(
                    anyhow!(
                        r#"invalid column value for `$enum_variant`, `$value`, 
                        `$target`"#
                    )
                );
            }
        }
    };
}

// Enumeration for the valid fields in the immudb table
#[derive(Debug, Deserialize, Hash, PartialEq, Eq, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
enum OrderField {
    Id,
    AuditType,
    Class,
    Command,
    Dbname,
    ServerTimestamp,
    SessionId,
    Statement,
    User,
}

#[derive(
    Debug, Default, Deserialize, Hash, PartialEq, Eq, EnumString, Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
enum AuditTable {
    #[default]
    PgauditHasura,
    PgauditKeycloak,
}

#[derive(Deserialize, Debug)]
pub struct GetPgauditBody {
    tenant_id: String,
    election_event_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
    filter: Option<HashMap<OrderField, String>>,
    order_by: Option<HashMap<OrderField, OrderDirection>>,
    #[serde(default)]
    audit_table: AuditTable,
}

impl GetPgauditBody {
    // Returns the SQL clauses related to the request
    #[instrument(ret)]
    fn as_sql_clauses(&self, to_count: bool) -> Result<String> {
        let mut clauses = Vec::new();
        let invalid_chars_re = Regex::new(r"['-/]")?;

        // Handle filters
        if let Some(filters_map) = &self.filter {
            let where_clauses: Vec<String> = filters_map
                .iter()
                .filter_map(|(field, value)| {
                    match field {
                        OrderField::Id => {
                            let int_value: i64 = value.parse().ok()?;
                            Some(format!("id = {int_value}"))
                        }
                        // Don't support filtering by timestamp yet
                        OrderField::ServerTimestamp => None,
                        _ => {
                            let sanitized_value =
                                invalid_chars_re.replace_all(value, "");
                            Some(format!(
                                "{field} LIKE '(?i){sanitized_value}'"
                            ))
                        }
                    }
                })
                .collect();
            if !where_clauses.is_empty() {
                clauses.push(format!("WHERE {}", where_clauses.join(" AND ")));
            }
        }

        // Handle order_by
        if !to_count {
            if let Some(order_by_map) = &self.order_by {
                let order_clauses: Vec<String> = order_by_map
                    .iter()
                    .map(|(field, direction)| format!("{field} {direction}"))
                    .collect();
                if !order_clauses.is_empty() {
                    clauses
                        .push(format!("ORDER BY {}", order_clauses.join(", ")));
                }
            }

            // Handle limit
            let limit = self
                .limit
                .unwrap_or(PgConfig::from_env()?.default_sql_limit.into());
            clauses.push(format!(
                "LIMIT {}",
                std::cmp::min(
                    limit,
                    PgConfig::from_env()?.low_sql_limit.into()
                )
            ));

            // Handle offset
            if let Some(offset) = self.offset {
                clauses.push(format!("OFFSET {}", std::cmp::max(offset, 0)));
            }
        }

        Ok(clauses.join(" "))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PgAuditRow {
    id: i64,
    audit_type: String,
    class: String,
    command: String,
    dbname: String,
    server_timestamp: i64,
    session_id: String,
    statement: String,
    user: String,
}

impl TryFrom<&Row> for PgAuditRow {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut id = 0;
        let _audit_type = String::from("");
        let mut class = String::from("");
        let mut command = String::from("");
        let mut dbname = String::from("");
        let mut server_timestamp: i64 = 0;
        let mut session_id = String::from("");
        let mut statement = String::from("");
        let mut user = String::from("");
        let mut audit_type = String::from("");

        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            match column.as_str() {
                c if c.ends_with(".id)") => assign_value!(SqlValue::N, value, id),
                c if c.ends_with(".audit_type)") => {
                    assign_value!(SqlValue::S, value, audit_type)
                }
                c if c.ends_with(".class)") => assign_value!(SqlValue::S, value, class),
                c if c.ends_with(".command)") => {
                    assign_value!(SqlValue::S, value, command)
                }
                c if c.ends_with(".dbname)") => assign_value!(SqlValue::S, value, dbname),
                c if c.ends_with(".server_timestamp)") => {
                    assign_value!(SqlValue::Ts, value, server_timestamp)
                }
                c if c.ends_with(".session_id)") => {
                    assign_value!(SqlValue::S, value, session_id)
                }
                c if c.ends_with(".statement)") => {
                    assign_value!(SqlValue::S, value, statement)
                }
                c if c.ends_with(".user)") => assign_value!(SqlValue::S, value, user),
                c if c.ends_with(".audit_type)") => {
                    assign_value!(SqlValue::S, value, audit_type)
                }
                _ => {
                    return Err(anyhow!(
                        "invalid column found '{}'",
                        column.as_str()
                    ))
                }
            }
        }
        Ok(PgAuditRow {
            id,
            audit_type,
            class,
            command,
            dbname,
            server_timestamp,
            session_id,
            statement,
            user,
        })
    }
}

impl TryFrom<&Row> for Aggregate {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut count = 0;

        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            match column.as_str() {
                _ => assign_value!(SqlValue::N, value, count),
            }
        }
        Ok(Aggregate { count })
    }
}

#[instrument]
#[post("/immudb/pgaudit-list", format = "json", data = "<body>")]
pub async fn list_pgaudit(
    body: Json<GetPgauditBody>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<DataList<PgAuditRow>>, Debug<anyhow::Error>> {
    let server_url = env::var("IMMUDB_SERVER_URL")
        .context("IMMUDB_SERVER_URL env var not set")?;
    let username =
        env::var("IMMUDB_USER").context("IMMUDB_USER env var not set")?;
    let password = env::var("IMMUDB_PASSWORD")
        .context("IMMUDB_PASSWORD env var not set")?;
    let input = body.into_inner();

    let mut client = Client::new(&server_url, &username, &password).await?;
    client.login().await?;

    client.open_session(&input.election_event_id).await?;
    let clauses = input.as_sql_clauses(false)?;
    let clauses_to_count = input.as_sql_clauses(true)?;
    let audit_table = input.audit_table;
    let sql = format!(
        r#"
        SELECT
            id,
            audit_type,
            class,
            command,
            dbname,
            server_timestamp,
            session_id,
            statement,
            user
        FROM {audit_table}
        {clauses}
        "#,
    );
    let sql_query_response = client.sql_query(&sql, vec![]).await?;
    let items = sql_query_response
        .get_ref()
        .rows
        .iter()
        .map(PgAuditRow::try_from)
        .collect::<Result<Vec<PgAuditRow>>>()?;

    let sql = format!(
        r#"
        SELECT
            COUNT(*)
        FROM {audit_table}
        {clauses_to_count}
        "#,
    );
    let sql_query_response = client.sql_query(&sql, vec![]).await?;
    let mut rows_iter = sql_query_response
        .get_ref()
        .rows
        .iter()
        .map(Aggregate::try_from);

    let aggregate = rows_iter
        .next() // get the first item
        .ok_or(anyhow!("No aggregate found"))??; // unwrap the Result and Option

    client.close_session().await?;
    Ok(Json(DataList {
        items: items,
        total: TotalAggregate {
            aggregate: aggregate,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_as_sql_clauses() {
        let get_pgaudit_body: GetPgauditBody = serde_json::from_value(json!({
            "tenant_id": "some_tenant",
            "election_event_id": "some_event",
            "order_by": {"id":"asc"}
        }))
        .unwrap();
        assert_eq!(
            get_pgaudit_body.as_sql_clauses(false).unwrap(),
            "ORDER BY id asc LIMIT 20"
        );

        let get_pgaudit_body: GetPgauditBody = serde_json::from_value(json!({
            "tenant_id": "some_tenant",
            "election_event_id": "some_event",
            "limit": 15,
            "offset": 5,
            "order_by": {"id":"asc"}
        }))
        .unwrap();
        assert_eq!(
            get_pgaudit_body.as_sql_clauses(false).unwrap(),
            "ORDER BY id asc LIMIT 15 OFFSET 5"
        );
        assert_eq!(get_pgaudit_body.as_sql_clauses(true).unwrap(), "");

        let get_pgaudit_body: GetPgauditBody = serde_json::from_value(json!({
            "tenant_id": "some_tenant",
            "election_event_id": "some_event",
            "limit": 1550
        }))
        .unwrap();
        assert_eq!(
            get_pgaudit_body.as_sql_clauses(false).unwrap(),
            "LIMIT 1000"
        );
        assert_eq!(get_pgaudit_body.as_sql_clauses(true).unwrap(), "");
    }
}
