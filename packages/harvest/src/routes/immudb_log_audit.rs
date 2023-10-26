// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use immudb_rs::{
    sql_value::Value as SqlValue, Client, NamedParam, Row, TxMode,
};
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use tracing::instrument;

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
#[derive(Debug, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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

// Enumeration for the valid order directions
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Deserialize, Debug)]
pub struct GetPgauditBody {
    tenant_id: String,
    election_event_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
    order_by: Option<HashMap<OrderField, OrderDirection>>,
}

impl GetPgauditBody {
    // Returns the SQL clauses related to the request
    fn as_sql_clauses(&self) -> String {
        let mut clauses = Vec::new();

        // Handle order_by
        if let Some(order_by_map) = &self.order_by {
            let order_clauses: Vec<String> = order_by_map
                .iter()
                .map(|(field, direction)| {
                    format!("{:?} {:?}", field, direction)
                })
                .collect();
            if !order_clauses.is_empty() {
                clauses.push(format!("ORDER BY {}", order_clauses.join(", ")));
            }
        }

        // Handle limit
        let limit = self.limit.unwrap_or(10);
        clauses.push(format!("LIMIT {}", std::cmp::min(limit, 500)));

        // Handle offset
        if let Some(offset) = self.offset {
            clauses.push(format!("OFFSET {}", std::cmp::max(offset, 0)));
        }

        clauses.join(" ")
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
        let mut audit_type = String::from("");
        let mut class = String::from("");
        let mut command = String::from("");
        let mut dbname = String::from("");
        let mut server_timestamp: i64 = 0;
        let mut session_id = String::from("");
        let mut statement = String::from("");
        let mut user = String::from("");
        let mut audit_type = String::from("");

        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            // FIXME for some reason columns names appear with parentheses
            match column.as_str() {
                "(pgaudit.id)" => assign_value!(SqlValue::N, value, id),
                "(pgaudit.audit_type)" => {
                    assign_value!(SqlValue::S, value, audit_type)
                }
                "(pgaudit.class)" => assign_value!(SqlValue::S, value, class),
                "(pgaudit.command)" => {
                    assign_value!(SqlValue::S, value, command)
                }
                "(pgaudit.dbname)" => assign_value!(SqlValue::S, value, dbname),
                "(pgaudit.server_timestamp)" => {
                    assign_value!(SqlValue::Ts, value, server_timestamp)
                }
                "(pgaudit.session_id)" => {
                    assign_value!(SqlValue::S, value, session_id)
                }
                "(pgaudit.statement)" => {
                    assign_value!(SqlValue::S, value, statement)
                }
                "(pgaudit.user)" => assign_value!(SqlValue::S, value, user),
                "(pgaudit.audit_type)" => {
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Aggregate {
    count: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TotalAggregate {
    aggregate: Aggregate,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct DataList<T> {
    items: Vec<T>,
    total: TotalAggregate,
}

#[instrument]
#[post("/immudb/pgaudit-list", format = "json", data = "<body>")]
pub async fn list_pgaudit(
    body: Json<GetPgauditBody>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<DataList<PgAuditRow>>, Debug<anyhow::Error>> {
    let server_url = env::var("IMMUDB_SERVER_URL")
        .context("IMMUDB_SERVER_URL env var not set")?;
    let username = env::var("IMMUDB_USERNAME")
        .context("IMMUDB_USERNAME env var not set")?;
    let password = env::var("IMMUDB_PASSWORD")
        .context("IMMUDB_PASSWORD env var not set")?;
    let input = body.into_inner();

    let mut client = Client::new(&server_url, &username, &password).await?;
    client.login(&username, &password).await?;

    client.open_session(&input.election_event_id).await?;
    let limit: i64 = input.limit.unwrap_or(10);
    let offset: i64 = input.offset.unwrap_or(0);
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
        FROM pgaudit
        {}
        "#,
        input.as_sql_clauses()
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
        FROM pgaudit
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
            get_pgaudit_body.as_sql_clauses(),
            "ORDER BY Id Asc LIMIT 10"
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
            get_pgaudit_body.as_sql_clauses(),
            "ORDER BY Id Asc LIMIT 15 OFFSET 5"
        );

        let get_pgaudit_body: GetPgauditBody = serde_json::from_value(json!({
            "tenant_id": "some_tenant",
            "election_event_id": "some_event",
            "limit": 550
        }))
        .unwrap();
        assert_eq!(get_pgaudit_body.as_sql_clauses(), "LIMIT 500");
    }
}
