// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Dedicated private mTLS CA assessment server; not a route on the normal
//! Harvest/Hasura listener. Link only in a reviewed private installation
//! profile with Rocket's mtls feature, independent trust roots/peer pin,
//! and the separate scope-limited assessment database role.

use deadpool_postgres::Pool;
use openssl::x509::{X509Ref, X509};
use rocket::config::{Config, MutualTls, TlsConfig};
use rocket::data::{Limits, ToByteUnit};
use rocket::http::Status;
use rocket::mtls::Certificate;
use rocket::serde::json::Json;
use rocket::{Build, Rocket, State};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use uuid::Uuid;
use windmill::services::signing::operational_ca::{
    assess_reserved_ca, DeviceCaProbe, PositiveCaAssessment, RequiredCaDevice,
};

struct Authority {
    database: Pool,
    tenant: Uuid,
    peer_pin: [u8; 32],
    profile: RequiredCaDevice,
    #[cfg(all(
        feature = "felix3-local-initialize-ca",
        not(feature = "operational-signing-demo-fixture")
    ))]
    local: crate::local_initialize_ca::Scope,
}

pub(crate) fn spki(
    cert: &X509Ref,
) -> Result<[u8; 32], openssl::error::ErrorStack> {
    Ok(Sha256::digest(cert.public_key()?.public_key_to_der()?).into())
}

#[tracing::instrument(skip_all)]
#[rocket::post(
    "/private/v1/operational-signing/initialize/ca",
    format = "json",
    data = "<probe>"
)]
async fn assess(
    state: &State<Authority>,
    peer: Certificate<'_>,
    probe: Json<DeviceCaProbe>,
) -> Result<Json<PositiveCaAssessment>, Status> {
    let pin = X509::from_der(peer.as_bytes())
        .and_then(|cert| spki(&cert))
        .map_err(|_| Status::Forbidden)?;
    if pin != state.peer_pin || probe.tenant != state.tenant {
        return Err(Status::Forbidden);
    }
    let mut connection = state
        .database
        .get()
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    let transaction = connection.build_transaction();
    #[cfg(all(
        feature = "felix3-local-initialize-ca",
        not(feature = "operational-signing-demo-fixture")
    ))]
    let transaction = transaction
        .isolation_level(tokio_postgres::IsolationLevel::ReadCommitted)
        .read_only(true);
    let tx = transaction
        .start()
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    #[cfg(all(
        feature = "felix3-local-initialize-ca",
        not(feature = "operational-signing-demo-fixture")
    ))]
    state.local.authorize(&tx, &probe).await?;
    let result = assess_reserved_ca(&tx, state.tenant, state.profile, &probe)
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    #[cfg(all(
        feature = "felix3-local-initialize-ca",
        not(feature = "operational-signing-demo-fixture")
    ))]
    if let Some(assessment) = &result {
        // Recheck after chain/CRL verification. Read committed also keeps
        // the original verifier's final CA-version query fresh.
        state.local.authorize(&tx, &probe).await?;
        if chrono::Utc::now().timestamp() >= assessment.usable_until {
            return Err(Status::Forbidden);
        }
    }
    tx.commit().await.map_err(|_| Status::ServiceUnavailable)?;
    result.map(Json).ok_or(Status::Forbidden)
}

/// Construct a separate application, not a route mount on the standard
/// server. Provisioning supplies credential bytes from its existing protected
/// runtime transport volume; do not generate, persist or log credentials here.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn unmounted_authority(
    address: SocketAddr,
    tenant: Uuid,
    peer_pin: [u8; 32],
    profile: RequiredCaDevice,
    database: Pool,
    server_chain: &[u8],
    server_key: &[u8],
    approved_device_client_ca: &[u8],
    #[cfg(all(
        feature = "felix3-local-initialize-ca",
        not(feature = "operational-signing-demo-fixture")
    ))]
    local: crate::local_initialize_ca::Scope,
) -> Result<Rocket<Build>, Status> {
    if tenant.is_nil()
        || peer_pin == [0; 32]
        || server_chain.is_empty()
        || server_key.is_empty()
        || approved_device_client_ca.is_empty()
    {
        return Err(Status::ServiceUnavailable);
    }
    let connection = database
        .get()
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    let role_is_safe = connection.query_one(
        "SELECT current_user=session_user
        AND pg_has_role(current_user,'sequent_os_initialize_ca','USAGE')
        AND has_function_privilege(current_user,'sequent_backend.operational_initialize_get_ca_enrollment(uuid,uuid,uuid,text,uuid,uuid,text)','EXECUTE')
        AND NOT pg_has_role(current_user,'sequent_os_ledger_api_owner','MEMBER')
        AND NOT pg_has_role(current_user,'sequent_os_ledger_data_owner','MEMBER')
        AND NOT pg_has_role(current_user,'sequent_os_initialize_orchestrator','MEMBER')
        AND NOT pg_has_role(current_user,'sequent_os_initialize_device','MEMBER')
        AND NOT pg_has_role(current_user,'sequent_os_device','MEMBER')
        AND NOT has_table_privilege(current_user,'sequent_backend.operational_signing_intent','SELECT, INSERT, UPDATE, DELETE')
        AND NOT has_any_column_privilege(current_user,'sequent_backend.operational_signing_intent','SELECT, INSERT, UPDATE, REFERENCES')
        AND NOT has_table_privilege(current_user,'sequent_backend.certificate_authority','SELECT, INSERT, UPDATE, DELETE')
        AND NOT has_any_column_privilege(current_user,'sequent_backend.certificate_authority','SELECT, INSERT, UPDATE, REFERENCES')
        AND NOT has_table_privilege(current_user,'sequent_backend.staff_crl','SELECT, INSERT, UPDATE, DELETE')
        AND NOT has_any_column_privilege(current_user,'sequent_backend.staff_crl','SELECT, INSERT, UPDATE, REFERENCES')
        AND NOT has_table_privilege(current_user,'sequent_backend.staff_certificate','SELECT, INSERT, UPDATE, DELETE')
        AND NOT has_any_column_privilege(current_user,'sequent_backend.staff_certificate','SELECT, INSERT, UPDATE, REFERENCES')",
        &[],
    ).await.map_err(|_| Status::ServiceUnavailable)?
      .try_get::<_,bool>(0).map_err(|_|Status::ServiceUnavailable)?;
    if !role_is_safe {
        return Err(Status::ServiceUnavailable);
    }
    drop(connection);

    let config = Config {
        address: address.ip(),
        port: address.port(),
        tls: Some(TlsConfig::from_bytes(server_chain, server_key).with_mutual(
            MutualTls::from_bytes(approved_device_client_ca).mandatory(true),
        )),
        limits: Limits::default().limit("json", 8.kibibytes()),
        ..Config::default()
    };
    if !config.mtls_enabled() {
        return Err(Status::ServiceUnavailable);
    }
    Ok(rocket::custom(config)
        .manage(Authority {
            database,
            tenant,
            peer_pin,
            profile,
            #[cfg(all(
                feature = "felix3-local-initialize-ca",
                not(feature = "operational-signing-demo-fixture")
            ))]
            local,
        })
        .mount("/", rocket::routes![assess]))
}
