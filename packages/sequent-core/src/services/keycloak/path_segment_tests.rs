// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{
    validate_keycloak_path_segment, validate_keycloak_scope,
    KeycloakAdminClient, PubKeycloakAdmin, RoleAction,
};
use keycloak::types::GroupRepresentation;
use keycloak::{KeycloakAdmin, KeycloakAdminToken, KeycloakError};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const RECORDER_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
    Content-Type: application/json\r\n\
    Content-Length: 2\r\n\
    Connection: close\r\n\r\n{}";

/// Record the request line of one connection, then answer it. A connection
/// that closes before sending a request line records nothing.
fn serve(
    stream: TcpStream,
    requests: &Mutex<Vec<String>>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }
    requests
        .lock()
        .unwrap()
        .push(request_line.trim_end().to_string());
    let mut length = 0;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header == "\r\n" {
            break;
        }
        if let Some(value) =
            header.to_ascii_lowercase().strip_prefix("content-length:")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    std::io::copy(&mut reader.by_ref().take(length), &mut std::io::sink())?;
    reader.get_mut().write_all(RECORDER_RESPONSE)
}

struct Recorder {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Recorder {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let captured = Arc::clone(&requests);
        let stopped = Arc::clone(&stop);
        let worker = thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let _ = serve(stream, &captured);
                    }
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock =>
                    {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(error) => panic!("HTTP recorder: {error}"),
                }
            }
        });
        Self {
            url,
            requests,
            stop,
            worker: Some(worker),
        }
    }

    fn client(&self) -> KeycloakAdminClient {
        let token: KeycloakAdminToken =
            serde_json::from_value(serde_json::json!({
                "access_token": "synthetic-test-token", "expires_in": 300,
                "scope": "openid", "token_type": "Bearer"
            }))
            .unwrap();
        KeycloakAdminClient {
            client: KeycloakAdmin::new(
                &self.url,
                token,
                reqwest::Client::new(),
            ),
        }
    }

    fn public_client(&self) -> PubKeycloakAdmin {
        PubKeycloakAdmin {
            url: self.url.clone(),
            client: reqwest::Client::new(),
            token_supplier: serde_json::from_value(serde_json::json!({
                "access_token": "synthetic-test-token", "expires_in": 300,
                "scope": "openid", "token_type": "Bearer"
            }))
            .unwrap(),
        }
    }

    fn assert_no_requests(&self) {
        assert!(
            self.requests.lock().unwrap().is_empty(),
            "invalid segment reached Keycloak"
        );
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn is_bad_segment(error: Option<&KeycloakError>) -> bool {
    matches!(error, Some(KeycloakError::HttpFailure { status: 400, .. }))
}

fn assert_bad_segment<T: std::fmt::Debug>(result: anyhow::Result<T>) {
    let error = result.expect_err(
        "invalid segment must be rejected before sending a request",
    );
    assert!(
        is_bad_segment(error.downcast_ref::<KeycloakError>()),
        "wrong error: {error:?}"
    );
}

#[rocket::async_test]
async fn user_path_segments_reject_traversal_before_requests() {
    let recorder = Recorder::new();
    for id in [
        "../../other-realm",
        "%2e%2e",
        "..",
        "user?query=x",
        "user#fragment",
        "user\\other",
        ".\n.",
        "",
    ] {
        assert_bad_segment(
            recorder.client().delete_user("tenant-valid", id).await,
        );
        assert_bad_segment(
            recorder.client().get_user("tenant-valid", id).await,
        );
        assert_bad_segment(
            recorder
                .client()
                .edit_user(
                    "tenant-valid",
                    id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await,
        );
        assert_bad_segment(
            recorder.client().get_user_groups("tenant-valid", id).await,
        );
    }
    assert_bad_segment(
        recorder
            .client()
            .delete_user("../../other-realm", "valid-user")
            .await,
    );
    recorder.assert_no_requests();
}

#[rocket::async_test]
async fn group_path_segments_reject_traversal_before_requests() {
    let recorder = Recorder::new();
    let bad = "../../other-realm";
    assert_bad_segment(
        recorder.client().list_user_roles("tenant-valid", bad).await,
    );
    assert_bad_segment(
        recorder
            .client()
            .set_user_role("tenant-valid", "valid-user", bad)
            .await,
    );
    assert_bad_segment(
        recorder
            .client()
            .set_user_role("tenant-valid", bad, "valid-group")
            .await,
    );
    assert_bad_segment(
        recorder
            .client()
            .delete_user_role("tenant-valid", "valid-user", bad)
            .await,
    );
    assert_bad_segment(
        recorder.client().delete_role("tenant-valid", bad).await,
    );
    recorder.assert_no_requests();
}

#[rocket::async_test]
async fn permission_path_segments_reject_traversal_before_requests() {
    let recorder = Recorder::new();
    let bad = "../../other-realm";
    assert_bad_segment(
        recorder
            .client()
            .set_role_permission("tenant-valid", bad, "user-read")
            .await,
    );
    assert_bad_segment(
        recorder
            .client()
            .delete_role_permission("tenant-valid", bad, "user-read")
            .await,
    );
    assert_bad_segment(
        recorder
            .client()
            .set_role_permission("tenant-valid", "valid-group", bad)
            .await,
    );
    assert_bad_segment(
        recorder
            .client()
            .set_role_permissions(
                "tenant-valid",
                "valid-group",
                &vec!["user-read".to_string(), bad.to_string()],
            )
            .await,
    );
    assert_bad_segment(
        recorder
            .client()
            .delete_permission("tenant-valid", bad)
            .await,
    );
    recorder.assert_no_requests();
}

#[rocket::async_test]
async fn valid_path_segments_preserve_native_keycloak_requests() {
    let recorder = Recorder::new();
    recorder
        .client()
        .delete_user("tenant-valid", "opaque-user_id-42")
        .await
        .unwrap();
    recorder
        .client()
        .delete_role("tenant-valid", "opaque-group_id-42")
        .await
        .unwrap();
    recorder
        .client()
        .delete_permission("tenant-valid", "Custom permission")
        .await
        .unwrap();
    assert_eq!(
        *recorder.requests.lock().unwrap(),
        vec![
        "DELETE /admin/realms/tenant-valid/users/opaque-user_id-42 HTTP/1.1",
        "DELETE /admin/realms/tenant-valid/groups/opaque-group_id-42 HTTP/1.1",
        "DELETE /admin/realms/tenant-valid/roles/Custom%20permission HTTP/1.1"
    ]
    );
}

#[rocket::async_test]
async fn realm_and_client_path_segments_reject_controls_before_requests() {
    let recorder = Recorder::new();
    let public = recorder.public_client();
    assert_bad_segment(
        recorder
            .client()
            .realm_delete(
                &public,
                "valid-tenant",
                "clients",
                "../../other-realm",
            )
            .await
            .map_err(Into::into),
    );
    assert_bad_segment(
        recorder
            .client()
            .realm_delete(&public, "../other-realm", "groups", "valid-group")
            .await
            .map_err(Into::into),
    );
    assert_bad_segment(
        recorder
            .client()
            .get_realm(&public, "../../other-realm")
            .await
            .map_err(Into::into),
    );
    assert_bad_segment(
        recorder
            .client()
            .get_flow_executions(&public, "tenant-valid", "../executions")
            .await
            .map_err(Into::into),
    );
    assert_bad_segment(
        recorder
            .client()
            .get_user_profile_attributes("../other-realm")
            .await,
    );
    recorder.assert_no_requests();
}

#[rocket::async_test]
async fn realm_helpers_reject_each_invalid_segment_before_requests() {
    let recorder = Recorder::new();
    let public = recorder.public_client();
    let bad = "../../other-realm";
    assert_bad_segment(
        recorder
            .client()
            .get_flow_executions(&public, bad, "valid-flow")
            .await
            .map_err(Into::into),
    );
    for (board, execution) in [(bad, "valid-flow"), ("valid-realm", bad)] {
        assert_bad_segment(
            recorder
                .client()
                .upsert_flow_execution(&public, board, execution, "{}")
                .await,
        );
    }
    assert_bad_segment(
        recorder
            .client()
            .partial_import_realm_with_cleanup(
                &public,
                bad,
                "valid-container",
                vec![],
                vec![],
                "SKIP",
            )
            .await,
    );
    assert_bad_segment(
        recorder
            .client()
            .realm_delete(&public, "valid-tenant", bad, "valid-id")
            .await
            .map_err(Into::into),
    );
    assert_bad_segment(
        recorder
            .client()
            .create_new_group(bad, "valid-group", &public)
            .await
            .map_err(Into::into),
    );
    for (tenant, group_id) in [(bad, "valid-group"), ("valid-tenant", bad)] {
        assert_bad_segment(
            recorder
                .client()
                .add_roles_to_group(
                    tenant,
                    &public,
                    group_id,
                    &vec![],
                    RoleAction::Add,
                )
                .await
                .map_err(Into::into),
        );
        let error = recorder
            .client()
            .get_group_assigned_roles(tenant, group_id, &public)
            .await
            .expect_err("invalid segment must be rejected");
        assert!(
            is_bad_segment(error.downcast_ref::<KeycloakError>()),
            "wrong error: {error:?}"
        );
        let group = GroupRepresentation {
            id: Some(group_id.to_string()),
            ..Default::default()
        };
        assert_bad_segment(
            recorder.client().update_group(tenant, &group).await,
        );
    }
    assert_bad_segment(
        recorder
            .client()
            .update_localization_texts_from_import(None, &public, bad)
            .await,
    );
    // A valid locale beside an invalid one must not be sent either.
    let locales = HashMap::from([
        ("en".to_string(), HashMap::new()),
        (bad.to_string(), HashMap::new()),
    ]);
    assert_bad_segment(
        recorder
            .client()
            .update_localization_texts_from_import(
                Some(locales),
                &public,
                "valid-tenant",
            )
            .await,
    );
    assert_bad_segment(
        recorder.client().delete_permission(bad, "user-read").await,
    );
    recorder.assert_no_requests();
}

#[test]
fn path_segments_reject_url_normalization_and_preserve_opaque_ids() {
    for invalid in [
        "", ".", "..", "%2e%2e", "%252e", "one/two", "one\\two", "one?two",
        "one#two", "\t..", "..\r", ".\n.", " one", "one ", "one\0two",
    ] {
        assert!(
            validate_keycloak_path_segment(invalid).is_err(),
            "accepted {invalid:?}"
        );
    }
    for valid in [
        "master",
        "tenant-123-event-456",
        "service-account-admin",
        "opaque_id-42",
        "user.read",
        "équipe",
        "Custom permission",
    ] {
        assert!(
            validate_keycloak_path_segment(valid).is_ok(),
            "rejected {valid:?}"
        );
    }
}

#[test]
fn path_segments_require_uuid_tenant_and_event_scope() {
    let tenant = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    let event = "086b8442-dbf1-4916-bcda-4ff67a2f85fb";
    assert!(validate_keycloak_scope(tenant, None).is_ok());
    assert!(validate_keycloak_scope(tenant, Some(event)).is_ok());
    for invalid in ["", "tenant-name", "../other", "%2e%2e", "master?query=x"] {
        assert!(validate_keycloak_scope(invalid, None).is_err());
        assert!(validate_keycloak_scope(tenant, Some(invalid)).is_err());
    }
}
