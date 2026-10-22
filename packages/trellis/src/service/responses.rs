use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::service::metrics::HttpMetricsSnapshot;
use crate::{ConsistencyProof, InclusionProof};

/// Unified API response wrapper
///
/// All endpoints return either Success(data) or `Error(error_info)`
/// This ensures consistent JSON structure across the API
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ApiResponse<T> {
    /// Successful response with data
    #[serde(rename = "ok")]
    Success(T),
    /// Error response with message and optional error type
    Error {
        /// Human-readable error message
        error: String,
        /// Machine-readable error type for client handling
        #[serde(skip_serializing_if = "Option::is_none")]
        error_type: Option<String>,
    },
}

impl<T: Serialize> ApiResponse<T> {
    /// Creates a successful response
    pub const fn success(data: T) -> Self {
        Self::Success(data)
    }

    /// Creates an error response with just a message
    pub fn error(error: impl Into<String>) -> Self {
        Self::Error {
            error: error.into(),
            error_type: None,
        }
    }

    /// Creates an error response with both message and error type
    pub fn error_with_type(error: impl Into<String>, error_type: impl Into<String>) -> Self {
        Self::Error {
            error: error.into(),
            error_type: Some(error_type.into()),
        }
    }
}

/// Convert `ApiResponse` to Axum response with appropriate status code
impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiResponse::Success(_) => StatusCode::OK,
            ApiResponse::Error { .. } => StatusCode::BAD_REQUEST,
        };

        (status, Json(self)).into_response()
    }
}

/// Typed error responses for different failure scenarios
#[derive(Debug, Clone)]
pub enum ApiError {
    /// The requested log does not exist
    LogNotFound(String),
    /// The log exists but has no entries yet
    EmptyTree(String),
    /// The request parameters are invalid
    InvalidRequest(String),
    /// Failed to generate the requested proof
    ProofGenerationFailed(String),
    /// The generated proof failed verification
    ProofVerificationFailed(String),
}

impl ApiError {
    /// Convert error to `ApiResponse` with appropriate error message
    #[must_use]
    pub fn to_response<T: Serialize>(self) -> ApiResponse<T> {
        match self {
            ApiError::LogNotFound(log_name) => {
                ApiResponse::error_with_type(format!("Log '{log_name}' not found"), "log_not_found")
            }
            ApiError::EmptyTree(log_name) => ApiResponse::error_with_type(
                format!("Merkle tree for log '{log_name}' is empty"),
                "empty_tree",
            ),
            ApiError::InvalidRequest(msg) => ApiResponse::error_with_type(msg, "invalid_request"),
            ApiError::ProofGenerationFailed(msg) => {
                ApiResponse::error_with_type(msg, "proof_generation_failed")
            }
            ApiError::ProofVerificationFailed(msg) => {
                ApiResponse::error_with_type(msg, "proof_verification_failed")
            }
        }
    }
}

/// Response for GET `/logs/{log_name}/root`
#[derive(Debug, Serialize, Deserialize)]
pub struct RootResponse {
    /// Name of the log
    pub log_name: String,
    /// Base64-encoded merkle root hash
    pub merkle_root: String,
    /// Number of leaves in the tree
    pub tree_size: u64,
}

impl RootResponse {
    /// Creates a `RootResponse` from a `MerkleState`
    #[must_use]
    pub fn from_merkle_state(log_name: String, state: &crate::service::state::MerkleState) -> Self {
        Self {
            log_name,
            merkle_root: base64::engine::general_purpose::STANDARD.encode(state.tree.root()),
            tree_size: state.tree.len(),
        }
    }
}

/// Response for GET `/logs/{log_name}/size`
#[derive(Debug, Serialize, Deserialize)]
pub struct SizeResponse {
    /// Name of the log
    pub log_name: String,
    /// Number of entries in the log
    pub size: u64,
}

/// Response for GET `/logs/{log_name}/proof` (inclusion proof)
#[derive(Debug, Serialize, Deserialize)]
pub struct InclusionProofResponse {
    /// Name of the log
    pub log_name: String,
    /// The inclusion proof
    pub proof: InclusionProof,
}

/// Response for GET `/logs/{log_name}/consistency` (consistency proof)
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsistencyProofResponse {
    /// Name of the log
    pub log_name: String,
    /// The consistency proof
    pub proof: ConsistencyProof,
}

/// Response for GET `/logs/{log_name}/has_leaf`
#[derive(Debug, Serialize, Deserialize)]
pub struct HasLeafResponse {
    /// Name of the log
    pub log_name: String,
    /// Whether the leaf exists in the log
    pub exists: bool,
}

/// Response for GET `/logs/{log_name}/has_root`
#[derive(Debug, Serialize, Deserialize)]
pub struct HasRootResponse {
    /// Name of the log
    pub log_name: String,
    /// Whether the root exists in the log's history
    pub exists: bool,
}

/// Response for GET `/logs/{log_name}/exists`
#[derive(Debug, Serialize, Deserialize)]
pub struct HasLogResponse {
    /// Name of the log
    pub log_name: String,
    /// Whether the log exists on the server
    pub exists: bool,
}

/// Per-log metrics snapshot
#[derive(Debug, Serialize, Deserialize)]
pub struct LogMetricsResponse {
    /// Number of rows in last batch
    pub last_batch_rows: u64,
    /// Number of leaves added in last batch
    pub last_batch_leaves: u64,
    /// Total processing time (ms)
    pub last_total_ms: u64,
    /// Time to copy source rows (ms)
    pub last_copy_ms: u64,
    /// Time for INSERT operation (ms) - includes CTE query, sort, and insert
    pub last_insert_ms: u64,
    /// Time to fetch from `merkle_log` (ms)
    pub last_fetch_merkle_log_ms: u64,
    /// Time to update merkle tree (ms)
    pub last_tree_update_ms: u64,
    /// Total batches processed
    pub total_batches: u64,
    /// Current tree size
    pub tree_size: u64,
    /// Estimated memory usage (bytes)
    pub tree_memory_bytes: u64,
    /// ISO8601 timestamp of last update
    pub last_update: String,
}

/// Global metrics snapshot
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalMetricsResponse {
    /// Duration of last processing cycle (ms)
    pub last_cycle_duration_ms: u64,
    /// Number of active logs in last cycle
    pub last_active_log_count: usize,
    /// Cycle duration as fraction of interval
    pub last_cycle_fraction: f64,
}

/// Response for GET /metrics
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    /// Per-log metrics keyed by log name
    pub logs: HashMap<String, LogMetricsResponse>,
    /// Global processing metrics
    pub global: GlobalMetricsResponse,
    /// HTTP endpoint metrics (optional for backwards compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpMetricsSnapshot>,
}
