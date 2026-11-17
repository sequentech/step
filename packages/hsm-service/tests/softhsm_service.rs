// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The whole service on SoftHSM, over real mTLS, through the transport-only
//! client. SoftHSM is a software token: passing here says nothing about a
//! YubiHSM 2, which has its own acceptance run on the device.
//!
//! Runs when `HSM_SERVICE_TEST_PKCS11_MODULE` names SoftHSM's module
//! (`/usr/lib/softhsm/libsofthsm2.so`); skipped otherwise.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::session::UserType;
use cryptoki::types::AuthPin;
use hsm_client::{Binding, ClientError, ClientSettings, HsmClient};
use hsm_service::audit::{self, AuditLog};
use hsm_service::officials::StepUpVerifier;
use hsm_service::pkcs11::{Pkcs11Backend, Pkcs11Provider};
use hsm_service::policy::Policy;
use hsm_service::registry::Registry;
use hsm_service::service::Service;
use hsm_service::{server, tls};
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair};
use serde_json::json;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const TOKEN: &str = "meta15035-test";
const USER_PIN: &str = "test-user-pin";
const ISSUER: &str = "https://ems.test/realms/tenant-test";
const AUDIENCE: &str = "hsm-service";

fn module() -> Option<String> {
    std::env::var("HSM_SERVICE_TEST_PKCS11_MODULE").ok()
}

/// A fresh SoftHSM token in its own directory.
fn init_token(dir: &Path, module: &str) {
    let tokens = dir.join("tokens");
    std::fs::create_dir_all(&tokens).unwrap();
    let conf = dir.join("softhsm2.conf");
    std::fs::write(
        &conf,
        format!(
            "directories.tokendir = {}\nobjectstore.backend = file\n",
            tokens.display()
        ),
    )
    .unwrap();
    std::env::set_var("SOFTHSM2_CONF", &conf);
    let pkcs11 = Pkcs11::new(module).unwrap();
    pkcs11
        .initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
        .unwrap();
    let slot = pkcs11.get_slots_with_token().unwrap()[0];
    let so = AuthPin::new("test-so-pin".into());
    pkcs11.init_token(slot, &so, TOKEN).unwrap();
    let slot = pkcs11
        .get_slots_with_token()
        .unwrap()
        .into_iter()
        .find(|s| {
            pkcs11
                .get_token_info(*s)
                .map(|i| i.label().trim() == TOKEN)
                .unwrap_or(false)
        })
        .unwrap();
    let session = pkcs11.open_rw_session(slot).unwrap();
    session.login(UserType::So, Some(&so)).unwrap();
    session.init_pin(&AuthPin::new(USER_PIN.into())).unwrap();
    session.logout().unwrap();
    drop(session);
    pkcs11.finalize().unwrap();
}

struct Pki {
    ca_pem: String,
    server_pem: String,
    server_key: String,
    callers: Vec<(String, String, String, String)>, // name, cert, key, spki sha256
}

fn pki(names: &[&str]) -> Pki {
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "test caller CA");
    let ca = ca_params.self_signed(&ca_key).unwrap();
    let issuer = Issuer::from_params(&ca_params, &ca_key);
    let server_key = KeyPair::generate().unwrap();
    let server = CertificateParams::new(vec!["localhost".to_string()])
        .unwrap()
        .signed_by(&server_key, &issuer)
        .unwrap();
    let callers = names
        .iter()
        .map(|name| {
            let key = KeyPair::generate().unwrap();
            let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
            params.distinguished_name.push(DnType::CommonName, *name);
            let cert = params.signed_by(&key, &issuer).unwrap();
            let spki = tls::spki_sha256(cert.der()).unwrap();
            (name.to_string(), cert.pem(), key.serialize_pem(), spki)
        })
        .collect();
    Pki {
        ca_pem: ca.pem(),
        server_pem: server.pem(),
        server_key: server_key.serialize_pem(),
        callers,
    }
}

fn client(pki: &Pki, name: &str, url: &str) -> HsmClient {
    let (_, cert, key, _) = pki.callers.iter().find(|c| c.0 == name).unwrap();
    let identity = Zeroizing::new(format!("{cert}\n{key}").into_bytes());
    HsmClient::new(&ClientSettings {
        url: url.to_string(),
        ca_pem: pki.ca_pem.clone().into_bytes(),
        identity_pem: identity,
    })
    .unwrap()
}

struct Realm {
    key: jsonwebtoken::EncodingKey,
    jwks: jsonwebtoken::jwk::JwkSet,
}

