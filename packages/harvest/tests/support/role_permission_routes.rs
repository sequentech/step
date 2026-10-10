// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Changing a role's permissions in Users and Roles: who may change which
//! permission, and that a sign permission's change is logged in the
//! election events with a rule for its action, only when it is a change and
//! only once Keycloak has made it.

use crate::adapters::memory::identity::LocalIdentityAdmin;
use crate::route_services::http::{Exchange, HttpServer};
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::signing::SigningAction;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};

const EDITOR_ID: &str = "role-editor";
const EDITOR: &str = "role.editor";
const ROLE_ID: &str = "group-1";

fn editor(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, EDITOR_ID)
        .username(EDITOR)
        .roles([Permissions::USER_PERMISSION_WRITE, Permissions::ROLE_WRITE])
}

/// The event gets a saved rule for `action`.
async fn rule(services: &Services, event: &Event, action: SigningAction) {
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.signing_rule
            (tenant_id, election_event_id, action, requirement, signatures,
             requester_signing, revision, updated_by)
         VALUES ($1::text::uuid, $2::text::uuid, $3, 'required', 2,
                 'not-allowed', 1, 'configuration-manager')",
        &[
            &event.tenant_id,
            &event.election_event_id,
            &action.to_string(),
        ],
    )
    .await;
}

/// The event's unposted signing log entries: (event type, user, description).
async fn logged(
    services: &Services,
    event: &Event,
) -> Vec<(String, Option<String>, Value)> {
    rows::query(
        &services.hasura,
        "SELECT event_type, username, body FROM sequent_backend.signing_log_outbox
         WHERE election_event_id = $1::text::uuid AND statement_kind = 'SigningPermissionChanged'
         ORDER BY id",
        &[&event.election_event_id],
    )
    .await
    .iter()
    .map(|row| (row.get(0), row.get(1), row.get::<_, Value>(2)["description"].clone()))
    .collect()
}

fn keycloak(peer: &HttpServer) -> LocalIdentityAdmin {
    LocalIdentityAdmin {
        url: Some(peer.url.clone()),
        ..Default::default()
    }
}

fn group(event: &Event, name: &str) -> Exchange {
    Exchange::json(
        "GET",
        &format!(
            "/admin/realms/{}/groups/{ROLE_ID}",
            get_tenant_realm(&event.tenant_id)
        ),
        200,
        json!({"id": ROLE_ID, "name": name}),
    )
}

fn permission(event: &Event, name: &str) -> Exchange {
    Exchange::json(
        "GET",
        &format!(
            "/admin/realms/{}/roles/{name}",
            get_tenant_realm(&event.tenant_id)
        ),
        200,
        json!({"id": format!("role-{name}"), "name": name}),
    )
}

fn mapping(event: &Event, method: &'static str, status: u16) -> Exchange {
    Exchange::json(
        method,
        &format!(
            "/admin/realms/{}/groups/{ROLE_ID}/role-mappings/realm",
            get_tenant_realm(&event.tenant_id)
        ),
        status,
        Value::Null,
    )
}

/// What the role holds, read under the events' locks.
fn held(event: &Event, permissions: &[&str]) -> Exchange {
    let roles: Vec<Value> = permissions
        .iter()
        .map(|name| json!({"name": name}))
        .collect();
    Exchange::json(
        "GET",
        &format!(
            "/admin/realms/{}/groups/{ROLE_ID}/role-mappings/realm",
            get_tenant_realm(&event.tenant_id)
        ),
        200,
        Value::Array(roles),
    )
}

/// The permissions each logged entry says allowed the change.
async fn allowed_by(services: &Services, event: &Event) -> Vec<Value> {
    rows::query(
        &services.hasura,
        "SELECT body FROM sequent_backend.signing_log_outbox
         WHERE election_event_id = $1::text::uuid ORDER BY id",
        &[&event.election_event_id],
    )
    .await
    .iter()
    .map(|row| row.get::<_, Value>(0)["details"]["allowed_by"].clone())
    .collect()
}

fn body(event: &Event, permission: &str) -> Value {
    json!({
        "tenant_id": event.tenant_id,
        "role_id": ROLE_ID,
        "permission_name": permission,
    })
}

