// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use bytes::Bytes;
use immudb_rs::{
    Client, LoginRequest, LoginResponse, NewTxResponse, OpenSessionRequest, OpenSessionResponse,
    SqlQueryResult, TxMode, UseDatabaseReply,
};
use prost::Message;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tracing_subscriber::fmt::format::FmtSpan;

const PASSWORD: &str = "synthetic-password-never-log-3197";
const TOKEN: &str = "synthetic-login-token-never-log-3197";
const DATABASE_TOKEN: &str = "synthetic-database-token-never-log-3197";
const SESSION: &str = "synthetic-session-never-log-3197";
const TRANSACTION: &str = "synthetic-transaction-never-log-3197";

#[derive(Clone, Default)]
struct LogCapture(Arc<Mutex<Vec<u8>>>);
impl Write for LogCapture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct Observation {
    method: String,
    authorization: Option<String>,
    session: Option<String>,
    transaction: Option<String>,
}

struct TestServer {
    address: String,
    task: tokio::task::JoinHandle<()>,
    observations: Arc<Mutex<Vec<Observation>>>,
}
impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

// A real HTTP/2 + gRPC transport using the production protobuf types, with explicitly
// synthetic responses. It does not emulate immudb storage or authentication enforcement.
async fn server(reject_delete: bool) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let observations = Arc::new(Mutex::new(Vec::new()));
    let observed = observations.clone();
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut connection = h2::server::handshake(stream).await.unwrap();
        while let Some(request) = connection.accept().await {
            let (request, mut respond) = request.unwrap();
            let method = request.uri().path().rsplit('/').next().unwrap().to_string();
            let header = |name| {
                request
                    .headers()
                    .get(name)
                    .map(|v| v.to_str().unwrap().to_string())
            };
            observed.lock().unwrap().push(Observation {
                method: method.clone(),
                authorization: header("authorization"),
                session: header("sessionid"),
                transaction: header("transactionid"),
            });
            tokio::spawn(async move {
                let mut incoming = request.into_body();
                let mut framed = Vec::new();
                while let Some(bytes) = incoming.data().await {
                    let bytes = bytes.unwrap();
                    incoming
                        .flow_control()
                        .release_capacity(bytes.len())
                        .unwrap();
                    framed.extend_from_slice(&bytes);
                }
                assert!(framed.len() >= 5);
                assert_eq!(framed[0], 0);
                assert_eq!(
                    u32::from_be_bytes(framed[1..5].try_into().unwrap()) as usize,
                    framed.len() - 5
                );
                let payload = match method.as_str() {
                    "Login" => {
                        let input = LoginRequest::decode(&framed[5..]).unwrap();
                        assert_eq!(input.password, PASSWORD.as_bytes());
                        LoginResponse {
                            token: TOKEN.into(),
                            ..Default::default()
                        }
                        .encode_to_vec()
                    }
                    "UseDatabase" => UseDatabaseReply {
                        token: DATABASE_TOKEN.into(),
                    }
                    .encode_to_vec(),
                    "OpenSession" => {
                        let input = OpenSessionRequest::decode(&framed[5..]).unwrap();
                        assert_eq!(input.password, PASSWORD.as_bytes());
                        OpenSessionResponse {
                            session_id: SESSION.into(),
                            ..Default::default()
                        }
                        .encode_to_vec()
                    }
                    "NewTx" => NewTxResponse {
                        transaction_id: TRANSACTION.into(),
                    }
                    .encode_to_vec(),
                    "UnarySQLQuery" | "TxSQLQuery" => SqlQueryResult::default().encode_to_vec(),
                    _ => Vec::new(),
                };
                let rejected = reject_delete && method == "DeleteDatabase";
                let response = http::Response::builder()
                    .status(200)
                    .header("content-type", "application/grpc")
                    .body(())
                    .unwrap();
                let mut output = respond.send_response(response, false).unwrap();
                if !rejected {
                    let mut data = vec![0];
                    data.extend_from_slice(&(payload.len() as u32).to_be_bytes());
                    data.extend_from_slice(&payload);
                    output.send_data(Bytes::from(data), false).unwrap();
                }
                let mut trailers = http::HeaderMap::new();
                trailers.insert(
                    "grpc-status",
                    if rejected { "7" } else { "0" }.parse().unwrap(),
                );
                if rejected {
                    trailers.insert("grpc-message", "synthetic-delete-denied".parse().unwrap());
                }
                output.send_trailers(trailers).unwrap();
            });
        }
    });
    TestServer {
        address,
        task,
        observations,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn authentication_remains_on_wire_and_out_of_diagnostics() {
    let log = LogCapture::default();
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .without_time()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let service = server(false).await;
    let mut client = Client::new(&service.address, "synthetic-user", PASSWORD)
        .await
        .unwrap();
    client.login().await.unwrap();
    client.use_database("synthetic-database").await.unwrap();
    client.open_session("synthetic-database").await.unwrap();
    client.sql_query("SELECT 1", vec![]).await.unwrap();
    let transaction = client.new_tx(TxMode::ReadWrite).await.unwrap();
    assert_eq!(transaction, TRANSACTION);
    client
        .tx_sql_exec("SELECT 1", &transaction, vec![])
        .await
        .unwrap();
    let mut rows = client
        .tx_sql_query("SELECT 1", &transaction, vec![])
        .await
        .unwrap()
        .into_inner();
    assert!(rows.message().await.unwrap().is_some());
    assert!(rows.message().await.unwrap().is_none());
    client.commit(&transaction).await.unwrap();
    let transaction = client.new_tx(TxMode::ReadWrite).await.unwrap();
    client.rollback(&transaction).await.unwrap();
    let diagnostic = format!("{client:?}");
    client.close_session().await.unwrap();
    client.logout().await.unwrap();
    client.list_databases().await.unwrap();
    let output = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
    for secret in [PASSWORD, TOKEN, DATABASE_TOKEN, SESSION, TRANSACTION] {
        assert!(
            !diagnostic.contains(secret),
            "Client Debug exposes a synthetic credential"
        );
        let leaked = output
            .lines()
            .filter(|line| line.contains(secret))
            .map(|line| line.replace(secret, "[SYNTHETIC-SECRET]"))
            .collect::<Vec<_>>();
        assert!(
            leaked.is_empty(),
            "captured trace exposes a synthetic credential: {leaked:?}"
        );
    }
    assert!(output.contains("immudb login completed"));
    assert!(diagnostic.contains("authenticated: true"));
    let seen = service.observations.lock().unwrap();
    let use_db = seen.iter().find(|r| r.method == "UseDatabase").unwrap();
    assert_eq!(use_db.authorization, Some(format!("Bearer {TOKEN}")));
    let query = seen.iter().find(|r| r.method == "UnarySQLQuery").unwrap();
    assert_eq!(query.authorization.as_deref(), Some(DATABASE_TOKEN));
    assert_eq!(query.session.as_deref(), Some(SESSION));
    for method in ["TxSQLExec", "TxSQLQuery", "Commit", "Rollback"] {
        let request = seen.iter().find(|r| r.method == method).unwrap();
        assert_eq!(request.transaction.as_deref(), Some(TRANSACTION));
        assert_eq!(request.authorization.as_deref(), Some(DATABASE_TOKEN));
        assert_eq!(request.session.as_deref(), Some(SESSION));
    }
    let last = seen.last().unwrap();
    assert_eq!(last.method, "DatabaseListV2");
    assert!(last.authorization.is_none() && last.session.is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn database_delete_reports_failure_after_successful_unload() {
    let service = server(true).await;
    let mut client = Client::new(&service.address, "synthetic-user", PASSWORD)
        .await
        .unwrap();
    let result = client.delete_database("synthetic-database").await;
    assert!(
        result.is_err(),
        "a rejected DeleteDatabase RPC must not report success"
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("synthetic-delete-denied"));
    let seen = service.observations.lock().unwrap();
    assert_eq!(
        seen.iter().map(|r| r.method.as_str()).collect::<Vec<_>>(),
        ["UnloadDatabase", "DeleteDatabase"]
    );
}
