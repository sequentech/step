// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Direct authenticated network-evidence ingress. Raw network input never
//! enters the electoral-log task queue, audit snapshots or process logs.
use crate::services::dependencies::HarvestServices;
use crate::services::service_auth::VerifiedServiceClaims;
use electoral_log::adapters::privacy::ForensicNetworkRecord;
use rocket::{
    http::{Header, Status},
    serde::json::Json,
    State,
};
use sequent_core::types::vote_privacy::{BallotIdentity, NetworkDataPolicy};
use serde::{Deserialize, Serialize};
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

/// Query only tenant/event policy before a producer sends any attribution.
/// The POST repeats this check, so a disabled policy cannot retain a later send.
async fn service_network_policy(
    services: &HarvestServices,
    claims: &VerifiedServiceClaims,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<NetworkDataPolicy, Refusal> {
    claims.require_tenant(tenant_id)?;
    let mut db = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    claims.require_event(&db, Some(election_event_id)).await?;
    let tx = db.transaction().await.map_err(internal)?;
    let event = get_election_event_by_id(&tx, tenant_id, election_event_id)
        .await
        .map_err(internal)?;
    let presentation = event
        .get_presentation()
        .map_err(internal)?
        .unwrap_or_default();
    Ok(
        if presentation.ballot_identity == Some(BallotIdentity::Pseudonym) {
            presentation.network_data_policy.unwrap_or_default()
        } else {
            NetworkDataPolicy::DoNotStore
        },
    )
}

#[derive(Serialize)]
pub struct SigninNetworkPolicy {
    network_data_policy: NetworkDataPolicy,
}

#[derive(rocket::response::Responder)]
pub struct SigninPolicyResponse {
    policy: Json<SigninNetworkPolicy>,
    cache_control: Header<'static>,
}

#[tracing::instrument(skip_all)]
#[get("/privacy/sign-in-network/policy?<tenant_id>&<election_event_id>")]
pub async fn signin_network_policy(
    claims: VerifiedServiceClaims,
    services: &State<HarvestServices>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<SigninPolicyResponse, Refusal> {
    let network_data_policy =
        service_network_policy(services, &claims, tenant_id, election_event_id)
            .await?;
    Ok(SigninPolicyResponse {
        policy: Json(SigninNetworkPolicy {
            network_data_policy,
        }),
        cache_control: Header::new("Cache-Control", "no-store"),
    })
}

/// A successful LOGIN arrives after its account commits, but its durable
/// REGISTER event may still await allocation. Never acknowledge that race as
/// retained, and never allocate a mapping from a sign-in request.
async fn signin_identity(
    services: &HarvestServices,
    tenant: &str,
    event: &str,
    voter: &str,
) -> Result<
    Option<(electoral_log::adapters::postgres::PostgresStore, String)>,
    Refusal,
> {
    let hasura = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    let published: bool=hasura.query_one(
        "SELECT EXISTS(SELECT 1 FROM sequent_backend.ballot_publication WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND published_at IS NOT NULL AND deleted_at IS NULL)",
        &[&tenant,&event]).await.map_err(internal)?.try_get(0).map_err(internal)?;
    if !published {
        return Ok(None);
    }
    drop(hasura);
    let store = get_event_store(event).await.map_err(internal)?;
    let registration=store.client().await.map_err(internal)?.query_opt(
        "SELECT i.mapping_purged_at IS NOT NULL,m.pseudonym FROM sequent_backend.ballot_identity i LEFT JOIN sequent_backend.voter_pseudonym m ON m.election_event_id=i.election_event_id AND m.voter_id=$2 WHERE i.election_event_id=$1::text::uuid",
        &[&event,&voter]).await.map_err(internal)?;
    if registration
        .as_ref()
        .map(|row| row.try_get::<_, bool>(0))
        .transpose()
        .map_err(internal)?
        .unwrap_or(false)
    {
        return Ok(None);
    }
    let keycloak = services
        .databases
        .keycloak()
        .await
        .get()
        .await
        .map_err(internal)?;
    if windmill::services::privacy_signin::registered_voter(
        &keycloak, tenant, event, voter,
    )
    .await
    .map_err(internal)?
        != Some(true)
    {
        return Ok(None);
    }
    let pseudonym = registration
        .map(|row| row.try_get::<_, Option<String>>(1))
        .transpose()
        .map_err(internal)?
        .flatten();
    match pseudonym {
        Some(pseudonym) => Ok(Some((store, pseudonym))),
        None => Err((
            Status::ServiceUnavailable,
            "Sign-in privacy registration pending".into(),
        )),
    }
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
    if service_network_policy(
        services,
        &claims,
        &input.tenant_id,
        &input.election_event_id,
    )
    .await?
        != NetworkDataPolicy::ForensicStore
    {
        return Ok(Status::NoContent);
    }
    let Some((store, pseudonym)) = signin_identity(
        services,
        &input.tenant_id,
        &input.election_event_id,
        &input.voter_id,
    )
    .await?
    else {
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
    async fn policy_preflight_is_service_scoped_and_immediately_reflects_disable(
    ) {
        use crate::route_services::{
            rows, service_tokens::RealmSigner, Services,
        };
        use crate::services::service_auth::DEFAULT_SERVICE_CLIENT;
        use crate::test_claims::Claims;
        use rocket::http::Header;
        use sequent_core::types::permissions::Permissions;
        use serde_json::{json, Value};
        let services = Services::on_test_database().await;
        let legacy = rows::event(&services.hasura).await;
        let event = uuid::Uuid::new_v4().to_string();
        let foreign = rows::event(&services.hasura).await;
        rows::execute(&services.hasura,
            "INSERT INTO sequent_backend.election_event (id,tenant_id,encryption_protocol,presentation) VALUES ($1::text::uuid,$2::text::uuid,'RSA256',$3)",
            &[&event,&legacy.tenant_id,&json!({"ballot_identity":"pseudonym","network_data_policy":"forensic-store",
                "voter_signing_policy":"with-signature","receipts":{"policy":"signed-by-ballot-box"}})]).await;
        let signer = RealmSigner::shared();
        services.trust(&legacy.tenant_id, signer);
        let client = services.client().await;
        let claims = Claims::new(&legacy.tenant_id, "service-account-keycloak")
            .azp(DEFAULT_SERVICE_CLIENT)
            .roles([Permissions::SERVICE_ACCOUNT]);
        let token = signer.sign(&claims, &legacy.tenant_id);
        let path = |tenant: &str, event: &str| {
            format!("/privacy/sign-in-network/policy?tenant_id={tenant}&election_event_id={event}")
        };
        let response = client
            .get(path(&legacy.tenant_id, &event))
            .header(Header::new("Authorization", token.clone()))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(
            response.headers().get_one("Cache-Control"),
            Some("no-store")
        );
        assert_eq!(
            response.into_json::<Value>().await.unwrap(),
            json!({"network_data_policy":"forensic-store"})
        );
        // No event log database is needed: this gate reads no private attribution.
        rows::execute(&services.hasura,
            "UPDATE sequent_backend.election_event SET presentation=jsonb_set(presentation,'{network_data_policy}', $2) WHERE id=$1::text::uuid",
            &[&event,&json!("do-not-store")]).await;
        for selected in [&event, &legacy.election_event_id] {
            let response = client
                .get(path(&legacy.tenant_id, selected))
                .header(Header::new("Authorization", token.clone()))
                .dispatch()
                .await;
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(
                response.into_json::<Value>().await.unwrap(),
                json!({"network_data_policy":"do-not-store"})
            );
        }
        for (tenant, event) in [
            (&legacy.tenant_id, &foreign.election_event_id),
            (&foreign.tenant_id, &foreign.election_event_id),
        ] {
            assert_eq!(
                client
                    .get(path(tenant, event))
                    .header(Header::new("Authorization", token.clone()))
                    .dispatch()
                    .await
                    .status(),
                Status::Forbidden
            );
        }
        let ordinary = Claims::new(&legacy.tenant_id, "ordinary")
            .azp(DEFAULT_SERVICE_CLIENT);
        assert_eq!(
            client
                .get(path(&legacy.tenant_id, &event))
                .header(Header::new(
                    "Authorization",
                    signer.sign(&ordinary, &legacy.tenant_id)
                ))
                .dispatch()
                .await
                .status(),
            Status::Unauthorized
        );
        assert_eq!(
            client
                .get(path(&legacy.tenant_id, &event))
                .dispatch()
                .await
                .status(),
            Status::Unauthorized
        );
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

#[cfg(test)]
#[path = "../../tests/support/privacy_signin_registration.rs"]
mod registration_tests;
