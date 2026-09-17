// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{HeaderMap, Request, Response},
    routing::post,
    Router,
};
use immudb_rs::{Client, LoginResponse, OpenSessionResponse, UseDatabaseReply};
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
        }));
        let app = Router::new()
            .fallback(post(respond))
            .with_state(state.clone());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { state, task, url }
    }
    async fn client(&self) -> Client {
        Client::new(&self.url, "synthetic-user", "synthetic-password")
            .await
            .unwrap()
    }
    fn finish(&self) -> Vec<Seen> {
        let mut state = self.state.lock().unwrap();
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
    let bytes = to_bytes(body, 1024 * 1024).await.unwrap();
    assert_eq!(bytes[0], 0, "requests must be uncompressed");
    let length = u32::from_be_bytes(bytes[1..5].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), length + 5);
    let mut state = state.lock().unwrap();
    let reply = state.replies.pop_front().expect("unexpected RPC");
    assert_eq!(
        parts.uri.path(),
        format!("/immudb.schema.ImmuService/{}", reply.method)
    );
    state.seen.push(Seen {
        method: reply.method.into(),
        headers: parts.headers,
        payload: bytes[5..].to_vec(),
    });
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

#[tokio::test]
async fn deletion_propagates_delete_failure_after_a_successful_unload() {
    bounded(async {
        let server = Server::start(vec![
            Reply::ok(
                "UnloadDatabase",
                immudb_rs::UnloadDatabaseResponse::default(),
            ),
            Reply::error("DeleteDatabase", 7, "synthetic-delete-denied"),
        ])
        .await;
        let error = server
            .client()
            .await
            .delete_database("owned_fixture")
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("synthetic-delete-denied"),
            "{error}"
        );
        let requests = server.finish();
        assert_eq!(
            immudb_rs::UnloadDatabaseRequest::decode(requests[0].payload.as_slice())
                .unwrap()
                .database,
            "owned_fixture"
        );
        assert_eq!(
            immudb_rs::DeleteDatabaseRequest::decode(requests[1].payload.as_slice())
                .unwrap()
                .database,
            "owned_fixture"
        );
    })
    .await
}

#[tokio::test]
async fn deletion_success_missing_database_and_unload_error_have_distinct_outcomes() {
    bounded(async {
        let success = Server::start(vec![
            Reply::ok(
                "UnloadDatabase",
                immudb_rs::UnloadDatabaseResponse::default(),
            ),
            Reply::ok(
                "DeleteDatabase",
                immudb_rs::DeleteDatabaseResponse::default(),
            ),
        ])
        .await;
        success
            .client()
            .await
            .delete_database("owned_fixture")
            .await
            .unwrap();
        success.finish();
        let absent = Server::start(vec![Reply::error(
            "UnloadDatabase",
            5,
            "database does not exist",
        )])
        .await;
        absent
            .client()
            .await
            .delete_database("owned_fixture")
            .await
            .unwrap();
        assert_eq!(absent.finish().len(), 1);
        let denied = Server::start(vec![Reply::error(
            "UnloadDatabase",
            7,
            "synthetic-unload-denied",
        )])
        .await;
        assert!(denied
            .client()
            .await
            .delete_database("owned_fixture")
            .await
            .unwrap_err()
            .to_string()
            .contains("synthetic-unload-denied"));
        assert_eq!(denied.finish().len(), 1);
    })
    .await
}

