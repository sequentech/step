// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Real request guards and readiness authorization must refuse invalid scope
//! before any readiness snapshot or signed append is attempted.
use crate::route_services::{json, post, rows, Services};
use crate::test_claims::Claims;
use rocket::http::{ContentType, Status};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use uuid::Uuid;

const TENANT: &str = "00000000-0000-4000-8000-000000000001";
const OFFICIAL: &str = "00000000-0000-4000-8000-000000000002";
const TESTED: &str = "00000000-0000-4000-8000-000000000003";

fn request(official: &str, tested: &str) -> Value {
    json!({"official_event_id": official, "tested_event_id": tested})
}

#[tokio::test]
async fn readiness_requires_a_verified_operator_and_valid_distinct_event_ids_before_database_access(
) {
    let services = Services::without_database();
    let client = services.client().await;
    let input = request(OFFICIAL, TESTED);
    let unsigned = client
        .post("/readiness-check")
        .header(ContentType::JSON)
        .body(input.to_string())
        .dispatch()
        .await;
    assert_eq!(unsigned.status(), Status::Unauthorized);

    let reader = Claims::new(TENANT, "observer")
        .roles([Permissions::ELECTION_EVENT_READ]);
    let (status, _) =
        json(post(&client, "/readiness-check", &reader, &input).await).await;
    assert_eq!(status, Status::Unauthorized);

    let operator =
        Claims::new(TENANT, "operator").roles([Permissions::READINESS_CHECK]);
    for input in [request(OFFICIAL, OFFICIAL), request("not-an-id", TESTED)] {
        let (status, _) =
            json(post(&client, "/readiness-check", &operator, &input).await)
                .await;
        assert_eq!(status, Status::BadRequest);
    }
    // A valid operator and request get past both gates and reach the first
    // backend, which this fixture deliberately leaves unavailable.
    let (status, _) =
        json(post(&client, "/readiness-check", &operator, &input).await).await;
    assert_eq!(status, Status::InternalServerError);
}

async fn audit_entries(services: &Services, tenant: &str) -> i64 {
    rows::query(&services.hasura, "SELECT count(*) FROM sequent_backend.audit_log_outbox WHERE tenant_id = $1", &[&Uuid::parse_str(tenant).unwrap()]).await[0].get(0)
}

#[test]
fn readiness_refuses_cross_tenant_and_either_events_missing_permission_labels_without_writes(
) {
    rows::run(async {
        let services = Services::on_test_database().await;
        let official = rows::event(&services.hasura).await;
        let tested =
            rows::event_of(&services.hasura, &official.tenant_id).await;
        let foreign = rows::event(&services.hasura).await;
        official
            .labelled_election(&services.hasura, "official-post")
            .await;
        tested
            .labelled_election(&services.hasura, "tested-post")
            .await;
        let client = services.client().await;
        let operator = Claims::new(&official.tenant_id, "operator")
            .roles([Permissions::READINESS_CHECK]);
        let before = audit_entries(&services, &official.tenant_id).await;
        let body =
            request(&official.election_event_id, &tested.election_event_id);
        for labels in [&["official-post"][..], &["tested-post"][..]] {
            let restricted = Claims::new(&official.tenant_id, "restricted")
                .roles([Permissions::READINESS_CHECK])
                .permission_labels(labels);
            let (status, _) = json(
                post(&client, "/readiness-check", &restricted, &body).await,
            )
            .await;
            assert_eq!(status, Status::Forbidden);
        }
        let full =
            operator.permission_labels(&["official-post", "tested-post"]);
        for body in [
            request(&official.election_event_id, &foreign.election_event_id),
            request(&foreign.election_event_id, &tested.election_event_id),
        ] {
            let (status, _) =
                json(post(&client, "/readiness-check", &full, &body).await)
                    .await;
            assert_eq!(status, Status::NotFound);
        }
        assert_eq!(audit_entries(&services, &official.tenant_id).await, before);
    });
}

#[test]
fn readiness_hasura_action_preserves_authentication_and_passes_only_event_ids()
{
    let text = include_str!("../../../../hasura/metadata/actions.yaml");
    let metadata: serde_yaml::Value = serde_yaml::from_str(text).unwrap();
    let metadata = serde_json::to_value(metadata).unwrap();
    let actions = metadata["actions"].as_array().unwrap();
    let matches: Vec<_> = actions
        .iter()
        .filter(|action| action["name"] == "readiness_check")
        .collect();
    assert_eq!(matches.len(), 1);
    let action = matches[0];
    assert_eq!(action["definition"]["kind"], "synchronous");
    assert_eq!(action["definition"]["type"], "mutation");
    assert_eq!(
        action["definition"]["handler"],
        "http://{{HARVEST_DOMAIN}}/readiness-check"
    );
    assert_eq!(action["definition"]["forward_client_headers"], true);
    assert_eq!(
        action["definition"]["request_transform"]["body"]["template"],
        "{{$body.input}}"
    );
    assert_eq!(action["permissions"], json!([{"role":"readiness-check"}]));
}
