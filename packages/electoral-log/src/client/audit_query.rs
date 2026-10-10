// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Bound queries over recoverable audit metadata. Column names and ordering
//! are closed enums; all caller values are PostgreSQL parameters.

use super::audit_types::{ElectoralLogVarCharColumn, SqlCompOperators};
use anyhow::{ensure, Result};
use sqlx::{Postgres, QueryBuilder};

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
    pub only_with_user: bool,
    /// `true` for descending, `false` for ascending.
    pub order: Vec<(AuditColumn, bool)>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl AuditQuery {
    pub(crate) fn build<'a>(
        &'a self,
        board: &'a str,
        count: bool,
    ) -> Result<QueryBuilder<'a, Postgres>> {
        ensure!(
            self.limit.is_none_or(|v| v >= 0) && self.offset.is_none_or(|v| v >= 0),
            "negative audit pagination"
        );
        let mut query = QueryBuilder::new(if count { "SELECT COUNT(*)" } else { "SELECT *" });
        query
            .push(" FROM sequent_audit.records WHERE board_name=")
            .push_bind(board);
        for predicate in &self.predicates {
            query
                .push(" AND ")
                .push(predicate.column.sql())
                .push(" ")
                .push(predicate.operator.to_string())
                .push(" ");
            match (&predicate.column, &predicate.value) {
                (AuditColumn::Text(_), AuditValue::Text(value)) => {
                    query.push_bind(value);
                }
                (
                    AuditColumn::Id | AuditColumn::Created | AuditColumn::StatementTimestamp,
                    AuditValue::Integer(value),
                ) => {
                    ensure!(
                        !matches!(predicate.operator, SqlCompOperators::Like),
                        "LIKE requires a text column"
                    );
                    query.push_bind(*value);
                }
                _ => anyhow::bail!("audit predicate type does not match its column"),
            }
        }
        if self.election_id.is_some() || !self.area_ids.is_empty() {
            query.push(" AND (");
            if let Some(election) = &self.election_id {
                query.push("election_id=").push_bind(election).push(" OR ");
            }
            if !self.area_ids.is_empty() {
                query
                    .push("area_id=ANY(")
                    .push_bind(&self.area_ids)
                    .push(") OR ");
            }
            query.push(
                "((election_id IS NULL OR election_id='') AND (area_id IS NULL OR area_id='')))",
            );
        }
        if self.only_with_user {
            query.push(" AND user_id IS NOT NULL AND user_id<>''");
        }
        if !count {
            query.push(" ORDER BY ");
            for (column, descending) in &self.order {
                query
                    .push(column.sql())
                    .push(if *descending { " DESC, " } else { " ASC, " });
            }
            query.push("id ASC");
            if let Some(limit) = self.limit {
                query.push(" LIMIT ").push_bind(limit);
            }
            if let Some(offset) = self.offset {
                query.push(" OFFSET ").push_bind(offset);
            }
        }
        Ok(query)
    }
}