#[tokio::test]
async fn login_session_and_logout_attach_then_clear_independent_metadata() {
    bounded(async {
        let server = Server::start(vec![
            Reply::ok(
                "Login",
                LoginResponse {
                    token: "login-token".into(),
                    ..Default::default()
                },
            ),
            Reply::ok(
                "OpenSession",
                OpenSessionResponse {
                    session_id: "session-7".into(),
                    ..Default::default()
                },
            ),
            Reply::ok(
                "UseDatabase",
                UseDatabaseReply {
                    token: "database-token".into(),
                },
            ),
            Reply::ok("CloseSession", ()),
            Reply::ok("Logout", ()),
            Reply::ok(
                "DatabaseListV2",
                immudb_rs::DatabaseListResponseV2::default(),
            ),
        ])
        .await;
        let mut client = server.client().await;
        client.login().await.unwrap();
        client.open_session("owned_fixture").await.unwrap();
        client.use_database("owned_fixture").await.unwrap();
        client.close_session().await.unwrap();
        client.logout().await.unwrap();
        client.list_databases().await.unwrap();
        let seen = server.finish();
        let login = immudb_rs::LoginRequest::decode(seen[0].payload.as_slice()).unwrap();
        assert_eq!(login.user, b"synthetic-user");
        assert_eq!(login.password, b"synthetic-password");
        let session = immudb_rs::OpenSessionRequest::decode(seen[1].payload.as_slice()).unwrap();
        assert_eq!(session.database_name, "owned_fixture");
        assert!(seen[0].headers.get("authorization").is_none());
        assert_eq!(seen[2].headers["authorization"], "Bearer login-token");
        assert_eq!(seen[2].headers["sessionid"], "session-7");
        assert_eq!(seen[3].headers["authorization"], "database-token");
        assert_eq!(seen[3].headers["sessionid"], "session-7");
        assert!(seen[4].headers.get("sessionid").is_none());
        assert_eq!(seen[4].headers["authorization"], "database-token");
        assert!(seen[5].headers.get("authorization").is_none());
        assert!(seen[5].headers.get("sessionid").is_none());
    })
    .await
}

#[tokio::test]
async fn failed_logout_keeps_authentication_for_a_retry() {
    bounded(async {
        let server = Server::start(vec![
            Reply::ok(
                "Login",
                LoginResponse {
                    token: "retry-token".into(),
                    ..Default::default()
                },
            ),
            Reply::error("Logout", 14, "temporarily-unavailable"),
            Reply::ok("Logout", ()),
        ])
        .await;
        let mut client = server.client().await;
        client.login().await.unwrap();
        assert!(client
            .logout()
            .await
            .unwrap_err()
            .to_string()
            .contains("temporarily-unavailable"));
        client.logout().await.unwrap();
        let seen = server.finish();
        assert_eq!(seen[1].headers["authorization"], "Bearer retry-token");
        assert_eq!(seen[2].headers["authorization"], "Bearer retry-token");
    })
    .await
}

#[tokio::test]
async fn sql_transactions_preserve_parameters_and_transaction_metadata() {
    bounded(async {
        let server = Server::start(vec![
            Reply::ok(
                "NewTx",
                immudb_rs::NewTxResponse {
                    transaction_id: "tx-17".into(),
                },
            ),
            Reply::ok("TxSQLExec", immudb_rs::SqlExecResult::default()),
            Reply::ok("Commit", immudb_rs::CommittedSqlTx::default()),
            Reply::ok("Rollback", ()),
        ])
        .await;
        let mut client = server.client().await;
        let tx = client.new_tx(immudb_rs::TxMode::ReadWrite).await.unwrap();
        assert_eq!(tx, "tx-17");
        let parameter = immudb_rs::NamedParam {
            name: "value".into(),
            value: Some(immudb_rs::SqlValue {
                value: Some(immudb_rs::sql_value::Value::N(17)),
            }),
        };
        client
            .tx_sql_exec(
                "INSERT INTO fixture(v) VALUES (@value)",
                &tx,
                vec![parameter.clone()],
            )
            .await
            .unwrap();
        client.commit(&tx).await.unwrap();
        client.rollback(&tx).await.unwrap();
        // Invalid metadata fails locally: no additional RPC may reach the fixture.
        assert!(client.commit(&"bad\ntransaction".into()).await.is_err());
        let seen = server.finish();
        let request = immudb_rs::SqlExecRequest::decode(seen[1].payload.as_slice()).unwrap();
        assert_eq!(request.sql, "INSERT INTO fixture(v) VALUES (@value)");
        assert_eq!(request.params, vec![parameter]);
        assert!(!request.no_wait);
        for request in &seen[1..] {
            assert_eq!(
                request.headers["transactionid"], "tx-17",
                "{}",
                request.method
            )
        }
    })
    .await
}

