// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The approval matrix on the migrated test database: versions are added,
//! never changed, and each is committed only with its electoral-log entry.

use crate::adapters::memory::electoral_log::{
    LoggedEntry, MemoryElectoralLogs,
};
use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use rocket::local::asynchronous::{Client, LocalResponse};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};

const USER_ID: &str = "3f2b8c1e-6d4a-4c1b-9e7f-2a5d8b0c9e1a";

fn admin(event: &Event, permissions: &[Permissions]) -> Claims {
    Claims::new(&event.tenant_id, USER_ID).roles(permissions.to_vec())
}

fn writer(event: &Event) -> Claims {
    admin(
        event,
        &[
            Permissions::APPLICATION_READ,
            Permissions::APPROVAL_MATRIX_WRITE,
        ],
    )
}

fn reader(event: &Event) -> Claims {
    admin(event, &[Permissions::APPLICATION_READ])
}

/// The association board election's matrix.
fn association() -> Value {
    json!({
        "compared_fields": ["firstName", "lastName", "dateOfBirth"],
        "rules": [
            {"when": {"already_enrolled": true}, "then": {"decision": "REJECTED", "reason": "ALREADY_APPROVED"}},
            {"when": {"identity": "MANUAL_ENTRY"}, "then": {"decision": "PENDING", "reason": "IDENTITY_NOT_VERIFIED"}},
            {"when": {"differing": "none"}, "then": {"decision": "ACCEPTED"}}
        ],
        "otherwise": {"decision": "PENDING", "reason": "NO_VOTER"}
    })
}

async fn get<'c>(
    client: &'c Client,
    event: &Event,
    claims: &Claims,
) -> LocalResponse<'c> {
    post(
        client,
        "/get-approval-matrix",
        claims,
        &json!({"election_event_id": event.election_event_id}),
    )
    .await
}

async fn save<'c>(
    client: &'c Client,
    event: &Event,
    claims: &Claims,
    matrix: Value,
) -> LocalResponse<'c> {
    post(
        client,
        "/save-approval-matrix",
        claims,
        &json!({"election_event_id": event.election_event_id, "matrix": matrix}),
    )
    .await
}

async fn evaluate<'c>(
    client: &'c Client,
    event: &Event,
    matrix: Value,
    enrollment: Value,
) -> LocalResponse<'c> {
    post(
        client,
        "/evaluate-approval-matrix",
        &reader(event),
        &json!({
            "election_event_id": event.election_event_id,
            "matrix": matrix,
            "enrollment": enrollment,
        }),
    )
    .await
}

async fn stored_versions(services: &Services, event: &Event) -> Vec<i32> {
    rows::query(
        &services.hasura,
        "SELECT version FROM sequent_backend.approval_matrix
         WHERE election_event_id = $1 ORDER BY version",
        &[&uuid::Uuid::parse_str(&event.election_event_id).unwrap()],
    )
    .await
    .iter()
    .map(|row| row.get(0))
    .collect()
}

#[rocket::async_test]
async fn an_event_without_a_saved_matrix_shows_the_built_in_version() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, matrix) =
        json(get(&client, &event, &reader(&event)).await).await;

    assert_eq!(status, Status::Ok, "{matrix}");
    assert_eq!(matrix["version"], 1);
    assert_eq!(matrix["source"], "BUILT_IN");
    assert_eq!(matrix["next_version"], 2);
    assert_eq!(matrix["created_at"], Value::Null);
    assert_eq!(matrix["matrix"]["rules"].as_array().unwrap().len(), 7);
    assert_eq!(
        matrix["matrix"]["otherwise"],
        json!({"decision": "REJECTED", "reason": "NO_VOTER"})
    );
    assert!(matrix["valid_ids"]
        .as_array()
        .unwrap()
        .contains(&json!("philippinePassport")));
}

#[rocket::async_test]
async fn each_save_adds_a_version_with_its_electoral_log_entry() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (_, built_in) = json(get(&client, &event, &reader(&event)).await).await;

    let (status, second) = json(
        save(&client, &event, &writer(&event), built_in["matrix"].clone())
            .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{second}");
    assert_eq!(second["version"], 2);
    assert_eq!(second["source"], "SAVED");
    assert_eq!(second["next_version"], 3);
    assert_eq!(second["matrix"], built_in["matrix"]);

    let (status, third) =
        json(save(&client, &event, &writer(&event), association()).await).await;
    assert_eq!(status, Status::Ok, "{third}");
    assert_eq!(third["version"], 3);

    assert_eq!(stored_versions(&services, &event).await, vec![2, 3]);
    let (_, current) = json(get(&client, &event, &reader(&event)).await).await;
    assert_eq!(current["version"], 3);
    assert_eq!(current["matrix"], association());
    assert_eq!(current["sha256"], third["sha256"]);
    assert_eq!(
        services.electoral_log.entries(),
        vec![
            LoggedEntry::ApprovalMatrixUpdated {
                election_event_id: event.election_event_id.clone(),
                admin_id: USER_ID.into(),
                version: 2,
                sha256: second["sha256"].as_str().unwrap().into(),
            },
            LoggedEntry::ApprovalMatrixUpdated {
                election_event_id: event.election_event_id.clone(),
                admin_id: USER_ID.into(),
                version: 3,
                sha256: third["sha256"].as_str().unwrap().into(),
            },
        ]
    );
}

