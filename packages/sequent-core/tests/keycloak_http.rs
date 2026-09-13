// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Contract tests over real loopback HTTP: verify scope, request bodies and
//! failure propagation without depending on a running identity provider.

#![cfg(feature = "keycloak")]

#[path = "support/http.rs"]
mod http;

use http::{Exchange, HttpServer};
use keycloak::types::{GroupRepresentation, RoleRepresentation};
use sequent_core::services::keycloak::RoleAction;
use sequent_core::types::keycloak::{Permission, Role};
use serde_json::{json, Value};

const REALM: &str = "tenant-north";
const GROUPS: &str = "/admin/realms/tenant-north/groups";
const ROLES: &str = "/admin/realms/tenant-north/roles";
const USER: &str = "/admin/realms/tenant-north/users/voter-1";
const MAPPINGS: &str =
    "/admin/realms/tenant-north/groups/group-1/role-mappings/realm";

#[rocket::async_test]
async fn resolved_permissions_do_not_turn_a_rejected_assignment_into_success() {
    for status in [204, 403] {
        let peer = HttpServer::start(vec![
            Exchange::json(
                "GET",
                &format!("{ROLES}/read"),
                200,
                json!({"id": "permission-1", "name": "read"}),
            ),
            Exchange::json(
                "POST",
                MAPPINGS,
                status,
                json!({"error": "write denied"}),
            ),
        ]);
        let result = peer
            .client()
            .set_role_permissions(REALM, "group-1", &vec!["read".into()])
            .await;
        if status == 204 {
            result.unwrap();
        } else {
            assert!(result.unwrap_err().to_string().contains("403"));
        }
        let requests = peer.finish();
        assert_eq!(
            requests[1].json(),
            json!([{"id": "permission-1", "name": "read"}])
        );
    }
}

#[rocket::async_test]
async fn realm_export_distinguishes_rejections_from_malformed_success_bodies() {
    use keycloak::KeycloakError;

    let path = "/admin/realms/tenant-north/partial-export";
    let peer = HttpServer::start(vec![
        Exchange::json("POST", path, 200, json!({"realm": REALM})),
        Exchange::json("POST", path, 403, json!({"error": "export denied"})),
        Exchange::json("POST", path, 502, Value::Null)
            .body("upstream unavailable"),
        Exchange::json("POST", path, 200, Value::Null).body("{truncated"),
    ]);
    let public = peer.public_client();
    assert_eq!(
        peer.client()
            .get_realm(&public, REALM)
            .await
            .unwrap()
            .realm
            .as_deref(),
        Some(REALM)
    );
    for (expected_status, expected_text) in [
        (403, r#"{"error":"export denied"}"#),
        (502, "upstream unavailable"),
    ] {
        match peer.client().get_realm(&public, REALM).await.unwrap_err() {
            KeycloakError::HttpFailure { status, text, .. } => {
                assert_eq!(status, expected_status);
                assert_eq!(text, expected_text);
            }
            error => panic!("expected HTTP rejection, got {error:?}"),
        }
    }
    let malformed = peer.client().get_realm(&public, REALM).await.unwrap_err();
    assert!(
        matches!(malformed, KeycloakError::ReqwestFailure(ref error) if error.is_decode())
    );
    assert_eq!(peer.finish().len(), 4);
}

#[rocket::async_test]
async fn invalid_flow_configuration_is_rejected_before_any_http_request() {
    let path =
        "/admin/realms/tenant-north/authentication/flows/browser/executions";
    let peer =
        HttpServer::start(vec![Exchange::json("PUT", path, 204, Value::Null)]);
    let public = peer.public_client();
    for malformed in ["{", r#"{"id":42}"#, r#"{"requirement":[]}"#] {
        let error = peer
            .client()
            .upsert_flow_execution(&public, REALM, "browser", malformed)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Failed to deserialize execution configuration"
        );
    }
    peer.client()
        .upsert_flow_execution(
            &public,
            REALM,
            "browser",
            r#"{"id":"execution-1","requirement":"REQUIRED"}"#,
        )
        .await
        .unwrap();
    let requests = peer.finish();
    assert_eq!(
        requests.len(),
        1,
        "invalid configurations must not reach Keycloak"
    );
    assert_eq!(requests[0].json()["requirement"], "REQUIRED");
}

