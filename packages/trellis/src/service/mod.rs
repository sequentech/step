/// HTTP client for interacting with the merkle log service
pub mod client;

/// Performance metrics tracking for batch processing
pub mod metrics;

/// Background batch processor that continuously merges data from source tables
pub mod processor;

/// HTTP response types and error handling for the API
pub mod responses;

/// HTTP route handlers for all verification endpoints
pub mod routes;

/// Server initialization and configuration
pub mod server;

/// Application state management including merkle tree storage
pub mod state;

/// Database schema validation utilities
pub mod validation;

pub use client::{Client, RootInfo};
pub use processor::{rebuild_all_logs, validate_log_name};
pub use responses::{
    ApiError, ApiResponse, ConsistencyProofResponse, GlobalMetricsResponse, HasLeafResponse,
    HasLogResponse, HasRootResponse, InclusionProofResponse, LogMetricsResponse, MetricsResponse,
    RootResponse, SizeResponse,
};
pub use routes::{
    AdminControlResponse, ConsistencyQuery, HasLeafQuery, HasRootQuery, InclusionQuery,
    admin_pause, admin_resume, admin_status, admin_stop, get_consistency_proof,
    get_inclusion_proof, get_log_size, get_merkle_root, has_leaf, has_root,
};
pub use server::{create_server, initialize_app_state, run_server};
pub use state::MerkleState;
pub use validation::{LogValidation, print_validation_report, validate_all_logs};
