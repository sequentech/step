// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Direct authenticated network-evidence ingress. Raw network input never
//! enters the electoral-log task queue, audit snapshots or process logs.
use crate::services::dependencies::HarvestServices;
use crate::services::service_auth::VerifiedServiceClaims;
use electoral_log::adapters::privacy::ForensicNetworkRecord;
use rocket::{http::Status, serde::json::Json, State};
use sequent_core::types::vote_privacy::{BallotIdentity, NetworkDataPolicy};
use serde::Deserialize;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::electoral_log_board::get_event_store;

type Refusal = (Status, String);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SigninNetworkInput {
    tenant_id: String,
    election_event_id: String,
    voter_id: String,
    delivery_id: String,
    occurred_at_ms: i64,
    ip: String,
    country: Option<String>,
    user_agent: Option<String>,
}

fn valid(input: &SigninNetworkInput) -> bool {
    uuid::Uuid::parse_str(&input.voter_id).is_ok()
        && !input.delivery_id.is_empty()
        && input.delivery_id.len() <= 128
        && input.ip.parse::<std::net::IpAddr>().is_ok()
        && input.country.as_ref().is_none_or(|s| {
            s.len() == 2 && s.bytes().all(|c| c.is_ascii_alphabetic())
        })
        && input.user_agent.as_ref().is_none_or(|s| s.len() <= 1024)
        && chrono::DateTime::from_timestamp_millis(input.occurred_at_ms)
            .is_some()
}
fn internal(_: impl std::fmt::Display) -> Refusal {
    (
        Status::InternalServerError,
        "Unable to retain sign-in network evidence".into(),
    )
}

#[tracing::instrument(skip_all)]
#[post("/privacy/sign-in-network", format = "json", data = "<body>")]
pub async fn retain_signin_network(
    claims: VerifiedServiceClaims,
    body: Json<SigninNetworkInput>,
    services: &State<HarvestServices>,
) -> Result<Status, Refusal> {
    let input = body.into_inner();
    claims.require_tenant(&input.tenant_id)?;
    if !valid(&input) {
        return Err((
            Status::BadRequest,
            "Invalid sign-in network evidence".into(),
        ));
    }
    let mut db = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    claims
        .require_event(&db, Some(&input.election_event_id))
        .await?;
    let tx = db.transaction().await.map_err(internal)?;
    let event = get_election_event_by_id(
        &tx,
        &input.tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(internal)?;
    let presentation = event
        .get_presentation()
        .map_err(internal)?
        .unwrap_or_default();
    if presentation.ballot_identity != Some(BallotIdentity::Pseudonym)
        || presentation.network_data_policy
            != Some(NetworkDataPolicy::ForensicStore)
    {
        return Ok(Status::NoContent);
    }
    let store = get_event_store(&input.election_event_id)
        .await
        .map_err(internal)?;
    let Some(pseudonym) = store
        .voter_pseudonym(&input.election_event_id, &input.voter_id)
        .await
        .map_err(internal)?
    else {
        // An unpublished or purged registry has no identity to attribute. It is
        // never allocated or reconstructed in an authentication request.
        return Ok(Status::NoContent);
    };
    let recorded_at =
        chrono::DateTime::from_timestamp_millis(input.occurred_at_ms)
            .ok_or_else(|| {
                (Status::BadRequest, "Invalid sign-in time".into())
            })?;
    store
        .record_forensic_network(&ForensicNetworkRecord {
            election_event_id: &input.election_event_id,
            election_id: None,
            area_id: None,
            event_kind: "sign-in",
            delivery_id: &format!("signin:{}", input.delivery_id),
            pseudonym: &pseudonym,
            ballot_id: None,
            ip: &input.ip,
            country: input.country.as_deref(),
            user_agent: input.user_agent.as_deref(),
            recorded_at,
        })
        .await
        .map_err(internal)?;
    Ok(Status::NoContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ingress_bounds_metadata_and_accepts_only_literal_network_addresses() {
        let mut input = SigninNetworkInput {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            voter_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
            delivery_id: "delivery".into(),
            occurred_at_ms: 1_800_000_000_000,
            ip: "2001:db8::7".into(),
            country: Some("TR".into()),
            user_agent: None,
        };
        assert!(valid(&input));
        input.ip = "example.com".into();
        assert!(!valid(&input));
        input.ip = "198.51.100.7".into();
        input.user_agent = Some("x".repeat(1025));
        assert!(!valid(&input));
        input.user_agent = None;
        input.country = Some("a person's name".into());
        assert!(!valid(&input));
    }
    #[tokio::test]
    async fn signed_service_ingress_is_tenant_scoped_and_honors_do_not_store() {
        use crate::route_services::{
            post_as, rows, service_tokens::RealmSigner, Services,
        };
        use crate::services::service_auth::DEFAULT_SERVICE_CLIENT;
        use crate::test_claims::Claims;
        use sequent_core::types::permissions::Permissions;
        use serde_json::json;
        let services = Services::on_test_database().await;
        let tenant = rows::event(&services.hasura).await;
        let event = rows::Event {
            tenant_id: tenant.tenant_id,
            election_event_id: uuid::Uuid::new_v4().to_string(),
        };
        let foreign = rows::event(&services.hasura).await;
        // Identity is immutable after creation, including in test fixtures.
        rows::execute(&services.hasura,
            "INSERT INTO sequent_backend.election_event (id,tenant_id,encryption_protocol,presentation) VALUES ($1::text::uuid,$2::text::uuid,'RSA256',$3)",
            &[&event.election_event_id,&event.tenant_id,&json!({"ballot_identity":"pseudonym","network_data_policy":"do-not-store",
                "voter_signing_policy":"with-signature","receipts":{"policy":"signed-by-ballot-box"}})]).await;
        let signer = RealmSigner::shared();
        services.trust(&event.tenant_id, signer);
        let client = services.client().await;
        let claims = Claims::new(&event.tenant_id, "service-account-keycloak")
            .azp(DEFAULT_SERVICE_CLIENT)
            .roles([Permissions::SERVICE_ACCOUNT]);
        let token = signer.sign(&claims, &event.tenant_id);
        let mut body = json!({"tenant_id":event.tenant_id,"election_event_id":event.election_event_id,
            "voter_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5","delivery_id":"synthetic-login",
            "occurred_at_ms":1_800_000_000_000_i64,"ip":"198.51.100.7","country":"TR","user_agent":"synthetic"});
        // No event log database exists. A 204 proves policy stops before private storage.
        assert_eq!(
            post_as(&client, "/privacy/sign-in-network", &token, &body)
                .await
                .status(),
            Status::NoContent
        );
        body["election_event_id"] = json!(foreign.election_event_id);
        assert_eq!(
            post_as(&client, "/privacy/sign-in-network", &token, &body)
                .await
                .status(),
            Status::Forbidden
        );
        body["tenant_id"] = json!(foreign.tenant_id);
        assert_eq!(
            post_as(&client, "/privacy/sign-in-network", &token, &body)
                .await
                .status(),
            Status::Forbidden
        );
        let ordinary = Claims::new(&event.tenant_id, "ordinary")
            .azp(DEFAULT_SERVICE_CLIENT);
        assert_eq!(
            post_as(
                &client,
                "/privacy/sign-in-network",
                &signer.sign(&ordinary, &event.tenant_id),
                &body
            )
            .await
            .status(),
            Status::Unauthorized
        );
    }
}
