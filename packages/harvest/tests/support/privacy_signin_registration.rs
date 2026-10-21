// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::route_services::{
    post_as, rows, service_tokens::RealmSigner, Services,
};
use crate::services::service_auth::DEFAULT_SERVICE_CLIENT;
use crate::test_claims::Claims;
use electoral_log::adapters::privacy::BallotIdentity;
use rocket::http::Status;
use sequent_core::types::permissions::Permissions;
use serde_json::json;

async fn publish(event: &rows::Event, services: &Services) -> String {
    let id = event
        .ballot_publication(&services.hasura, false, false)
        .await;
    rows::execute(&services.hasura,
        "UPDATE sequent_backend.ballot_publication SET is_generated=true,published_at=now() WHERE id=$1::text::uuid AND set_config('sequent.trusted_write','on',true)='on'",
        &[&id]).await;
    id
}

/// The real REGISTER handler eventually makes the identical retained request
/// succeed. An unavailable registry must never be acknowledged as retained.
#[test]
fn first_signin_retries_registration_and_stops_at_the_purge_tombstone() {
    rows::run(async {
        let services = Services::on_test_database().await;
        let seed = rows::event(&services.hasura).await;
        let event = rows::Event {
            tenant_id: seed.tenant_id,
            election_event_id: uuid::Uuid::new_v4().to_string(),
        };
        rows::execute(&services.hasura,
            "INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol,presentation) VALUES($1::text::uuid,$2::text::uuid,'RSA256',$3)",
            &[&event.election_event_id,&event.tenant_id,&json!({"ballot_identity":"pseudonym","network_data_policy":"forensic-store","voter_signing_policy":"with-signature","receipts":{"policy":"signed-by-ballot-box"}})]).await;
        let realm_name = sequent_core::services::keycloak::get_event_realm(
            &event.tenant_id,
            &event.election_event_id,
        );
        let realm = rows::keycloak_realm(&services.keycloak, &realm_name).await;
        let voter = rows::keycloak_user(
            &services.keycloak,
            &realm,
            "synthetic-first-login",
            true,
        )
        .await;
        let nonvoter = rows::keycloak_user(
            &services.keycloak,
            &realm,
            "synthetic-staff",
            true,
        )
        .await;
        let group = uuid::Uuid::new_v4().to_string();
        let group_name = std::env::var("KEYCLOAK_VOTER_GROUP_NAME")
            .expect("voter group configured");
        rows::execute(
            &services.keycloak,
            "INSERT INTO keycloak_group(id,name,realm_id) VALUES($1,$2,$3)",
            &[&group, &group_name, &realm],
        )
        .await;
        rows::execute(
            &services.keycloak,
            "INSERT INTO user_group_membership(user_id,group_id) VALUES($1,$2)",
            &[&voter, &group],
        )
        .await;
        let signer = RealmSigner::shared();
        services.trust(&event.tenant_id, signer);
        let client = services.client().await;
        let claims = Claims::new(&event.tenant_id, "service-account-keycloak")
            .azp(DEFAULT_SERVICE_CLIENT)
            .roles([Permissions::SERVICE_ACCOUNT]);
        let token = signer.sign(&claims, &event.tenant_id);
        let mut body = json!({"tenant_id":event.tenant_id,"election_event_id":event.election_event_id,"voter_id":voter,
            "delivery_id":"synthetic-first-login","occurred_at_ms":1_800_000_000_000_i64,"ip":"198.51.100.73","country":"AQ","user_agent":"synthetic-device"});
        assert_eq!(
            post_as(&client, "/privacy/sign-in-network", &token, &body)
                .await
                .status(),
            Status::NoContent
        );
        let publication = publish(&event, &services).await;
        rows::execute(&services.hasura,"UPDATE sequent_backend.ballot_publication SET deleted_at=now() WHERE id=$1::text::uuid",&[&publication]).await;
        assert_eq!(
            post_as(&client, "/privacy/sign-in-network", &token, &body)
                .await
                .status(),
            Status::NoContent
        );
        publish(&event, &services).await;
        let store = event.ballot_box(&services.hasura).await;
        store
            .set_ballot_identity(
                &event.election_event_id,
                BallotIdentity::Pseudonym,
            )
            .await
            .unwrap();
        assert_eq!(
            post_as(&client, "/privacy/sign-in-network", &token, &body)
                .await
                .status(),
            Status::ServiceUnavailable
        );
        assert!(store
            .voter_pseudonym(&event.election_event_id, &voter)
            .await
            .unwrap()
            .is_none());
        body["voter_id"] = json!(nonvoter);
        assert_eq!(
            post_as(&client, "/privacy/sign-in-network", &token, &body)
                .await
                .status(),
            Status::NoContent
        );
        body["voter_id"] = json!(voter);
        rows::execute(&services.keycloak,"UPDATE user_entity SET service_account_client_link='service' WHERE id=$1",&[&voter]).await;
        assert_eq!(
            post_as(&client, "/privacy/sign-in-network", &token, &body)
                .await
                .status(),
            Status::NoContent
        );
        rows::execute(&services.keycloak,"UPDATE user_entity SET service_account_client_link=NULL WHERE id=$1",&[&voter]).await;
        // This is the actual durable REGISTER consumer, not sign-in allocation.
        let mut hasura = services.hasura.get().await.unwrap();
        let mut keycloak = services.keycloak.get().await.unwrap();
        let tx = hasura.transaction().await.unwrap();
        let kc = keycloak.transaction().await.unwrap();
        windmill::services::privacy_signin::allocate_registered_voters(&tx,&kc,&[
            windmill::tasks::electoral_log::LogEventInput {
                delivery_id:Some("synthetic-register".into()),tenant_id:event.tenant_id.clone(),election_event_id:event.election_event_id.clone(),
                user_id:Some(voter.clone()),username:None,message_type:windmill::tasks::electoral_log::LogMessageType::KeycloakEvent("REGISTER".into()),
                body:windmill::tasks::electoral_log::LogEventBody::Plain("null".into()),event_time_ms:None,details:None,admin_event:None,
            }
        ]).await.unwrap();
        tx.commit().await.unwrap();
        kc.commit().await.unwrap();
        for _ in 0..2 {
            assert_eq!(
                post_as(&client, "/privacy/sign-in-network", &token, &body)
                    .await
                    .status(),
                Status::NoContent
            );
        }
        let count = || async {
            store.client().await.unwrap().query_one("SELECT count(*) FROM sequent_backend.forensic_network_record WHERE election_event_id=$1::text::uuid",&[&event.election_event_id]).await.unwrap().get::<_,i64>(0)
        };
        assert_eq!(count().await, 1);
        store
            .allocate_voter_pseudonyms(
                &event.election_event_id,
                &["00000000-0000-4000-8000-000000000000".into()],
            )
            .await
            .unwrap();
        assert_eq!(
            store
                .purge_voter_pseudonyms_batch(&event.election_event_id, 1)
                .await
                .unwrap(),
            1
        );
        assert!(store
            .voter_pseudonym(&event.election_event_id, &voter)
            .await
            .unwrap()
            .is_some());
        body["delivery_id"] = json!("after-purge");
        assert_eq!(
            post_as(&client, "/privacy/sign-in-network", &token, &body)
                .await
                .status(),
            Status::NoContent
        );
        assert_eq!(
            count().await,
            1,
            "a tombstone applies before the last mapping batch is erased"
        );
        assert!(store
            .allocate_voter_pseudonyms(&event.election_event_id, &[voter])
            .await
            .is_err());
        windmill::services::electoral_log_board::get_event_databases()
            .await
            .unwrap()
            .drop_event(&event.tenant_id, &event.election_event_id)
            .await
            .unwrap();
    });
}
