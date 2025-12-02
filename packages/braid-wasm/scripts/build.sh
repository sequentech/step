cargo build --target wasm32-unknown-unknown --release
wasm-bindgen ../target/wasm32-unknown-unknown/release/braid_wasm.wasm \
        --out-dir pkg \
        --target web
npm pack ./pkg