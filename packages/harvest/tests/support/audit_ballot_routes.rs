// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::route_services::{post, rows, Services};
use crate::test_claims::Claims;
use electoral_log::adapters::ballot_box_received::{
    ReceiveBallot, ReceiveOutcome, ReceivedSignature,
};
use serde_json::json;

#[tokio::test]
async fn audit_requires_voter_scope_before_database() {
    let services = Services::without_database();
    let client = services.client().await;
    let claims = Claims::new("00000000-0000-4000-8000-000000000001", "staff");
    let response = post(&client, "/audit-ballot", &claims, &json!({"election_id":"00000000-0000-4000-8000-000000000002","ballot_id":"ABCD-1234"})).await;
    assert!(
        [Status::Forbidden, Status::Unauthorized].contains(&response.status())
    );
}

#[test]
#[ignore = "needs isolated electoral-log PostgreSQL"]
fn audit_route_uses_token_owner_and_acknowledges_only_audited_receipts() {
    rows::run(async {
        let services = Services::on_test_database().await;
        let seed = rows::event(&services.hasura).await;
        let event = rows::Event {
            tenant_id: seed.tenant_id,
            election_event_id: uuid::Uuid::new_v4().to_string(),
        };
        rows::execute(&services.hasura,
            "INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol,presentation) VALUES($1::text::uuid,$2::text::uuid,'RSA256',$3)",
            &[&event.election_event_id,&event.tenant_id,&json!({"ballot_identity":"pseudonym","voter_signing_policy":"with-signature","receipts":{"policy":"signed-by-ballot-box"}})]).await;
        let election = event.election(&services.hasura).await;
        let area = event.area(&services.hasura, "Synthetic audit area").await;
        let databases =
            windmill::services::electoral_log_board::get_event_databases()
                .await
                .unwrap();
        let store = databases
            .create_event(&event.tenant_id, &event.election_event_id)
            .await
            .unwrap();
        store
            .create_ballot_box(&event.election_event_id)
            .await
            .unwrap();
        store
            .set_ballot_identity(
                &event.election_event_id,
                electoral_log::adapters::privacy::BallotIdentity::Pseudonym,
            )
            .await
            .unwrap();
        store
            .allocate_voter_pseudonyms(
                &event.election_event_id,
                &["owner".into()],
            )
            .await
            .unwrap();
        let pseudonym = store
            .voter_pseudonym(&event.election_event_id, "owner")
            .await
            .unwrap()
            .unwrap();
        let received = store
            .receive_ballot(
                &ReceiveBallot {
                    election_event_id: &event.election_event_id,
                    election_id: &election,
                    area_id: &area,
                    voter_id: &pseudonym,
                    ballot_hash: "synthetic-hash",
                    content: "encrypted",
                    voter_signing_pk: "public",
                    voter_ballot_signature: "signature",
                },
                chrono::Utc::now(),
                |_| {
                    Ok(ReceivedSignature {
                        ballot_id: "ABCD-1234".into(),
                        key_id: "key".into(),
                        received_signature: "signature".into(),
                    })
                },
            )
            .await
            .unwrap();
        assert!(matches!(received, ReceiveOutcome::Received(_)));
        let claims = |voter| {
            Claims::new(&event.tenant_id, voter)
                .azp("voting-portal")
                .area(&area)
                .election_event(&event.election_event_id)
                .authorized_elections(&[&election])
                .roles([VoterPermissions::CAST_VOTE])
        };
        let client = services.client().await;
        let body = json!({"election_id":election,"ballot_id":"ABCD-1234"});
        assert_eq!(
            post(&client, "/audit-ballot", &claims("other"), &body)
                .await
                .status(),
            Status::NotFound
        );
        let wrong_area =
            claims("owner").area("00000000-0000-4000-8000-000000000099");
        assert_eq!(
            post(&client, "/audit-ballot", &wrong_area, &body)
                .await
                .status(),
            Status::NotFound
        );
        assert_eq!(post(&client, "/audit-ballot", &claims("owner"), &json!({"election_id":"00000000-0000-4000-8000-000000000098","ballot_id":"ABCD-1234"})).await.status(), Status::Unauthorized);
        for _ in 0..2 {
            let response =
                post(&client, "/audit-ballot", &claims("owner"), &body).await;
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(
                response.into_json::<serde_json::Value>().await.unwrap(),
                json!({"ballot_id":"ABCD-1234","status":"audited"})
            );
        }
        let row = store
            .received_ballot(
                &event.election_event_id,
                &election,
                &pseudonym,
                "synthetic-hash",
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, electoral_log::adapters::ballot_box_received::ReceivedStatus::Audited);
        databases
            .drop_event(&event.tenant_id, &event.election_event_id)
            .await
            .unwrap();
    });
}
