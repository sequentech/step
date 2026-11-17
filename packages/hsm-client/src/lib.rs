// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The client every component uses to reach the central HSM service. It only
//! carries requests: it holds no key, links no PKCS#11 or KMS library, and
//! does no cryptography beyond computing the SHA-256 that binds a gated
//! signature to its message. Its only credential is the caller's mTLS
//! certificate.

use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("the key service refused ({status}): {message}")]
    Refused { status: u16, message: String },
    #[error("the key service is unreachable: {0}")]
    Transport(String),
    #[error("unexpected answer: {0}")]
    Answer(String),
}

pub type ClientResult<T> = Result<T, ClientError>;

/// A request binding for gated keys: the signing request it belongs to and,
/// for an official's approval, the official's fresh step-up token.
#[derive(Clone, Debug, Default)]
pub struct Binding {
    pub request_id: Option<String>,
    pub official_token: Option<Zeroizing<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Signature {
    pub key_id: String,
    pub key_version: u32,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct KeyVersion {
    pub version: u32,
    pub state: String,
    pub public_key: String,
    pub created_at: String,
    pub retired_at: Option<String>,
    pub destroyed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct KeyRecord {
    pub key_id: String,
    pub algorithm: String,
    pub versions: Vec<KeyVersion>,
}

#[derive(Clone)]
pub struct HsmClient {
    base: String,
    http: reqwest::Client,
}

/// Settings read from `<PREFIX>HSM_URL`, `<PREFIX>HSM_CA`,
/// `<PREFIX>HSM_CLIENT_CERT` and `<PREFIX>HSM_CLIENT_KEY` (PEM files).
pub struct ClientSettings {
    pub url: String,
    pub ca_pem: Vec<u8>,
    pub identity_pem: Zeroizing<Vec<u8>>,
}

impl ClientSettings {
    pub fn from_env(prefix: &str) -> anyhow::Result<Self> {
        let var = |name: &str| {
            std::env::var(format!("{prefix}HSM_{name}"))
                .map_err(|_| anyhow::anyhow!("{prefix}HSM_{name} is not set"))
        };
        let mut identity = Zeroizing::new(std::fs::read(var("CLIENT_CERT")?)?);
        identity.extend_from_slice(b"\n");
        identity.extend_from_slice(&Zeroizing::new(std::fs::read(var("CLIENT_KEY")?)?));
        Ok(ClientSettings {
            url: var("URL")?,
            ca_pem: std::fs::read(var("CA")?)?,
            identity_pem: identity,
        })
    }
}

fn transport(error: reqwest::Error) -> ClientError {
    ClientError::Transport(error.without_url().to_string())
}

fn bytes(value: &Value, field: &str) -> ClientResult<Vec<u8>> {
    let text = value[field]
        .as_str()
        .ok_or_else(|| ClientError::Answer(format!("no {field}")))?;
    STANDARD
        .decode(text)
        .map_err(|_| ClientError::Answer(format!("{field} is not base64")))
}

fn version(value: &Value) -> ClientResult<u32> {
    value["key_version"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| ClientError::Answer("no key_version".to_string()))
}

impl HsmClient {
    pub fn new(settings: &ClientSettings) -> ClientResult<Self> {
        let ca = reqwest::Certificate::from_pem(&settings.ca_pem).map_err(transport)?;
        let identity = reqwest::Identity::from_pem(&settings.identity_pem).map_err(transport)?;
        let http = reqwest::Client::builder()
            .use_rustls_tls()
            .tls_built_in_root_certs(false)
            .add_root_certificate(ca)
            .identity(identity)
            .https_only(true)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(transport)?;
        Ok(HsmClient {
            base: settings.url.trim_end_matches('/').to_string(),
            http,
        })
    }

    async fn call(&self, path: &str, body: Option<Value>) -> ClientResult<Value> {
        let url = format!("{}{path}", self.base);
        let request = match body {
            Some(body) => self.http.post(url).json(&body),
            None => self.http.get(url),
        };
        let response = request.send().await.map_err(transport)?;
        let status = response.status();
        let value: Value = response.json().await.map_err(transport)?;
        if !status.is_success() {
            return Err(ClientError::Refused {
                status: status.as_u16(),
                message: value["error"].as_str().unwrap_or_default().to_string(),
            });
        }
        Ok(value)
    }

    pub async fn health(&self) -> ClientResult<Value> {
        self.call("/v1/health", None).await
    }

    pub async fn create_key(&self, key_id: &str, algorithm: &str) -> ClientResult<KeyRecord> {
        let value = self
            .call(
                "/v1/keys/create",
                Some(json!({"key_id": key_id, "algorithm": algorithm})),
            )
            .await?;
        serde_json::from_value(value).map_err(|e| ClientError::Answer(e.to_string()))
    }

    pub async fn describe(&self, key_id: &str) -> ClientResult<KeyRecord> {
        let value = self
            .call("/v1/keys/describe", Some(json!({"key_id": key_id})))
            .await?;
        serde_json::from_value(value).map_err(|e| ClientError::Answer(e.to_string()))
    }

    pub async fn rotate(&self, key_id: &str) -> ClientResult<KeyRecord> {
        let value = self
            .call("/v1/keys/rotate", Some(json!({"key_id": key_id})))
            .await?;
        serde_json::from_value(value).map_err(|e| ClientError::Answer(e.to_string()))
    }

    pub async fn retire(&self, key_id: &str, version: u32) -> ClientResult<KeyRecord> {
        let value = self
            .call(
                "/v1/keys/retire",
                Some(json!({"key_id": key_id, "version": version})),
            )
            .await?;
        serde_json::from_value(value).map_err(|e| ClientError::Answer(e.to_string()))
    }

    pub async fn destroy(&self, key_id: &str, version: u32) -> ClientResult<KeyRecord> {
        let value = self
            .call(
                "/v1/keys/destroy",
                Some(json!({"key_id": key_id, "version": version})),
            )
            .await?;
        serde_json::from_value(value).map_err(|e| ClientError::Answer(e.to_string()))
    }

    pub async fn attributes(&self, key_id: &str, version: u32) -> ClientResult<Value> {
        self.call(
            "/v1/keys/attributes",
            Some(json!({"key_id": key_id, "version": version})),
        )
        .await
    }

    /// Signs the exact bytes of `message`; see the service for each
    /// algorithm's signature format.
    pub async fn sign(
        &self,
        key_id: &str,
        purpose: &str,
        message: &[u8],
        binding: &Binding,
    ) -> ClientResult<Signature> {
        let mut body =
            json!({"key_id": key_id, "purpose": purpose, "message": STANDARD.encode(message)});
        if let Some(request_id) = &binding.request_id {
            body["request_id"] = json!(request_id);
            body["payload_sha256"] = json!(hex::encode(Sha256::digest(message)));
        }
        if let Some(token) = &binding.official_token {
            body["official_token"] = json!(token.as_str());
        }
        let value = self.call("/v1/sign", Some(body)).await?;
        Ok(Signature {
            key_id: key_id.to_string(),
            key_version: version(&value)?,
            signature: bytes(&value, "signature")?,
        })
    }

    /// One signature per message, each exactly as [`HsmClient::sign`] gives.
    pub async fn sign_batch(
        &self,
        key_id: &str,
        purpose: &str,
        messages: &[Vec<u8>],
    ) -> ClientResult<(u32, Vec<Vec<u8>>)> {
        let body = json!({
            "key_id": key_id,
            "purpose": purpose,
            "messages": messages.iter().map(|m| STANDARD.encode(m)).collect::<Vec<_>>(),
        });
        let value = self.call("/v1/sign-batch", Some(body)).await?;
        let signatures = value["signatures"]
            .as_array()
            .ok_or_else(|| ClientError::Answer("no signatures".to_string()))?
            .iter()
            .map(|s| {
                s.as_str()
                    .and_then(|s| STANDARD.decode(s).ok())
                    .ok_or_else(|| ClientError::Answer("a signature is not base64".to_string()))
            })
            .collect::<ClientResult<Vec<_>>>()?;
        if signatures.len() != messages.len() {
            return Err(ClientError::Answer(
                "one signature per message expected".to_string(),
            ));
        }
        Ok((version(&value)?, signatures))
    }

    pub async fn ecdh(
        &self,
        key_id: &str,
        purpose: &str,
        peer_point: &[u8],
        binding: &Binding,
    ) -> ClientResult<(u32, Zeroizing<Vec<u8>>)> {
        let mut body = json!({"key_id": key_id, "purpose": purpose, "peer_point": STANDARD.encode(peer_point)});
        if let Some(request_id) = &binding.request_id {
            body["request_id"] = json!(request_id);
            body["payload_sha256"] = json!(hex::encode(Sha256::digest(peer_point)));
        }
        let value = self.call("/v1/ecdh", Some(body)).await?;
        Ok((
            version(&value)?,
            Zeroizing::new(bytes(&value, "shared_secret")?),
        ))
    }

    pub async fn encrypt(
        &self,
        key_id: &str,
        purpose: &str,
        plaintext: &[u8],
        aad: &[u8],
    ) -> ClientResult<(u32, Vec<u8>)> {
        let body = json!({"key_id": key_id, "purpose": purpose, "plaintext": STANDARD.encode(plaintext), "aad": STANDARD.encode(aad)});
        let value = self.call("/v1/encrypt", Some(body)).await?;
        Ok((version(&value)?, bytes(&value, "sealed")?))
    }

    pub async fn decrypt(
        &self,
        key_id: &str,
        purpose: &str,
        key_version: u32,
        sealed: &[u8],
        aad: &[u8],
    ) -> ClientResult<Zeroizing<Vec<u8>>> {
        let body = json!({"key_id": key_id, "purpose": purpose, "key_version": key_version, "sealed": STANDARD.encode(sealed), "aad": STANDARD.encode(aad)});
        let value = self.call("/v1/decrypt", Some(body)).await?;
        Ok(Zeroizing::new(bytes(&value, "plaintext")?))
    }

    pub async fn random(&self, length: usize) -> ClientResult<Vec<u8>> {
        let value = self
            .call("/v1/random", Some(json!({"length": length})))
            .await?;
        bytes(&value, "random")
    }
}
