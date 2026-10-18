// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Opening, pausing and closing an event or a Post needs its database before
//! anything changes, including naming the admin the change is logged for.

use crate::route_services::{json, post, Services};
use crate::test_claims::Claims;
use chrono::Utc;
use rocket::http::Status;
use sequent_core::types::permissions::Permissions;
use serde_json::json;

const USER_ID: &str = "5c0a9f3e-2b7d-4e18-9a6c-1d3f8e4b7a20";

/// Changing a status needs a recent sign-in at the gold level.
fn operator(tenant_id: &str) -> Claims {
    Claims::new(tenant_id, USER_ID)
        .roles(vec![Permissions::ELECTION_STATE_WRITE])
        .acr(&Permissions::GOLD.to_string())
        .auth_time(Utc::now().timestamp())
}

#[rocket::async_test]
async fn an_unreachable_database_changes_no_status() {
    let services = Services::without_database();
    let client = services.client().await;
    let tenant_id = uuid::Uuid::new_v4().to_string();
    let election_event_id = uuid::Uuid::new_v4().to_string();
    let claims = operator(&tenant_id);

    let (status, error) = json(
        post(
            &client,
            "/update-event-voting-status",
            &claims,
            &json!({
                "election_event_id": election_event_id,
                "voting_status": "OPEN",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError, "{error}");
    // The cause is logged, not answered.
    assert_eq!(
        error["message"], "Could not update voting status.",
        "{error}"
    );

    let (status, error) = json(
        post(
            &client,
            "/update-election-voting-status",
            &claims,
            &json!({
                "election_event_id": election_event_id,
                "election_id": uuid::Uuid::new_v4().to_string(),
                "voting_status": "PAUSED",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::InternalServerError, "{error}");
    // The cause is logged, not answered.
    assert_eq!(
        error["message"], "Could not update voting status.",
        "{error}"
    );
    assert!(services.electoral_log.entries().is_empty());
}
