use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};

use crate::service::responses::{
    ApiError, ApiResponse, ConsistencyProofResponse, GlobalMetricsResponse, HasLeafResponse,
    HasLogResponse, HasRootResponse, InclusionProofResponse, LogMetricsResponse, MetricsResponse,
    RootResponse, SizeResponse,
};
use crate::service::state::AppState;
use crate::{ConsistencyProof, InclusionProof};

// HTTP handlers - read-only access to in-memory DashMap state
// All database operations are handled exclusively by the batch processor
// This ensures clean separation: HTTP handlers never block on database I/O

/// Query parameters for inclusion proof requests
///
/// Used by the `/logs/{log_name}/proof` endpoint to specify which leaf to prove inclusion for.
#[derive(Debug, Serialize, Deserialize)]
pub struct InclusionQuery {
    /// Base64-encoded leaf hash to generate inclusion proof for
    #[serde(with = "crate::service::routes::proof_bytes_format")]
    pub hash: Vec<u8>,
}

/// Query parameters for consistency proof requests
///
/// Used by the `/logs/{log_name}/consistency` endpoint to prove consistency
/// between two tree states.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsistencyQuery {
    /// The root hash of the historical tree to prove consistency with
    #[serde(with = "crate::service::routes::proof_bytes_format")]
    pub old_root: Vec<u8>,
}

/// Query parameters for leaf existence checks
///
/// Used by the `/logs/{log_name}/has_leaf` endpoint to check if a leaf exists
/// in the current tree state.
#[derive(Debug, Serialize, Deserialize)]
pub struct HasLeafQuery {
    /// The leaf hash to check for
    #[serde(with = "crate::service::routes::proof_bytes_format")]
    pub hash: Vec<u8>,
}

/// Query parameters for root existence checks
///
/// Used by the `/logs/{log_name}/has_root` endpoint to check if a root hash
/// appears in the log's merkle tree history.
#[derive(Debug, Serialize, Deserialize)]
pub struct HasRootQuery {
    /// The root hash to check for
    #[serde(with = "crate::service::routes::proof_bytes_format")]
    pub root: Vec<u8>,
}

/// Get the current merkle root for a log
///
/// Endpoint: `GET /logs/{log_name}/root`
///
/// Returns the current merkle root hash, tree size, and last processed entry ID.
/// This is the primary endpoint for clients to discover the current log state.
///
/// # Errors
///
/// Returns `LogNotFound` if the log doesn't exist, or `EmptyTree` if the log
/// has no entries yet.
pub async fn get_merkle_root(
    Path(log_name): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Get or create the merkle state for this log
    let merkle_state_arc = match state.merkle_states.get(&log_name) {
        Some(arc) => arc.clone(),
        None => {
            return ApiError::LogNotFound(log_name).to_response();
        }
    };

    let merkle_state = merkle_state_arc.read();
    if merkle_state.tree.is_empty() {
        return ApiError::EmptyTree(log_name).to_response();
    }

    ApiResponse::success(RootResponse::from_merkle_state(log_name, &merkle_state))
}

/// Get the size of a merkle log
///
/// Endpoint: `GET /logs/{log_name}/size`
///
/// Returns the number of leaves in the merkle tree. Returns 0 if the log
/// doesn't exist yet. Use `/logs/{log_name}/exists` to distinguish between
/// an empty log and a non-existent log.
pub async fn get_log_size(
    Path(log_name): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Get the merkle state for this log
    let size = match state.merkle_states.get(&log_name) {
        Some(arc) => {
            let merkle_state = arc.read();
            merkle_state.tree.len()
        }
        None => {
            // Log doesn't exist yet - return size 0
            0
        }
    };

    ApiResponse::success(SizeResponse { log_name, size })
}

