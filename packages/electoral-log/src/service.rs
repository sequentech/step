// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::*;
use crate::ports::ElectoralLogStore;
use anyhow::Result;
use std::{collections::HashMap, fmt::Display, sync::Arc};

#[derive(Clone)]
pub struct BoardClient {
    store: Arc<dyn ElectoralLogStore>,
}

impl BoardClient {
    pub fn new(store: Arc<dyn ElectoralLogStore>) -> Self {
        Self { store }
    }

    pub async fn create_board(&self, board: &str) -> Result<()> {
        self.store.create_board(board).await
    }
    pub async fn delete_board(&self, board: &str) -> Result<()> {
        self.store.delete_board(board).await
    }
    pub async fn has_board(&self, board: &str) -> Result<bool> {
        self.store.has_board(board).await
    }

    pub async fn append(&self, board: &str, entries: &[LogEntry]) -> Result<()> {
        self.append_iter(board, &mut entries.iter().cloned().map(Ok))
            .await
    }

    pub async fn append_iter(
        &self,
        board: &str,
        entries: &mut (dyn Iterator<Item = Result<LogEntry>> + Send),
    ) -> Result<()> {
        self.store.append(board, entries).await
    }

    pub async fn query(&self, board: &str, query: &LogQuery) -> Result<Vec<ElectoralLogMessage>> {
        query.validate()?;
        self.store.query(board, query).await
    }

    pub async fn count(&self, board: &str, query: &LogQuery) -> Result<i64> {
        self.store.count(board, query).await
    }

    pub async fn get_electoral_log_messages_filtered<K: Display, V: Display>(
        &self,
        board: &str,
        columns: Option<WhereClauseBTreeMap>,
        min_ts: Option<i64>,
        max_ts: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
        order: Option<HashMap<K, V>>,
    ) -> Result<Vec<ElectoralLogMessage>> {
        let mut query = LogQuery::default().with_text_filters(columns);
        if let Some(value) = min_ts {
            query.filters.push(Filter::Number(
                NumberColumn::Created,
                NumberComparison::GreaterThanOrEqual,
                value,
            ));
        }
        if let Some(value) = max_ts {
            query.filters.push(Filter::Number(
                NumberColumn::Created,
                NumberComparison::LessThanOrEqual,
                value,
            ));
        }
        if let Some(limit) = limit {
            query.limit = limit;
        }
        if let Some(offset) = offset {
            query.offset = offset;
        }
        if let Some(order) = order {
            // HashMap iteration must not change precedence between requests.
            let mut order: Vec<_> = order
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            order.sort();
            query.order = order
                .into_iter()
                .map(|(k, v)| Ok((k.parse()?, v.parse()?)))
                .collect::<Result<_>>()?;
        }
        self.query(board, &query).await
    }

    pub async fn count_electoral_log_messages(
        &self,
        board: &str,
        columns: Option<WhereClauseBTreeMap>,
    ) -> Result<i64> {
        self.count(board, &LogQuery::default().with_text_filters(columns))
            .await
    }

    pub async fn get_electoral_log_messages_batch(
        &self,
        board: &str,
        limit: i64,
        last_id: i64,
    ) -> Result<Vec<ElectoralLogMessage>> {
        self.query(
            board,
            &LogQuery {
                filters: vec![Filter::Number(
                    NumberColumn::Id,
                    NumberComparison::GreaterThan,
                    last_id,
                )],
                order: vec![(OrderColumn::Id, SortDirection::Asc)],
                limit,
                ..LogQuery::default()
            },
        )
        .await
    }

    pub async fn get_electoral_log_messages_at_offset(
        &self,
        board: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ElectoralLogMessage>> {
        self.query(
            board,
            &LogQuery {
                order: vec![(OrderColumn::Id, SortDirection::Asc)],
                limit,
                offset,
                ..LogQuery::default()
            },
        )
        .await
    }
}
