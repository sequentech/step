// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WASM bindings for election verification

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

use crate::protocol::trustee::Trustee;
use crate::protocol::verify::verifier::Verifier;
use crate::wasm::board::{WasmHttpBoard, WasmHttpBoardParams};
use crate::protocol::board::NoOpStorage;

use cryptography::context::{RistrettoCtx, Context};
use cryptography::utils::symm;
use cryptography::utils::signatures::SignatureScheme;

/// Individual verification check result
#[derive(Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub metadata: String,
}

/// Batch verification results
#[derive(Serialize, Deserialize)]
pub struct BatchResult {
    pub batch_name: String,
    pub checks: Vec<CheckResult>,
}

/// Verification result summary for JavaScript
#[derive(Serialize, Deserialize)]
pub struct VerificationSummary {
    pub board_name: String,
    pub valid: bool,
    pub root_checks: Vec<CheckResult>,
    pub batches: Vec<BatchResult>,
}

/// WASM verifier for browser-based election verification
#[wasm_bindgen]
pub struct WasmVerifier {
    b4_url: String,
    board_name: Option<String>,
}

#[wasm_bindgen]
impl WasmVerifier {
    /// Create a new WASM verifier
    /// 
    /// # Arguments
    /// * `b4_url` - URL of the B4 server (e.g., "http://localhost:8000")
    #[wasm_bindgen(constructor)]
    pub fn new(b4_url: String) -> WasmVerifier {
        console_error_panic_hook::set_once();
        
        WasmVerifier {
            b4_url,
            board_name: None,
        }
    }

    /// Verify an election board
    /// 
    /// This performs comprehensive verification including:
    /// - Configuration validity
    /// - Message signature verification
    /// - Public key construction verification
    /// - Mixing proofs verification
    /// - Decryption proofs verification
    /// 
    /// Returns a JSON summary of verification results
    pub async fn verify_board(&mut self, board_name: String) -> Result<JsValue, JsValue> {
        self.board_name = Some(board_name.clone());
        
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "🔍 Starting verification of board '{}'",
            board_name
        )));

        // Generate dummy trustee credentials (not used for verification)
        let mut rng = RistrettoCtx::get_rng();
        let dummy_sk = <<RistrettoCtx as Context>::SignatureScheme as SignatureScheme<_>>::gen_signing_key(&mut rng);
        let dummy_encryption_key = symm::gen_key().unwrap();

        // Create NoOp storage (verifier doesn't need persistence)
        let storage = NoOpStorage::new();

        // Create verifier trustee
        let trustee: Trustee<RistrettoCtx, NoOpStorage> = Trustee::new(
            "BrowserVerifier".to_string(),
            board_name.clone(),
            dummy_sk,
            dummy_encryption_key,
            storage,
            None, // No max_concurrent_actions limit
        );

        // Create WASM HTTP board
        let board_params = WasmHttpBoardParams {
            b4_url: self.b4_url.clone(),
        };
        let board = WasmHttpBoard::new(board_params);

        // Create and run verifier
        let mut verifier: Verifier<RistrettoCtx, WasmHttpBoard, NoOpStorage> = 
            Verifier::new(trustee, board, &board_name);

        let vr = verifier.run().await.map_err(|e| {
            let error_msg = format!("Verification failed: {:?}", e);
            web_sys::console::error_1(&JsValue::from_str(&error_msg));
            JsValue::from_str(&error_msg)
        })?;

        web_sys::console::log_1(&JsValue::from_str("✅ Verification complete"));

        // Extract check results in hierarchical form
        let root_checks: Vec<CheckResult> = vr.get_root_checks()
            .into_iter()
            .map(|(name, passed, metadata)| CheckResult { name, passed, metadata })
            .collect();

        let batches: Vec<BatchResult> = vr.get_batch_checks()
            .into_iter()
            .map(|(batch_name, checks)| {
                let batch_checks = checks.into_iter()
                    .map(|(name, passed, metadata)| CheckResult { name, passed, metadata })
                    .collect();
                BatchResult {
                    batch_name,
                    checks: batch_checks,
                }
            })
            .collect();

        let summary = VerificationSummary {
            board_name,
            valid: vr.all_passed(),
            root_checks,
            batches,
        };

        // Serialize to JSON string for JavaScript consumption
        let json_string = serde_json::to_string(&summary)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
        
        Ok(JsValue::from_str(&json_string))
    }

    /// Get the current board name being verified
    #[wasm_bindgen(getter)]
    pub fn board_name(&self) -> Option<String> {
        self.board_name.clone()
    }

    /// Get the B4 URL
    #[wasm_bindgen(getter)]
    pub fn b4_url(&self) -> String {
        self.b4_url.clone()
    }
}
