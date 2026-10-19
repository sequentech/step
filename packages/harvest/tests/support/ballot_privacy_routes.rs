// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::route_services::{post, rows, Services};
use crate::test_claims::Claims;

const TENANT: &str = "00000000-0000-4000-8000-000000000001";
const EVENT: &str = "00000000-0000-4000-8000-000000000002";
const ELECTION: &str = "00000000-0000-4000-8000-000000000003";
const AREA: &str = "00000000-0000-4000-8000-000000000004";
fn voter() -> Claims {
    Claims::new(TENANT, "voter")
        .azp("voting-portal")
        .area(AREA)
        .election_event(EVENT)
        .authorized_elections(&[ELECTION])
        .roles([VoterPermissions::CAST_VOTE])
}
fn input() -> Value {
    json!({"election_event_id":EVENT,"election_id":ELECTION,"ballot_id":"ABCD-EFGH","document_id":"00000000-0000-4000-8000-000000000005","pseudonym":"7QKM-2HXD","reason":"Official investigation","ip":"198.51.100.7"})
}

#[tokio::test]
async fn privacy_routes_refuse_missing_permission_before_any_database() {
    let services = Services::without_database();
    let client = services.client().await;
    let claims = Claims::new(TENANT, "unprivileged");
    for path in [
        "/receipt-document",
        "/ballot-box-log",
        "/ballot-box-own",
        "/ballot-box-entry",
        "/ballot-box-keys",
        "/resolve-voter-pseudonym",
        "/forensic-network-summary",
        "/forensic-network-detail",
        "/ballot-privacy-settings",
    ] {
        let response = post(&client, path, &claims, &input()).await;
        assert!(
            [Status::Unauthorized, Status::Forbidden]
                .contains(&response.status()),
            "{path}: {}",
            response.status()
        );
    }
}

#[tokio::test]
async fn voter_event_and_election_gates_run_before_storage_reads() {
    let services = Services::without_database();
    let client = services.client().await;
    for path in [
        "/ballot-box-log",
        "/ballot-box-own",
        "/ballot-box-entry",
        "/ballot-box-keys",
    ] {
        let wrong_event =
            voter().election_event("00000000-0000-4000-8000-000000000099");
        assert_eq!(
            post(&client, path, &wrong_event, &input()).await.status(),
            Status::Forbidden,
            "{path}"
        );
        let wrong_election =
            voter().authorized_elections(&["another-election"]);
        assert_eq!(
            post(&client, path, &wrong_election, &input())
                .await
                .status(),
            Status::Forbidden,
            "{path}"
        );
        let missing_area = Claims::new(TENANT, "voter")
            .azp("voting-portal")
            .election_event(EVENT)
            .authorized_elections(&[ELECTION])
            .roles([VoterPermissions::CAST_VOTE]);
        assert_eq!(
            post(&client, path, &missing_area, &input()).await.status(),
            Status::Forbidden,
            "{path}"
        );
    }
}

#[tokio::test]
async fn separately_privileged_admin_cannot_read_another_tenants_event() {
    let services = Services::on_test_database().await;
    let event = rows::event(&services.hasura).await;
    let client = services.client().await;
    let claims = Claims::new(TENANT, "investigator").roles([
        Permissions::LOGS_READ,
        Permissions::FORENSIC_READ,
        Permissions::VOTER_PSEUDONYM_RESOLVE,
        Permissions::ELECTION_EVENT_READ,
    ]);
    let mut body = input();
    body["election_event_id"] = json!(event.election_event_id);
    for path in [
        "/ballot-box-entry",
        "/ballot-box-keys",
        "/resolve-voter-pseudonym",
        "/forensic-network-summary",
        "/forensic-network-detail",
        "/ballot-privacy-settings",
    ] {
        assert_eq!(
            post(&client, path, &claims, &body).await.status(),
            Status::NotFound,
            "{path}"
        );
    }
}

#[test]
fn viewer_and_investigator_permissions_do_not_imply_each_other() {
    let reader = Claims::new(TENANT, "reader")
        .roles([Permissions::LOGS_READ])
        .build();
    assert!(proof_token_scope(&reader, EVENT, ELECTION).is_ok());
    for permission in [
        Permissions::FORENSIC_READ,
        Permissions::VOTER_PSEUDONYM_RESOLVE,
    ] {
        assert!(authorize(
            &reader,
            false,
            Some(TENANT.into()),
            vec![permission.clone()]
        )
        .is_err());
        let investigator = Claims::new(TENANT, "investigator")
            .roles([permission])
            .build();
        assert!(proof_token_scope(&investigator, EVENT, ELECTION).is_err());
    }
}

#[test]
fn scope_cache_and_cursor_cannot_cross_tenant_area_or_election() {
    let claims = voter().build();
    let (area, elections) =
        voter_token_scope(&claims, EVENT, Some(ELECTION)).unwrap();
    assert_eq!(area, AREA);
    assert_eq!(elections, vec![ELECTION]);
    let key = page_cache_key(TENANT, EVENT, AREA, &elections, 25);
    for other in [
        page_cache_key("other", EVENT, AREA, &elections, 25),
        page_cache_key(TENANT, "other", AREA, &elections, 25),
        page_cache_key(TENANT, EVENT, "other", &elections, 25),
        page_cache_key(TENANT, EVENT, AREA, &["other".into()], 25),
        page_cache_key(TENANT, EVENT, AREA, &elections, 100),
    ] {
        assert_ne!(key, other);
    }
    let scope = scope_key(EVENT, AREA, &elections, "");
    let cursor = encode_cursor(&scope, 100);
    assert!(decode_cursor(
        &scope_key(EVENT, "other", &elections, ""),
        Some(&cursor)
    )
    .is_err());
}

#[tokio::test]
async fn private_receipt_polling_refuses_other_event_and_missing_voter_scope_before_database(
) {
    let services = Services::without_database();
    let client = services.client().await;
    let wrong_event = voter().election_event("other-event");
    assert_eq!(
        post(&client, "/receipt-document", &wrong_event, &input())
            .await
            .status(),
        Status::Forbidden
    );
    let no_area = Claims::new(TENANT, "voter")
        .azp("voting-portal")
        .election_event(EVENT)
        .authorized_elections(&[ELECTION])
        .roles([VoterPermissions::CAST_VOTE]);
    assert_eq!(
        post(&client, "/receipt-document", &no_area, &input())
            .await
            .status(),
        Status::Forbidden
    );
}

#[test]
fn received_only_admin_proof_is_pending_without_exposing_uncast_ballots_to_voters(
) {
    use electoral_log::adapters::ballot_box_received::ReceivedStatus;
    for status in [ReceivedStatus::Received, ReceivedStatus::Audited] {
        assert!(ensure_proof_receipt_visible(false, status).is_ok());
        assert_eq!(proof_channel(status, None).unwrap(), "");
        assert_eq!(
            ensure_proof_receipt_visible(true, status).unwrap_err().0,
            Status::NotFound
        );
    }
    assert!(ensure_proof_receipt_visible(true, ReceivedStatus::Cast).is_ok());
    assert_eq!(
        proof_channel(ReceivedStatus::Cast, Some("online".into())).unwrap(),
        "online"
    );
    assert!(proof_channel(ReceivedStatus::Cast, None).is_err());
}
