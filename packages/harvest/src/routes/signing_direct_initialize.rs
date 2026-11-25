// SPDX-License-Identifier: AGPL-3.0-only
//! Authenticated ordinary Admin Portal ingress. Only the matched private
//! PKCS#11 device accepts a reservation, not a browser-supplied sign message.
use super::{
    hasura, internal, require, signing_failure, tenant_of, uuid_of, writer, HarvestServices,
    RequestIdOutput, SigningFailure, SigningReply,
};
use anyhow::{ensure, Context, Result};
use chrono::{DateTime, Utc};
use openssl::x509::X509;
use openssl::{
    pkey::{Id, PKey, Private},
    sign::Signer,
};
use reqwest::{Certificate, Client, Identity, StatusCode, Url};
use rocket::serde::json::Json;
use rocket::{routes, Build, Rocket, State};
use sequent_core::{services::jwt::JwtClaims, types::permissions::Permissions};
use serde::{Deserialize, Serialize};
use std::{fs, io::Read, net::SocketAddrV4, os::unix::fs::MetadataExt, path::Path, time::Duration};
use uuid::Uuid;
use windmill::services::signing::direct_actions::{self, DirectReservation};
use windmill::services::signing::direct_initialize::{
    approved, authorize, commit, verify_proof, verify_signature, DirectScope,
};
use windmill::services::signing::direct_protocol::{
    DirectOperation, DirectPurpose, DirectRequestScope,
};

const ROOT: &str = "/run/sequent-comelec-demo-direct-hsm/harvest";
const AUTHORITY_DOMAIN: &[u8] = b"sequent.local.initialize.authorization/v1\0";
const DEVICE_ROUTE: &str = "private/v1/operational-signing/initialize/direct";
const ACTION_DEVICE_ROUTE: &str = "/private/v2/operational-signing/direct";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    tenant: Uuid,
    registered_issuer: String,
    portal_client: String,
    device_peer: SocketAddrV4,
    device_ca_sha256: String,
    harvest_client_spki_sha256: String,
    authorization_public_key_hex: String,
}

pub(crate) struct DirectDevice {
    registration: Registration,
    client: Client,
    url: Url,
    authorization_key: PKey<Private>,
}

fn protected(name: &str, private: bool) -> Option<Vec<u8>> {
    let path = Path::new(ROOT).join(name);
    for parent in path.ancestors().skip(1) {
        let meta = fs::symlink_metadata(parent).ok()?;
        if !meta.is_dir() || meta.uid() != 0 || meta.mode() & 0o022 != 0 {
            return None;
        }
    }
    let meta = fs::symlink_metadata(&path).ok()?;
    if !meta.is_file()
        || meta.uid() != 0
        || meta.mode() & 0o222 != 0
        || (private && meta.mode() & 0o007 != 0)
    {
        return None;
    }
    let file = fs::File::open(path).ok()?;
    let opened = file.metadata().ok()?;
    if (opened.dev(), opened.ino()) != (meta.dev(), meta.ino()) || opened.len() > 65536 {
        return None;
    }
    let mut buf = Vec::new();
    file.take(65537).read_to_end(&mut buf).ok()?;
    (!buf.is_empty() && buf.len() <= 65536).then_some(buf)
}

