// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::audit;
use crate::services::authorization::{
    authorize, authorize_tally_session_permission_labels,
};
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use protocol_board::{TrusteeActionResult, TrusteeEventUpdates};
use rocket::{http::Status, serde::json::Json, State};
use sequent_core::{services::jwt::JwtClaims, types::permissions::Permissions};
use serde::Serialize;
use windmill::services::trustee_updates::{
    self, ServerCustodyInput, ServerCustodyReply,
};
use windmill::tasks::audit_log_outbox::kick_audit_log_outbox;

#[derive(Serialize)]
pub struct ActionAcknowledgement {
    accepted: bool,
}

fn authorization(claims: &JwtClaims) -> Result<(), JsonError> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )
    .map_err(refusal)
}

#[get("/trustee/event-updates")]
pub async fn trustee_event_updates(
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<TrusteeEventUpdates>, JsonError> {
    authorization(&claims)?;
    let mut db = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    let tx = db.transaction().await.map_err(internal)?;
    let updates = trustee_updates::event_updates(&tx, &claims)
        .await
        .map_err(internal)?;
    Ok(Json(updates))
}

/// An authenticated file upload requests work, never a ceremony completion.
#[post("/trustee/request-server-custody", format = "json", data = "<body>")]
pub async fn request_server_custody(
    body: Json<ServerCustodyInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<ServerCustodyReply>, JsonError> {
    authorization(&claims)?;
    let mut db = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    let tx = db.transaction().await.map_err(internal)?;
    audit::act_for(&tx, &claims).await.map_err(refusal)?;
    if let Some(id) = &body.tally_session_id {
        authorize_tally_session_permission_labels(
            &tx,
            &claims,
            &body.election_event_id,
            id,
        )
        .await
        .map_err(refusal)?;
    }
    let reply = trustee_updates::request_server_custody(&tx, &claims, &body)
        .await
        .map_err(refused)?;
    tx.commit().await.map_err(internal)?;
    kick_audit_log_outbox();
    Ok(Json(reply))
}

/// Store a verified server proof for the operator's existing approval flow.
#[post("/trustee/action-result", format = "json", data = "<body>")]
pub async fn submit_trustee_action_result(
    body: Json<TrusteeActionResult>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<Json<ActionAcknowledgement>, JsonError> {
    authorization(&claims)?;
    let mut db = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    let tx = db.transaction().await.map_err(internal)?;
    audit::act_for(&tx, &claims).await.map_err(refusal)?;
    trustee_updates::submit_action_result(&tx, &claims, &body)
        .await
        .map_err(refused)?;
    tx.commit().await.map_err(internal)?;
    kick_audit_log_outbox();
    Ok(Json(ActionAcknowledgement { accepted: true }))
}

fn refusal((status, reason): (Status, String)) -> JsonError {
    let code = if status == Status::Unauthorized || status == Status::Forbidden
    {
        ErrorCode::Unauthorized
    } else {
        ErrorCode::InvalidTrusteeAction
    };
    ErrorResponse::new(status, &reason, code)
}

fn internal(error: impl std::fmt::Display) -> JsonError {
    tracing::error!("Trustee updates failed: {error}");
    ErrorResponse::new(
        Status::InternalServerError,
        "The trustee update service is unavailable",
        ErrorCode::InternalServerError,
    )
}
fn refused(error: impl std::fmt::Display) -> JsonError {
    ErrorResponse::new(
        Status::BadRequest,
        &error.to_string(),
        ErrorCode::InvalidTrusteeAction,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[get("/refusal")]
    fn refusal_route() -> JsonError {
        refused(
            "The ceremony file does not match this trustee and key ceremony",
        )
    }

    #[rocket::async_test]
    async fn refusals_use_the_json_contract_hasura_forwards_to_the_portal() {
        let client = rocket::local::asynchronous::Client::tracked(
            rocket::build().mount("/", routes![refusal_route]),
        )
        .await
        .unwrap();
        let response = client.get("/refusal").dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        let body = response
            .into_json::<serde_json::Value>()
            .await
            .expect("Hasura JSON webhook error");
        assert_eq!(
            body["message"],
            "The ceremony file does not match this trustee and key ceremony"
        );
    }

    fn trustee_header(
        event: &crate::route_services::rows::Event,
    ) -> rocket::http::Header<'static> {
        let mut claims =
            crate::test_claims::Claims::new(&event.tenant_id, "trustee-user")
                .roles([Permissions::TRUSTEE_CEREMONY])
                .build();
        claims.trustee = Some("alice".into());
        rocket::http::Header::new(
            "Authorization",
            format!(
                "Bearer {}",
                crate::test_claims::signing::token(
                    &serde_json::to_value(claims).unwrap()
                )
            ),
        )
    }

    async fn custody_post<'c>(
        client: &'c rocket::local::asynchronous::Client,
        event: &crate::route_services::rows::Event,
        path: &str,
        body: &serde_json::Value,
    ) -> (Status, serde_json::Value) {
        crate::route_services::json(
            client
                .post(path)
                .header(rocket::http::ContentType::JSON)
                .header(trustee_header(event))
                .body(body.to_string())
                .dispatch()
                .await,
        )
        .await
    }

    #[rocket::async_test]
    async fn authenticated_custody_actions_are_durable_and_do_not_confirm_human_custody(
    ) {
        use crate::route_services::{rows, Services};
        use protocol_board::{
            generate_manager, Committee, CustodyRequest, DkgBoard,
            RawTrusteeRecord, TrusteeSecrets,
        };
        use serde_json::{json, Value};
        use uuid::Uuid;

        let services = Services::on_test_database().await;
        let client = services.client().await;
        let event = rows::event(&services.hasura).await;
        let tenant = Uuid::parse_str(&event.tenant_id).unwrap();
        let event_id = Uuid::parse_str(&event.election_event_id).unwrap();
        let keys_id = Uuid::new_v4();
        let trustee_ids = [Uuid::new_v4(), Uuid::new_v4()];
        let secrets = TrusteeSecrets::generate();
        let public = secrets.public_keys().unwrap();
        let peer = TrusteeSecrets::generate().public_keys().unwrap();
        let committee = Committee::new(&[
            RawTrusteeRecord {
                name: "alice".into(),
                signing_public_key: Some(public.signing_public_key.clone()),
                share_encryption_public_key: Some(
                    public.share_encryption_public_key.clone(),
                ),
            },
            RawTrusteeRecord {
                name: "bob".into(),
                signing_public_key: Some(peer.signing_public_key),
                share_encryption_public_key: Some(
                    peer.share_encryption_public_key,
                ),
            },
        ])
        .unwrap();
        let board = DkgBoard::new(&keys_id, &generate_manager(), &committee, 2)
            .unwrap();
        rows::execute(&services.hasura, "UPDATE sequent_backend.election_event SET encryption_protocol='RSA256' WHERE id=$1", &[&event_id]).await;
        for (id, name) in trustee_ids.iter().zip(["alice", "bob"]) {
            rows::execute(&services.hasura, "INSERT INTO sequent_backend.trustee(id,tenant_id,name) VALUES ($1,$2,$3)", &[id,&tenant,&name]).await;
        }
        let initial_status = json!({"public_key": "verified-joint-key", "public_key_hash": "verified-hash", "logs": [], "trustees": [{"name": "alice", "status": "CUSTODY_REQUEST_RETRIEVED"}, {"name": "bob", "status": "KEY_GENERATED"}]});
        rows::execute(&services.hasura, "INSERT INTO sequent_backend.keys_ceremony(id,tenant_id,election_event_id,trustee_ids,threshold,status,execution_status) VALUES ($1,$2,$3,$4,2,$5,'IN_PROGRESS')", &[&keys_id,&tenant,&event_id,&trustee_ids.to_vec(),&initial_status]).await;
        rows::execute(&services.hasura, "INSERT INTO sequent_backend.protocol_board(id,tenant_id,election_event_id,keys_ceremony_id,name,manager_message) VALUES ($1,$2,$3,$4,$5,$6)", &[&Uuid::new_v4(),&tenant,&event_id,&keys_id,&board.name.as_str(),&board.configuration.to_bytes()]).await;
        let request = CustodyRequest {
            tenant_id: event.tenant_id.clone(),
            election_event_id: event.election_event_id.clone(),
            keys_ceremony_id: keys_id.to_string(),
            tally_session_id: None,
            trustee_name: "alice".into(),
            signing_public_key: public.signing_public_key,
            share_encryption_public_key: public.share_encryption_public_key,
        };
        let input = json!({"election_event_id": event.election_event_id, "keys_ceremony_id": keys_id, "custody_request_json": serde_json::to_string(&request).unwrap()});
        assert_eq!(
            custody_post(
                &client,
                &event,
                "/trustee/request-server-custody",
                &input
            )
            .await,
            (Status::Ok, json!({"status":"PENDING"}))
        );
        let (status, updates) = crate::route_services::json(
            client
                .get("/trustee/event-updates")
                .header(trustee_header(&event))
                .dispatch()
                .await,
        )
        .await;
        assert_eq!(status, Status::Ok);
        assert_eq!(updates["actions"].as_array().unwrap().len(), 1);
        let action = &updates["actions"][0];
        assert_eq!(action["kind"], "CREATE_CUSTODY_PROOF");
        assert_eq!(action["request"], serde_json::to_value(&request).unwrap());
        let proof = request.attest(&secrets).unwrap();
        let mut wrong_proof: Value = serde_json::from_str(&proof).unwrap();
        wrong_proof["request"]["tenant_id"] = json!("another-tenant");
        let mut result = json!({"id":action["id"],"request":request,"status":"READY","custody_proof_json":wrong_proof.to_string()});
        let (status, error) =
            custody_post(&client, &event, "/trustee/action-result", &result)
                .await;
        assert_eq!(status, Status::BadRequest);
        assert_eq!(error["message"], "Invalid server custody proof");
        result["custody_proof_json"] = json!(proof);
        assert_eq!(
            custody_post(&client, &event, "/trustee/action-result", &result)
                .await,
            (Status::Ok, json!({"accepted":true}))
        );
        let (status, ready) = custody_post(
            &client,
            &event,
            "/trustee/request-server-custody",
            &input,
        )
        .await;
        assert_eq!(status, Status::Ok);
        assert_eq!(ready["status"], "READY");
        assert!(request
            .verifies(ready["custody_proof_json"].as_str().unwrap())
            .unwrap());
        let row = services.hasura.get().await.unwrap().query_one("SELECT execution_status,status FROM sequent_backend.keys_ceremony WHERE id=$1", &[&keys_id]).await.unwrap();
        assert_eq!(row.get::<_, String>(0), "IN_PROGRESS");
        assert_eq!(row.get::<_, Value>(1), initial_status);
    }

    #[rocket::async_test]
    async fn authenticated_action_routes_hide_backend_connection_errors() {
        use crate::route_services::{rows::Event, Services};
        use serde_json::json;
        let services = Services::without_database();
        let client = services.client().await;
        let event = Event {
            tenant_id: uuid::Uuid::new_v4().to_string(),
            election_event_id: uuid::Uuid::new_v4().to_string(),
        };
        let request = json!({"tenant_id":event.tenant_id,"election_event_id":event.election_event_id,"keys_ceremony_id":uuid::Uuid::new_v4(),"trustee_name":"alice","signing_public_key":"signing","share_encryption_public_key":"encryption"});
        let mut responses = vec![
            crate::route_services::json(
                client
                    .get("/trustee/event-updates")
                    .header(trustee_header(&event))
                    .dispatch()
                    .await,
            )
            .await,
        ];
        for (path, body) in [
            (
                "/trustee/request-server-custody",
                json!({"election_event_id":event.election_event_id,"keys_ceremony_id":request["keys_ceremony_id"],"custody_request_json":request.to_string()}),
            ),
            (
                "/trustee/action-result",
                json!({"id":"action","request":request,"status":"FAILED","detail":"keys unavailable"}),
            ),
        ] {
            responses.push(custody_post(&client, &event, path, &body).await);
        }
        for (status, body) in responses {
            assert_eq!(status, Status::InternalServerError);
            assert_eq!(
                body["message"],
                "The trustee update service is unavailable"
            );
        }
    }
}
