// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::resources::{
    Aggregate, DataList, OrderDirection, TotalAggregate,
};
use anyhow::{anyhow, Context, Result};
use electoral_log::assign_value;
use immudb_rs::{sql_value::Value, Client, NamedParam, Row, SqlValue};
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use strum_macros::{Display, EnumString};
use tracing::instrument;
use windmill::services::database::PgConfig;

#[instrument(err)]
pub async fn get_immudb_client() -> Result<Client> {
    let username =
        env::var("IMMUDB_USER").context("IMMUDB_USER must be set")?;
    let password =
        env::var("IMMUDB_PASSWORD").context("IMMUDB_PASSWORD must be set")?;
    let server_url = env::var("IMMUDB_SERVER_URL")
        .context("IMMUDB_SERVER_URL must be set")?;

    let mut client = Client::new(&server_url, &username, &password).await?;
    client.login().await?;

    Ok(client)
}

// Helper function to create a NamedParam
pub fn create_named_param(name: String, value: Value) -> NamedParam {
    NamedParam {
        name,
        value: Some(SqlValue { value: Some(value) }),
    }
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
    // Returns the SQL clauses related to the request along with the parameters
    #[instrument(ret)]
    fn as_sql(&self, to_count: bool) -> Result<(String, Vec<NamedParam>)> {
        let mut clauses = Vec::new();
        let mut params = Vec::new();

        // Handle filters
        if let Some(filters_map) = &self.filter {
            let mut where_clauses = Vec::new();

            for (field, value) in filters_map {
                let param_name = format!("param_{field}");
                match field {
                    OrderField::Id => {
                        let int_value: i64 = value.parse()?;
                        where_clauses.push(format!("id = @{}", param_name));
                        params.push(create_named_param(
                            param_name,
                            Value::N(int_value),
                        ));
                    }
                    OrderField::ServerTimestamp => {} // Not supported
                    _ => {
                        where_clauses
                            .push(format!("{field} LIKE @{}", param_name));
                        params.push(create_named_param(
                            param_name,
                            Value::S(value.to_string()),
                        ));
                    }
                }
            }

            if !where_clauses.is_empty() {
                clauses.push(format!("WHERE {}", where_clauses.join(" AND ")));
            }
        }

        // Handle order_by
        if !to_count && self.order_by.is_some() {
            let order_by_clauses: Vec<String> = self
                .order_by
                .as_ref()
                .unwrap()
                .iter()
                .map(|(field, direction)| format!("{field} {direction}"))
                .collect();
            clauses.push(format!("ORDER BY {}", order_by_clauses.join(", ")));
        }

        // Handle limit
        if !to_count {
            let limit_param_name = String::from("limit");
            let limit_value = self
                .limit
                .unwrap_or(PgConfig::from_env()?.default_sql_limit.into());
            let limit = std::cmp::min(
                limit_value,
                PgConfig::from_env()?.low_sql_limit.into(),
            );
            clauses.push(format!("LIMIT @{limit_param_name}"));
            params.push(create_named_param(limit_param_name, Value::N(limit)));
        }

        // Handle offset
        if !to_count && self.offset.is_some() {
            let offset_param_name = String::from("offset");
            let offset = std::cmp::max(self.offset.unwrap(), 0);
            clauses.push(format!("OFFSET @{}", offset_param_name));
            params
                .push(create_named_param(offset_param_name, Value::N(offset)));
        }

        Ok((clauses.join(" "), params))
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
                c if c.ends_with(".id)") => {
                    assign_value!(Value::N, value, id)
                }
                c if c.ends_with(".audit_type)") => {
                    assign_value!(Value::S, value, audit_type)
                }
                c if c.ends_with(".class)") => {
                    assign_value!(Value::S, value, class)
                }
                c if c.ends_with(".command)") => {
                    assign_value!(Value::S, value, command)
                }
                c if c.ends_with(".dbname)") => {
                    assign_value!(Value::S, value, dbname)
                }
                c if c.ends_with(".server_timestamp)") => {
                    assign_value!(Value::Ts, value, server_timestamp)
                }
                c if c.ends_with(".session_id)") => {
                    assign_value!(Value::S, value, session_id)
                }
                c if c.ends_with(".statement)") => {
                    assign_value!(Value::S, value, statement)
                }
                c if c.ends_with(".user)") => {
                    assign_value!(Value::S, value, user)
                }
                c if c.ends_with(".audit_type)") => {
                    assign_value!(Value::S, value, audit_type)
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

async fn audit_list_service(
    input: GetPgauditBody,
) -> Result<Json<DataList<PgAuditRow>>, Debug<anyhow::Error>> {
    let mut client = get_immudb_client().await?;

    client.open_session(&input.election_event_id).await?;
    let (clauses, params) = input.as_sql(false)?;
    let (clauses_to_count, count_params) = input.as_sql(true)?;

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
    let sql_query_response = client.sql_query(&sql, params).await?;
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
    let sql_query_response = client.sql_query(&sql, count_params).await?;
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

#[instrument]
#[post("/immudb/pgaudit-list", format = "json", data = "<body>")]
pub async fn list_pgaudit(
    body: Json<GetPgauditBody>,
    claims: JwtClaims,
) -> Result<Json<DataList<PgAuditRow>>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::LOGS_READ],
    )?;
    let result = audit_list_service(input)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::serialization::deserialize_with_path::deserialize_value;
    use serde_json::json;

    #[test]
    fn test_as_sql() {
        // Test with order_by
        let get_pgaudit_body: GetPgauditBody = deserialize_value(json!({
            "tenant_id": "some_tenant",
            "election_event_id": "some_event",
            "order_by": {"id": "asc"}
        }))
        .unwrap();
        let (sql, params) = get_pgaudit_body.as_sql(false).unwrap();
        assert!(sql.contains("ORDER BY id asc LIMIT @limit"));
        assert_eq!(params[0].name, "limit");
        assert_eq!(params[0].value.as_ref().unwrap().value, Some(Value::N(20)));

        // Test with limit, offset, and order_by
        let get_pgaudit_body: GetPgauditBody = deserialize_value(json!({
            "tenant_id": "some_tenant",
            "election_event_id": "some_event",
            "limit": 15,
            "offset": 5,
            "order_by": {"id": "asc"}
        }))
        .unwrap();
        let (sql, params) = get_pgaudit_body.as_sql(false).unwrap();
        assert!(sql.contains("ORDER BY id asc LIMIT @limit OFFSET @offset"));
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "limit");
        assert_eq!(params[0].value.as_ref().unwrap().value, Some(Value::N(15)));
        assert_eq!(params[1].name, "offset");
        assert_eq!(params[1].value.as_ref().unwrap().value, Some(Value::N(5)));

        // Test as_sql(true) without any parameters
        let (sql, params) = get_pgaudit_body.as_sql(true).unwrap();
        assert!(sql.is_empty());
        assert!(params.is_empty());

        // Test with high limit value
        let get_pgaudit_body: GetPgauditBody = deserialize_value(json!({
            "tenant_id": "some_tenant",
            "election_event_id": "some_event",
            "limit": 1550
        }))
        .unwrap();
        let (sql, params) = get_pgaudit_body.as_sql(false).unwrap();
        assert!(sql.contains("LIMIT @limit"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "limit");
        assert_eq!(
            params[0].value.as_ref().unwrap().value,
            Some(Value::N(1000))
        );
        // Check limit value based on PgConfig settings

        // Test as_sql(true) without any parameters
        let (sql, params) = get_pgaudit_body.as_sql(true).unwrap();
        assert!(sql.is_empty());
        assert!(params.is_empty());
    }
}