fn decode_lower(hex_string: &str, min: usize, max: usize) -> Option<Vec<u8>> {
    (hex_string.len() >= min * 2
        && hex_string.len() <= max * 2
        && hex_string
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    .then(|| hex::decode(hex_string).ok())
    .flatten()
}

impl DirectDevice {
    fn load() -> Result<Self> {
        let registration: Registration = serde_json::from_slice(
            &protected("registration.json", false)
                .context("Missing direct HSM Harvest registration")?,
        )?;
        ensure!(
            !registration.tenant.is_nil()
                && registration.portal_client == "admin-portal"
                && registration.registered_issuer
                    == format!(
                        "http://localhost:18090/realms/tenant-{}",
                        registration.tenant
                    ),
            "Direct HSM requires the installed normal local admin issuer/client"
        );
        let addr = registration.device_peer;
        ensure!(
            addr.ip().is_private() && addr.port() == 23443,
            "Direct HSM peer must be the fixed internal listener"
        );
        let ca = protected("device-ca.pem", false)
            .context("Missing direct device transport trust pin")?;
        let anchors = X509::stack_from_pem(&ca)?;
        ensure!(
            anchors.len() == 1
                && hex::encode(openssl::sha::sha256(&anchors[0].to_der()?))
                    == registration.device_ca_sha256,
            "Unexpected private direct-device transport authority"
        );
        let cert =
            protected("client.pem", false).context("Missing direct-device client identity")?;
        let client_spki = X509::from_pem(&cert)?.public_key()?.public_key_to_der()?;
        ensure!(
            hex::encode(openssl::sha::sha256(&client_spki))
                == registration.harvest_client_spki_sha256,
            "Unexpected private direct-device caller"
        );
        let mut key = protected("client-key.pem", true)
            .context("Missing private direct-device caller key")?;
        let identity = Identity::from_pkcs8_pem(&cert, &key);
        key.fill(0);
        let client = Client::builder()
            .use_native_tls()
            .https_only(true)
            .http1_only()
            .tls_built_in_root_certs(false)
            .add_root_certificate(Certificate::from_pem(&ca)?)
            .identity(identity?)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(12))
            .pool_max_idle_per_host(0)
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .build()?;
        let url = Url::parse(&format!("https://{addr}/"))?.join(DEVICE_ROUTE)?;
        let mut key = protected("authorization-key.pem", true)
            .context("Missing private direct HSM authorization key")?;
        let authorization_key = PKey::private_key_from_pem(&key);
        key.fill(0);
        let authorization_key = authorization_key?;
        ensure!(
            authorization_key.id() == Id::ED25519
                && hex::encode(authorization_key.raw_public_key()?)
                    == registration.authorization_public_key_hex,
            "Unexpected direct HSM authority key"
        );
        Ok(Self {
            registration,
            client,
            url,
            authorization_key,
        })
    }

    fn portal_actor(&self, claims: &JwtClaims, tenant: Uuid) -> bool {
        self.registration.tenant == tenant
            && claims.typ == "Bearer"
            && claims.iss == self.registration.registered_issuer
            && claims.azp == self.registration.portal_client
            && Uuid::parse_str(&claims.sub).is_ok_and(|actor| {
                !actor.is_nil()
                    && actor.to_string() == claims.hasura_claims.user_id
                    && claims.sub == claims.hasura_claims.user_id
            })
    }

    async fn sign(
        &self,
        reservation_id: Uuid,
        nonce: Uuid,
        deadline: DateTime<Utc>,
        scope: &DirectScope,
        digest: &str,
    ) -> std::result::Result<(Vec<u8>, Vec<u8>), SigningFailure> {
        let expires_at_unix = deadline.timestamp();
        let now = Utc::now().timestamp();
        if expires_at_unix <= now || expires_at_unix > now + 45 {
            return Err(unavailable());
        }
        let proof_message = scope.message(nonce).map_err(signing_failure)?;
        let mut envelope = Vec::with_capacity(AUTHORITY_DOMAIN.len() + 24 + proof_message.len());
        envelope.extend_from_slice(AUTHORITY_DOMAIN);
        envelope.extend_from_slice(reservation_id.as_bytes());
        envelope.extend_from_slice(&expires_at_unix.to_be_bytes());
        envelope.extend_from_slice(&proof_message);
        let signature = Signer::new_without_digest(&self.authorization_key)
            .and_then(|mut signer| signer.sign_oneshot_to_vec(&envelope))
            .map_err(internal)?;
        #[derive(Serialize)]
        struct Input<'a> {
            reservation_id: Uuid,
            installation: Uuid,
            tenant: Uuid,
            event: Uuid,
            post: Uuid,
            publication: Uuid,
            request: Uuid,
            actor: Uuid,
            nonce: Uuid,
            payload_sha256: &'a str,
            expires_at_unix: i64,
            authorization_sig_hex: String,
        }
        let input = Input {
            reservation_id,
            installation: scope.installation(),
            tenant: scope.tenant(),
            event: scope.event(),
            post: scope.post(),
            publication: scope.publication(),
            request: scope.request(),
            actor: scope.actor(),
            nonce,
            payload_sha256: digest,
            expires_at_unix,
            authorization_sig_hex: hex::encode(signature),
        };
        self.send(self.url.clone(), &input, reservation_id).await
    }

    async fn sign_action(
        &self,
        held: &DirectReservation,
        operation: &DirectOperation,
    ) -> std::result::Result<(Vec<u8>, Vec<u8>), SigningFailure> {
        let expires_at_unix = held.expires_at.timestamp();
        let now = Utc::now().timestamp();
        if expires_at_unix <= now || expires_at_unix > now + 45 {
            return Err(unavailable());
        }
        let purpose = DirectPurpose::ApprovalPayload;
        let authorization = operation
            .authorization(held.id, held.nonce, expires_at_unix, purpose)
            .ok_or_else(unavailable)?;
        let signature = Signer::new_without_digest(&self.authorization_key)
            .and_then(|mut signer| signer.sign_oneshot_to_vec(&authorization))
            .map_err(internal)?;
        #[derive(Serialize)]
        struct Input {
            installation: Uuid,
            tenant: Uuid,
            event: Uuid,
            post: Uuid,
            publication: Uuid,
            request: Uuid,
            actor: Uuid,
            nonce: Uuid,
            reservation_id: Uuid,
            payload_sha256: String,
            expires_at_unix: i64,
            authorization_sig_hex: String,
            action: String,
            purpose: DirectPurpose,
            scope: DirectRequestScope,
            artifact_sha256: String,
        }
        let anchor = operation.anchor();
        let input = Input {
            installation: anchor.installation,
            tenant: anchor.tenant,
            event: anchor.event,
            post: anchor.post,
            publication: anchor.publication,
            request: operation.request(),
            actor: operation.actor(),
            nonce: held.nonce,
            reservation_id: held.id,
            payload_sha256: hex::encode(operation.payload_digest()),
            expires_at_unix,
            authorization_sig_hex: hex::encode(signature),
            action: operation.action().to_string(),
            purpose,
            scope: operation.scope(),
            artifact_sha256: hex::encode(
                operation.artifact_digest(purpose).ok_or_else(unavailable)?,
            ),
        };
        let url = self.url.join(ACTION_DEVICE_ROUTE).map_err(internal)?;
        self.send(url, &input, held.id).await
    }

    async fn send(
        &self,
        url: Url,
        input: &impl Serialize,
        reservation_id: Uuid,
    ) -> std::result::Result<(Vec<u8>, Vec<u8>), SigningFailure> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Proof {
            reservation_id: Uuid,
            profile: String,
            spki_der_hex: String,
            signature_der_hex: String,
        }
        let mut reply = self
            .client
            .post(url)
            .json(input)
            .send()
            .await
            .map_err(|_| unavailable())?;
        if reply.status() != StatusCode::OK || reply.content_length().is_some_and(|n| n > 1024) {
            return Err(unavailable());
        }
        let mut raw = Vec::new();
        while let Some(bytes) = reply.chunk().await.map_err(|_| unavailable())? {
            if raw.len() + bytes.len() > 1024 {
                return Err(unavailable());
            }
            raw.extend_from_slice(&bytes);
        }
        let proof: Proof = serde_json::from_slice(&raw).map_err(|_| unavailable())?;
        if proof.reservation_id != reservation_id || proof.profile != "softhsm-test" {
            return Err(unavailable());
        }
        let spki = decode_lower(&proof.spki_der_hex, 91, 91).ok_or_else(unavailable)?;
        let sig = decode_lower(&proof.signature_der_hex, 8, 72).ok_or_else(unavailable)?;
        Ok((spki, sig))
    }
}

