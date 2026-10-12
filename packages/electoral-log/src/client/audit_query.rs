// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Bound queries over electoral-log metadata stored in immudb.
//! Column names and ordering are closed enums; caller values are named parameters.

use super::board_client::{ElectoralLogVarCharColumn, SqlCompOperators};
use anyhow::{ensure, Result};
use immudb_rs::{sql_value::Value, NamedParam, SqlValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditColumn {
    Id,
    Created,
    StatementTimestamp,
    Text(ElectoralLogVarCharColumn),
}

impl AuditColumn {
    pub fn sql(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::Created => "created",
            Self::StatementTimestamp => "statement_timestamp",
            Self::Text(column) => match column {
                ElectoralLogVarCharColumn::StatementKind => "statement_kind",
                ElectoralLogVarCharColumn::UserId => "user_id",
                ElectoralLogVarCharColumn::BallotId => "ballot_id",
                ElectoralLogVarCharColumn::Username => "username",
                ElectoralLogVarCharColumn::SenderPk => "sender_pk",
                ElectoralLogVarCharColumn::ElectionId => "election_id",
                ElectoralLogVarCharColumn::AreaId => "area_id",
                ElectoralLogVarCharColumn::Version => "version",
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum AuditValue {
    Text(String),
    Integer(i64),
}

#[derive(Debug, Clone)]
pub struct AuditPredicate {
    pub column: AuditColumn,
    pub operator: SqlCompOperators,
    pub value: AuditValue,
}

#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    pub predicates: Vec<AuditPredicate>,
    /// Preserve event-wide entries alongside the selected election or areas.
    pub election_id: Option<String>,
    pub area_ids: Vec<String>,
    /// Server-derived election permissions; an empty list keeps event-wide rows only.
    pub permitted_election_ids: Option<Vec<String>>,
    pub only_with_user: bool,
    pub scoped: bool,
    /// `true` for descending, `false` for ascending.
    pub order: Vec<(AuditColumn, bool)>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl AuditQuery {
    pub(crate) fn build(&self, count: bool) -> Result<(String, Vec<NamedParam>)> {
        ensure!(
            self.limit.is_none_or(|v| v >= 0) && self.offset.is_none_or(|v| v >= 0),
            "negative audit pagination"
        );
        let mut query = String::from(if count { "SELECT COUNT(*)" } else { "SELECT *" });
        query.push_str(" FROM electoral_log_messages WHERE id >= 0");
        let mut params = Vec::new();
        for predicate in &self.predicates {
            ensure!(
                !matches!(
                    predicate.operator,
                    SqlCompOperators::In | SqlCompOperators::NotIn
                ),
                "audit predicate requires a scalar comparison"
            );
            let value = match (&predicate.column, &predicate.value) {
                (AuditColumn::Text(_), AuditValue::Text(value)) => Value::S(value.clone()),
                (
                    AuditColumn::Id | AuditColumn::Created | AuditColumn::StatementTimestamp,
                    AuditValue::Integer(value),
                ) => {
                    ensure!(
                        !matches!(predicate.operator, SqlCompOperators::Like),
                        "LIKE requires a text column"
                    );
                    if predicate.column == AuditColumn::Id {
                        Value::N(*value)
                    } else {
                        Value::Ts(*value)
                    }
                }
                _ => anyhow::bail!("audit predicate type does not match its column"),
            };
            let parameter = bind(&mut params, value);
            query.push_str(&format!(
                " AND {} {} {parameter}",
                predicate.column.sql(),
                predicate.operator
            ));
        }
        if self.scoped || self.election_id.is_some() || !self.area_ids.is_empty() {
            query.push_str(" AND (");
            if let Some(election) = &self.election_id {
                let parameter = bind(&mut params, Value::S(election.clone()));
                query.push_str(&format!("election_id LIKE {parameter} OR "));
            }
            for area in &self.area_ids {
                let parameter = bind(&mut params, Value::S(area.clone()));
                query.push_str(&format!("area_id={parameter} OR "));
            }
            query.push_str(
                "((election_id IS NULL OR election_id='') AND (area_id IS NULL OR area_id='')))",
            );
        }
        if let Some(permitted) = &self.permitted_election_ids {
            query.push_str(" AND (election_id IS NULL OR election_id=''");
            for election in permitted {
                let parameter = bind(&mut params, Value::S(election.clone()));
                query.push_str(&format!(" OR election_id={parameter}"));
            }
            query.push(')');
        }
        if self.only_with_user {
            query.push_str(" AND user_id IS NOT NULL AND user_id<>''");
        }
        if !count {
            query.push_str(" ORDER BY ");
            let mut order = self.order.clone();
            if !order.iter().any(|(column, _)| *column == AuditColumn::Id) {
                order.push((AuditColumn::Id, false));
            }
            for (index, (column, descending)) in order.into_iter().enumerate() {
                if index > 0 {
                    query.push_str(", ");
                }
                query.push_str(column.sql());
                query.push_str(if descending { " DESC" } else { " ASC" });
            }
            if let Some(limit) = self.limit {
                let parameter = bind(&mut params, Value::N(limit));
                query.push_str(&format!(" LIMIT {parameter}"));
            }
            if let Some(offset) = self.offset {
                let parameter = bind(&mut params, Value::N(offset));
                query.push_str(&format!(" OFFSET {parameter}"));
            }
        }
        Ok((query, params))
    }
}

fn bind(params: &mut Vec<NamedParam>, value: Value) -> String {
    let name = format!("audit_{}", params.len());
    params.push(NamedParam {
        name: name.clone(),
        value: Some(SqlValue { value: Some(value) }),
    });
    format!("@{name}")
}

impl std::str::FromStr for AuditColumn {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self> {
        use ElectoralLogVarCharColumn as Text;
        Ok(match value {
            "id" => Self::Id,
            "created" => Self::Created,
            "statement_timestamp" => Self::StatementTimestamp,
            "statement_kind" => Self::Text(Text::StatementKind),
            "user_id" => Self::Text(Text::UserId),
            "username" => Self::Text(Text::Username),
            "sender_pk" => Self::Text(Text::SenderPk),
            "version" => Self::Text(Text::Version),
            "election_id" => Self::Text(Text::ElectionId),
            "area_id" => Self::Text(Text::AreaId),
            "ballot_id" => Self::Text(Text::BallotId),
            _ => anyhow::bail!("unsupported audit column: {value}"),
        })
    }
}

impl From<super::board_client::WhereClauseBTreeMap> for AuditQuery {
    fn from(filters: super::board_client::WhereClauseBTreeMap) -> Self {
        Self {
            predicates: filters
                .into_iter()
                .map(|(column, (operator, value))| AuditPredicate {
                    column: AuditColumn::Text(column),
                    operator,
                    value: AuditValue::Text(value),
                })
                .collect(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod permission_tests {
    use super::*;

    #[test]
    fn permission_scope_is_bound_and_keeps_event_wide_rows() {
        let query = AuditQuery {
            election_id: Some("north".into()),
            permitted_election_ids: Some(vec!["north".into(), "south' OR TRUE".into()]),
            ..Default::default()
        };
        for count in [true, false] {
            let (sql, params) = query.build(count).unwrap();
            assert!(
                sql.contains(" AND (election_id IS NULL OR election_id='' OR election_id=@audit_1 OR election_id=@audit_2)")
            );
            assert!(!sql.contains("south"));
            assert_eq!(
                params[2].value.as_ref().unwrap().value,
                Some(Value::S("south' OR TRUE".into()))
            );
        }
    }

    #[test]
    fn empty_permission_scope_allows_only_event_wide_rows() {
        let query = AuditQuery {
            permitted_election_ids: Some(vec![]),
            ..Default::default()
        };
        let (sql, params) = query.build(true).unwrap();
        assert!(sql.contains(" AND (election_id IS NULL OR election_id='')"));
        assert!(params.is_empty());
    }
}
