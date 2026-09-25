// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The election event dashboard statistics, counted on the migrated test
//! database and its Keycloak tables.

use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, text, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use sequent_core::types::permissions::Permissions;
use serde_json::json;

fn dashboard(event: &Event) -> Claims {
    Claims::new(&event.tenant_id, "dashboard-viewer")
        .roles([Permissions::ADMIN_DASHBOARD_VIEW])
}

/// The days around today, when the fixture's votes are cast.
fn stats_request(event: &Event) -> serde_json::Value {
    let today = chrono::Utc::now().date_naive();
    json!({
        "election_event_id": event.election_event_id,
        "start_date": (today - chrono::Days::new(2)).to_string(),
        "end_date": (today + chrono::Days::new(2)).to_string(),
        "user_timezone": "UTC",
    })
}

#[rocket::async_test]
async fn the_dashboard_counts_the_events_voters_elections_and_areas() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let election_id = event.election(&services.hasura).await;
    event.election(&services.hasura).await;
    let area_id = event.area(&services.hasura, "North").await;
    event.area(&services.hasura, "South").await;
    event.area(&services.hasura, "East").await;
    // Unlimited revotes: one voter votes twice, and counts once.
    rows::execute(
        &services.hasura,
        "UPDATE sequent_backend.election SET num_allowed_revotes = 0 WHERE id = $1",
        &[&uuid::Uuid::parse_str(&election_id).unwrap()],
    )
    .await;
    for voter in ["voter-1", "voter-1", "voter-2"] {
        event
            .cast_vote(&services.hasura, &election_id, &area_id, voter)
            .await;
    }
    let realm_id =
        rows::keycloak_realm(&services.keycloak, &event.realm()).await;
    rows::keycloak_user(&services.keycloak, &realm_id, "voter-1", true).await;
    rows::keycloak_user(&services.keycloak, &realm_id, "voter-2", true).await;
    rows::keycloak_user(&services.keycloak, &realm_id, "voter-3", false).await;

    let (status, body) = json(
        post(
            &client,
            "/election-event/stats",
            &dashboard(&event),
            &stats_request(&event),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["total_eligible_voters"], 2);
    assert_eq!(body["total_distinct_voters"], 2);
    assert_eq!(body["total_elections"], 2);
    assert_eq!(body["total_areas"], 3);
    assert_eq!(
        body["voters_by_channel"],
        json!([{"channel": "ONLINE", "count": 2}])
    );
    let votes: i64 = body["votes_per_day"]
        .as_array()
        .unwrap()
        .iter()
        .map(|day| day["day_count"].as_i64().unwrap_or_default())
        .sum();
    assert_eq!(votes, 3, "{body}");
}

#[rocket::async_test]
async fn the_top_voting_addresses_of_an_event_without_votes_are_empty() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;

    let (status, body) = json(
        post(
            &client,
            "/election-event/top-votes-by-ip",
            &dashboard(&event),
            &json!({"election_event_id": event.election_event_id}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(
        body,
        json!({"items": [], "total": {"aggregate": {"count": 0}}})
    );
}

#[rocket::async_test]
async fn dashboard_routes_answer_500_when_the_database_is_unreachable() {
    let services = Services::without_database();
    let client = services.client().await;
    let event = Event {
        tenant_id: uuid::Uuid::new_v4().to_string(),
        election_event_id: uuid::Uuid::new_v4().to_string(),
    };
    for (path, body) in [
        ("/election-event/stats", stats_request(&event)),
        (
            "/election-event/top-votes-by-ip",
            json!({"election_event_id": event.election_event_id}),
        ),
    ] {
        let (status, message) =
            text(post(&client, path, &dashboard(&event), &body).await).await;
        assert_eq!(status, Status::InternalServerError);
        assert!(
            message.starts_with("Error loading hasura db client: "),
            "{path}: {message}"
        );
    }
}