/// Two configurations: the change, the action, the role and the route
/// differ, and the entries name them.
#[rocket::async_test]
async fn a_sign_permission_change_is_logged_once_keycloak_makes_it() {
    let cases = [
        (
            "/set-role-permission",
            "POST",
            SigningAction::CloseVoting,
            "sbei",
            false,
            "Role sbei updated: sign-close-voting added",
        ),
        (
            "/delete-role-permission",
            "DELETE",
            SigningAction::ApproveConfiguration,
            "Security Officer",
            true,
            "Role Security Officer updated: sign-approve-configuration removed",
        ),
    ];
    for (path, method, action, role, holds, description) in cases {
        let base = Services::on_test_database().await;
        let event = rows::event(&base.hasura).await;
        rule(&base, &event, action).await;
        let name = action.sign_permission().to_string();
        let holding: &[&str] = if holds { &[name.as_str()] } else { &[] };
        let peer = HttpServer::start(vec![
            held(&event, holding),
            group(&event, role),
            permission(&event, &name),
            mapping(&event, method, 204),
        ]);
        let services = base.with_identity(keycloak(&peer));
        let client = services.client().await;

        let (status, answer) = json(
            post(&client, path, &editor(&event), &body(&event, &name)).await,
        )
        .await;
        assert_eq!(status, Status::Ok, "{path}: {answer}");
        peer.finish();
        assert_eq!(
            logged(&services, &event).await,
            vec![
                ("USER".into(), Some(EDITOR.into()), json!(description)),
                ("SYSTEM".into(), None, json!(description)),
            ],
            "{path}"
        );
    }
}

