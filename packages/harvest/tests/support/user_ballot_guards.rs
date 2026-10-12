// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Deleting or editing a voter is refused while the change would leave a
//! valid or in-progress ballot out of the tally.

use super::*;
use crate::adapters::memory::database::FixedDatabasePools;
use crate::adapters::memory::identity::LocalIdentityAdmin;
use crate::route_services::{http, rows, Services};
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use serde_json::json;

// Ballots record their voter as the Keycloak user id.
const VOTER: &str = "3f2b8c1e-6d4a-4c1b-9e7f-2a5d8b0c9e1a";

fn databases(services: &Services) -> FixedDatabasePools {
    FixedDatabasePools {
        hasura: services.hasura.clone(),
        keycloak: services.keycloak.clone(),
    }
}

fn voter(area: &str) -> User {
    User {
        id: Some(VOTER.into()),
        attributes: Some(HashMap::from([(
            AREA_ID_ATTR_NAME.to_string(),
            vec![area.to_string()],
        )])),
        email: None,
        email_verified: None,
        enabled: Some(true),
        first_name: None,
        last_name: None,
        username: Some("voter".into()),
        area: None,
        votes_info: None,
    }
}

fn area_edit(area: &str) -> HashMap<String, Vec<String>> {
    HashMap::from([(AREA_ID_ATTR_NAME.to_string(), vec![area.to_string()])])
}

async fn ballot(services: &Services, event: &rows::Event, status: &str) {
    let election = event.election(&services.hasura).await;
    let area = event.area(&services.hasura, "area").await;
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.cast_vote
            (id, tenant_id, election_event_id, election_id, area_id,
             voter_id_string, status)
         VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6)",
        &[
            &uuid::Uuid::parse_str(&event.tenant_id).unwrap(),
            &uuid::Uuid::parse_str(&event.election_event_id).unwrap(),
            &uuid::Uuid::parse_str(&election).unwrap(),
            &uuid::Uuid::parse_str(&area).unwrap(),
            &VOTER,
            &status,
        ],
    )
    .await;
}

async fn delete_guard(
    databases: &FixedDatabasePools,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(), (Status, String)> {
    ensure_voter_has_no_active_ballot(
        databases,
        tenant_id,
        election_event_id,
        VOTER,
    )
    .await
}

async fn edit_guard(
    services: &Services,
    databases: &FixedDatabasePools,
    event: &rows::Event,
    area: &str,
) -> Result<(), (Status, String)> {
    let mut client = services.hasura.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    ensure_census_edit_keeps_ballots(
        databases,
        &transaction,
        &event.tenant_id,
        &event.election_event_id,
        &voter("old-area"),
        None,
        &area_edit(area),
    )
    .await
}

#[tokio::test]
async fn a_voter_without_ballots_can_be_deleted() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;

    assert_eq!(
        delete_guard(
            &databases(&services),
            &event.tenant_id,
            &event.election_event_id
        )
        .await,
        Ok(())
    );
}

#[tokio::test]
async fn a_voter_with_a_valid_or_in_progress_ballot_cannot_be_deleted() {
    for status in ["valid", "in-progress"] {
        let services = Services::on_test_database().await;
        let event = rows::event(&services.hasura).await;
        ballot(&services, &event, status).await;

        let (code, message) = delete_guard(
            &databases(&services),
            &event.tenant_id,
            &event.election_event_id,
        )
        .await
        .unwrap_err();
        assert_eq!(code, Status::Conflict, "{status}");
        assert_eq!(message, "Cannot delete a voter who has cast a ballot");
    }
}

#[tokio::test]
async fn a_voter_whose_ballots_are_all_resolved_can_be_deleted() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    ballot(&services, &event, "discarded").await;

    assert_eq!(
        delete_guard(
            &databases(&services),
            &event.tenant_id,
            &event.election_event_id
        )
        .await,
        Ok(())
    );
}

#[tokio::test]
async fn deleting_with_malformed_ids_or_an_unreachable_database_is_an_error() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let pools = databases(&services);
    let (code, _) =
        delete_guard(&pools, "not-a-uuid", &event.election_event_id)
            .await
            .unwrap_err();
    assert_eq!(code, Status::BadRequest);
    let (code, _) = delete_guard(&pools, &event.tenant_id, "not-a-uuid")
        .await
        .unwrap_err();
    assert_eq!(code, Status::BadRequest);

    let down = Services::without_database();
    let (code, message) = delete_guard(
        &databases(&down),
        &event.tenant_id,
        &event.election_event_id,
    )
    .await
    .unwrap_err();
    assert_eq!(code, Status::InternalServerError);
    assert!(
        message.starts_with("Error checking the voter's ballots: "),
        "{message}"
    );
}

#[tokio::test]
async fn changing_a_census_attribute_of_a_voter_with_a_ballot_is_refused() {
    for status in ["valid", "in-progress"] {
        let services = Services::on_test_database().await;
        let event = rows::event(&services.hasura).await;
        ballot(&services, &event, status).await;

        let (code, message) =
            edit_guard(&services, &databases(&services), &event, "new-area")
                .await
                .unwrap_err();
        assert_eq!(code, Status::Conflict, "{status}");
        assert_eq!(
            message,
            format!(
                "Cannot change {AREA_ID_ATTR_NAME} of a voter who has cast a ballot"
            )
        );
    }
}

#[tokio::test]
async fn edits_that_keep_the_census_or_touch_a_voter_without_ballots_pass() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let pools = databases(&services);

    assert_eq!(
        edit_guard(&services, &pools, &event, "new-area").await,
        Ok(())
    );
    ballot(&services, &event, "valid").await;
    assert_eq!(
        edit_guard(&services, &pools, &event, "old-area").await,
        Ok(())
    );
}

#[tokio::test]
async fn a_failed_census_check_is_an_internal_error() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let down = Services::without_database();

    let (code, message) =
        edit_guard(&services, &databases(&down), &event, "new-area")
            .await
            .unwrap_err();
    assert_eq!(code, Status::InternalServerError);
    assert!(
        message.starts_with("Error checking the voter's ballots: "),
        "{message}"
    );
}

#[tokio::test]
async fn the_voter_is_loaded_from_the_event_realm() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let peer = http::HttpServer::start(vec![http::Exchange::json(
        "GET",
        &format!("/admin/realms/{}/users/{VOTER}", event.realm()),
        200,
        json!({"id": VOTER, "username": "voter", "enabled": true}),
    )]);
    let identity = LocalIdentityAdmin {
        url: Some(peer.url.clone()),
        ..Default::default()
    };

    let user = load_voter(
        &identity,
        &event.tenant_id,
        &event.election_event_id,
        VOTER,
    )
    .await
    .unwrap();
    assert_eq!(user.username.as_deref(), Some("voter"));
    assert!(load_voter(
        &LocalIdentityAdmin::default(),
        &event.tenant_id,
        &event.election_event_id,
        VOTER
    )
    .await
    .is_err());
}
