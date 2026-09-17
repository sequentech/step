---
id: miru
title: Miru component tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Use Rust 1.96.0, its `wasm32-wasip2` target and WASI SDK 27.0. The pinned download
and checksum are in `.github/workflows/miru-unit-tests.yml`. From `packages/`:

```sh
export PATH="$WASI_SDK_PATH/bin:$PATH"
export CC_wasm32_wasip2="$WASI_SDK_PATH/bin/clang"
export AR_wasm32_wasip2="$WASI_SDK_PATH/bin/llvm-ar"
export CFLAGS_wasm32_wasip2="--sysroot=$WASI_SDK_PATH/share/wasi-sysroot"
cargo build --locked --release -p miru --target wasm32-wasip2
export MIRU_TEST_COMPONENT="$PWD/target/wasm32-wasip2/release/miru.wasm"
cargo test --locked -p miru --test component_contracts
cargo clippy --locked -p miru --lib
```

The release build is intentional: this pinned toolchain's debug component link
can crash while emitting Sequent Core's auxiliary cdylib. The release artifact is
executed directly through Wasmtime; no generated bindings or production imports
are replaced. The host supplies synthetic authorization responses and checks the
literal claims, tenant, super-admin flag and permission list. Success, denial,
malformed JSON, missing/wrongly typed claims and the manifest route are exercised.

The WASI host inherits no environment, filesystem directories or sockets. Fuel
bounds guest execution; store limits cap each linear memory at 64 MiB and each
table at 10,000 elements. The test host compiles only for native targets. Each test
owns its store, so authorization state cannot
leak between tests. The real host's JWT verification and transaction services
remain separate integration boundaries. These WASM contracts do not publish
native LLVM coverage or imply that host services were exercised.
