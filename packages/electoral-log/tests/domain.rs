// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Log queries and board names, which need no server.

use electoral_log::util::{get_event_board, init_log};
use electoral_log::{
    Filter, LogQuery, OrderColumn, SortDirection, SqlCompOperators, TextColumn, WhereClauseBTreeMap,
};

#[test]
fn a_default_query_reads_the_newest_records_first() {
    let query = LogQuery::default();
    assert!(query.filters.is_empty());
    assert!(query.visibility.is_none());
    assert!(!query.only_with_user);
    assert!(matches!(
        query.order.as_slice(),
        [(OrderColumn::Id, SortDirection::Desc)]
    ));
    assert_eq!((query.limit, query.offset), (900, 0));
    assert!(query.validate().is_ok());
}

#[test]
fn a_query_refuses_a_negative_limit_or_offset() {
    let limit = LogQuery {
        limit: -1,
        ..LogQuery::default()
    };
    assert!(limit.validate().is_err());
    let offset = LogQuery {
        offset: -1,
        ..LogQuery::default()
    };
    assert!(offset.validate().is_err());
}

#[test]
fn text_filters_are_added_to_a_query() {
    assert!(LogQuery::default()
        .with_text_filters(None)
        .filters
        .is_empty());

    let mut filters = WhereClauseBTreeMap::new();
    filters.insert(TextColumn::UserId, (SqlCompOperators::Equal, "user".into()));
    filters.insert(
        TextColumn::StatementKind,
        (SqlCompOperators::Like, "Admin%".into()),
    );
    let query = LogQuery::default().with_text_filters(Some(filters));
    let columns: Vec<_> = query
        .filters
        .iter()
        .map(|filter| match filter {
            Filter::Text(column, _, value) => (*column, value.as_str()),
            other => panic!("not a text filter: {other:?}"),
        })
        .collect();
    assert_eq!(
        columns,
        [
            (TextColumn::StatementKind, "Admin%"),
            (TextColumn::UserId, "user"),
        ]
    );
}

#[test]
fn an_event_board_is_named_without_hyphens() {
    assert_eq!(
        get_event_board(
            "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "0d1b2c3d-4e5f-4071-8293-a4b5c6d7e8f9"
        ),
        "tenant90505c8a23a94cdfa26b4e19f6a097d5event0d1b2c3d4e5f40718293a4b5c6d7e8f9"
    );
}

#[test]
fn the_log_is_set_up_without_becoming_the_global_subscriber() {
    // It bridges `log` records to tracing, which a process does once.
    init_log(false);
    assert!(tracing_log::LogTracer::init().is_err());
}
