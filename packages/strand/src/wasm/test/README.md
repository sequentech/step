<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
## Test wasm build

* install [webpack](https://rustwasm.github.io/wasm-pack/installer/)
* `wasm-pack build --out-name index --release --target web --features=wasmtest`
* `python3 src/wasm/test/serve.py`
* http://localhost:8080/src/wasm/test/test.html

## Test wasm build with no workers (Necessary for Firefox due to [this bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1247687))

* install [webpack](https://rustwasm.github.io/wasm-pack/installer/)
* `wasm-pack build --out-name index --release --target web --features=wasmtest`
* `python3 src/wasm/test/serve.py`
* http://localhost:8080/src/wasm/test/test_noworker.html

## Test wasm build with multithreading

* install [webpack](https://rustwasm.github.io/wasm-pack/installer/)
* Build with [multithreading](https://github.com/GoogleChromeLabs/wasm-bindgen-rayon)

```bash
export RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals' 
wasm-pack build \
    --out-name index \
    --release \
    --target web \
    --features=wasmtest,wasmrayon \
    -- -Z build-std=panic_abort,std

```
* `python3 src/wasm/test/serve.py`
* http://localhost:8080/src/wasm/test/test.html?threaded=true

In order for this test to work the browser must have [cross origin
isolation](https://web.dev/cross-origin-isolation-guide/) enabled. This requires
sending the appropriate headers (see `serve.py`) and cannot work without https
unless testing on localhost.

This build allows you to deploy the demo, that is available in the 
http://localhost:8080/src/wasm/test/demo.html when served with `serve.py`.

## Github pages demo deployment

This repository is set up to deploy automatically any push to the `main` branch
on Github Pages and makes the demo available at
https://sequentech.github.io/strand/demo.html . Since Github Pages doesn't allow
to setup the appropriate headers, these are setup using CloudFlare.