fn realm() -> Realm {
    let key = KeyPair::generate().unwrap();
    let raw = key.public_key_raw();
    let jwks: jsonwebtoken::jwk::JwkSet = serde_json::from_value(json!({"keys": [{
        "kty": "EC", "crv": "P-256", "kid": "realm-key", "alg": "ES256", "use": "sig",
        "x": URL_SAFE_NO_PAD.encode(&raw[1..33]), "y": URL_SAFE_NO_PAD.encode(&raw[33..65]),
    }]}))
    .unwrap();
    Realm {
        key: jsonwebtoken::EncodingKey::from_ec_pem(key.serialize_pem().as_bytes()).unwrap(),
        jwks,
    }
}

fn token(realm: &Realm, sub: &str, jti: &str, auth_age: i64, acr: &str) -> Zeroizing<String> {
    let now = chrono::Utc::now().timestamp();
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
    header.kid = Some("realm-key".to_string());
    let claims = json!({
        "iss": ISSUER, "aud": AUDIENCE, "sub": sub, "jti": jti, "exp": now + 300,
        "iat": now, "auth_time": now - auth_age, "acr": acr, "tenant-id": "tenant-test",
    });
    Zeroizing::new(jsonwebtoken::encode(&header, &claims, &realm.key).unwrap())
}

