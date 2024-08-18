// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tasks_execution::{insert_tasks_execution, update_task_execution_status};
use crate::services::database::PgConfig;
use crate::services::protocol_manager::get_protocol_manager;
use crate::services::protocol_manager::{create_named_param, get_board_client, get_immudb_client};
use crate::types::resources::{Aggregate, DataList, OrderDirection, TotalAggregate};
use anyhow::{anyhow, Context, Result};
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde::{Deserialize, Serialize};

pub async fn post(
    tenant_id: &str,
    election_event_id: &str,
    task_type: &str,
) -> Result<(), anyhow::Error> {
    let task = insert_tasks_execution(
        tenant_id,
        election_event_id,
        "Export Election Event", // TODO: delete
        task_type,
        TasksExecutionStatus::IN_PROGRESS,
        None,      // Optional annotations
        None,      // Optional labels
        None,      // Optional logs
        tenant_id, // TODO: Replace with the actual user ID or obtain it dynamically
    )
    .await
    .context("Failed to insert task execution record")?;

    Ok(())
}

pub async fn update(task_id: &str, status: TasksExecutionStatus) -> Result<(), anyhow::Error> {
    update_task_execution_status(task_id, status)
        .await
        .context("Failed to update task execution record")?;
    Ok(())
}

//     // Enumeration for the valid fields in the immudb table
//     #[derive(Debug, Deserialize, Hash, PartialEq, Eq, EnumString, Display)]
//     #[serde(rename_all = "snake_case")]
//     #[strum(serialize_all = "snake_case")]
//     pub enum OrderField {
//         Id,
//         Created,
//         StatementTimestamp,
//         StatementKind,
//         Message,
//     }

//     #[derive(Deserialize, Debug)]
//     pub struct GetElectoralLogBody {
//         pub tenant_id: String,
//         pub election_event_id: String,
//         pub limit: Option<i64>,
//         pub offset: Option<i64>,
//         pub filter: Option<HashMap<OrderField, String>>,
//         pub order_by: Option<HashMap<OrderField, OrderDirection>>,
//     }

//     impl GetElectoralLogBody {
//         // Returns the SQL clauses related to the request along with the parameters
//         #[instrument(ret)]
//         fn as_sql(&self, to_count: bool) -> Result<(String, Vec<NamedParam>)> {
//             let mut clauses = Vec::new();
//             let mut params = Vec::new();

//             // Handle filters
//             if let Some(filters_map) = &self.filter {
//                 let mut where_clauses = Vec::new();

//                 for (field, value) in filters_map {
//                     let param_name = format!("param_{field}");
//                     match field {
//                         OrderField::Id => {
//                             let int_value: i64 = value.parse()?;
//                             where_clauses.push(format!("id = @{}", param_name));
//                             params.push(create_named_param(param_name, Value::N(int_value)));
//                         }
//                         OrderField::StatementTimestamp | OrderField::Created | OrderField::Message => {}
//                         _ => {
//                             where_clauses.push(format!("{field} LIKE @{}", param_name));
//                             params.push(create_named_param(param_name, Value::S(value.to_string())));
//                         }
//                     }
//                 }

//                 if !where_clauses.is_empty() {
//                     clauses.push(format!("WHERE {}", where_clauses.join(" AND ")));
//                 }
//             }

//             // Handle order_by
//             if !to_count && self.order_by.is_some() {
//                 let order_by_clauses: Vec<String> = self
//                     .order_by
//                     .as_ref()
//                     .unwrap()
//                     .iter()
//                     .map(|(field, direction)| format!("{field} {direction}"))
//                     .collect();
//                 clauses.push(format!("ORDER BY {}", order_by_clauses.join(", ")));
//             }

//             // Handle limit
//             if !to_count {
//                 let limit_param_name = String::from("limit");
//                 let limit_value = self
//                     .limit
//                     .unwrap_or(PgConfig::from_env()?.default_sql_limit.into());
//                 let limit = std::cmp::min(limit_value, PgConfig::from_env()?.low_sql_limit.into());
//                 clauses.push(format!("LIMIT @{limit_param_name}"));
//                 params.push(create_named_param(limit_param_name, Value::N(limit)));
//             }

