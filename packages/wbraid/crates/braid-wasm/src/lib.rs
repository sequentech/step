pub mod trustee;

pub use trustee::WasmTrustee;

// Re-export wasm-bindgen-rayon's initThreadPool for browser usage
// This provides parallel computation support via Web Workers
pub use wasm_bindgen_rayon::init_thread_pool as initThreadPool;


// mod bulletin_board;
// mod local_storage;
// mod s3;

// pub use bulletin_board::BulletinBoardClient;
// pub use local_storage::LocalStorage;

// use wasm_bindgen::prelude::*;

// Initialize the WASM module with console logging
/* #[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    
    // Set up tracing to output to browser console
    tracing_wasm::set_as_global_default();
}*/

// Re-export console_error_panic_hook
// use console_error_panic_hook;