/// Generate an inclusion proof for a leaf in the merkle tree
///
/// Endpoint: `GET /logs/{log_name}/proof?hash=<base64_hash>`
///
/// Generates a cryptographic proof that the specified leaf exists in the tree.
/// The proof is verified before being returned to ensure correctness.
///
/// # Query Parameters
///
/// - `hash`: Base64-encoded leaf hash to prove inclusion for
///
/// # Errors
///
/// - `InvalidRequest`: Query parameter parsing failed or hash not base64
/// - `LogNotFound`: The specified log doesn't exist
/// - `ProofGenerationFailed`: Failed to generate the proof (leaf not found)
/// - `ProofVerificationFailed`: Generated proof failed verification (internal error)
///
/// # Panics
///
/// Infallible: Index must exist after proof generation succeeds
pub async fn get_inclusion_proof(
    Path(log_name): Path<String>,
    query_result: Result<Query<InclusionQuery>, axum::extract::rejection::QueryRejection>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();

    // Handle query parameter parsing errors
    let Query(query) = match query_result {
        Ok(query) => query,
        Err(e) => {
            tracing::debug!(error = %e, "Invalid proof request");
            state.http_metrics.inclusion.record_error(start.elapsed());
            return ApiError::InvalidRequest(format!(
                "Invalid request parameters: {e}. The hash parameter must be base64 encoded."
            ))
            .to_response();
        }
    };

    // Get the merkle state for this log
    let Some(merkle_state_arc) = state.merkle_states.get(&log_name) else {
        state.http_metrics.inclusion.record_error(start.elapsed());
        return ApiError::LogNotFound(log_name).to_response();
    };
    let merkle_state_arc = merkle_state_arc.clone();

    let merkle_state = merkle_state_arc.read();

    let tree = &merkle_state.tree;
    // Try to generate and verify proof using our new tree's method
    let proof = match tree.prove_inclusion(&query.hash) {
        Ok(proof) => proof,
        Err(e) => {
            state.http_metrics.inclusion.record_error(start.elapsed());
            return ApiError::ProofGenerationFailed(format!(
                "Failed to generate inclusion proof: {e:?}"
            ))
            .to_response();
        }
    };

    // Verify the proof as a sanity check
    match tree.verify_inclusion(&query.hash, &proof) {
        Ok(()) => {
            // Get the index (safe because prove_inclusion succeeded)
            let index = tree
                .get_index(&query.hash)
                .expect("Index must exist after proof generation succeeds");

            // Create InclusionProof with all necessary data
            let inclusion_proof = InclusionProof {
                index,
                proof_bytes: proof.as_bytes().to_vec(),
                root: tree.root(),
                tree_size: tree.len(),
            };

            state.http_metrics.inclusion.record_success(start.elapsed());
            ApiResponse::success(InclusionProofResponse {
                log_name,
                proof: inclusion_proof,
            })
        }
        Err(e) => {
            state.http_metrics.inclusion.record_error(start.elapsed());
            ApiError::ProofVerificationFailed(format!("Proof verification error: {e:?}"))
                .to_response()
        }
    }
}

/// Generate a consistency proof between two tree states
///
/// Endpoint: `GET /logs/{log_name}/consistency?old_root=<base64_root>`
///
/// Generates a cryptographic proof that the tree grew in an append-only manner
/// from the historical state (identified by `old_root`) to the current state.
/// This is essential for Certificate Transparency-style auditing.
///
/// # Query Parameters
///
/// - `old_root`: Base64-encoded root hash of the historical tree state
///
/// # Errors
///
/// - `InvalidRequest`: Query parameter parsing failed or `old_root` not base64
/// - `LogNotFound`: The specified log doesn't exist
/// - `ProofGenerationFailed`: Failed to generate the proof (`old_root` not found in history)
/// - `ProofVerificationFailed`: Generated proof failed verification (internal error)
///
/// # Panics
///
/// Infallible: Old tree size must exist after proof generation succeeds
pub async fn get_consistency_proof(
    Path(log_name): Path<String>,
    query_result: Result<Query<ConsistencyQuery>, axum::extract::rejection::QueryRejection>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();

    // Handle query parameter parsing errors
    let Query(query) = match query_result {
        Ok(query) => query,
        Err(e) => {
            tracing::debug!(error = %e, "Invalid consistency proof request");
            state.http_metrics.consistency.record_error(start.elapsed());
            return ApiError::InvalidRequest(format!(
                "Invalid request parameters: {e}. The old_root parameter must be base64 encoded."
            ))
            .to_response();
        }
    };

    // Get the merkle state for this log
    let Some(merkle_state_arc) = state.merkle_states.get(&log_name) else {
        state.http_metrics.consistency.record_error(start.elapsed());
        return ApiError::LogNotFound(log_name).to_response();
    };
    let merkle_state_arc = merkle_state_arc.clone();

    let merkle_state = merkle_state_arc.read();
    let tree = &merkle_state.tree;

    // Try to generate consistency proof
    let proof = match tree.prove_consistency(&query.old_root) {
        Ok(proof) => proof,
        Err(e) => {
            state.http_metrics.consistency.record_error(start.elapsed());
            return ApiError::ProofGenerationFailed(format!(
                "Failed to generate consistency proof: ProofError: {e:?}"
            ))
            .to_response();
        }
    };

    // Verify the proof
    match tree.verify_consistency(&query.old_root, &proof) {
        Ok(()) => {
            let old_size = tree
                .get_size_for_root(&query.old_root)
                .expect("Old tree size must exist after proof generation succeeds");

            // Return the verified proof
            let consistency_proof = ConsistencyProof {
                old_tree_size: old_size,
                proof_bytes: proof.as_bytes().to_vec(),
                new_root: tree.root(),
                new_tree_size: tree.len(),
            };

            state
                .http_metrics
                .consistency
                .record_success(start.elapsed());
            ApiResponse::success(ConsistencyProofResponse {
                log_name,
                proof: consistency_proof,
            })
        }
        Err(e) => {
            state.http_metrics.consistency.record_error(start.elapsed());
            ApiError::ProofVerificationFailed(format!("Proof verification failed: {e:?}"))
                .to_response()
        }
    }
}