fn unavailable() -> SigningFailure {
    SigningFailure::conflict("Device approval is unavailable or not yet confirmed. Check the request before trying again.")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DirectApprovalBody {
    request_id: String,
    payload_sha256: String,
}

#[rocket::post(
    "/signing-requests/direct-initialize",
    format = "json",
    data = "<body>"
)]
pub(crate) async fn direct_initialize_approval(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    device: &State<DirectDevice>,
    body: Json<DirectApprovalBody>,
) -> SigningReply<RequestIdOutput> {
    let tenant = tenant_of(&claims)?;
    require(&claims, vec![Permissions::SIGN_INITIALIZE_VOTING])?;
    if !device.portal_actor(&claims, tenant) {
        return Err(SigningFailure::forbidden(
            "Not the registered Admin Portal account",
        ));
    }
    let request = uuid_of(&body.request_id, "request_id")?;
    let digest = decode_lower(&body.payload_sha256, 32, 32).ok_or_else(|| {
        SigningFailure::invalid("payload_sha256 must name the reviewed request digest")
    })?;
    let caller = writer(&claims, tenant, services).await?;
    let mut client = hasura(services).await?;

    let tx = client.transaction().await.map_err(internal)?;
    let scope = authorize(&tx, &caller, tenant, request, &body.payload_sha256)
        .await
        .map_err(signing_failure)?;
    if approved(&tx, &scope)
        .await
        .map_err(signing_failure)?
        .is_some()
    {
        return Ok(Json(RequestIdOutput {
            request_id: request,
        }));
    }
    let held=tx.query_opt(
        "SELECT id,nonce,expires_at FROM felix3_initialize_direct.reserve($1,$2,$3,$4,$5,$6,$7,$8)",
        &[&scope.installation(),&scope.tenant(),&scope.event(),&scope.post(),&scope.publication(),
          &scope.request(),&scope.actor(),&digest],
    ).await.map_err(internal)?.ok_or_else(unavailable)?;
    let reservation: Uuid = held.try_get("id").map_err(internal)?;
    let nonce: Uuid = held.try_get("nonce").map_err(internal)?;
    let deadline: DateTime<Utc> = held.try_get("expires_at").map_err(internal)?;
    tx.commit().await.map_err(internal)?;

    let (spki, signature) = device
        .sign(reservation, nonce, deadline, &scope, &body.payload_sha256)
        .await?;
    verify_signature(&scope, nonce, &spki, &signature).map_err(signing_failure)?;
    let tx = client.transaction().await.map_err(internal)?;
    let recorded: bool = tx
        .query_one(
            "SELECT sequent_backend.record_local_initialize_direct_key($1,$2)",
            &[&reservation, &spki],
        )
        .await
        .map_err(internal)?
        .try_get(0)
        .map_err(internal)?;
    if !recorded {
        return Err(unavailable());
    }
    let proof = verify_proof(&tx, &scope, nonce, &signature)
        .await
        .map_err(signing_failure)?;
    tx.commit().await.map_err(internal)?;
    commit(&mut client, &services.signing, &caller, proof, Utc::now())
        .await
        .map_err(signing_failure)?;
    Ok(Json(RequestIdOutput {
        request_id: request,
    }))
}

