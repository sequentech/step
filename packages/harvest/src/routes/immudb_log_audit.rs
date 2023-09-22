// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use immudb_rs::{
    sql_value::Value as SqlValue, Client, NamedParam, Row, TxMode,
};
use rocket::response::Debug;
use rocket::serde::json::{Json, Value};
use rocket::serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

use crate::connection;

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

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct GetPgauditBody {
    tenant_id: String,
    election_event_id: String,
    limit: Option<i64>,
    after_id: Option<i64>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
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

#[post("/immudb/pgaudit-list", format = "json", data = "<body>")]
pub async fn list_pgaudit(
    body: Json<GetPgauditBody>,
    auth_headers: connection::AuthHeaders,
) -> Result<Json<Vec<PgAuditRow>>, Debug<anyhow::Error>> {
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
    let after_id: i64 = input.after_id.unwrap_or(0);
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
    WHERE id > {}
    LIMIT {}
    ORDER BY id ASC
    "#,
        after_id,
        limit,
    );
    let sql_query_response = client.sql_query(&sql, vec![]).await?;
    let rows = sql_query_response
        .get_ref()
        .rows
        .iter()
        .map(PgAuditRow::try_from)
        .collect::<Result<Vec<PgAuditRow>>>()?;
    client.close_session().await?;
    Ok(Json(rows))
}