#[rocket::async_test]
async fn realm_export_and_flow_updates_report_connection_failures() {
    // Retain ownership of the port but stop listening. This guarantees a local
    // connection failure without racing another test to reuse a freed port.
    let socket = rocket::tokio::net::TcpSocket::new_v4().unwrap();
    socket.bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let peer = HttpServer::start(vec![]);
    let mut public = peer.public_client();
    public.url = format!("http://{}", socket.local_addr().unwrap());
    let export = peer.client().get_realm(&public, REALM).await.unwrap_err();
    assert!(
        matches!(export, keycloak::KeycloakError::ReqwestFailure(ref error) if error.is_connect())
    );
    let update = peer
        .client()
        .upsert_flow_execution(
            &public,
            REALM,
            "browser",
            r#"{"id":"execution-1"}"#,
        )
        .await
        .unwrap_err();
    assert!(update
        .to_string()
        .starts_with("Error sending update request to"));
    assert!(update.chain().any(|cause| cause
        .downcast_ref::<reqwest::Error>()
        .is_some_and(|error| error.is_connect())));
    assert!(peer.finish().is_empty());
}

#[rocket::async_test]
async fn group_creation_without_location_does_not_invent_an_identifier() {
    let peer =
        HttpServer::start(vec![Exchange::json("POST", GROUPS, 201, json!({}))]);
    assert_eq!(
        peer.client()
            .create_new_group("north", "Clerks", &peer.public_client())
            .await
            .unwrap(),
        None
    );
    assert_eq!(peer.finish()[0].json(), json!({"name": "Clerks"}));
}

#[rocket::async_test]
async fn group_creation_rejects_a_non_ascii_location_header() {
    let peer =
        HttpServer::start(vec![Exchange::json("POST", GROUPS, 201, json!({}))
            .header("Location", "https://identity.invalid/groups/é")]);
    let error = peer
        .client()
        .create_new_group("north", "Clerks", &peer.public_client())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        keycloak::KeycloakError::HttpFailure { status: 201, .. }
    ));
    assert_eq!(peer.finish().len(), 1);
}

fn role() -> Role {
    serde_json::from_value(
        json!({"id": "group-1", "name": "Clerks", "permissions": ["read"]}),
    )
    .unwrap()
}

fn permission() -> Permission {
    serde_json::from_value(json!({"id": "permission-1", "name": "read", "description": "Read ballots"})).unwrap()
}