/// Keycloak refuses the change: nothing is logged.
#[rocket::async_test]
async fn a_sign_permission_change_keycloak_refuses_is_not_logged() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    rule(&base, &event, SigningAction::OpenVoting).await;
    let name = Permissions::SIGN_OPEN_VOTING.to_string();
    let peer = HttpServer::start(vec![
        held(&event, &[]),
        group(&event, "sbei"),
        permission(&event, &name),
        mapping(&event, "POST", 403),
    ]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;

    let (status, _) = text(
        post(
            &client,
            "/set-role-permission",
            &editor(&event),
            &body(&event, &name),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    peer.finish();
    assert_eq!(logged(&services, &event).await, vec![]);
}

/// Other permissions, the tab's own included, change as before: no role
/// lookup and no log entry.
#[rocket::async_test]
async fn other_permission_changes_are_not_signing_log_entries() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    rule(&base, &event, SigningAction::CloseVoting).await;
    let name = Permissions::SIGNING_RULES_WRITE.to_string();
    let peer = HttpServer::start(vec![
        permission(&event, &name),
        mapping(&event, "POST", 204),
    ]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;

    let (status, answer) = json(
        post(
            &client,
            "/set-role-permission",
            &editor(&event),
            &body(&event, &name),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{answer}");
    peer.finish();
    assert_eq!(logged(&services, &event).await, vec![]);
}

fn realm_path(event: &Event, rest: &str) -> String {
    format!("/admin/realms/{}{rest}", get_tenant_realm(&event.tenant_id))
}

/// A role created with sign permissions logs each one, attributed to its
/// creator and to `role-create`, in the events with a rule for it.
#[rocket::async_test]
async fn a_role_created_with_sign_permissions_is_logged() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    rule(&base, &event, SigningAction::CloseVoting).await;
    let sign = Permissions::SIGN_CLOSE_VOTING.to_string();
    let peer = HttpServer::start(vec![
        Exchange::json("POST", &realm_path(&event, "/groups"), 201, json!({})),
        Exchange::json(
            "GET",
            &realm_path(&event, "/groups"),
            200,
            json!([{"id": ROLE_ID, "name": "Post signers"}]),
        ),
        permission(&event, &sign),
        permission(&event, "role-read"),
        mapping(&event, "POST", 204),
    ]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;
    let creator = Claims::new(&event.tenant_id, EDITOR_ID)
        .username(EDITOR)
        .roles([Permissions::ROLE_CREATE]);

    let (status, answer) = json(
        post(
            &client,
            "/create-role",
            &creator,
            &json!({"tenant_id": event.tenant_id, "role": {
                "name": "Post signers", "permissions": [sign, "role-read"],
            }}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{answer}");
    peer.finish();
    let description = "Role Post signers updated: sign-close-voting added";
    assert_eq!(
        logged(&services, &event).await,
        vec![
            ("USER".into(), Some(EDITOR.into()), json!(description)),
            ("SYSTEM".into(), None, json!(description)),
        ]
    );
}

/// Deleting a role that holds sign permissions logs their removal; a role
/// without any is deleted as before.
#[rocket::async_test]
async fn deleting_a_role_logs_the_sign_permissions_it_held() {
    for (holding, expected) in [
        (
            &["sign-approve-voter", "role-read"][..],
            vec!["Role OFOV updated: sign-approve-voter removed"],
        ),
        (&["role-read"][..], vec![]),
    ] {
        let base = Services::on_test_database().await;
        let event = rows::event(&base.hasura).await;
        rule(&base, &event, SigningAction::ApproveVoter).await;
        let mut exchanges = vec![held(&event, holding)];
        if !expected.is_empty() {
            // Read again under the events' locks.
            exchanges.push(held(&event, holding));
            exchanges.push(group(&event, "OFOV"));
        }
        exchanges.push(Exchange::json(
            "DELETE",
            &realm_path(&event, &format!("/groups/{ROLE_ID}")),
            204,
            Value::Null,
        ));
        let peer = HttpServer::start(exchanges);
        let services = base.with_identity(keycloak(&peer));
        let client = services.client().await;
        let deleter = Claims::new(&event.tenant_id, EDITOR_ID)
            .username(EDITOR)
            .roles([Permissions::ROLE_WRITE]);

        let (status, answer) = json(
            post(
                &client,
                "/delete-role",
                &deleter,
                &json!({"tenant_id": event.tenant_id, "role_id": ROLE_ID}),
            )
            .await,
        )
        .await;
        assert_eq!(status, Status::Ok, "{answer}");
        peer.finish();
        let descriptions: Vec<Value> = logged(&services, &event)
            .await
            .into_iter()
            .map(|(_, _, description)| description)
            .collect();
        let expected: Vec<Value> = expected
            .iter()
            .flat_map(|description| [json!(description), json!(description)])
            .collect();
        assert_eq!(descriptions, expected);
    }
}

/// Keycloak refuses to delete the role: nothing is logged.
#[rocket::async_test]
async fn a_role_deletion_keycloak_refuses_is_not_logged() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    rule(&base, &event, SigningAction::ApproveVoter).await;
    let peer = HttpServer::start(vec![
        held(&event, &["sign-approve-voter"]),
        held(&event, &["sign-approve-voter"]),
        group(&event, "OFOV"),
        Exchange::json(
            "DELETE",
            &realm_path(&event, &format!("/groups/{ROLE_ID}")),
            403,
            json!({"error": "forbidden"}),
        ),
    ]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;
    let deleter = Claims::new(&event.tenant_id, EDITOR_ID)
        .roles([Permissions::ROLE_WRITE]);

    let (status, _) = text(
        post(
            &client,
            "/delete-role",
            &deleter,
            &json!({"tenant_id": event.tenant_id, "role_id": ROLE_ID}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError);
    peer.finish();
    assert_eq!(logged(&services, &event).await, vec![]);
}

/// Setting a permission the role already holds, or removing one it doesn't,
/// changes nothing, so nothing is logged.
#[rocket::async_test]
async fn a_change_that_changes_nothing_is_not_logged() {
    for (path, method, holding) in [
        ("/set-role-permission", "POST", true),
        ("/delete-role-permission", "DELETE", false),
    ] {
        let base = Services::on_test_database().await;
        let event = rows::event(&base.hasura).await;
        rule(&base, &event, SigningAction::OpenVoting).await;
        let name = Permissions::SIGN_OPEN_VOTING.to_string();
        let holds: &[&str] = if holding { &[name.as_str()] } else { &[] };
        let peer = HttpServer::start(vec![
            held(&event, holds),
            permission(&event, &name),
            mapping(&event, method, 204),
        ]);
        let services = base.with_identity(keycloak(&peer));
        let client = services.client().await;

        let (status, answer) = json(
            post(&client, path, &editor(&event), &body(&event, &name)).await,
        )
        .await;
        assert_eq!(status, Status::Ok, "{path}: {answer}");
        peer.finish();
        assert_eq!(logged(&services, &event).await, vec![], "{path}");
    }
}

/// Whoever administers who signs (`role-write` without
/// `user-permission-write`) turns sign permissions on and off, logged as
/// allowed by `role-write`, and nothing else.
#[rocket::async_test]
async fn role_write_alone_changes_only_sign_permissions() {
    let base = Services::on_test_database().await;
    let event = rows::event(&base.hasura).await;
    rule(&base, &event, SigningAction::CloseVoting).await;
    let name = Permissions::SIGN_CLOSE_VOTING.to_string();
    let peer = HttpServer::start(vec![
        held(&event, &[]),
        group(&event, "sbei"),
        permission(&event, &name),
        mapping(&event, "POST", 204),
    ]);
    let services = base.with_identity(keycloak(&peer));
    let client = services.client().await;
    let security_officer = Claims::new(&event.tenant_id, EDITOR_ID)
        .username(EDITOR)
        .roles([Permissions::ROLE_WRITE, Permissions::USER_PERMISSION_READ]);

    let (status, answer) = json(
        post(
            &client,
            "/set-role-permission",
            &security_officer,
            &body(&event, &name),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{answer}");
    peer.finish();
    assert_eq!(
        allowed_by(&services, &event).await,
        vec![json!(["role-write"]), json!(["role-write"])]
    );
    // Any other permission, the Signatures tab's own included, needs
    // user-permission-write too; the request never reaches Keycloak.
    for path in ["/set-role-permission", "/delete-role-permission"] {
        for other in [
            Permissions::ROLE_WRITE,
            Permissions::USER_PERMISSION_WRITE,
            Permissions::SIGNING_RULES_WRITE,
        ] {
            let (status, _) = text(
                post(
                    &client,
                    path,
                    &security_officer,
                    &body(&event, &other.to_string()),
                )
                .await,
            )
            .await;
            assert_eq!(status, Status::Unauthorized, "{path} {other}");
        }
    }
}

#[rocket::async_test]
async fn role_changes_without_their_permissions_are_refused() {
    let services = Services::without_database();
    let client = services.client().await;
    let event = Event {
        tenant_id: "00000000-0000-4000-8000-00000000000a".into(),
        election_event_id: "00000000-0000-4000-8000-00000000000e".into(),
    };
    let sign = Permissions::SIGN_CLOSE_VOTING.to_string();
    let reader = Claims::new(&event.tenant_id, EDITOR_ID)
        .roles([Permissions::ROLE_READ, Permissions::USER_PERMISSION_READ]);
    for path in ["/set-role-permission", "/delete-role-permission"] {
        for permission in [sign.as_str(), "role-read"] {
            let (status, _) = text(
                post(&client, path, &reader, &body(&event, permission)).await,
            )
            .await;
            assert_eq!(status, Status::Unauthorized, "{path} {permission}");
        }
    }
    let (status, _) = text(
        post(
            &client,
            "/create-role",
            &Claims::new(&event.tenant_id, EDITOR_ID)
                .roles([Permissions::ROLE_WRITE]),
            &json!({"tenant_id": event.tenant_id, "role": {
                "name": "Post signers", "permissions": [sign],
            }}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Unauthorized);
}

/// The platform's own permissions can't be deleted, even with
/// `user-permission-write`; Keycloak is never asked.
#[rocket::async_test]
async fn a_built_in_permission_cannot_be_deleted() {
    let services = Services::without_database();
    let client = services.client().await;
    let tenant_id = "00000000-0000-4000-8000-00000000000a";
    let administrator = Claims::new(tenant_id, EDITOR_ID)
        .roles([Permissions::USER_PERMISSION_WRITE, Permissions::ROLE_WRITE]);
    for built_in in [
        Permissions::SIGN_CLOSE_VOTING,
        Permissions::SIGNING_RULES_READ,
        Permissions::ELECTION_EVENT_SIGNATURES_TAB,
        Permissions::ROLE_WRITE,
    ] {
        let (status, message) = text(
            post(
                &client,
                "/delete-permission",
                &administrator,
                &json!({"tenant_id": tenant_id, "permission_name": built_in.to_string()}),
            )
            .await,
        )
        .await;
        assert_eq!(status, Status::Forbidden, "{built_in}");
        assert_eq!(
            message,
            format!("{built_in} is a built-in permission and can't be deleted")
        );
    }
}

const RESERVED_NAMES: [&str; 5] = [
    "admin",
    "service-account",
    "datafix-account",
    "super-admin-user",
    "cli-account-admin",
];

fn tenant_event() -> Event {
    Event {
        tenant_id: "00000000-0000-4000-8000-00000000000a".into(),
        election_event_id: "00000000-0000-4000-8000-00000000000e".into(),
    }
}

/// A permission named after a role the platform keeps for itself can't be
/// created, assigned or set on a new role; Keycloak is never asked.
#[rocket::async_test]
async fn reserved_permission_names_are_refused_before_keycloak() {
    let event = tenant_event();
    for name in RESERVED_NAMES {
        let peer = HttpServer::start(vec![]);
        let services =
            Services::without_database().with_identity(keycloak(&peer));
        let client = services.client().await;
        let cases = [
            (
                "/create-permission",
                vec![Permissions::USER_PERMISSION_CREATE],
                json!({"tenant_id": event.tenant_id, "permission": {"name": name}}),
            ),
            (
                "/set-role-permission",
                vec![
                    Permissions::USER_PERMISSION_WRITE,
                    Permissions::ROLE_WRITE,
                ],
                body(&event, name),
            ),
            (
                "/create-role",
                vec![Permissions::ROLE_CREATE],
                json!({"tenant_id": event.tenant_id, "role": {
                    "name": "Clerks", "permissions": ["role-read", name],
                }}),
            ),
        ];
        for (path, roles, request) in cases {
            let caller = Claims::new(&event.tenant_id, EDITOR_ID).roles(roles);

            let (status, message) =
                text(post(&client, path, &caller, &request).await).await;

            assert_eq!(status, Status::BadRequest, "{path} {name}");
            assert!(message.contains(name), "{path}: {message}");
        }
        assert!(peer.finish().is_empty(), "{name} reached Keycloak");
    }
}

/// Other names are still assigned to a role and set on a new one.
#[rocket::async_test]
async fn ordinary_permission_names_are_still_accepted() {
    let event = tenant_event();
    let peer = HttpServer::start(vec![
        permission(&event, "election-event-read"),
        mapping(&event, "POST", 204),
        Exchange::json("POST", &realm_path(&event, "/groups"), 201, json!({})),
        Exchange::json(
            "GET",
            &realm_path(&event, "/groups"),
            200,
            json!([{"id": ROLE_ID, "name": "Clerks"}]),
        ),
        permission(&event, "role-read"),
        mapping(&event, "POST", 204),
    ]);
    let services = Services::without_database().with_identity(keycloak(&peer));
    let client = services.client().await;
    let cases = [
        (
            "/set-role-permission",
            vec![Permissions::USER_PERMISSION_WRITE, Permissions::ROLE_WRITE],
            body(&event, "election-event-read"),
        ),
        (
            "/create-role",
            vec![Permissions::ROLE_CREATE],
            json!({"tenant_id": event.tenant_id, "role": {
                "name": "Clerks", "permissions": ["role-read"],
            }}),
        ),
    ];
    for (path, roles, request) in cases {
        let caller = Claims::new(&event.tenant_id, EDITOR_ID).roles(roles);

        let (status, answer) =
            json(post(&client, path, &caller, &request).await).await;

        assert_eq!(status, Status::Ok, "{path}: {answer}");
    }
    peer.finish();
}

/// A reserved permission a role already holds can still be taken off it.
#[rocket::async_test]
async fn a_reserved_permission_can_still_be_removed_from_a_role() {
    let event = tenant_event();
    let peer = HttpServer::start(vec![
        permission(&event, "service-account"),
        mapping(&event, "DELETE", 204),
    ]);
    let services = Services::without_database().with_identity(keycloak(&peer));
    let client = services.client().await;
    let caller = Claims::new(&event.tenant_id, EDITOR_ID)
        .roles([Permissions::USER_PERMISSION_WRITE, Permissions::ROLE_WRITE]);

    let (status, answer) = json(
        post(
            &client,
            "/delete-role-permission",
            &caller,
            &body(&event, "service-account"),
        )
        .await,
    )
    .await;

    assert_eq!(status, Status::Ok, "{answer}");
    peer.finish();
}