// The request, JWT, and database policy determine the action and scope.
// Hasura's roles are an additional gate, not authority to choose an action.
#[rocket::post("/signing-requests/direct-action", format = "json", data = "<body>")]
pub(crate) async fn direct_action_approval(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    device: &State<DirectDevice>,
    body: Json<DirectApprovalBody>,
) -> SigningReply<RequestIdOutput> {
    let tenant = tenant_of(&claims)?;
    if !device.portal_actor(&claims, tenant) {
        return Err(SigningFailure::forbidden(
            "Not the registered Admin Portal account",
        ));
    }
    let request = uuid_of(&body.request_id, "request_id")?;
    if decode_lower(&body.payload_sha256, 32, 32).is_none() {
        return Err(SigningFailure::invalid(
            "payload_sha256 must name the reviewed request digest",
        ));
    }
    let caller = writer(&claims, tenant, services).await?;
    let mut client = hasura(services).await?;
    let tx = client.transaction().await.map_err(internal)?;
    let operation = direct_actions::authorize(&tx, &caller, tenant, request, &body.payload_sha256)
        .await
        .map_err(signing_failure)?;
    if direct_actions::already_approved(&tx, &operation)
        .await
        .map_err(signing_failure)?
    {
        return Ok(Json(RequestIdOutput {
            request_id: request,
        }));
    }
    direct_actions::verify_review_bytes(&tx, &operation)
        .await
        .map_err(signing_failure)?;
    let held = direct_actions::reserve(&tx, &operation)
        .await
        .map_err(signing_failure)?;
    tx.commit().await.map_err(internal)?;

    let (spki, signature) = device.sign_action(&held, &operation).await?;
    let tx = client.transaction().await.map_err(internal)?;
    let proof = direct_actions::verify_and_pin(&tx, &operation, held, &spki, &signature)
        .await
        .map_err(signing_failure)?;
    tx.commit().await.map_err(internal)?;
    direct_actions::commit(&mut client, &services.signing, &caller, proof, Utc::now())
        .await
        .map_err(signing_failure)?;
    Ok(Json(RequestIdOutput {
        request_id: request,
    }))
}

pub(crate) fn mount_direct_initialize(rocket: Rocket<Build>) -> Result<Rocket<Build>> {
    Ok(rocket.manage(DirectDevice::load()?).mount(
        "/",
        routes![direct_initialize_approval, direct_action_approval],
    ))
}
