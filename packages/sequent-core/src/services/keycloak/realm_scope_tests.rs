// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{get_tenant_realm, KeycloakAdminClient};
use crate::types::keycloak::Permission;
use keycloak::{KeycloakAdmin, KeycloakAdminToken, KeycloakError};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

const TENANT_ID: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
const GROUP_ID: &str = "7b5d27d5-9eb6-4261-8265-6d5fd48e1cfb";

struct RequestRecorder {
    url: String,
    request_lines: Arc<Mutex<Vec<String>>>,
}

impl RequestRecorder {
    fn start() -> Self {
        let listener =
            TcpListener::bind("127.0.0.1:0").expect("bind request recorder");
        let url = format!(
            "http://{}",
            listener.local_addr().expect("request recorder address")
        );
        let request_lines = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&request_lines);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    break;
                };
                let mut buffer = [0; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]);
                if let Some(line) = request.lines().next() {
                    recorded
                        .lock()
                        .expect("request recorder lock")
                        .push(line.to_string());
                }
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                      Content-Length: 2\r\nConnection: close\r\n\r\n{}",
                );
            }
        });
        Self { url, request_lines }
    }

    fn client(&self) -> KeycloakAdminClient {
        let token: KeycloakAdminToken =
            serde_json::from_value(serde_json::json!({
                "access_token": "test-token",
                "expires_in": 300,
                "scope": "openid",
                "token_type": "Bearer"
            }))
            .expect("test token");
        KeycloakAdminClient {
            client: KeycloakAdmin::new(
                &self.url,
                token,
                reqwest::Client::new(),
            ),
        }
    }

    fn request_lines(&self) -> Vec<String> {
        self.request_lines
            .lock()
            .expect("request recorder lock")
            .clone()
    }
}

fn assert_rejected_without_request<T: std::fmt::Debug>(
    recorder: &RequestRecorder,
    result: anyhow::Result<T>,
) {
    assert_eq!(
        recorder.request_lines(),
        Vec::<String>::new(),
        "the request must not reach Keycloak"
    );
    let error = result.expect_err("the request must be rejected");
    assert!(
        matches!(
            error.downcast_ref::<KeycloakError>(),
            Some(KeycloakError::HttpFailure { status: 400, .. })
        ),
        "unexpected error: {error:?}"
    );
}

#[rocket::async_test]
async fn delete_permission_rejects_names_that_are_not_one_path_segment() {
    let realm = get_tenant_realm(TENANT_ID);
    for permission_name in [
        "../../other-realm",
        "..",
        "%2e%2e",
        ".\t.",
        "..\\..\\other-realm",
        "user-read?x=",
        "user-read#x",
        "",
    ] {
        let recorder = RequestRecorder::start();
        let result = recorder
            .client()
            .delete_permission(&realm, permission_name)
            .await;
        assert_rejected_without_request(&recorder, result);
    }

    let recorder = RequestRecorder::start();
    let result = recorder
        .client()
        .delete_permission(&format!("{realm}/../other-realm"), "user-read")
        .await;
    assert_rejected_without_request(&recorder, result);
}

#[rocket::async_test]
async fn role_permission_updates_reject_ids_and_names_that_are_not_one_path_segment(
) {
    let realm = get_tenant_realm(TENANT_ID);
    let other_realm = format!("{realm}/../other-realm");
    for (realm, role_id, permission_name) in [
        (
            realm.as_str(),
            "../../other-realm/groups/other-group",
            "user-read",
        ),
        (
            realm.as_str(),
            GROUP_ID,
            "../../other-realm/roles/user-write",
        ),
        (other_realm.as_str(), GROUP_ID, "user-read"),
    ] {
        let recorder = RequestRecorder::start();
        let result = recorder
            .client()
            .set_role_permission(realm, role_id, permission_name)
            .await;
        assert_rejected_without_request(&recorder, result);

        let recorder = RequestRecorder::start();
        let result = recorder
            .client()
            .delete_role_permission(realm, role_id, permission_name)
            .await;
        assert_rejected_without_request(&recorder, result);

        let recorder = RequestRecorder::start();
        let result = recorder
            .client()
            .set_role_permissions(
                realm,
                role_id,
                &vec!["user-read".to_string(), permission_name.to_string()],
            )
            .await;
        assert_rejected_without_request(&recorder, result);
    }
}

#[rocket::async_test]
async fn permission_listing_and_creation_reject_realms_that_are_not_one_path_segment(
) {
    let realm = format!("{}/../other-realm", get_tenant_realm(TENANT_ID));
    let permission = Permission {
        id: None,
        attributes: None,
        container_id: None,
        description: None,
        name: Some("custom-permission".to_string()),
    };

    let recorder = RequestRecorder::start();
    let result = recorder
        .client()
        .list_permissions(&realm, None, None, None)
        .await;
    assert_rejected_without_request(&recorder, result);

    let recorder = RequestRecorder::start();
    let result = recorder
        .client()
        .create_permission(&realm, &permission)
        .await;
    assert_rejected_without_request(&recorder, result);
}

#[rocket::async_test]
async fn valid_permission_requests_keep_their_paths() {
    let realm = get_tenant_realm(TENANT_ID);
    let recorder = RequestRecorder::start();

    recorder
        .client()
        .delete_permission(&realm, "user-read")
        .await
        .expect("delete permission");

    assert_eq!(
        recorder.request_lines(),
        vec![format!(
            "DELETE /admin/realms/tenant-{TENANT_ID}/roles/user-read HTTP/1.1"
        )]
    );
}

#[rocket::async_test]
async fn valid_role_permission_requests_keep_their_paths() {
    let realm = get_tenant_realm(TENANT_ID);
    let recorder = RequestRecorder::start();

    recorder
        .client()
        .set_role_permission(&realm, GROUP_ID, "custom permission")
        .await
        .expect("set role permission");
    recorder
        .client()
        .delete_role_permission(&realm, GROUP_ID, "user-read")
        .await
        .expect("delete role permission");
    recorder
        .client()
        .set_role_permissions(&realm, GROUP_ID, &vec!["user-read".to_string()])
        .await
        .expect("set role permissions");

    assert_eq!(
        recorder.request_lines(),
        vec![
            format!(
                "GET /admin/realms/tenant-{TENANT_ID}/roles/custom%20permission HTTP/1.1"
            ),
            format!(
                "POST /admin/realms/tenant-{TENANT_ID}/groups/{GROUP_ID}/role-mappings/realm HTTP/1.1"
            ),
            format!(
                "GET /admin/realms/tenant-{TENANT_ID}/roles/user-read HTTP/1.1"
            ),
            format!(
                "DELETE /admin/realms/tenant-{TENANT_ID}/groups/{GROUP_ID}/role-mappings/realm HTTP/1.1"
            ),
            format!(
                "GET /admin/realms/tenant-{TENANT_ID}/roles/user-read HTTP/1.1"
            ),
            format!(
                "POST /admin/realms/tenant-{TENANT_ID}/groups/{GROUP_ID}/role-mappings/realm HTTP/1.1"
            ),
        ]
    );
}
