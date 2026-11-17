// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The HTTP API, version 1. Every route takes and returns JSON; binary values
//! are standard base64. The caller is the mTLS identity the connection
//! established, never something the request says about itself.

use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::provider::Algorithm;
use crate::service::{Binding, Service, ServiceError};

/// The authenticated caller's policy name, set per connection.
#[derive(Clone, Debug)]
pub struct CallerName(pub String);

pub fn router(service: Arc<Service>) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/keys", get(list_keys))
        .route("/v1/keys/create", post(create_key))
        .route("/v1/keys/describe", post(describe))
        .route("/v1/keys/rotate", post(rotate))
        .route("/v1/keys/retire", post(retire))
        .route("/v1/keys/destroy", post(destroy))
        .route("/v1/keys/attributes", post(attributes))
        .route("/v1/sign", post(sign))
        .route("/v1/sign-batch", post(sign_batch))
        .route("/v1/ecdh", post(ecdh))
        .route("/v1/encrypt", post(encrypt))
        .route("/v1/decrypt", post(decrypt))
        .route("/v1/random", post(random))
        .with_state(service)
}

pub struct ApiError(ServiceError);

impl From<ServiceError> for ApiError {
    fn from(error: ServiceError) -> Self {
        ApiError(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            ServiceError::Forbidden(_) => StatusCode::FORBIDDEN,
            ServiceError::NotFound(_) => StatusCode::NOT_FOUND,
            ServiceError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ServiceError::Conflict(_) => StatusCode::CONFLICT,
            ServiceError::StepUp(_) => StatusCode::UNAUTHORIZED,
            ServiceError::Device(_) | ServiceError::Audit(_) => StatusCode::SERVICE_UNAVAILABLE,
        };
        // Device and audit errors are logged here, never answered in detail.
        let message = match &self.0 {
            ServiceError::Device(_) | ServiceError::Audit(_) => {
                tracing::error!(error = %self.0, "operation failed");
                "the key service could not complete the operation".to_string()
            }
            other => other.to_string(),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

fn decode(field: &str, value: &str) -> Result<Vec<u8>, ApiError> {
    STANDARD
        .decode(value)
        .map_err(|_| ApiError(ServiceError::BadRequest(format!("{field} is not base64"))))
}

async fn health(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
) -> Json<serde_json::Value> {
    Json(json!({ "caller": caller.0, "provider": service.info() }))
}

async fn list_keys(State(service): State<Arc<Service>>) -> ApiResult<serde_json::Value> {
    Ok(Json(json!({ "keys": service.list()? })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateKey {
    pub key_id: String,
    pub algorithm: Algorithm,
}

async fn create_key(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<CreateKey>,
) -> ApiResult<serde_json::Value> {
    let record = service.create_key(&caller.0, &body.key_id, body.algorithm)?;
    Ok(Json(json!(record)))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyRef {
    pub key_id: String,
    #[serde(default)]
    pub version: Option<u32>,
}

fn version(body: &KeyRef) -> Result<u32, ApiError> {
    body.version
        .ok_or_else(|| ApiError(ServiceError::BadRequest("version is required".to_string())))
}

async fn describe(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<KeyRef>,
) -> ApiResult<serde_json::Value> {
    Ok(Json(json!(service.describe(&caller.0, &body.key_id)?)))
}

async fn rotate(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<KeyRef>,
) -> ApiResult<serde_json::Value> {
    Ok(Json(json!(service.rotate(&caller.0, &body.key_id)?)))
}

async fn retire(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<KeyRef>,
) -> ApiResult<serde_json::Value> {
    let v = version(&body)?;
    Ok(Json(json!(service.retire(&caller.0, &body.key_id, v)?)))
}

async fn destroy(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<KeyRef>,
) -> ApiResult<serde_json::Value> {
    let v = version(&body)?;
    Ok(Json(json!(service.destroy(&caller.0, &body.key_id, v)?)))
}

async fn attributes(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<KeyRef>,
) -> ApiResult<serde_json::Value> {
    let v = version(&body)?;
    Ok(Json(json!(service.attributes(
        &caller.0,
        &body.key_id,
        v
    )?)))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignRequest {
    pub key_id: String,
    pub purpose: String,
    pub message: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub payload_sha256: Option<String>,
    #[serde(default)]
    pub official_token: Option<String>,
}

#[derive(Serialize)]
pub struct SignResponse {
    pub key_id: String,
    pub key_version: u32,
    pub signature: String,
}

async fn sign(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<SignRequest>,
) -> ApiResult<SignResponse> {
    let message = decode("message", &body.message)?;
    let binding = Binding {
        request_id: body.request_id,
        payload_sha256: body.payload_sha256,
        official_token: body.official_token,
    };
    let signed = service
        .sign(&caller.0, &body.key_id, &body.purpose, &message, &binding)
        .await?;
    Ok(Json(SignResponse {
        key_id: body.key_id,
        key_version: signed.key_version,
        signature: STANDARD.encode(signed.signature),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignBatchRequest {
    pub key_id: String,
    pub purpose: String,
    pub messages: Vec<String>,
}

async fn sign_batch(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<SignBatchRequest>,
) -> ApiResult<serde_json::Value> {
    let messages = body
        .messages
        .iter()
        .map(|m| decode("messages", m))
        .collect::<Result<Vec<_>, _>>()?;
    let (version, signatures) = service
        .sign_batch(&caller.0, &body.key_id, &body.purpose, &messages)
        .await?;
    Ok(Json(json!({
        "key_id": body.key_id,
        "key_version": version,
        "signatures": signatures.iter().map(|s| STANDARD.encode(s)).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcdhRequest {
    pub key_id: String,
    pub purpose: String,
    pub peer_point: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub payload_sha256: Option<String>,
}

async fn ecdh(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<EcdhRequest>,
) -> ApiResult<serde_json::Value> {
    let peer = decode("peer_point", &body.peer_point)?;
    let binding = Binding {
        request_id: body.request_id,
        payload_sha256: body.payload_sha256,
        official_token: None,
    };
    let (version, shared) = service
        .ecdh(&caller.0, &body.key_id, &body.purpose, &peer, &binding)
        .await?;
    Ok(Json(
        json!({ "key_version": version, "shared_secret": STANDARD.encode(shared.as_slice()) }),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncryptRequest {
    pub key_id: String,
    pub purpose: String,
    pub plaintext: String,
    #[serde(default)]
    pub aad: String,
}

async fn encrypt(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<EncryptRequest>,
) -> ApiResult<serde_json::Value> {
    let plaintext = zeroize::Zeroizing::new(decode("plaintext", &body.plaintext)?);
    let aad = decode("aad", &body.aad)?;
    let (version, sealed) = service
        .encrypt(&caller.0, &body.key_id, &body.purpose, &plaintext, &aad)
        .await?;
    Ok(Json(
        json!({ "key_version": version, "sealed": STANDARD.encode(sealed) }),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecryptRequest {
    pub key_id: String,
    pub purpose: String,
    pub key_version: u32,
    pub sealed: String,
    #[serde(default)]
    pub aad: String,
}

async fn decrypt(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<DecryptRequest>,
) -> ApiResult<serde_json::Value> {
    let sealed = decode("sealed", &body.sealed)?;
    let aad = decode("aad", &body.aad)?;
    let opened = service
        .decrypt(
            &caller.0,
            &body.key_id,
            &body.purpose,
            body.key_version,
            &sealed,
            &aad,
        )
        .await?;
    Ok(Json(
        json!({ "plaintext": STANDARD.encode(opened.as_slice()) }),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomRequest {
    pub length: usize,
}

async fn random(
    State(service): State<Arc<Service>>,
    Extension(caller): Extension<CallerName>,
    Json(body): Json<RandomRequest>,
) -> ApiResult<serde_json::Value> {
    Ok(Json(
        json!({ "random": STANDARD.encode(service.random(&caller.0, body.length).await?) }),
    ))
}