/// Check if a leaf exists in the merkle tree
///
/// Endpoint: `GET /logs/{log_name}/has_leaf?hash=<base64_hash>`
///
/// Performs an O(1) lookup to check if the specified leaf exists in the current
/// tree state. This is faster than requesting an inclusion proof when you only
/// need existence checking.
///
/// # Query Parameters
///
/// - `hash`: Base64-encoded leaf hash to check for
///
/// # Errors
///
/// - `InvalidRequest`: Query parameter parsing failed or hash not base64
/// - `LogNotFound`: The specified log doesn't exist
pub async fn has_leaf(
    Path(log_name): Path<String>,
    query_result: Result<Query<HasLeafQuery>, axum::extract::rejection::QueryRejection>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Handle query parameter parsing errors
    let Query(query) = match query_result {
        Ok(query) => query,
        Err(e) => {
            tracing::debug!(error = %e, "Invalid has_leaf request");
            return ApiError::InvalidRequest(format!(
                "Invalid request parameters: {e}. The hash parameter must be base64 encoded."
            ))
            .to_response();
        }
    };

    // Get the merkle state for this log
    let merkle_state_arc = match state.merkle_states.get(&log_name) {
        Some(arc) => arc.clone(),
        None => {
            return ApiError::LogNotFound(log_name).to_response();
        }
    };

    let merkle_state = merkle_state_arc.read();
    let tree = &merkle_state.tree;

    // Check if the leaf exists using O(1) index lookup
    let exists = tree.get_index(&query.hash).is_some();

    ApiResponse::success(HasLeafResponse { log_name, exists })
}

/// Check if a root hash exists in the log's history
///
/// Endpoint: `GET /logs/{log_name}/has_root?root=<base64_root>`
///
/// Performs an O(1) lookup to check if the specified root hash appears in the
/// merkle tree's history. Useful for verifying that you've observed a particular
/// tree state before requesting a consistency proof.
///
/// # Query Parameters
///
/// - `root`: Base64-encoded root hash to check for
///
/// # Errors
///
/// - `InvalidRequest`: Query parameter parsing failed or root not base64
/// - `LogNotFound`: The specified log doesn't exist
pub async fn has_root(
    Path(log_name): Path<String>,
    query_result: Result<Query<HasRootQuery>, axum::extract::rejection::QueryRejection>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Handle query parameter parsing errors
    let Query(query) = match query_result {
        Ok(query) => query,
        Err(e) => {
            tracing::debug!(error = %e, "Invalid has_root request");
            return ApiError::InvalidRequest(format!(
                "Invalid request parameters: {e}. The root parameter must be base64 encoded."
            ))
            .to_response();
        }
    };

    // Get the merkle state for this log
    let merkle_state_arc = match state.merkle_states.get(&log_name) {
        Some(arc) => arc.clone(),
        None => {
            return ApiError::LogNotFound(log_name).to_response();
        }
    };

    let merkle_state = merkle_state_arc.read();
    let tree = &merkle_state.tree;

    // Check if the root exists using O(1) root lookup
    let exists = tree.get_size_for_root(&query.root).is_some();

    ApiResponse::success(HasRootResponse { log_name, exists })
}

/// Check if a log exists on the server
///
/// Endpoint: `GET /logs/{log_name}/exists`
///
/// Returns whether the named log has been discovered and initialized by the
/// batch processor. This disambiguates between an empty log and a non-existent
/// log, which both return size 0 from the `/size` endpoint.
pub async fn has_log(
    Path(log_name): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Check if the log exists in the merkle states map
    let exists = state.merkle_states.contains_key(&log_name);

    ApiResponse::success(HasLogResponse { log_name, exists })
}

