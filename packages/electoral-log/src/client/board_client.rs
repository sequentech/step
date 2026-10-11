// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Application audit API over the supplied wbraid board storage.
pub use super::audit_query::*;
pub use super::audit_types::*;
pub use super::wbraid_store::{BoardClient, WbraidAuditStore};
use anyhow::{ensure, Result};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fmt::Display};

impl BoardClient {
    pub async fn insert_electoral_log_messages(
        &mut self,
        board: &str,
        rows: &[ElectoralLogMessage],
    ) -> Result<()> {
        // Per-record identities stay stable across retries and CSV batch sizes.
        for batch in rows.chunks(1000) {
            let hashes: Vec<String> = batch
                .iter()
                .map(|row| {
                    let mut canonical = row.clone();
                    canonical.id = 0;
                    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
                })
                .collect::<Result<_>>()?;
            let deliveries: Vec<_> = hashes
                .iter()
                .zip(batch)
                .map(|(hash, row)| (hash.as_str(), hash.as_str(), std::slice::from_ref(row)))
                .collect();
            self.append_deliveries(board, &deliveries).await?;
        }
        Ok(())
    }

    pub async fn get_electoral_log_messages(
        &mut self,
        board: &str,
    ) -> Result<Vec<ElectoralLogMessage>> {
        self.query_messages(board, &AuditQuery::default()).await
    }

    pub async fn get_electoral_log_messages_at_offset(
        &mut self,
        board: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ElectoralLogMessage>> {
        self.query_messages(
            board,
            &AuditQuery {
                limit: Some(limit),
                offset: Some(offset),
                ..Default::default()
            },
        )
        .await
    }

    pub async fn count_electoral_log_messages(
        &mut self,
        board: &str,
        filters: Option<WhereClauseBTreeMap>,
    ) -> Result<i64> {
        self.count_query(
            board,
            Some(&filters.map(AuditQuery::from).unwrap_or_default()),
        )
        .await
    }

    pub async fn get_electoral_log_messages_filtered<K: Display, V: Display>(
        &mut self,
        board: &str,
        filters: Option<WhereClauseBTreeMap>,
        min_ts: Option<i64>,
        max_ts: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
        order_by: Option<HashMap<K, V>>,
    ) -> Result<Vec<ElectoralLogMessage>> {
        let mut query = filters.map(AuditQuery::from).unwrap_or_default();
        for (value, operator) in [
            (min_ts, SqlCompOperators::GreaterThanOrEqual),
            (max_ts, SqlCompOperators::LessThanOrEqual),
        ] {
            if let Some(value) = value {
                query.predicates.push(AuditPredicate {
                    column: AuditColumn::Created,
                    operator,
                    value: AuditValue::Integer(value),
                });
            }
        }
        let mut order: Vec<_> = order_by
            .unwrap_or_default()
            .into_iter()
            .map(|(column, direction)| (column.to_string(), direction.to_string()))
            .collect();
        order.sort();
        for (column, direction) in order {
            ensure!(
                matches!(direction.to_lowercase().as_str(), "asc" | "desc"),
                "unsupported audit sort direction"
            );
            query
                .order
                .push((column.parse()?, direction.eq_ignore_ascii_case("desc")));
        }
        query.limit = limit;
        query.offset = offset;
        self.query_messages(board, &query).await
    }
}
