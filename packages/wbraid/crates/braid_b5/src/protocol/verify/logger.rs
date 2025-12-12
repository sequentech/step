// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Logging abstraction for verifier that works in both native and WASM contexts

/// Trait for verifier-specific logging that can be implemented differently
/// for native (with colors) vs WASM (console.log)
pub trait VerifierLogger {
    /// Log a step/phase message (blue in native)
    fn log_step(&self, message: &str);
    
    /// Log a title/header message (bold in native)
    fn log_title(&self, message: &str);
    
    /// Log an info message
    fn log_info(&self, message: &str);
    
    /// Log a success indicator (green in native)
    fn log_success(&self, message: &str);
    
    /// Log a failure indicator (red in native)
    fn log_failure(&self, message: &str);
}

/// Native implementation with colored output
#[cfg(feature = "native")]
pub struct NativeLogger;

#[cfg(feature = "native")]
impl VerifierLogger for NativeLogger {
    fn log_step(&self, message: &str) {
        use colored::*;
        tracing::info!("{}", message.blue());
    }
    
    fn log_title(&self, message: &str) {
        use colored::*;
        tracing::info!("{}", message.bold());
    }
    
    fn log_info(&self, message: &str) {
        tracing::info!("{}", message);
    }
    
    fn log_success(&self, message: &str) {
        use colored::*;
        tracing::info!("{}", message.green());
    }
    
    fn log_failure(&self, message: &str) {
        use colored::*;
        tracing::info!("{}", message.red());
    }
}

/// WASM implementation using web_sys console
#[cfg(not(feature = "native"))]
pub struct WasmLogger;

#[cfg(not(feature = "native"))]
impl VerifierLogger for WasmLogger {
    fn log_step(&self, message: &str) {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("🔵 {}", message)));
        tracing::info!("{}", message);
    }
    
    fn log_title(&self, message: &str) {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("📋 {}", message)));
        tracing::info!("{}", message);
    }
    
    fn log_info(&self, message: &str) {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(message));
        tracing::info!("{}", message);
    }
    
    fn log_success(&self, message: &str) {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("✅ {}", message)));
        tracing::info!("{}", message);
    }
    
    fn log_failure(&self, message: &str) {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("❌ {}", message)));
        tracing::info!("{}", message);
    }
}

/// Create the appropriate logger for the current platform
pub fn create_logger() -> Box<dyn VerifierLogger> {
    #[cfg(feature = "native")]
    {
        Box::new(NativeLogger)
    }
    #[cfg(not(feature = "native"))]
    {
        Box::new(WasmLogger)
    }
}
