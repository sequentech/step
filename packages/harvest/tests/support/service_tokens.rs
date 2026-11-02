// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Service tokens signed the way a tenant realm signs them, for the routes
//! that verify tokens themselves (`/messages/*`).

use crate::test_claims::Claims;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use jsonwebtoken::jwk::{Jwk, JwkSet};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use std::sync::OnceLock;

pub const KID: &str = "realm-signing-key";

/// An RSA key a test realm signs with.
pub struct RealmSigner {
    kid: String,
    encoding: EncodingKey,
    jwk: Jwk,
}

impl RealmSigner {
    pub fn new(kid: &str) -> Self {
        let rsa = openssl::rsa::Rsa::generate(2048).unwrap();
        let encoding =
            EncodingKey::from_rsa_pem(&rsa.private_key_to_pem().unwrap())
                .unwrap();
        let jwk = serde_json::from_value(json!({
            "kty": "RSA", "use": "sig", "alg": "RS256", "kid": kid,
            "n": URL_SAFE_NO_PAD.encode(rsa.n().to_vec()),
            "e": URL_SAFE_NO_PAD.encode(rsa.e().to_vec()),
        }))
        .unwrap();
        Self {
            kid: kid.to_string(),
            encoding,
            jwk,
        }
    }

    /// One signer shared by the tests that only need a valid token.
    pub fn shared() -> &'static Self {
        static SIGNER: OnceLock<RealmSigner> = OnceLock::new();
        SIGNER.get_or_init(|| RealmSigner::new(KID))
    }

    pub fn jwks(&self) -> JwkSet {
        JwkSet {
            keys: vec![self.jwk.clone()],
        }
    }

    /// The `Authorization` value of `payload` signed with this key.
    pub fn sign_payload(&self, payload: &Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.kid.clone());
        format!(
            "Bearer {}",
            encode(&header, payload, &self.encoding).unwrap()
        )
    }

    /// `claims`, issued by `tenant_id`'s realm and valid for an hour.
    pub fn sign(&self, claims: &Claims, tenant_id: &str) -> String {
        self.sign_payload(&issued(claims, tenant_id))
    }
}

/// The payload of `claims`, as issued by the realm of `tenant_id`.
pub fn issued(claims: &Claims, tenant_id: &str) -> Value {
    let bearer = claims.bearer();
    let encoded = bearer.split('.').nth(1).expect("a fixture payload");
    let mut payload: Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(encoded).unwrap())
            .unwrap();
    let now = chrono::Utc::now().timestamp();
    payload["iss"] =
        json!(format!("http://keycloak:8090/realms/tenant-{tenant_id}"));
    payload["iat"] = json!(now);
    payload["exp"] = json!(now + 3600);
    payload
}

pub fn realm(tenant_id: &str) -> String {
    format!("tenant-{tenant_id}")
}