//             // Handle offset
//             if !to_count && self.offset.is_some() {
//                 let offset_param_name = String::from("offset");
//                 let offset = std::cmp::max(self.offset.unwrap(), 0);
//                 clauses.push(format!("OFFSET @{}", offset_param_name));
//                 params.push(create_named_param(offset_param_name, Value::N(offset)));
//             }

//             Ok((clauses.join(" "), params))
//         }
//     }

//     #[derive(Serialize, Deserialize, Debug)]
//     pub struct ElectoralLogRow {
//         id: i64,
//         created: i64,
//         statement_timestamp: i64,
//         statement_kind: String,
//         message: String,
//     }

//     impl TryFrom<&Row> for ElectoralLogRow {
//         type Error = anyhow::Error;

//         fn try_from(row: &Row) -> Result<Self, Self::Error> {
//             let mut id = 0;
//             let mut created: i64 = 0;
//             let mut sender_pk = String::from("");
//             let mut statement_timestamp: i64 = 0;
//             let mut statement_kind = String::from("");
//             let mut message = vec![];

//             for (column, value) in row.columns.iter().zip(row.values.iter()) {
//                 match column.as_str() {
//                     c if c.ends_with(".id)") => {
//                         assign_value!(Value::N, value, id)
//                     }
//                     c if c.ends_with(".created)") => {
//                         assign_value!(Value::Ts, value, created)
//                     }
//                     c if c.ends_with(".sender_pk)") => {
//                         assign_value!(Value::S, value, sender_pk)
//                     }
//                     c if c.ends_with(".statement_timestamp)") => {
//                         assign_value!(Value::Ts, value, statement_timestamp)
//                     }
//                     c if c.ends_with(".statement_kind)") => {
//                         assign_value!(Value::S, value, statement_kind)
//                     }
//                     c if c.ends_with(".message)") => {
//                         assign_value!(Value::Bs, value, message)
//                     }
//                     _ => return Err(anyhow!("invalid column found '{}'", column.as_str())),
//                 }
//             }
//             Ok(ElectoralLogRow {
//                 id,
//                 created,
//                 statement_timestamp,
//                 statement_kind,
//                 message: serde_json::to_string_pretty(
//                     &Message::strand_deserialize(&message)
//                         .with_context(|| "Error deserializing message")?,
//                 )
//                 .with_context(|| "Error serializing message to json")?,
//             })
//         }
//     }

//     #[instrument(err)]
//     pub async fn list_electoral_log(input: GetElectoralLogBody) -> Result<DataList<ElectoralLogRow>> {
//         let mut client = get_immudb_client().await?;
//         let board_name = get_event_board(input.tenant_id.as_str(), input.election_event_id.as_str());

//         event!(Level::INFO, "database name = {board_name}");

//         client.open_session(&board_name).await?;
//         let (clauses, params) = input.as_sql(false)?;
//         let (clauses_to_count, count_params) = input.as_sql(true)?;

//         let sql = format!(
//             r#"
//             SELECT
//                 id,
//                 created,
//                 sender_pk,
//                 statement_timestamp,
//                 statement_kind,
//                 message
//             FROM electoral_log_messages
//             {clauses}
//             "#,
//         );
//         let sql_query_response = client.sql_query(&sql, params).await?;
//         let items = sql_query_response
//             .get_ref()
//             .rows
//             .iter()
//             .map(ElectoralLogRow::try_from)
//             .collect::<Result<Vec<ElectoralLogRow>>>()?;

//         let sql = format!(
//             r#"
//             SELECT
//                 COUNT(*)
//             FROM electoral_log_messages
//             {clauses_to_count}
//             "#,
//         );
//         let sql_query_response = client.sql_query(&sql, count_params).await?;
//         let mut rows_iter = sql_query_response
//             .get_ref()
//             .rows
//             .iter()
//             .map(Aggregate::try_from);

//         let aggregate = rows_iter
//             // get the first item
//             .next()
//             // unwrap the Result and Option
//             .ok_or(anyhow!("No aggregate found"))??;

//         client.close_session().await?;
//         Ok(DataList {
//             items: items,
//             total: TotalAggregate {
//                 aggregate: aggregate,
//             },
//         })
// }