#[rocket::async_test]
async fn role_and_permission_pages_preserve_total_count_and_handle_large_offsets(
) {
    // Clients can request another page after rows have been deleted. Such an
    // offset is a legitimate empty page, and must not panic or wrap arithmetic.
    for (offset, limit, expected) in [
        (None, None, vec!["first", "second"]),
        (Some(1), Some(1), vec!["second"]),
        (Some(2), None, vec![]),
        (Some(3), Some(1), vec![]),
        (Some(usize::MAX), Some(usize::MAX), vec![]),
        (Some(1), Some(usize::MAX), vec!["second"]),
        (Some(0), Some(0), vec![]),
    ] {
        let peer = HttpServer::start(vec![
            Exchange::json(
                "GET",
                GROUPS,
                200,
                json!([{"name": "first"}, {"name": "second"}]),
            ),
            Exchange::json(
                "GET",
                ROLES,
                200,
                json!([{"name": "first"}, {"name": "second"}]),
            ),
        ]);
        let (roles, total) = peer
            .client()
            .list_roles(REALM, Some("clerk & auditor".into()), limit, offset)
            .await
            .unwrap();
        assert_eq!(total, 2);
        assert_eq!(
            roles
                .iter()
                .map(|role| role.name.as_deref().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
        let (permissions, total) = peer
            .client()
            .list_permissions(REALM, None, limit, offset)
            .await
            .unwrap();
        assert_eq!(total, 2);
        assert_eq!(
            permissions
                .iter()
                .map(|permission| permission.name.as_deref().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
        let requests = peer.finish();

        assert_eq!(
            requests[0].headers["authorization"],
            "Bearer synthetic-access-token"
        );
    }
}

#[rocket::async_test]
async fn role_membership_and_permission_writes_use_the_requested_realm_and_identifiers(
) {
    let membership = format!("{USER}/groups/group-1");
    let peer = HttpServer::start(vec![
        Exchange::json("PUT", &membership, 204, Value::Null),
        Exchange::json("DELETE", &membership, 204, Value::Null),
        Exchange::json(
            "GET",
            &format!("{ROLES}/read"),
            200,
            json!({"id": "permission-1", "name": "read"}),
        ),
        Exchange::json("POST", MAPPINGS, 204, Value::Null),
        Exchange::json(
            "GET",
            &format!("{ROLES}/read"),
            200,
            json!({"id": "permission-1", "name": "read"}),
        ),
        Exchange::json("DELETE", MAPPINGS, 204, Value::Null),
    ]);
    peer.client()
        .set_user_role(REALM, "voter-1", "group-1")
        .await
        .unwrap();
    peer.client()
        .delete_user_role(REALM, "voter-1", "group-1")
        .await
        .unwrap();
    peer.client()
        .set_role_permission(REALM, "group-1", "read")
        .await
        .unwrap();
    peer.client()
        .delete_role_permission(REALM, "group-1", "read")
        .await
        .unwrap();
    let requests = peer.finish();
    for request in [&requests[3], &requests[5]] {
        assert_eq!(request.json()[0]["id"], "permission-1");
        assert_eq!(request.json()[0]["name"], "read");
    }
}

#[rocket::async_test]
async fn bulk_permission_assignment_is_complete_or_reports_the_failed_lookup() {
    for status in [200, 404] {
        let mut exchanges = vec![
            Exchange::json(
                "GET",
                &format!("{ROLES}/read"),
                200,
                json!({"name": "read"}),
            ),
            Exchange::json(
                "GET",
                &format!("{ROLES}/write"),
                status,
                json!({"name": "write"}),
            ),
        ];
        if status == 200 {
            exchanges.push(Exchange::json("POST", MAPPINGS, 204, Value::Null));
        }
        let peer = HttpServer::start(exchanges);
        let result = peer
            .client()
            .set_role_permissions(
                REALM,
                "group-1",
                &vec!["read".into(), "write".into()],
            )
            .await;
        let requests = peer.finish();
        if status == 200 {
            result.unwrap();
            let assignment = requests
                .iter()
                .find(|request| request.method == "POST")
                .unwrap()
                .json();
            assert_eq!(assignment.as_array().unwrap().len(), 2);
            assert_eq!(assignment[0]["name"], "read");
            assert_eq!(assignment[1]["name"], "write");
        } else {
            assert!(result.is_err());
            assert!(requests.iter().all(|request| request.method == "GET"),
                "a missing permission must stop the write, not silently assign a subset");
        }
    }
}

#[rocket::async_test]
async fn role_and_permission_creation_keep_attributes_and_do_not_invent_server_ids(
) {
    let peer = HttpServer::start(vec![
        Exchange::json("POST", GROUPS, 201, Value::Null),
        Exchange::json("POST", ROLES, 201, Value::Null),
        Exchange::json(
            "DELETE",
            &format!("{GROUPS}/group-1"),
            204,
            Value::Null,
        ),
        Exchange::json("DELETE", &format!("{ROLES}/read"), 204, Value::Null),
    ]);
    let created = peer.client().create_role(REALM, &role()).await.unwrap();
    assert_eq!(created.name.as_deref(), Some("Clerks"));
    let created = peer
        .client()
        .create_permission(REALM, &permission())
        .await
        .unwrap();
    assert_eq!(created.name.as_deref(), Some("read"));
    peer.client().delete_role(REALM, "group-1").await.unwrap();
    peer.client()
        .delete_permission(REALM, "read")
        .await
        .unwrap();
    let requests = peer.finish();
    assert_eq!(requests[0].json()["realmRoles"], json!(["read"]));
    assert_eq!(requests[1].json()["description"], "Read ballots");
}

#[rocket::async_test]
async fn role_lookup_and_user_membership_preserve_the_server_representation() {
    let peer = HttpServer::start(vec![
        Exchange::json(
            "GET",
            GROUPS,
            200,
            json!([{"id": "server-id", "name": "Clerks", "realmRoles": ["read"]}]),
        ),
        Exchange::json("GET", GROUPS, 200, json!([])),
        Exchange::json(
            "GET",
            &format!("{USER}/groups"),
            200,
            json!([{"id": "group-1", "name": "Clerks"}]),
        ),
        Exchange::json(
            "GET",
            &format!("{USER}/groups"),
            403,
            json!({"error": "read denied"}),
        ),
    ]);
    let found = peer
        .client()
        .get_role_by_name(REALM, &role())
        .await
        .unwrap();
    assert_eq!(found.id.as_deref(), Some("server-id"));
    assert_eq!(found.permissions, Some(vec!["read".into()]));
    assert_eq!(
        peer.client()
            .get_role_by_name(REALM, &role())
            .await
            .unwrap()
            .id,
        role().id
    );
    let roles = peer
        .client()
        .list_user_roles(REALM, "voter-1")
        .await
        .unwrap();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].name.as_deref(), Some("Clerks"));
    assert!(peer
        .client()
        .list_user_roles(REALM, "voter-1")
        .await
        .unwrap_err()
        .to_string()
        .contains("403"));
    peer.finish();
}

#[rocket::async_test]
async fn users_are_filtered_on_the_server_and_edits_preserve_unspecified_attributes(
) {
    let saved = json!({"id": "voter-1", "username": "voter", "enabled": true,
        "email": "old@example.invalid", "firstName": "Old", "lastName": "Name",
        "attributes": {"area-id": ["north"], "locale": ["en"]}});
    let peer = HttpServer::start(vec![
        Exchange::json(
            "GET",
            "/admin/realms/tenant-north/users",
            200,
            json!([saved.clone()]),
        ),
        Exchange::json(
            "GET",
            "/admin/realms/tenant-north/users/count",
            200,
            json!(17),
        ),
        Exchange::json("GET", USER, 200, saved.clone()),
        Exchange::json("PUT", USER, 204, Value::Null),
        Exchange::json("GET", USER, 200, saved),
        Exchange::json("DELETE", USER, 204, Value::Null),
    ]);
    let (users, total) = peer
        .client()
        .list_users(
            "north",
            "event",
            REALM,
            Some("voter".into()),
            Some("old@example.invalid".into()),
            Some(10),
            Some(20),
        )
        .await
        .unwrap();
    assert_eq!(total, 17);
    assert_eq!(users[0].id.as_deref(), Some("voter-1"));
    let edited = peer
        .client()
        .edit_user(
            REALM,
            "voter-1",
            Some(false),
            Some([("locale".into(), vec!["fr".into()])].into()),
            Some("new@example.invalid".into()),
            None,
            None,
            None,
            Some("synthetic-password".into()),
            None,
        )
        .await
        .unwrap();
    assert_eq!(edited.enabled, Some(false));
    assert_eq!(edited.first_name.as_deref(), Some("Old"));
    assert_eq!(
        peer.client()
            .get_user(REALM, "voter-1")
            .await
            .unwrap()
            .username
            .as_deref(),
        Some("voter")
    );
    peer.client().delete_user(REALM, "voter-1").await.unwrap();
    let requests = peer.finish();
    assert_eq!(
        requests[0].query().get("first").map(String::as_str),
        Some("20"),
        "requests: {requests:?}"
    );
    assert_eq!(requests[0].query()["max"], "10");
    assert_eq!(requests[1].query()["search"], "voter");
    let update = requests[3].json();
    assert_eq!(update["attributes"]["area-id"], json!(["north"]));
    assert_eq!(update["attributes"]["locale"], json!(["fr"]));
    assert_eq!(update["credentials"][0]["temporary"], true);
    assert_eq!(update["credentials"][0]["value"], "synthetic-password");
}

#[rocket::async_test]
async fn rejected_role_and_user_operations_are_errors() {
    // Check each public write boundary: a forbidden response is never success.
    let membership = format!("{USER}/groups/group-1");
    let peer = HttpServer::start(vec![
        Exchange::json("GET", GROUPS, 403, json!({})),
        Exchange::json("GET", ROLES, 403, json!({})),
        Exchange::json("PUT", &membership, 403, json!({})),
        Exchange::json("DELETE", &membership, 403, json!({})),
        Exchange::json("DELETE", &format!("{GROUPS}/group-1"), 403, json!({})),
        Exchange::json("DELETE", &format!("{ROLES}/read"), 403, json!({})),
        Exchange::json("POST", GROUPS, 403, json!({})),
        Exchange::json("POST", ROLES, 403, json!({})),
        Exchange::json("GET", USER, 403, json!({})),
        Exchange::json("DELETE", USER, 403, json!({})),
    ]);
    assert!(peer
        .client()
        .list_roles(REALM, None, None, None)
        .await
        .is_err());
    assert!(peer
        .client()
        .list_permissions(REALM, None, None, None)
        .await
        .is_err());
    assert!(peer
        .client()
        .set_user_role(REALM, "voter-1", "group-1")
        .await
        .is_err());
    assert!(peer
        .client()
        .delete_user_role(REALM, "voter-1", "group-1")
        .await
        .is_err());
    assert!(peer.client().delete_role(REALM, "group-1").await.is_err());
    assert!(peer
        .client()
        .delete_permission(REALM, "read")
        .await
        .is_err());
    assert!(peer.client().create_role(REALM, &role()).await.is_err());
    assert!(peer
        .client()
        .create_permission(REALM, &permission())
        .await
        .is_err());
    assert!(peer.client().get_user(REALM, "voter-1").await.is_err());
    assert!(peer.client().delete_user(REALM, "voter-1").await.is_err());
    peer.finish();
}

#[rocket::async_test]
async fn realm_export_flow_import_and_deletion_send_scoped_payloads() {
    let export = "/admin/realms/tenant-north/partial-export";
    let executions =
        "/admin/realms/tenant-north/authentication/flows/browser/executions";
    let peer = HttpServer::start(vec![
        Exchange::json("POST", export, 200, json!({"realm": REALM})),
        Exchange::json(
            "GET",
            executions,
            200,
            json!([{"id": "execution-1", "requirement": "REQUIRED"}]),
        ),
        Exchange::json("PUT", executions, 204, Value::Null),
        Exchange::json(
            "POST",
            "/admin/realms/tenant-north/partialImport",
            200,
            json!({}),
        ),
        Exchange::json(
            "DELETE",
            &format!("{GROUPS}/group-1"),
            204,
            Value::Null,
        ),
    ]);
    let public = peer.public_client();
    assert_eq!(
        peer.client()
            .get_realm(&public, REALM)
            .await
            .unwrap()
            .realm
            .as_deref(),
        Some(REALM)
    );
    let flow = peer
        .client()
        .get_flow_executions(&public, REALM, "browser")
        .await
        .unwrap();
    assert_eq!(flow[0].id.as_deref(), Some("execution-1"));
    peer.client()
        .upsert_flow_execution(
            &public,
            REALM,
            "browser",
            r#"{"id":"execution-1","requirement":"DISABLED"}"#,
        )
        .await
        .unwrap();
    peer.client()
        .partial_import_realm_with_cleanup(
            &public,
            "north",
            "container-1",
            vec![GroupRepresentation::from(role())],
            vec![RoleRepresentation::from(permission())],
            "OVERWRITE",
        )
        .await
        .unwrap();
    peer.client()
        .realm_delete(&public, "north", "groups", "group-1")
        .await
        .unwrap();
    let requests = peer.finish();
    assert_eq!(requests[0].query()["exportClients"], "true");
    assert_eq!(requests[0].query()["exportGroupsAndRoles"], "true");
    assert_eq!(requests[2].json()["requirement"], "DISABLED");
    assert_eq!(requests[3].json()["realm"], REALM);
    assert_eq!(requests[3].json()["ifResourceExists"], "OVERWRITE");
    assert_eq!(requests[3].json()["roles"]["realm"][0]["name"], "read");
}

#[rocket::async_test]
async fn group_creation_and_role_mapping_use_location_and_check_http_status() {
    for status in [201, 403] {
        let location =
            "https://identity.invalid/admin/realms/tenant-north/groups/group-1";
        let peer = HttpServer::start(vec![Exchange::json(
            "POST",
            GROUPS,
            status,
            json!({}),
        )
        .header("Location", location)]);
        let result = peer
            .client()
            .create_new_group("north", "Clerks", &peer.public_client())
            .await;
        assert_eq!(peer.finish()[0].json(), json!({"name": "Clerks"}));
        if status == 201 {
            assert_eq!(result.unwrap().as_deref(), Some("group-1"));
        } else {
            assert!(
                result.is_err(),
                "a Location header cannot turn a rejection into success"
            );
        }
    }
    let peer = HttpServer::start(vec![
        Exchange::json("POST", MAPPINGS, 204, Value::Null),
        Exchange::json("DELETE", MAPPINGS, 204, Value::Null),
        Exchange::json(
            "GET",
            MAPPINGS,
            200,
            json!([{"id": "permission-1", "name": "read"}]),
        ),
    ]);
    for action in [RoleAction::Add, RoleAction::Remove] {
        peer.client()
            .add_roles_to_group(
                "north",
                &peer.public_client(),
                "group-1",
                &vec![permission().into()],
                action,
            )
            .await
            .unwrap();
    }
    let assigned = peer
        .client()
        .get_group_assigned_roles("north", "group-1", &peer.public_client())
        .await
        .unwrap();
    assert_eq!(assigned[0].name.as_deref(), Some("read"));
    let requests = peer.finish();
    assert_eq!(
        requests[0].json(),
        json!([{"id": "permission-1", "name": "read"}])
    );
    assert_eq!(requests[1].json(), requests[0].json());
}

#[rocket::async_test]
async fn rejected_group_reads_and_localization_updates_cannot_report_success() {
    let localization = "/admin/realms/tenant-north/localization/en";
    let peer = HttpServer::start(vec![
        Exchange::json("GET", MAPPINGS, 403, json!([])),
        Exchange::json("POST", localization, 403, json!({})),
    ]);
    let roles = peer
        .client()
        .get_group_assigned_roles("north", "group-1", &peer.public_client())
        .await;
    let messages = Some(
        [("en".into(), [("welcome".into(), "Welcome".into())].into())].into(),
    );
    let update = peer
        .client()
        .update_localization_texts_from_import(
            messages,
            &peer.public_client(),
            "north",
        )
        .await;
    peer.finish();
    assert!(
        roles.is_err(),
        "an error with a JSON array body is still an error"
    );
    assert!(
        update.is_err(),
        "a rejected translation update must reach the caller"
    );
}

#[rocket::async_test]
async fn role_search_filters_names_and_user_counts_use_the_same_search_as_the_page(
) {
    let peer =
        HttpServer::start(vec![Exchange::json("GET", GROUPS, 200, json!([]))]);
    peer.client()
        .list_roles(REALM, Some("clerk & auditor".into()), None, None)
        .await
        .unwrap();
    let request = peer.finish().remove(0);
    assert_eq!(
        request.query().get("search").map(String::as_str),
        Some("clerk & auditor")
    );
    assert!(
        !request.query().contains_key("q"),
        "q searches custom attributes, not group names"
    );
}

#[rocket::async_test]
async fn permission_page_beyond_the_end_is_empty() {
    let peer =
        HttpServer::start(vec![Exchange::json("GET", ROLES, 200, json!([]))]);
    let (page, total) = peer
        .client()
        .list_permissions(REALM, None, Some(usize::MAX), Some(1))
        .await
        .unwrap();
    assert_eq!(total, 0);
    assert!(page.is_empty());
    peer.finish();
}

#[rocket::async_test]
async fn user_profile_and_group_queries_preserve_metadata_and_missing_group_names(
) {
    let profile = json!({"attributes": [{"name": "locale", "permissions": {"edit": ["admin"]}}],
        "groups": [{"name": "contact", "displayHeader": "Contact"}]});
    let path = "/admin/realms/tenant-north/users/profile";
    let peer = HttpServer::start(vec![
        Exchange::json("GET", path, 200, profile.clone()),
        Exchange::json("GET", path, 200, profile),
        Exchange::json(
            "GET",
            &format!("{USER}/groups"),
            200,
            json!([{"id": "group-1", "name": "Clerks"}, {}]),
        ),
        Exchange::json("GET", path, 403, json!({})),
        Exchange::json("GET", &format!("{USER}/groups"), 403, json!({})),
    ]);
    let profile = peer
        .client()
        .get_user_profile_configuration(REALM)
        .await
        .unwrap();
    assert_eq!(profile.attributes[0].name.as_deref(), Some("locale"));
    assert_eq!(profile.groups[0].name.as_deref(), Some("contact"));
    assert_eq!(
        peer.client()
            .get_user_profile_attributes(REALM)
            .await
            .unwrap()
            .len(),
        1
    );
    let groups = peer
        .client()
        .get_user_groups(REALM, "voter-1")
        .await
        .unwrap();
    assert_eq!(groups[0].group_id, "group-1");
    assert_eq!(groups[1].group_id, "Unknown Group ID");
    assert_eq!(groups[1].group_name, "Unknown Group");
    assert!(peer
        .client()
        .get_user_profile_attributes(REALM)
        .await
        .is_err());
    assert!(peer
        .client()
        .get_user_groups(REALM, "voter-1")
        .await
        .is_err());
    peer.finish();
}

#[rocket::async_test]
async fn user_write_rejections_keep_profile_validation_details_for_the_caller()
{
    use sequent_core::services::keycloak::get_user_profile_validation_errors;
    let peer = HttpServer::start(vec![
        Exchange::json("GET", USER, 200, json!({"id": "voter-1"})),
        Exchange::json(
            "PUT",
            USER,
            400,
            json!({"field": "email", "errorMessage": "invalid-email", "params": ["email"]}),
        ),
    ]);
    let result = peer
        .client()
        .edit_user(
            REALM,
            "voter-1",
            None,
            None,
            Some("bad-email".into()),
            Some("First".into()),
            Some("Last".into()),
            Some("username".into()),
            None,
            Some(false),
        )
        .await;
    let errors = get_user_profile_validation_errors(&result.unwrap_err());
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field.as_deref(), Some("email"));
    assert_eq!(errors[0].error_message.as_deref(), Some("invalid-email"));
    let body = peer.finish()[1].json();
    assert_eq!(body["firstName"], "First");
    assert_eq!(body["lastName"], "Last");
    assert_eq!(body["username"], "username");
    assert!(body["credentials"].is_null());
}

#[rocket::async_test]
async fn failed_user_lookups_stop_the_page_or_edit_without_inventing_a_result()
{
    let path = "/admin/realms/tenant-north/users";
    let peer = HttpServer::start(vec![
        Exchange::json("GET", path, 403, json!({})),
        Exchange::json("GET", path, 200, json!([])),
        Exchange::json("GET", &format!("{path}/count"), 403, json!({})),
        Exchange::json("GET", USER, 404, json!({})),
    ]);
    for _ in 0..2 {
        assert!(peer
            .client()
            .list_users("north", "event", REALM, None, None, None, None)
            .await
            .is_err());
    }
    assert!(peer
        .client()
        .edit_user(
            REALM, "voter-1", None, None, None, None, None, None, None, None
        )
        .await
        .is_err());
    assert_eq!(
        peer.finish().len(),
        4,
        "failed reads must not trigger a write"
    );
}

#[rocket::async_test]
async fn absent_permissions_and_rejected_assignments_are_not_reported_as_success(
) {
    let read = format!("{ROLES}/read");
    let peer = HttpServer::start(vec![
        Exchange::json("GET", &read, 404, json!({})),
        Exchange::json("GET", &read, 404, json!({})),
        Exchange::json("GET", &read, 200, json!({"name": "read"})),
        Exchange::json("POST", MAPPINGS, 403, json!({})),
        Exchange::json("GET", &read, 200, json!({"name": "read"})),
        Exchange::json("DELETE", MAPPINGS, 403, json!({})),
    ]);
    assert!(peer
        .client()
        .set_role_permission(REALM, "group-1", "read")
        .await
        .is_err());
    assert!(peer
        .client()
        .delete_role_permission(REALM, "group-1", "read")
        .await
        .is_err());
    assert!(peer
        .client()
        .set_role_permission(REALM, "group-1", "read")
        .await
        .is_err());
    assert!(peer
        .client()
        .delete_role_permission(REALM, "group-1", "read")
        .await
        .is_err());
    peer.finish();
}