fn refused(result: Result<impl std::fmt::Debug, ClientError>) -> u16 {
    match result {
        Err(ClientError::Refused { status, .. }) => status,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_service_on_softhsm() {
    let Some(module) = module() else {
        eprintln!("HSM_SERVICE_TEST_PKCS11_MODULE is not set: skipped");
        return;
    };
    let _ = rustls::crypto::ring::default_provider().install_default();
    let dir = tempfile::tempdir().unwrap();
    init_token(dir.path(), &module);

    // A YubiHSM backend refuses the SoftHSM module: no silent stand-in.
    let wrong = Pkcs11Provider::open(
        Pkcs11Backend::YubiHsm,
        Path::new(&module),
        TOKEN,
        Zeroizing::new(USER_PIN.to_string()),
    );
    assert!(
        wrong.is_err(),
        "a SoftHSM module must not pass as a YubiHSM"
    );

    let provider = Pkcs11Provider::open(
        Pkcs11Backend::SoftHsm,
        Path::new(&module),
        TOKEN,
        Zeroizing::new(USER_PIN.to_string()),
    )
    .unwrap();
    let info = hsm_service::provider::Provider::info(&provider);
    assert_eq!(info.backend, "softhsm");
    assert!(!info.hardware, "SoftHSM is never reported as hardware");

    let pki = pki(&["windmill", "admin", "stranger", "executor"]);
    let pin = |name: &str| pki.callers.iter().find(|c| c.0 == name).unwrap().3.clone();
    let policy = Policy::parse(
        &json!({
            "callers": [
                {"name": "windmill", "spki_sha256": pin("windmill")},
                {"name": "admin", "spki_sha256": pin("admin")},
                {"name": "executor", "spki_sha256": pin("executor")}
            ],
            "key_administrators": ["admin"],
            "keys": [
                {"key_id": "log/tenant-test", "algorithm": "ed25519", "operations": ["sign"],
                 "purposes": ["electoral-log-entry", "electoral-log-checkpoint"], "users": ["windmill"],
                 "gate": {"type": "none"}},
                {"key_id": "official/*", "algorithm": "ecdsa-p256", "operations": ["sign"],
                 "purposes": ["step-signing-approval"], "users": ["windmill"],
                 "gate": {"type": "official-approval", "key_id_prefix": "official/"}},
                {"key_id": "transmission/device-1", "algorithm": "ecdsa-p256", "operations": ["sign"],
                 "purposes": ["transmission-package"], "users": ["windmill", "executor"],
                 "gate": {"type": "completed-execution", "executors": ["executor"]}},
                {"key_id": "messaging/hmac", "algorithm": "hmac-sha256", "operations": ["sign"],
                 "purposes": ["messaging-hmac"], "users": ["windmill"], "gate": {"type": "none"}},
                {"key_id": "vault/master-wrap", "algorithm": "aes256-gcm", "operations": ["encrypt", "decrypt"],
                 "purposes": ["vault-master-secret"], "users": ["windmill"], "gate": {"type": "none"}},
                {"key_id": "ccs/ecdh", "algorithm": "ecdh-p256", "operations": ["ecdh"],
                 "purposes": ["transmission-seal"], "users": ["windmill"], "gate": {"type": "none"}}
            ],
            "officials": {"issuer": ISSUER, "jwks_url": "https://unused.invalid/certs", "audience": AUDIENCE,
                          "max_auth_age_seconds": 60, "acr_values": ["gold"], "tenant_id": "tenant-test"}
        })
        .to_string(),
    )
    .unwrap();
    let realm = realm();
    let verifier = StepUpVerifier::with_keys(policy.officials.clone().unwrap(), realm.jwks.clone());
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let audit_path = state.join("audit.jsonl");
    let service = Arc::new(Service::new(
        policy,
        Arc::new(provider),
        Registry::open(&state.join("keys.json")).unwrap(),
        AuditLog::open(&audit_path).unwrap(),
        Some(verifier),
    ));
    let files = dir.path().join("pki");
    std::fs::create_dir_all(&files).unwrap();
    std::fs::write(files.join("ca.pem"), &pki.ca_pem).unwrap();
    std::fs::write(files.join("server.pem"), &pki.server_pem).unwrap();
    std::fs::write(files.join("server.key"), &pki.server_key).unwrap();
    let tls = tls::server_config(
        &files.join("server.pem"),
        &files.join("server.key"),
        &files.join("ca.pem"),
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    tokio::spawn(server::serve(listener, tls, service.clone()));

    let admin = client(&pki, "admin", &url);
    let windmill = client(&pki, "windmill", &url);
    let executor = client(&pki, "executor", &url);
    let stranger = client(&pki, "stranger", &url);

    let health = windmill.health().await.unwrap();
    assert_eq!(health["caller"], "windmill");
    assert_eq!(health["provider"]["hardware"], false);
    assert!(
        stranger.health().await.is_err(),
        "a certificate the policy doesn't name gets no API"
    );

    // Only administrators create keys; keys need a policy rule.
    assert_eq!(
        refused(windmill.create_key("log/tenant-test", "ed25519").await),
        403
    );
    assert_eq!(refused(admin.create_key("unlisted", "ed25519").await), 403);
    assert_eq!(
        refused(admin.create_key("log/tenant-test", "ecdsa-p256").await),
        400
    );
    let log_key = admin
        .create_key("log/tenant-test", "ed25519")
        .await
        .unwrap();
    for (id, alg) in [
        ("official/alice", "ecdsa-p256"),
        ("official/bob", "ecdsa-p256"),
        ("transmission/device-1", "ecdsa-p256"),
        ("messaging/hmac", "hmac-sha256"),
        ("vault/master-wrap", "aes256-gcm"),
        ("ccs/ecdh", "ecdh-p256"),
    ] {
        admin.create_key(id, alg).await.unwrap();
    }

    // Keys are generated in the token and can't leave it.
    let attributes = admin.attributes("log/tenant-test", 1).await.unwrap();
    assert_eq!(attributes["extractable"], false);
    assert_eq!(attributes["sensitive"], true);
    assert_eq!(attributes["never_extractable"], true);
    assert_eq!(attributes["local"], true);
    assert_eq!(
        refused(windmill.attributes("log/tenant-test", 1).await),
        403
    );

    // Ed25519 from the HSM verifies with the platform's existing verifier,
    // over the exact message bytes.
    let v1 = hex::decode(&log_key.versions[0].public_key).unwrap();
    let verifier_v1 =
        voting_crypto::signing::VerificationKey::from_bytes(v1.clone().try_into().unwrap())
            .unwrap();
    let entry = b"exact electoral log entry bytes".to_vec();
    let signed = windmill
        .sign(
            "log/tenant-test",
            "electoral-log-entry",
            &entry,
            &Binding::default(),
        )
        .await
        .unwrap();
    assert_eq!(signed.key_version, 1);
    let signature =
        voting_crypto::signing::Signature::from_bytes(signed.signature.clone().try_into().unwrap())
            .unwrap();
    verifier_v1.verify(&signature, &entry).unwrap();
    assert!(verifier_v1.verify(&signature, b"another message").is_err());

    // Purpose, caller and operation are each enforced.
    assert_eq!(
        refused(
            windmill
                .sign("log/tenant-test", "ballot", &entry, &Binding::default())
                .await
        ),
        403
    );
    assert_eq!(
        refused(
            executor
                .sign(
                    "log/tenant-test",
                    "electoral-log-entry",
                    &entry,
                    &Binding::default()
                )
                .await
        ),
        403
    );
    assert_eq!(
        refused(
            windmill
                .encrypt("log/tenant-test", "electoral-log-entry", b"x", b"")
                .await
        ),
        403
    );

    // Per-entry signatures in a batch are exactly the single signatures.
    let entries: Vec<Vec<u8>> = (0..200)
        .map(|i| format!("entry {i}").into_bytes())
        .collect();
    let started = Instant::now();
    let (batch_version, batch) = windmill
        .sign_batch("log/tenant-test", "electoral-log-entry", &entries)
        .await
        .unwrap();
    let batch_rate = entries.len() as f64 / started.elapsed().as_secs_f64();
    assert_eq!(batch_version, 1);
    for (message, signature) in entries.iter().zip(&batch) {
        let signature =
            voting_crypto::signing::Signature::from_bytes(signature.clone().try_into().unwrap())
                .unwrap();
        verifier_v1.verify(&signature, message).unwrap();
    }
    let started = Instant::now();
    for message in entries.iter().take(50) {
        windmill
            .sign(
                "log/tenant-test",
                "electoral-log-entry",
                message,
                &Binding::default(),
            )
            .await
            .unwrap();
    }
    let single_rate = 50.0 / started.elapsed().as_secs_f64();
    eprintln!("SOFTHSM (no hardware claim) Ed25519 per-entry signing: batch {batch_rate:.0}/s, single requests {single_rate:.0}/s");

    // Rotation: v2 signs, v1 is retired, v1 signatures still verify.
    let rotated = admin.rotate("log/tenant-test").await.unwrap();
    assert_eq!(rotated.versions.len(), 2);
    assert_eq!(rotated.versions[0].state, "retired");
    assert_eq!(rotated.versions[1].state, "active");
    let after = windmill
        .sign(
            "log/tenant-test",
            "electoral-log-checkpoint",
            &entry,
            &Binding::default(),
        )
        .await
        .unwrap();
    assert_eq!(after.key_version, 2);
    let v2 = hex::decode(&rotated.versions[1].public_key).unwrap();
    let verifier_v2 =
        voting_crypto::signing::VerificationKey::from_bytes(v2.try_into().unwrap()).unwrap();
    let after_signature =
        voting_crypto::signing::Signature::from_bytes(after.signature.try_into().unwrap()).unwrap();
    verifier_v2.verify(&after_signature, &entry).unwrap();
    verifier_v1.verify(&signature, &entry).unwrap();
    // Active versions can't be destroyed; retired ones can, and stay listed.
    assert_eq!(refused(admin.destroy("log/tenant-test", 2).await), 409);
    let destroyed = admin.destroy("log/tenant-test", 1).await.unwrap();
    assert_eq!(destroyed.versions[0].state, "destroyed");
    assert_eq!(
        destroyed.versions[0].public_key,
        log_key.versions[0].public_key
    );

    // Official approvals: the official's own fresh step-up token, their own
    // key, the request named, no completed quorum needed, one use per token.
    let approval = b"{\"domain\":\"step-signing/v1\",\"request_id\":\"r-1\"}".to_vec();
    let bound = |token: Option<Zeroizing<String>>| Binding {
        request_id: Some("r-1".to_string()),
        official_token: token,
    };
    assert_eq!(
        refused(
            windmill
                .sign(
                    "official/alice",
                    "step-signing-approval",
                    &approval,
                    &bound(None)
                )
                .await
        ),
        401
    );
    assert_eq!(
        refused(
            windmill
                .sign(
                    "official/alice",
                    "step-signing-approval",
                    &approval,
                    &Binding {
                        request_id: None,
                        official_token: Some(token(&realm, "alice", "t0", 5, "gold"))
                    }
                )
                .await
        ),
        400
    );
    assert_eq!(
        refused(
            windmill
                .sign(
                    "official/alice",
                    "step-signing-approval",
                    &approval,
                    &bound(Some(token(&realm, "alice", "t1", 600, "gold")))
                )
                .await
        ),
        401,
        "a stale login is refused"
    );
    assert_eq!(
        refused(
            windmill
                .sign(
                    "official/alice",
                    "step-signing-approval",
                    &approval,
                    &bound(Some(token(&realm, "alice", "t2", 5, "silver")))
                )
                .await
        ),
        401,
        "a login without step-up is refused"
    );
    assert_eq!(
        refused(
            windmill
                .sign(
                    "official/alice",
                    "step-signing-approval",
                    &approval,
                    &bound(Some(token(&realm, "bob", "t3", 5, "gold")))
                )
                .await
        ),
        403,
        "bob can't use alice's key"
    );
    let alice_token = token(&realm, "alice", "t4", 5, "gold");
    let approved = windmill
        .sign(
            "official/alice",
            "step-signing-approval",
            &approval,
            &bound(Some(alice_token.clone())),
        )
        .await
        .unwrap();
    let alice = admin.describe("official/alice").await.unwrap();
    {
        use p256::ecdsa::signature::Verifier;
        let key = p256::ecdsa::VerifyingKey::from_sec1_bytes(
            &hex::decode(&alice.versions[0].public_key).unwrap(),
        )
        .unwrap();
        let signature = p256::ecdsa::Signature::try_from(approved.signature.as_slice()).unwrap();
        key.verify(&approval, &signature).unwrap();
        let der = hsm_service::formats::ecdsa_raw_to_der(&approved.signature).unwrap();
        let from_der = p256::ecdsa::Signature::from_der(&der).unwrap();
        key.verify(&approval, &from_der).unwrap();
    }
    assert_eq!(
        refused(
            windmill
                .sign(
                    "official/alice",
                    "step-signing-approval",
                    &approval,
                    &bound(Some(alice_token))
                )
                .await
        ),
        401,
        "a step-up token signs once"
    );

    // Completed-execution keys: only the executor, naming the request.
    let package = b"transmission package".to_vec();
    let execution = Binding {
        request_id: Some("r-9".to_string()),
        official_token: None,
    };
    assert_eq!(
        refused(
            windmill
                .sign(
                    "transmission/device-1",
                    "transmission-package",
                    &package,
                    &execution
                )
                .await
        ),
        403
    );
    assert_eq!(
        refused(
            executor
                .sign(
                    "transmission/device-1",
                    "transmission-package",
                    &package,
                    &Binding::default()
                )
                .await
        ),
        400
    );
    executor
        .sign(
            "transmission/device-1",
            "transmission-package",
            &package,
            &execution,
        )
        .await
        .unwrap();
    assert_eq!(
        refused(
            windmill
                .sign_batch(
                    "official/alice",
                    "step-signing-approval",
                    std::slice::from_ref(&approval)
                )
                .await
        ),
        403,
        "gated keys can't batch"
    );

    // HMAC, sealing and ECDH.
    let mac = windmill
        .sign(
            "messaging/hmac",
            "messaging-hmac",
            b"otp",
            &Binding::default(),
        )
        .await
        .unwrap();
    assert_eq!(mac.signature.len(), 32);
    let (version, sealed) = windmill
        .encrypt(
            "vault/master-wrap",
            "vault-master-secret",
            b"master secret",
            b"deployment-a",
        )
        .await
        .unwrap();
    let opened = windmill
        .decrypt(
            "vault/master-wrap",
            "vault-master-secret",
            version,
            &sealed,
            b"deployment-a",
        )
        .await
        .unwrap();
    assert_eq!(opened.as_slice(), b"master secret");
    assert!(windmill
        .decrypt(
            "vault/master-wrap",
            "vault-master-secret",
            version,
            &sealed,
            b"deployment-b"
        )
        .await
        .is_err());
    let peer = p256::SecretKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
    let peer_point = p256::EncodedPoint::from(peer.public_key());
    let (_, shared) = windmill
        .ecdh(
            "ccs/ecdh",
            "transmission-seal",
            peer_point.as_bytes(),
            &Binding::default(),
        )
        .await
        .unwrap();
    let ccs = admin.describe("ccs/ecdh").await.unwrap();
    let ccs_public =
        p256::PublicKey::from_sec1_bytes(&hex::decode(&ccs.versions[0].public_key).unwrap())
            .unwrap();
    let expected = p256::ecdh::diffie_hellman(peer.to_nonzero_scalar(), ccs_public.as_affine());
    assert_eq!(shared.as_slice(), expected.raw_secret_bytes().as_slice());

    let random = windmill.random(32).await.unwrap();
    assert_eq!(random.len(), 32);
    assert_eq!(refused(windmill.random(4096).await), 400);

    // Every operation, refused or not, is in the audit chain; messages only
    // as digests.
    let (records, _) = audit::verify(&audit_path).unwrap();
    assert!(records > 20);
    let text = std::fs::read_to_string(&audit_path).unwrap();
    assert!(!text.contains("exact electoral log entry bytes"));
    assert!(text.contains(&hex::encode(Sha256::digest(&entry))));
    assert!(text.contains("unknown:"));
    assert!(text.contains("\"official\":\"alice\""));
}
