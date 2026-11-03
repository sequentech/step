// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Deleting or editing a voter is refused while the change would leave a
//! valid or in-progress ballot out of the tally.

use super::*;
use crate::adapters::memory::database::FixedDatabasePools;
use crate::adapters::memory::identity::LocalIdentityAdmin;
use crate::route_services::{http, rows, Services};
use electoral_log::adapters::ballot_box::{
    AcceptBallot, AcceptOutcome, BallotStatus,
};
use electoral_log::adapters::postgres::PostgresStore;
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use serde_json::json;
use windmill::services::ballot_box::BallotFormat;

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

/// A ballot of the voter with `status` in the event's ballot box.
async fn ballot(
    services: &Services,
    ballot_box: &PostgresStore,
    event: &rows::Event,
    status: BallotStatus,
) {
    let election = event.election(&services.hasura).await;
    let area = event.area(&services.hasura, "area").await;
    let ballot_id = uuid::Uuid::new_v4().simple().to_string();
    let format = BallotFormat::HashableBallot.to_string();
    let outcome = ballot_box
        .accept_ballot(&AcceptBallot {
            election_event_id: &event.election_event_id,
            election_id: &election,
            area_id: &area,
            voter_id: VOTER,
            ballot_id: &ballot_id,
            format: &format,
            content: "ballot",
            voter_signature: None,
            pseudonym_hash: &[0; 64],
            ballot_hash: &[0; 64],
            voting_channel: "ONLINE",
            status,
            voter_ip: None,
            voter_country: None,
            username: Some("voter"),
            allowed_votes: 0,
        })
        .await
        .expect("accepted ballot");
    assert!(
        matches!(outcome, AcceptOutcome::Accepted { .. }),
        "{outcome:?}"
    );
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

#[test]
fn a_voter_without_ballots_can_be_deleted() {
    rows::run(async {
        let services = Services::on_test_database().await;
        let event = rows::event(&services.hasura).await;
        event.ballot_box(&services.hasura).await;

        assert_eq!(
            delete_guard(
                &databases(&services),
                &event.tenant_id,
                &event.election_event_id
            )
            .await,
            Ok(())
        );
    })
}

#[test]
fn a_voter_with_a_valid_or_in_progress_ballot_cannot_be_deleted() {
    rows::run(async {
        for status in [BallotStatus::Valid, BallotStatus::Pending] {
            let services = Services::on_test_database().await;
            let event = rows::event(&services.hasura).await;
            let ballot_box = event.ballot_box(&services.hasura).await;
            ballot(&services, &ballot_box, &event, status).await;

            let (code, message) = delete_guard(
                &databases(&services),
                &event.tenant_id,
                &event.election_event_id,
            )
            .await
            .unwrap_err();
            assert_eq!(code, Status::Conflict, "{status:?}");
            assert_eq!(message, "Cannot delete a voter who has cast a ballot");
        }
    })
}

#[test]
fn a_voter_whose_ballots_are_all_resolved_can_be_deleted() {
    rows::run(async {
        let services = Services::on_test_database().await;
        let event = rows::event(&services.hasura).await;
        let ballot_box = event.ballot_box(&services.hasura).await;
        ballot(&services, &ballot_box, &event, BallotStatus::Rejected).await;

        assert_eq!(
            delete_guard(
                &databases(&services),
                &event.tenant_id,
                &event.election_event_id
            )
            .await,
            Ok(())
        );
    })
}

#[test]
fn deleting_with_malformed_ids_or_an_unreachable_database_is_an_error() {
    rows::run(async {
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
    })
}

#[test]
fn changing_a_census_attribute_of_a_voter_with_a_ballot_is_refused() {
    rows::run(async {
        for status in [BallotStatus::Valid, BallotStatus::Pending] {
            let services = Services::on_test_database().await;
            let event = rows::event(&services.hasura).await;
            let ballot_box = event.ballot_box(&services.hasura).await;
            ballot(&services, &ballot_box, &event, status).await;

            let (code, message) = edit_guard(
                &services,
                &databases(&services),
                &event,
                "new-area",
            )
            .await
            .unwrap_err();
            assert_eq!(code, Status::Conflict, "{status:?}");
            assert_eq!(
                message,
                format!(
                    "Cannot change {AREA_ID_ATTR_NAME} of a voter who has cast a ballot"
                )
            );
        }
    })
}

#[test]
fn edits_that_keep_the_census_or_touch_a_voter_without_ballots_pass() {
    rows::run(async {
        let services = Services::on_test_database().await;
        let event = rows::event(&services.hasura).await;
        let ballot_box = event.ballot_box(&services.hasura).await;
        let pools = databases(&services);

        assert_eq!(
            edit_guard(&services, &pools, &event, "new-area").await,
            Ok(())
        );
        ballot(&services, &ballot_box, &event, BallotStatus::Valid).await;
        assert_eq!(
            edit_guard(&services, &pools, &event, "old-area").await,
            Ok(())
        );
    })
}

#[test]
fn a_failed_census_check_is_an_internal_error() {
    rows::run(async {
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
    })
}

#[test]
fn the_voter_is_loaded_from_the_event_realm() {
    rows::run(async {
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
    })
}
