// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Separate, mandatory mTLS bridge to an already approved independent staff
//! CA operator. It never issues a CA certificate, accepts key material,
//! grants trust, or mounts on the ordinary browser/app listener.
use deadpool_postgres::Pool;
use openssl::x509::X509;
use rocket::config::{Config, MutualTls, TlsConfig};
use rocket::data::{Limits, ToByteUnit};
use rocket::http::Status;
use rocket::mtls::Certificate;
use rocket::serde::json::Json;
use rocket::{Build, Rocket, State};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use uuid::Uuid;
use windmill::services::signing::{
    log::Actor,
    operational_enrollment::{attach_setup_certificate, setup_certificate_material},
};

struct SetupCa {
    tenant: Uuid,
    officer: Uuid,
    peer_pin: [u8; 32],
    database: Pool,
}

impl SetupCa {
    fn authorize(&self, peer: &Certificate<'_>) -> Result<(), Status> {
        let pin = X509::from_der(peer.as_bytes())
            .and_then(|cert| cert.public_key())
            .and_then(|key| key.public_key_to_der())
            .ok()
            .map(|key| <[u8; 32]>::from(Sha256::digest(key)));
        if pin == Some(self.peer_pin) { Ok(()) } else { Err(Status::Forbidden) }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MaterialInput {
    event: Uuid,
    ticket: Uuid,
}

#[derive(Serialize)]
struct MaterialReply {
    tenant: Uuid,
    event: Uuid,
    ticket: Uuid,
    official: Uuid,
    post: Option<Uuid>,
    csr: String,
    spki_sha256: String,
}

#[tracing::instrument(skip_all)]
#[rocket::post("/private/v2/operational-signing/setup-ca/material", format = "json", data = "<input>")]
async fn load(
    state: &State<SetupCa>,
    peer: Certificate<'_>,
    input: Json<MaterialInput>,
) -> Result<Json<MaterialReply>, Status> {
    state.authorize(&peer)?;
    let mut db = state.database.get().await.map_err(|_| Status::ServiceUnavailable)?;
    let tx = db.transaction().await.map_err(|_| Status::ServiceUnavailable)?;
    let material = setup_certificate_material(&tx, state.tenant, input.event, input.ticket, state.officer)
        .await.map_err(|_| Status::ServiceUnavailable)?
        .ok_or(Status::Forbidden)?;
    tx.commit().await.map_err(|_| Status::ServiceUnavailable)?;
    Ok(Json(MaterialReply {
        tenant: state.tenant, event: input.event, ticket: input.ticket,
        official: material.official, post: material.post, csr: material.csr, spki_sha256: material.spki,
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AttachmentInput {
    event: Uuid,
    ticket: Uuid,
    chain_pem: Vec<String>,
}

#[derive(Serialize)]
struct AttachmentReply {
    certificate: Uuid,
}

#[tracing::instrument(skip_all)]
#[rocket::post("/private/v2/operational-signing/setup-ca/attach", format = "json", data = "<input>")]
async fn attach(
    state: &State<SetupCa>,
    peer: Certificate<'_>,
    input: Json<AttachmentInput>,
) -> Result<Json<AttachmentReply>, Status> {
    state.authorize(&peer)?;
    let input = input.into_inner();
    let mut db = state.database.get().await.map_err(|_| Status::ServiceUnavailable)?;
    let tx = db.transaction().await.map_err(|_| Status::ServiceUnavailable)?;
    let material = setup_certificate_material(&tx, state.tenant, input.event, input.ticket, state.officer)
        .await.map_err(|_| Status::ServiceUnavailable)?
        .ok_or(Status::Forbidden)?;
    let officer = Actor {user_id: state.officer.to_string(), username: state.officer.to_string()};
    let holder = Actor {user_id: material.official.to_string(), username: material.official.to_string()};
    let certificate = attach_setup_certificate(
        &tx, state.tenant, input.event, input.ticket, &officer, holder, input.chain_pem,
    ).await.map_err(|_| Status::Forbidden)?;
    tx.commit().await.map_err(|_| Status::ServiceUnavailable)?;
    Ok(Json(AttachmentReply {certificate}))
}

/// Call from a distinct private CA registration process, using the existing
/// application registrar capability plus only the new setup-CA SQL role.
/// It must never be installed by browser configuration or the main app.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn product_setup_ca(
    address: SocketAddr,
    tenant: Uuid,
    officer: Uuid,
    peer_pin: [u8; 32],
    database: Pool,
    server_chain: &[u8],
    server_key: &[u8],
    approved_client_ca: &[u8],
) -> Result<Rocket<Build>, Status> {
    if tenant.is_nil() || officer.is_nil() || peer_pin == [0; 32]
        || server_chain.is_empty() || server_key.is_empty() || approved_client_ca.is_empty()
        || address.port() == 0 || address.ip().is_unspecified()
    { return Err(Status::ServiceUnavailable); }

    let db = database.get().await.map_err(|_| Status::ServiceUnavailable)?;
    let role_ok: bool = db.query_one(
        "SELECT current_user=session_user
         AND pg_has_role(current_user,'sequent_os_product_setup_ca','USAGE')
         AND NOT pg_has_role(current_user,'sequent_os_product_setup_actor','MEMBER')
         AND NOT pg_has_role(current_user,'sequent_os_product_setup_dispatch','MEMBER')
         AND NOT pg_has_role(current_user,'sequent_os_product_setup_device','MEMBER')
         AND NOT pg_has_role(current_user,'sequent_os_setup_authority','MEMBER')
         AND NOT pg_has_role(current_user,'sequent_os_setup_device','MEMBER')
         AND NOT pg_has_role(current_user,'sequent_os_approve','MEMBER')
         AND NOT pg_has_role(current_user,'sequent_os_ledger_api_owner','MEMBER')
         AND NOT pg_has_role(current_user,'sequent_os_ledger_data_owner','MEMBER')
         AND NOT has_any_column_privilege(current_user,'sequent_backend.operational_event_setup_request','SELECT,INSERT,UPDATE,REFERENCES')
         AND NOT has_any_column_privilege(current_user,'sequent_backend.certificate_authority','INSERT,UPDATE,REFERENCES')",
        &[],
    ).await.map_err(|_| Status::ServiceUnavailable)?.try_get(0)
      .map_err(|_| Status::ServiceUnavailable)?;
    if !role_ok { return Err(Status::ServiceUnavailable); }
    drop(db);
    let config = Config {
        address: address.ip(), port: address.port(),
        tls: Some(TlsConfig::from_bytes(server_chain,server_key).with_mutual(
            MutualTls::from_bytes(approved_client_ca).mandatory(true)
        )),
        limits: Limits::default().limit("json", 384.kibibytes()),
        ..Config::default()
    };
    if !config.mtls_enabled() { return Err(Status::ServiceUnavailable); }
    Ok(rocket::custom(config).manage(SetupCa {tenant,officer,peer_pin,database})
        .mount("/",rocket::routes![load,attach]))
}
