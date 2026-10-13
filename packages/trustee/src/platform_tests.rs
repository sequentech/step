// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use protocol_board::{BoardName, TrusteeReportKind};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};
use std::time::Instant;

const TOKEN: &str = r#"{"access_token":"issued-token","expires_in":300}"#;
const DKG: &str = "dkg_d9792af071b849528aac94bc0fead5f7";
const TALLY: &str = "tally_0f6a3b2c9d8e4f10a1b2c3d4e5f60718_3";
const BOARDS: &str = r#"{"data":{"list_trustee_boards":{"boards":[{"name":"dkg_d9792af071b849528aac94bc0fead5f7","kind":"DKG","parent":null},{"name":"tally_0f6a3b2c9d8e4f10a1b2c3d4e5f60718_3","kind":"TALLY","parent":"dkg_d9792af071b849528aac94bc0fead5f7"}]}}}"#;
const ACCEPTED: &str = r#"{"data":{"report_trustee_board":{"accepted":true}}}"#;

struct Request {
    line: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

/// Own the bound port and use bounded I/O; no backend services are required.
fn platform(
    answers: Vec<(u16, &'static str)>,
) -> (PlatformClient, JoinHandle<Vec<Request>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body) in answers {
            let deadline = Instant::now() + Duration::from_secs(5);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock =>
                    {
                        assert!(
                            Instant::now() < deadline,
                            "Expected HTTP request did not arrive"
                        );
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept failed: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut headers = HashMap::new();
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header == "\r\n" {
                    break;
                }
                let (name, value) =
                    header.split_once(':').expect("HTTP header");
                headers.insert(name.to_lowercase(), value.trim().to_string());
            }
            let length = headers
                .get("content-length")
                .map(|value| value.parse::<usize>().unwrap())
                .unwrap_or(0);
            let mut request_body = vec![0; length];
            reader.read_exact(&mut request_body).unwrap();
            requests.push(Request {
                line: line.trim_end().to_string(),
                headers,
                body: request_body,
            });
            write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        requests
    });
    let client = PlatformClient::new(
        &format!("{url}/v1/graphql"),
        Credentials {
            keycloak_url: url,
            realm: "tenant-fixture".into(),
            client_id: "api-key-client".into(),
            client_secret: None,
            username: "trustee1".into(),
            password: "fixture-password".into(),
        },
    )
    .unwrap();
    (client, server)
}

fn report() -> TrusteeReport {
    TrusteeReport {
        board: BoardName::parse(DKG).unwrap(),
        kind: TrusteeReportKind::HALTED,
        detail: "rewritten board".into(),
    }
}

fn graphql_body(request: &Request) -> Value {
    assert_eq!(request.line, "POST /v1/graphql HTTP/1.1");
    assert_eq!(request.headers["authorization"], "Bearer issued-token");
    assert_eq!(request.headers["x-hasura-role"], "trustee-ceremony");
    assert_eq!(request.headers["content-type"], "application/json");
    serde_json::from_slice(&request.body).unwrap()
}

#[tokio::test]
async fn discovery_and_reports_use_the_browser_graphql_wire_contract() {
    let (mut client, server) =
        platform(vec![(200, TOKEN), (200, BOARDS), (200, ACCEPTED)]);
    assert_eq!(
        client.list().await.unwrap(),
        vec![
            TrusteeBoard::of(DKG, None).unwrap(),
            TrusteeBoard::of(TALLY, Some(DKG)).unwrap(),
        ]
    );
    client.report(&report()).await.unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 3, "Reuse the still-valid trustee token");
    assert_eq!(
        requests[0].line,
        "POST /realms/tenant-fixture/protocol/openid-connect/token HTTP/1.1"
    );
    assert_eq!(
        graphql_body(&requests[1]),
        json!({
            "query": "query ListTrusteeBoards { list_trustee_boards { boards { name kind parent } } }",
            "variables": {},
        })
    );
    assert_eq!(
        graphql_body(&requests[2]),
        json!({
            "query": "mutation ReportTrusteeBoard($report: jsonb!) { report_trustee_board(report: $report) { accepted } }",
            "variables": {"report": {"board": DKG, "kind": "HALTED", "detail": "rewritten board"}},
        })
    );
}

#[tokio::test]
async fn discovery_rejects_graphql_errors_even_with_http_success_or_partial_data(
) {
    for answer in [
        r#"{"errors":[{"message":"permission denied"}]}"#,
        r#"{"data":{"list_trustee_boards":{"boards":[]}},"errors":[{"message":"permission denied"}]}"#,
    ] {
        let (mut client, server) = platform(vec![(200, TOKEN), (200, answer)]);
        let error = client.list().await.unwrap_err();
        assert!(
            format!("{error:#}").contains("permission denied"),
            "{error:#}"
        );
        server.join().unwrap();
    }
}

#[tokio::test]
async fn discovery_rejects_missing_data_and_malformed_board_answers() {
    for answer in [
        r#"{}"#,
        r#"{"data":null}"#,
        r#"{"data":{"list_trustee_boards":null}}"#,
        r#"{"data":{"list_trustee_boards":{"boards":[{"name":"foreign","kind":"DKG","parent":null}]}}}"#,
        "not JSON",
    ] {
        let (mut client, server) = platform(vec![(200, TOKEN), (200, answer)]);
        assert!(
            client.list().await.is_err(),
            "Accepted malformed answer: {answer}"
        );
        server.join().unwrap();
    }
}