/// Get current metrics for all logs and global statistics
pub async fn metrics(State(app_state): State<AppState>) -> impl IntoResponse {
    let metrics = app_state.metrics.get_snapshot();
    let http_metrics = app_state.http_metrics.snapshot();

    let mut logs = std::collections::HashMap::new();
    for (log_name, log_metrics) in metrics.0 {
        logs.insert(
            log_name,
            LogMetricsResponse {
                last_batch_rows: log_metrics.last_batch_rows,
                last_batch_leaves: log_metrics.last_batch_leaves,
                last_total_ms: log_metrics.last_total_ms,
                last_copy_ms: log_metrics.last_copy_ms,
                last_insert_ms: log_metrics.last_insert_ms,
                last_fetch_merkle_log_ms: log_metrics.last_fetch_merkle_log_ms,
                last_tree_update_ms: log_metrics.last_tree_update_ms,
                total_batches: log_metrics.batches_processed,
                tree_size: log_metrics.tree_size,
                tree_memory_bytes: log_metrics.tree_memory_bytes,
                last_update: log_metrics.last_updated.to_rfc3339(),
            },
        );
    }

    let global = GlobalMetricsResponse {
        last_cycle_duration_ms: metrics.1.last_cycle_duration_ms,
        last_active_log_count: metrics.1.last_active_log_count,
        last_cycle_fraction: metrics.1.last_cycle_fraction,
    };

    ApiResponse::success(MetricsResponse {
        logs,
        global,
        http: Some(http_metrics),
    })
}
// Admin endpoints for processor control
// These endpoints allow graceful pause/resume/stop of the batch processor

/// Response for admin control endpoints
#[derive(Debug, Serialize)]
pub struct AdminControlResponse {
    /// Human-readable status message
    pub message: String,
    /// Current processor state: "running", "paused", or "stopping"
    pub state: String,
}

/// Pause the batch processor (stops processing but keeps server running)
pub async fn admin_pause(State(app_state): State<AppState>) -> impl IntoResponse {
    app_state
        .processor_state
        .store(1, std::sync::atomic::Ordering::Relaxed);
    ApiResponse::success(AdminControlResponse {
        message: "Batch processor paused".to_string(),
        state: "paused".to_string(),
    })
}

/// Resume the batch processor from paused state
pub async fn admin_resume(State(app_state): State<AppState>) -> impl IntoResponse {
    app_state
        .processor_state
        .store(0, std::sync::atomic::Ordering::Relaxed);
    ApiResponse::success(AdminControlResponse {
        message: "Batch processor resumed".to_string(),
        state: "running".to_string(),
    })
}

/// Stop the batch processor (will exit the processing loop)
pub async fn admin_stop(State(app_state): State<AppState>) -> impl IntoResponse {
    app_state
        .processor_state
        .store(2, std::sync::atomic::Ordering::Relaxed);
    ApiResponse::success(AdminControlResponse {
        message: "Batch processor stopping (will shut down entire application)".to_string(),
        state: "stopping".to_string(),
    })
}

/// Get current processor state
pub async fn admin_status(State(app_state): State<AppState>) -> impl IntoResponse {
    let state_value = app_state
        .processor_state
        .load(std::sync::atomic::Ordering::Relaxed);
    let state_name = match state_value {
        1 => "paused",
        2 => "stopping",
        _ => "running",
    };

    ApiResponse::success(AdminControlResponse {
        message: format!("Batch processor is {state_name}"),
        state: state_name.to_string(),
    })
}

/// Custom serde module for base64 encoding of byte arrays
///
/// This module provides serialization and deserialization for `Vec<u8>` fields
/// that should be represented as base64 strings in JSON. Used by query parameter
/// structs to accept base64-encoded hashes from clients.
pub mod proof_bytes_format {
    use super::{BASE64, Deserialize, Engine};
    use serde::{Deserializer, Serializer};

    /// Serialize a byte vector as a base64 string
    ///
    /// # Errors
    ///
    /// Returns a serialization error if the serializer fails to encode the base64 string.
    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&BASE64.encode(bytes))
    }

    /// Deserialize a base64 string into a byte vector
    ///
    /// # Errors
    ///
    /// Returns a deserialization error if the input is not a valid base64 string.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer).and_then(|string| {
            BASE64
                .decode(string.as_bytes())
                .map_err(|err| Error::custom(err.to_string()))
        })
    }
}
