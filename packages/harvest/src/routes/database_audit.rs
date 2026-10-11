// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::resources::{
    Aggregate, DataList, OrderDirection, TotalAggregate,
};
use anyhow::Result;
use electoral_log::{
    client::database_audit::*, AuditColumn, AuditPredicate, AuditQuery,
    AuditValue, SqlCompOperators,
};
use rocket::{http::Status, serde::json::Json};
use sequent_core::{services::jwt::JwtClaims, types::permissions::Permissions};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::instrument;
use windmill::services::{
    database::PgConfig, electoral_log_board::get_board_client,
};

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
impl OrderField {
    fn column(&self) -> AuditColumn {
        let text = match self {
            Self::Id => return AuditColumn::Id,
            Self::ServerTimestamp => return AuditColumn::Created,
            Self::AuditType => DatabaseAuditColumn::AuditType,
            Self::Class => DatabaseAuditColumn::Class,
            Self::Command => DatabaseAuditColumn::Command,
            Self::Dbname => DatabaseAuditColumn::Dbname,
            Self::SessionId => DatabaseAuditColumn::SessionId,
            Self::Statement => DatabaseAuditColumn::Statement,
            Self::User => DatabaseAuditColumn::User,
        };
        AuditColumn::DatabaseText(text)
    }
}
#[derive(Debug, Deserialize)]
pub struct GetPgauditBody {
    tenant_id: String,
    election_event_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
    filter: Option<HashMap<OrderField, String>>,
    order_by: Option<HashMap<OrderField, OrderDirection>>,
    #[serde(default)]
    audit_table: DatabaseAuditTable,
}
impl GetPgauditBody {
    fn query(&self, config: &PgConfig) -> Result<AuditQuery> {
        let mut query = AuditQuery {
            limit: Some(
                self.limit
                    .unwrap_or(config.default_sql_limit.into())
                    .clamp(0, config.low_sql_limit.into()),
            ),
            offset: Some(self.offset.unwrap_or(0).max(0)),
            ..Default::default()
        };
        for (field, value) in self.filter.as_ref().into_iter().flatten() {
            let column = field.column();
            if matches!(field, OrderField::ServerTimestamp) {
                continue;
            }
            let (operator, value) = if column == AuditColumn::Id {
                (SqlCompOperators::Equal, AuditValue::Integer(value.parse()?))
            } else {
                (SqlCompOperators::Like, AuditValue::Text(value.clone()))
            };
            query.predicates.push(AuditPredicate {
                column,
                operator,
                value,
            });
        }
        let mut order: Vec<_> = self
            .order_by
            .as_ref()
            .into_iter()
            .flatten()
            .map(|(field, direction)| {
                (field.column(), matches!(direction, OrderDirection::Desc))
            })
            .collect();
        order.sort_by_key(|(column, _)| column.sql());
        query.order = order;
        Ok(query)
    }
}
async fn audit_list_service(
    input: GetPgauditBody,
) -> Result<Json<DataList<DatabaseAuditRow>>> {
    let mut client = get_board_client().await?;
    let board = database_audit_board(
        &input.tenant_id,
        &input.election_event_id,
        input.audit_table,
    )?;
    let query = input.query(&PgConfig::from_env()?)?;
    let mut items = Vec::new();
    for record in client.query_messages(&board, &query).await? {
        let mut row: DatabaseAuditRow =
            serde_json::from_slice(&record.message)?;
        row.id = record.id;
        items.push(row);
    }
    let count = client.count_query(&board, Some(&query)).await?;
    Ok(Json(DataList {
        items,
        total: TotalAggregate {
            aggregate: Aggregate { count },
        },
    }))
}

#[instrument]
#[post("/audit/pgaudit-list", format = "json", data = "<body>")]
pub async fn list_pgaudit(
    body: Json<GetPgauditBody>,
    claims: JwtClaims,
) -> Result<Json<DataList<DatabaseAuditRow>>, (Status, String)> {
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
    #[test]
    fn native_audit_filters_are_typed_and_tables_reject_injection() {
        let request:GetPgauditBody = serde_json::from_value(serde_json::json!({"tenant_id":"t","election_event_id":"e","filter":{"statement":"%'; DROP TABLE records; --"},"order_by":{"id":"desc"},"limit":1550,"offset":-2})).unwrap();
        let query = request.query(&PgConfig::default()).unwrap();
        assert_eq!(query.limit, Some(1000));
        assert_eq!(query.offset, Some(0));
        assert!(matches!(query.predicates[0].value, AuditValue::Text(_)));
        assert_eq!(query.order, vec![(AuditColumn::Id, true)]);
        assert!(serde_json::from_value::<GetPgauditBody>(serde_json::json!({"tenant_id":"t","election_event_id":"e","audit_table":"pgaudit_hasura; SELECT 1"})).is_err());
    }
}
