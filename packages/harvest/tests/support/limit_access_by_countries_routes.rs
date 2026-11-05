// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tenant log's entry of the countries a tenant's rules block: the
//! lists before and after. Cloudflare holds the rules, so the entry is
//! queued once they are set, and a change Cloudflare did not make leaves
//! none.

use super::*;
use crate::route_services::rows;
use crate::route_services::{json as answer, post, Services};
use crate::services::authorization::test_claims::{
    admin_claims, CALLER_TENANT_ID,
};
use crate::test_claims::Claims;

const USER_ID: &str = "7a3e9c1b-2d4f-4b6a-9e8c-1f3a5c7e9b2d";

fn input(voting: &[&str], enroll: &[&str]) -> LimitAccessByCountriesInput {
    serde_json::from_value(json!({
        "voting_countries": voting,
        "enroll_countries": enroll,
    }))
    .unwrap()
}

fn tenant(settings: Option<Value>) -> Tenant {
    Tenant {
        id: CALLER_TENANT_ID.to_string(),
        slug: "acme".to_string(),
        created_at: None,
        updated_at: None,
        labels: None,
        annotations: None,
        is_active: true,
        voting_channels: None,
        settings,
        test: None,
    }
}

#[test]
fn the_entry_has_the_country_lists_before_and_after() {
    let claims = admin_claims(CALLER_TENANT_ID, &["cloudflare-write"]);
    let current = tenant(Some(json!({
        "voting_countries": ["ES"],
        "enroll_countries": null,
        "logs_timezone": "Europe/Madrid",
    })));
    let entry = countries_change(
        &claims,
        Some(&current),
        &input(&["ES", "FR"], &["PH"]),
    )
    .unwrap();

    assert_eq!(entry.target.tenant_id.to_string(), CALLER_TENANT_ID);
    assert_eq!(entry.target.election_event_id, None);
    assert_eq!(entry.action, AdminAction::Update);
    assert_eq!(
        entry.details().unwrap()["changes"],
        json!([
            {"path": "enroll_countries", "old": [], "new": ["PH"]},
            {"path": "voting_countries", "old": ["ES"], "new": ["ES", "FR"]},
        ])
    );
    assert_eq!(
        entry.describe(),
        "Updated tenant \"acme\": enroll_countries, voting_countries"
    );
}

#[test]
fn rules_set_again_unchanged_are_an_entry_without_changes() {
    let claims = admin_claims(CALLER_TENANT_ID, &["cloudflare-write"]);
    // A tenant without settings, or without the tenant row, has empty lists.
    for current in [Some(tenant(None)), None] {
        let entry =
            countries_change(&claims, current.as_ref(), &input(&[], &[]))
                .unwrap();
        assert!(entry.changes.is_empty(), "{:?}", entry.changes);
        assert_eq!(
            entry.object.name.as_deref(),
            current.as_ref().map(|_| "acme")
        );
    }
}

#[test]
fn an_entry_needs_the_callers_tenant_to_be_a_uuid() {
    let claims = admin_claims("acme", &["cloudflare-write"]);
    let (status, _) = countries_change(&claims, None, &input(&[], &[]))
        .map(|_| ())
        .unwrap_err();
    assert_eq!(status, Status::BadRequest);
}

async fn limit<'c>(
    client: &'c rocket::local::asynchronous::Client,
    tenant_id: &str,
) -> (Status, Value) {
    let claims =
        Claims::new(tenant_id, USER_ID).roles([Permissions::CLOUDFLARE_WRITE]);
    answer(
        post(
            client,
            "/limit-access-by-countries",
            &claims,
            &json!({"voting_countries": ["FR"], "enroll_countries": []}),
        )
        .await,
    )
    .await
}

/// Without Cloudflare's settings no rule is set, and nothing is logged.
#[rocket::async_test]
async fn rules_cloudflare_did_not_set_leave_no_entry() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, refusal) = limit(&client, &event.tenant_id).await;
    assert_eq!(status, Status::InternalServerError, "{refusal}");
    assert!(
        refusal["message"].as_str().unwrap().contains("cloudflare"),
        "{refusal}"
    );
    let logged = rows::query(
        &services.hasura,
        "SELECT count(*) FROM sequent_backend.audit_log_outbox
         WHERE tenant_id = $1::text::uuid AND statement_kind = 'AdminChange'",
        &[&event.tenant_id],
    )
    .await;
    assert_eq!(logged[0].get::<_, i64>(0), 0);
}

/// The current lists are read before Cloudflare is asked for anything.
#[rocket::async_test]
async fn rules_are_not_set_when_the_current_ones_cannot_be_read() {
    let services = Services::without_database();
    let client = services.client().await;
    let (status, refusal) = limit(&client, CALLER_TENANT_ID).await;
    assert_eq!(status, Status::InternalServerError);
    assert_eq!(refusal["extensions"]["code"], "InternalServerError");
    let message = refusal["message"].as_str().unwrap();
    assert!(!message.to_lowercase().contains("cloudflare"), "{message}");
}