#[tokio::test]
async fn reporting_requires_explicit_acknowledgement_without_graphql_errors() {
    for answer in [
        r#"{"data":{"report_trustee_board":{"accepted":false}}}"#,
        r#"{"data":{"report_trustee_board":null}}"#,
        r#"{"data":{"report_trustee_board":{}}}"#,
        r#"{"data":null}"#,
        r#"{"errors":[{"message":"report rejected"}]}"#,
        r#"{"data":{"report_trustee_board":{"accepted":true}},"errors":[{"message":"report rejected"}]}"#,
    ] {
        let (mut client, server) = platform(vec![(200, TOKEN), (200, answer)]);
        assert!(
            client.report(&report()).await.is_err(),
            "Settled unacknowledged report: {answer}"
        );
        server.join().unwrap();
    }
}

#[tokio::test]
async fn reporting_retries_after_a_graphql_error_using_the_cached_token() {
    let (mut client, server) = platform(vec![
        (200, TOKEN),
        (200, r#"{"errors":[{"message":"try again"}]}"#),
        (200, ACCEPTED),
    ]);
    assert!(client.report(&report()).await.is_err());
    client.report(&report()).await.unwrap();
    let requests = server.join().unwrap();
    assert_eq!(graphql_body(&requests[1]), graphql_body(&requests[2]));
}

#[tokio::test]
async fn unsuccessful_http_responses_remain_errors_for_discovery_and_reporting()
{
    let (mut client, server) = platform(vec![
        (200, TOKEN),
        (401, "invalid JWT"),
        (503, "unavailable"),
    ]);
    assert!(
        format!("{:#}", client.list().await.unwrap_err()).contains("HTTP 401")
    );
    assert!(format!("{:#}", client.report(&report()).await.unwrap_err())
        .contains("HTTP 503"));
    server.join().unwrap();
}

#[tokio::test]
async fn updates_fetch_boards_and_actions_together_and_results_need_acknowledgement(
) {
    let (mut client, server) = platform(vec![
        (200, TOKEN),
        (
            200,
            r#"{"data":{"trustee_event_updates":{"boards":[],"actions":[]}}}"#,
        ),
        (
            200,
            r#"{"data":{"submit_trustee_action_result":{"accepted":true}}}"#,
        ),
    ]);
    let updates = client.updates().await.unwrap();
    assert!(updates.boards.is_empty());
    assert!(updates.actions.is_empty());
    let result = protocol_board::TrusteeActionResult {
        id: "action".into(),
        request: protocol_board::CustodyRequest {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            keys_ceremony_id: "ceremony".into(),
            tally_session_id: None,
            trustee_name: "trustee1".into(),
            signing_public_key: "signing".into(),
            share_encryption_public_key: "encryption".into(),
        },
        outcome: protocol_board::TrusteeActionOutcome::Failed {
            detail: "keys unavailable".into(),
        },
    };
    client.action_result(&result).await.unwrap();
    let requests = server.join().unwrap();
    assert_eq!(
        graphql_body(&requests[1])["query"],
        "query TrusteeEventUpdates { trustee_event_updates }"
    );
    assert_eq!(
        graphql_body(&requests[2])["variables"]["result"],
        serde_json::to_value(result).unwrap()
    );
}

#[tokio::test]
async fn updates_fall_back_only_when_the_new_field_is_absent() {
    for absent in [
        r#"{"errors":[{"message":"Cannot query field \"trustee_event_updates\" on type \"query_root\"."}]}"#,
        r#"{"errors":[{"message":"field 'trustee_event_updates' not found in type: 'query_root'","extensions":{"code":"validation-failed"}}]}"#,
    ] {
        let (mut client, server) = platform(vec![
            (200, TOKEN),
            (200, absent),
            (200, r#"{"data":{"list_trustee_boards":{"boards":[]}}}"#),
        ]);
        assert!(client.updates().await.unwrap().actions.is_empty());
        let requests = server.join().unwrap();
        assert!(graphql_body(&requests[2])["query"]
            .as_str()
            .unwrap()
            .contains("ListTrusteeBoards"));
    }
    let (mut client, server) = platform(vec![
        (200, TOKEN),
        (200, r#"{"errors":[{"message":"permission denied"}]}"#),
    ]);
    assert!(client.updates().await.is_err());
    assert_eq!(server.join().unwrap().len(), 2);
}

#[tokio::test]
async fn action_results_require_explicit_acknowledgement_and_are_retried() {
    let result = protocol_board::TrusteeActionResult {
        id: "action".into(),
        request: protocol_board::CustodyRequest {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            keys_ceremony_id: "ceremony".into(),
            tally_session_id: None,
            trustee_name: "trustee1".into(),
            signing_public_key: "signing".into(),
            share_encryption_public_key: "encryption".into(),
        },
        outcome: protocol_board::TrusteeActionOutcome::Failed {
            detail: "keys unavailable".into(),
        },
    };
    for rejected in [
        r#"{"data":{"submit_trustee_action_result":{"accepted":false}}}"#,
        r#"{"data":{"submit_trustee_action_result":null}}"#,
        r#"{"data":{"submit_trustee_action_result":{"accepted":true}},"errors":[{"message":"try again"}]}"#,
    ] {
        let (mut client, server) = platform(vec![
            (200, TOKEN),
            (200, rejected),
            (
                200,
                r#"{"data":{"submit_trustee_action_result":{"accepted":true}}}"#,
            ),
        ]);
        assert!(client.action_result(&result).await.is_err());
        client.action_result(&result).await.unwrap();
        let requests = server.join().unwrap();
        assert_eq!(graphql_body(&requests[1]), graphql_body(&requests[2]));
    }
}
