// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{HeaderMap, Request, Response},
    routing::post,
    Router,
};
use electoral_log::{BoardClient, ElectoralLogMessage};
use immudb_rs::{
    sql_value::Value, LoginResponse, Row, SqlExecRequest, SqlQueryRequest, SqlQueryResult, SqlValue,
};
use prost::Message;
use std::{
    collections::VecDeque,
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};

struct Reply {
    method: &'static str,
    bytes: Vec<u8>,
    status: u16,
    error: &'static str,
}
impl Reply {
    fn ok(method: &'static str, response: impl Message) -> Self {
        Self {
            method,
            bytes: response.encode_to_vec(),
            status: 0,
            error: "",
        }
    }
    fn error(method: &'static str, status: u16, error: &'static str) -> Self {
        Self {
            method,
            bytes: vec![],
            status,
            error,
        }
    }
}
struct Seen {
    method: String,
    headers: HeaderMap,
    payload: Vec<u8>,
}
#[derive(Default)]
struct Script {
    replies: VecDeque<Reply>,
    seen: Vec<Seen>,
    errors: Vec<String>,
}
struct Server {
    state: Arc<Mutex<Script>>,
    task: tokio::task::JoinHandle<()>,
    url: String,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn start(replies: Vec<Reply>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(Script {
            replies: replies.into(),
            seen: vec![],
            errors: vec![],
        }));
        let app = Router::new()
            .fallback(post(respond))
            .with_state(state.clone());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { state, task, url }
    }
    async fn client(&self) -> BoardClient {
        BoardClient::new(&self.url, "synthetic-user", "synthetic-password")
            .await
            .unwrap()
    }
    fn finish(&self) -> Vec<Seen> {
        let mut state = self.state.lock().unwrap();
        assert!(
            state.errors.is_empty(),
            "fixture errors: {:?}",
            state.errors
        );
        assert!(
            state.replies.is_empty(),
            "not all expected RPCs were called"
        );
        std::mem::take(&mut state.seen)
    }
}
async fn respond(
    State(state): State<Arc<Mutex<Script>>>,
    request: Request<Body>,
) -> Response<Body> {
    let (parts, body) = request.into_parts();
    let body = to_bytes(body, 1024 * 1024).await;
    let mut state = state.lock().unwrap();
    let actual_method = parts.uri.path().to_owned();
    let bytes = match body {
        Ok(bytes) => bytes,
        Err(error) => {
            state.errors.push(format!("{actual_method}: {error}"));
            return Response::builder().status(500).body(Body::empty()).unwrap();
        }
    };
    if bytes.len() < 5
        || bytes[0] != 0
        || bytes.len() != 5 + u32::from_be_bytes(bytes[1..5].try_into().unwrap()) as usize
    {
        state
            .errors
            .push(format!("invalid gRPC frame for {actual_method}"));
        return Response::builder().status(500).body(Body::empty()).unwrap();
    }
    state.seen.push(Seen {
        method: actual_method
            .strip_prefix("/immudb.schema.ImmuService/")
            .unwrap_or(&actual_method)
            .into(),
        headers: parts.headers,
        payload: bytes[5..].to_vec(),
    });
    let Some(reply) = state.replies.pop_front() else {
        state
            .errors
            .push(format!("unexpected RPC: {actual_method}"));
        return Response::builder()
            .header("content-type", "application/grpc")
            .header("grpc-status", "13")
            .header("grpc-message", "unexpected RPC")
            .body(Body::empty())
            .unwrap();
    };
    if actual_method != format!("/immudb.schema.ImmuService/{}", reply.method) {
        state.errors.push(format!(
            "expected {}, received {actual_method}",
            reply.method
        ));
    }
    let mut response = Response::builder()
        .header("content-type", "application/grpc")
        .header("grpc-status", reply.status.to_string());
    if reply.status != 0 {
        return response
            .header("grpc-message", reply.error)
            .body(Body::empty())
            .unwrap();
    }
    let mut frame = vec![0];
    frame.extend((reply.bytes.len() as u32).to_be_bytes());
    frame.extend(reply.bytes);
    response = response.header("grpc-message", "");
    response.body(Body::from(frame)).unwrap()
}
async fn bounded(work: impl Future<Output = ()>) {
    tokio::time::timeout(Duration::from_secs(10), work)
        .await
        .expect("RPC fixture timed out")
}

fn login() -> Reply {
    Reply::ok("Login", LoginResponse::default())
}
fn message() -> ElectoralLogMessage {
    ElectoralLogMessage {
        id: 0,
        created: 123,
        statement_timestamp: 456,
        sender_pk: "signer".into(),
        statement_kind: "CastVote".into(),
        message: vec![1, 2],
        version: "2".into(),
        user_id: Some("voter".into()),
        username: None,
        election_id: Some("election".into()),
        area_id: None,
        ballot_id: Some("ballot".into()),
    }
}
fn receipt(value: Option<Value>) -> SqlQueryResult {
    SqlQueryResult {
        rows: vec![Row {
            columns: vec!["payload_hash".into()],
            values: vec![SqlValue { value }],
        }],
        ..Default::default()
    }
}