#[rocket::async_test]
async fn a_saved_version_cannot_be_changed() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    save(&client, &event, &writer(&event), association()).await;

    let pool = services.hasura.clone();
    let connection = pool.get().await.unwrap();
    let error = connection
        .execute(
            "UPDATE sequent_backend.approval_matrix SET rules = '[]'
             WHERE election_event_id = $1",
            &[&uuid::Uuid::parse_str(&event.election_event_id).unwrap()],
        )
        .await
        .unwrap_err();

    assert!(
        error.to_string().contains("immutable")
            || format!("{error:?}").contains("immutable"),
        "{error:?}"
    );
}

#[rocket::async_test]
async fn only_users_with_the_permission_save() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, _) =
        json(save(&client, &event, &reader(&event), association()).await).await;

    assert_eq!(status, Status::Unauthorized);
    assert!(stored_versions(&services, &event).await.is_empty());
    assert!(services.electoral_log.entries().is_empty());
}

#[rocket::async_test]
async fn a_matrix_that_breaks_an_invariant_is_not_saved() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    for (change, code) in [
        (
            json!({"rules": [{"when": {"identity": "MANUAL_ENTRY"}, "then": {"decision": "ACCEPTED"}}]}),
            "rule 1: ACCEPTS_MANUAL_ENTRY",
        ),
        (
            json!({"rules": [{"when": {"already_enrolled": true}, "then": {"decision": "ACCEPTED"}}]}),
            "rule 1: ACCEPTS_ALREADY_ENROLLED",
        ),
        (
            json!({"otherwise": {"decision": "ACCEPTED"}}),
            "OTHERWISE_ACCEPTS",
        ),
        (
            json!({"otherwise": {"decision": "REJECTED"}}),
            "MISSING_REASON",
        ),
    ] {
        let mut matrix = association();
        for (key, value) in change.as_object().unwrap() {
            matrix[key] = value.clone();
        }

        let (status, error) =
            json(save(&client, &event, &writer(&event), matrix).await).await;

        assert_eq!(status, Status::BadRequest, "{error}");
        assert_eq!(error["extensions"]["code"], "InvalidApprovalMatrix");
        assert!(error["message"].as_str().unwrap().contains(code), "{error}");
    }
    assert!(stored_versions(&services, &event).await.is_empty());
    assert!(services.electoral_log.entries().is_empty());
}

#[rocket::async_test]
async fn unknown_keys_are_refused() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let mut matrix = association();
    matrix["rules"][0]["when"]["nationality"] = json!("PH");

    let response = save(&client, &event, &writer(&event), matrix).await;

    assert_eq!(response.status(), Status::UnprocessableEntity);
    assert!(stored_versions(&services, &event).await.is_empty());
}

#[rocket::async_test]
async fn a_version_whose_electoral_log_entry_fails_is_not_saved() {
    let services = Services::on_test_database()
        .await
        .with_electoral_log(MemoryElectoralLogs::refusing());
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, error) =
        json(save(&client, &event, &writer(&event), association()).await).await;

    assert_eq!(status, Status::InternalServerError);
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .starts_with("Failed to post the electoral log message"),
        "{error}"
    );
    assert!(stored_versions(&services, &event).await.is_empty());
}

#[rocket::async_test]
async fn the_test_panel_decides_with_unsaved_rules() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let enrollment = |fields: Value| json!({"identity": "VERIFIED", "voter_found": true, "already_enrolled": false, "valid_id": null, "fields": fields});
    let all_match = json!({"firstName": "MATCHES", "lastName": "MATCHES", "dateOfBirth": "MATCHES"});

    let (status, decision) = json(
        evaluate(
            &client,
            &event,
            association(),
            enrollment(all_match.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{decision}");
    assert_eq!(
        decision,
        json!({"rule": 3, "decision": "ACCEPTED", "reason": null, "invariant": null, "errors": []})
    );

    let (_, decision) = json(
        evaluate(
            &client,
            &event,
            association(),
            enrollment(json!({"firstName": "DIFFERS", "lastName": "MATCHES", "dateOfBirth": "MATCHES"})),
        )
        .await,
    )
    .await;
    assert_eq!(
        decision,
        json!({"rule": null, "decision": "PENDING", "reason": "NO_VOTER", "invariant": null, "errors": []})
    );

    let mut manual = enrollment(all_match);
    manual["identity"] = json!("MANUAL_ENTRY");
    let (_, decision) =
        json(evaluate(&client, &event, association(), manual).await).await;
    assert_eq!(decision["rule"], 2);
    assert_eq!(decision["decision"], "PENDING");
    assert_eq!(decision["reason"], "IDENTITY_NOT_VERIFIED");

    let (_, decision) = json(
        evaluate(
            &client,
            &event,
            association(),
            json!({"identity": null, "voter_found": false, "valid_id": null}),
        )
        .await,
    )
    .await;
    assert_eq!(decision["rule"], Value::Null);
    assert_eq!(decision["decision"], "PENDING");

    assert!(stored_versions(&services, &event).await.is_empty());
}

#[rocket::async_test]
async fn the_test_panel_reports_what_keeps_the_rules_from_being_saved() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let mut matrix = association();
    matrix["rules"][1]["then"] = json!({"decision": "ACCEPTED"});

    let (status, decision) = json(
        evaluate(
            &client,
            &event,
            matrix,
            json!({"identity": "MANUAL_ENTRY", "voter_found": true, "valid_id": null}),
        )
        .await,
    )
    .await;

    assert_eq!(status, Status::Ok, "{decision}");
    assert_eq!(decision["decision"], Value::Null);
    assert_eq!(
        decision["errors"],
        json!([{"code": "ACCEPTS_MANUAL_ENTRY", "rule": 2}])
    );
}