#[tokio::test]
async fn database_presence_requires_both_exact_name_and_loaded_state() {
    bounded(async {
        let list = immudb_rs::DatabaseListResponseV2 {
            databases: vec![
                immudb_rs::DatabaseInfo {
                    name: "poll".into(),
                    loaded: true,
                    ..Default::default()
                },
                immudb_rs::DatabaseInfo {
                    name: "archive".into(),
                    loaded: false,
                    ..Default::default()
                },
            ],
        };
        let server = Server::start(
            (0..4)
                .map(|_| Reply::ok("DatabaseListV2", list.clone()))
                .collect(),
        )
        .await;
        let mut client = server.client().await;
        assert!(client.has_database("poll").await.unwrap());
        for name in ["Poll", "archive", "missing"] {
            assert!(!client.has_database(name).await.unwrap(), "{name}")
        }
        server.finish();
    })
    .await
}

#[tokio::test]
async fn query_modes_and_parameters_reach_the_rpc_boundary() {
    bounded(async {
        let result = immudb_rs::SqlQueryResult {
            rows: vec![immudb_rs::Row {
                columns: vec!["value".into()],
                values: vec![immudb_rs::SqlValue {
                    value: Some(immudb_rs::sql_value::Value::N(17)),
                }],
            }],
            columns: vec![],
        };
        let server = Server::start(vec![
            Reply::ok("ListTables", immudb_rs::SqlQueryResult::default()),
            Reply::ok("ListTables", result.clone()),
            Reply::ok("UnarySQLQuery", result.clone()),
            Reply::ok("SQLQuery", result.clone()),
            Reply::ok("TxSQLQuery", result.clone()),
            Reply::ok("SQLExec", immudb_rs::SqlExecResult::default()),
            Reply::ok(
                "CreateDatabaseV2",
                immudb_rs::CreateDatabaseResponse::default(),
            ),
        ])
        .await;
        let mut client = server.client().await;
        assert!(!client.has_tables().await.unwrap());
        assert!(client.has_tables().await.unwrap());
        assert_eq!(
            client
                .sql_query("SELECT 17", vec![])
                .await
                .unwrap()
                .into_inner(),
            result
        );
        let mut stream = client
            .streaming_sql_query("SELECT 18", vec![])
            .await
            .unwrap()
            .into_inner();
        assert_eq!(stream.message().await.unwrap(), Some(result.clone()));
        assert!(stream.message().await.unwrap().is_none());
        let mut tx_stream = client
            .tx_sql_query("SELECT 19", &"tx-19".into(), vec![])
            .await
            .unwrap()
            .into_inner();
        assert_eq!(tx_stream.message().await.unwrap(), Some(result));
        client
            .sql_exec("CREATE TABLE fixture(v INTEGER)", vec![])
            .await
            .unwrap();
        client.create_database("fixture").await.unwrap();
        let seen = server.finish();
        for (index, sql, accept_stream) in [
            (2, "SELECT 17", false),
            (3, "SELECT 18", true),
            (4, "SELECT 19", false),
        ] {
            let request =
                immudb_rs::SqlQueryRequest::decode(seen[index].payload.as_slice()).unwrap();
            assert_eq!(request.sql, sql);
            assert_eq!(request.accept_stream, accept_stream);
        }
        assert_eq!(seen[4].headers["transactionid"], "tx-19");
        let create = immudb_rs::CreateDatabaseRequest::decode(seen[6].payload.as_slice()).unwrap();
        assert_eq!(create.name, "fixture");
        assert!(create.if_not_exists);
        assert!(create.settings.is_none());
    })
    .await
}

#[tokio::test]
async fn invalid_session_metadata_is_rejected_before_sending_an_authenticated_request() {
    bounded(async {
        let server = Server::start(vec![Reply::ok(
            "OpenSession",
            OpenSessionResponse {
                session_id: "invalid\nsession".into(),
                ..Default::default()
            },
        )])
        .await;
        let mut client = server.client().await;
        client.open_session("fixture").await.unwrap();
        assert!(client.list_databases().await.is_err());
        assert_eq!(server.finish().len(), 1);
    })
    .await
}