#[tokio::test]
async fn receipt_and_audit_rows_use_the_same_transaction_with_bound_identity_parameters() {
    bounded(async {
        let server = Server::start(vec![login(),
            Reply::ok("SQLExec", immudb_rs::SqlExecResult::default()),
            Reply::ok("TxSQLQuery", SqlQueryResult::default()),
            Reply::ok("TxSQLExec", ()), Reply::ok("TxSQLExec", ()),
        ]).await;
        let mut client = server.client().await;
        client.ensure_electoral_log_delivery_receipts().await.unwrap();
        assert!(client.insert_electoral_log_delivery(&"owned-tx".into(), &"a".repeat(64), &"b".repeat(64), &[message()]).await.unwrap());
        let requests = server.finish();
        let schema = SqlExecRequest::decode(requests[1].payload.as_slice()).unwrap();
        assert_eq!(schema.sql, "CREATE TABLE IF NOT EXISTS electoral_log_delivery_receipts (delivery_id VARCHAR[64], payload_hash VARCHAR[64], PRIMARY KEY delivery_id)");
        assert!(schema.params.is_empty());
        let query = SqlQueryRequest::decode(requests[2].payload.as_slice()).unwrap();
        assert_eq!(query.sql, "SELECT payload_hash FROM electoral_log_delivery_receipts WHERE delivery_id = @delivery_id");
        assert_eq!(query.params[0].name, "delivery_id");
        assert_eq!(query.params[0].value.as_ref().unwrap().value, Some(Value::S("a".repeat(64))));
        let audit = SqlExecRequest::decode(requests[3].payload.as_slice()).unwrap();
        assert!(audit.sql.contains("INSERT INTO electoral_log_messages"));
        assert_eq!(audit.params.iter().find(|p| p.name == "ballot_id").unwrap().value.as_ref().unwrap().value, Some(Value::S("ballot".into())));
        let insert = SqlExecRequest::decode(requests[4].payload.as_slice()).unwrap();
        assert_eq!(insert.sql, "INSERT INTO electoral_log_delivery_receipts (delivery_id, payload_hash) VALUES (@delivery_id, @payload_hash)");
        assert_eq!(insert.params.iter().map(|param| param.name.as_str()).collect::<Vec<_>>(), ["delivery_id", "payload_hash"]);
        assert_eq!(insert.params[1].value.as_ref().unwrap().value, Some(Value::S("b".repeat(64))));
        for request in &requests[2..] { assert_eq!(request.headers["transactionid"], "owned-tx"); }
        assert_eq!(requests.iter().map(|r| r.method.as_str()).collect::<Vec<_>>(), ["Login", "SQLExec", "TxSQLQuery", "TxSQLExec", "TxSQLExec"]);
    }).await;
}

#[tokio::test]
async fn matched_receipt_is_a_noop_but_changed_or_malformed_receipt_is_an_error() {
    bounded(async {
        for (value, expected_error) in [
            (Some(Value::S("b".repeat(64))), None),
            (
                Some(Value::S("c".repeat(64))),
                Some("electoral log delivery ID reused with different input"),
            ),
            (None, Some("invalid electoral log delivery receipt")),
        ] {
            let server =
                Server::start(vec![login(), Reply::ok("TxSQLQuery", receipt(value))]).await;
            let result = server
                .client()
                .await
                .insert_electoral_log_delivery(
                    &"tx".into(),
                    &"a".repeat(64),
                    &"b".repeat(64),
                    &[message()],
                )
                .await;
            match expected_error {
                Some(error) => assert_eq!(result.unwrap_err().to_string(), error),
                None => assert!(!result.unwrap()),
            }
            assert_eq!(
                server.finish().len(),
                2,
                "receipt lookup must not write on these paths"
            );
        }
        let mut duplicate = receipt(Some(Value::S("b".repeat(64))));
        duplicate.rows.push(duplicate.rows[0].clone());
        let server = Server::start(vec![login(), Reply::ok("TxSQLQuery", duplicate)]).await;
        let error = server
            .client()
            .await
            .insert_electoral_log_delivery(
                &"tx".into(),
                &"a".repeat(64),
                &"b".repeat(64),
                &[message()],
            )
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "duplicate electoral log delivery receipts"
        );
        server.finish();
    })
    .await;
}

#[tokio::test]
async fn query_audit_insert_and_receipt_insert_failures_propagate_without_commit() {
    bounded(async {
        for failure_at in 0..3 {
            let mut replies = vec![login()];
            if failure_at > 0 {
                replies.push(Reply::ok("TxSQLQuery", SqlQueryResult::default()));
            }
            if failure_at > 1 {
                replies.push(Reply::ok("TxSQLExec", ()));
            }
            replies.push(Reply::error(
                if failure_at == 0 {
                    "TxSQLQuery"
                } else {
                    "TxSQLExec"
                },
                14,
                "injected-rpc-failure",
            ));
            let server = Server::start(replies).await;
            let error = server
                .client()
                .await
                .insert_electoral_log_delivery(
                    &"tx".into(),
                    &"a".repeat(64),
                    &"b".repeat(64),
                    &[message()],
                )
                .await
                .unwrap_err();
            let status = error.downcast_ref::<tonic::Status>().unwrap();
            assert_eq!(status.code(), tonic::Code::Unavailable);
            assert_eq!(status.message(), "injected-rpc-failure");
            server.finish();
        }
    })
    .await;
}

#[tokio::test]
async fn invalid_identity_or_empty_delivery_is_rejected_before_database_access() {
    bounded(async {
        for (id, hash, messages, expected_error) in [
            (
                "".to_string(),
                "b".repeat(64),
                vec![message()],
                "electoral log delivery identity must be a SHA-256 digest",
            ),
            (
                "a".repeat(64),
                "'".repeat(64),
                vec![message()],
                "electoral log delivery identity must be a SHA-256 digest",
            ),
            (
                "a".repeat(64),
                "b".repeat(64),
                vec![],
                "electoral log delivery has no messages",
            ),
        ] {
            let server = Server::start(vec![login()]).await;
            let error = server
                .client()
                .await
                .insert_electoral_log_delivery(&"tx".into(), &id, &hash, &messages)
                .await
                .unwrap_err();
            assert_eq!(error.to_string(), expected_error);
            assert_eq!(server.finish().len(), 1);
        }
    })
    .await;
}
