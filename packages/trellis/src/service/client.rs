use crate::service::responses::{
    ApiResponse, ConsistencyProofResponse, HasLeafResponse, HasLogResponse, HasRootResponse,
    InclusionProofResponse, MetricsResponse, RootResponse, SizeResponse,
};
use crate::{ConsistencyProof, InclusionProof};
use anyhow::Result;
use base64::Engine;
use reqwest::Client as HttpClient;
use sha2::{Digest, Sha256};
use std::time::Duration;

/// Root information returned by `get_root`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootInfo {
    /// The root hash bytes
    pub root: Vec<u8>,
    /// The tree size at this root
    pub tree_size: u64,
}

/// Production HTTP client for interacting with the merkle tree service
pub struct Client {
    /// HTTP client for making requests to the API
    http_client: HttpClient,
    /// Base URL of the merkle tree service API
    api_base_url: String,
}

impl Client {
    /// Creates a new HTTP client for the merkle tree service
    ///
    /// # Errors
    ///
    /// Returns an error if the `HTTPClient` build fails.
    pub fn new(api_base_url: &str) -> Result<Self> {
        // Set up HTTP client with reasonable timeouts
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            http_client,
            api_base_url: api_base_url.to_string(),
        })
    }

    /// Gets the current size (number of entries) of the log
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, if the response is invalid or malformed,
    /// or if the server returns an error status.
    pub async fn get_log_size(&self, log_name: &str) -> Result<u64> {
        let response = self
            .http_client
            .get(format!("{}/logs/{}/size", self.api_base_url, log_name))
            .send()
            .await?
            .json::<ApiResponse<SizeResponse>>()
            .await?;

        match response {
            ApiResponse::Success(data) => Ok(data.size),
            ApiResponse::Error { error, .. } => {
                Err(anyhow::anyhow!("Error getting log size: {error}"))
            }
        }
    }

    /// Gets the current merkle root and tree size
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, if the response is invalid or malformed,
    /// if the base64 decoding of the root bytes fails, or if the server returns an error status.
    pub async fn get_root(&self, log_name: &str) -> Result<RootInfo> {
        let response = self
            .http_client
            .get(format!("{}/logs/{}/root", self.api_base_url, log_name))
            .send()
            .await?
            .json::<ApiResponse<RootResponse>>()
            .await?;

        match response {
            ApiResponse::Success(data) => {
                // Decode base64 root to bytes
                let root_bytes =
                    base64::engine::general_purpose::STANDARD.decode(&data.merkle_root)?;
                Ok(RootInfo {
                    root: root_bytes,
                    tree_size: data.tree_size,
                })
            }
            ApiResponse::Error { error, .. } => Err(anyhow::anyhow!("Error getting root: {error}")),
        }
    }

    /// Gets an inclusion proof for a given hash or piece of data
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, if the response is invalid or malformed,
    /// or if the server returns an error status.
    pub async fn get_inclusion_proof(&self, log_name: &str, data: &[u8]) -> Result<InclusionProof> {
        let query = crate::service::InclusionQuery {
            hash: data.to_owned(),
        };

        // Make the request with the base64 encoded hash
        let response = self
            .http_client
            .get(format!("{}/logs/{}/proof", self.api_base_url, log_name))
            .query(&query)
            .send()
            .await?
            .json::<ApiResponse<InclusionProofResponse>>()
            .await?;

        match response {
            ApiResponse::Success(data_) => Ok(data_.proof),
            ApiResponse::Error { error, .. } => {
                Err(anyhow::anyhow!("Error getting inclusion proof: {error}"))
            }
        }
    }

    /// Gets a consistency proof proving that the current tree state is consistent with an older root hash
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, if the response is invalid or malformed,
    /// or if the server returns an error status.
    pub async fn get_consistency_proof(
        &self,
        log_name: &str,
        old_root: Vec<u8>,
    ) -> Result<ConsistencyProof> {
        let query = crate::service::ConsistencyQuery { old_root };

        let response = self
            .http_client
            .get(format!(
                "{}/logs/{}/consistency",
                self.api_base_url, log_name
            ))
            .query(&query)
            .send()
            .await?
            .json::<ApiResponse<ConsistencyProofResponse>>()
            .await?;

        match response {
            ApiResponse::Success(data) => Ok(data.proof),
            ApiResponse::Error { error, .. } => {
                Err(anyhow::anyhow!("Error getting consistency proof: {error}"))
            }
        }
    }

    /// Verify extension of a saved checkpoint and return the complete verified successor.
    ///
    /// The caller must retain both root and size; neither may be replaced by response metadata.
    /// # Errors
    /// Returns an error if fetching or verifying the consistency proof fails.
    pub async fn verify_tree_consistency(
        &self,
        log_name: &str,
        old: &RootInfo,
    ) -> Result<RootInfo> {
        let proof = self
            .get_consistency_proof(log_name, old.root.clone())
            .await?;
        self.verify_consistency_proof(old, &proof)?;
        Ok(RootInfo {
            root: proof.new_root,
            tree_size: proof.new_tree_size,
        })
    }

    /// Checks if a leaf (identified by its hash) exists in the log
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, if the response is invalid or malformed,
    /// or if the server returns an error status.
    pub async fn has_leaf(&self, log_name: &str, data: &str) -> Result<bool> {
        // Compute the hash from the data
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash_result = hasher.finalize();

        self.has_leaf_hash(log_name, &hash_result).await
    }

    /// Checks if a leaf (identified by its pre-computed hash) exists in the log
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, if the response is invalid or malformed,
    /// or if the server returns an error status.
    pub async fn has_leaf_hash(&self, log_name: &str, hash: &[u8]) -> Result<bool> {
        let query = crate::service::HasLeafQuery {
            hash: hash.to_vec(),
        };

        // Make the request
        let response = self
            .http_client
            .get(format!("{}/logs/{}/has_leaf", self.api_base_url, log_name))
            .query(&query)
            .send()
            .await?
            .json::<ApiResponse<HasLeafResponse>>()
            .await?;

        match response {
            ApiResponse::Success(data_) => Ok(data_.exists),
            ApiResponse::Error { error, .. } => {
                Err(anyhow::anyhow!("Error checking leaf existence: {error}"))
            }
        }
    }

    /// Verifies an inclusion proof for a given leaf hash
    ///
    /// # Errors
    ///
    /// Returns an error if proof verification fails due to cryptographic mismatch.
    pub fn verify_inclusion_proof(&self, leaf_hash: &[u8], proof: &InclusionProof) -> Result<()> {
        proof
            .verify(leaf_hash)
            .map_err(|e| anyhow::anyhow!("Inclusion proof verification failed: {e}"))
    }

    /// Verifies a consistency proof against a saved root and tree size
    ///
    /// # Errors
    ///
    /// Returns an error if proof verification fails due to cryptographic mismatch.
    pub fn verify_consistency_proof(&self, old: &RootInfo, proof: &ConsistencyProof) -> Result<()> {
        proof
            .verify(&old.root, old.tree_size)
            .map_err(|e| anyhow::anyhow!("Consistency proof verification failed: {e}"))
    }

    /// Checks if a historical root exists in the log
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, if the response is invalid or malformed,
    /// or if the server returns an error status.
    pub async fn has_root(&self, log_name: &str, root: Vec<u8>) -> Result<bool> {
        let query = crate::service::HasRootQuery { root };

        // Make the request
        let response = self
            .http_client
            .get(format!("{}/logs/{}/has_root", self.api_base_url, log_name))
            .query(&query)
            .send()
            .await?
            .json::<ApiResponse<HasRootResponse>>()
            .await?;

        match response {
            ApiResponse::Success(data) => Ok(data.exists),
            ApiResponse::Error { error, .. } => {
                Err(anyhow::anyhow!("Error checking root existence: {error}"))
            }
        }
    }

    /// Checks if a log exists on the server
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, if the response is invalid or malformed,
    /// or if the server returns an error status.
    pub async fn has_log(&self, log_name: &str) -> Result<bool> {
        let response = self
            .http_client
            .get(format!("{}/logs/{}/exists", self.api_base_url, log_name))
            .send()
            .await?
            .json::<ApiResponse<HasLogResponse>>()
            .await?;

        match response {
            ApiResponse::Success(data) => Ok(data.exists),
            ApiResponse::Error { error, .. } => {
                Err(anyhow::anyhow!("Error checking log existence: {error}"))
            }
        }
    }

    /// Gets performance metrics from the server
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, if the response is invalid or malformed,
    /// or if the server returns an error status.
    pub async fn get_metrics(&self) -> Result<MetricsResponse> {
        let response = self
            .http_client
            .get(format!("{}/metrics", self.api_base_url))
            .send()
            .await?
            .json::<ApiResponse<MetricsResponse>>()
            .await?;

        match response {
            ApiResponse::Success(data) => Ok(data),
            ApiResponse::Error { error, .. } => {
                Err(anyhow::anyhow!("Error getting metrics: {error}"))
            }
        }
    }
}
