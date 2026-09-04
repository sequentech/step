<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Provenance of `packages/wbraid`

## Import

`packages/wbraid` was imported from the `sequentech/step` branch
`feat/braid-0.6.3/main` at commit `9b159b2582` (author: David Ruescas). Only the
`packages/wbraid` subtree was taken; nothing outside it was carried over.

Two subtrees were deleted at import time and are recoverable from the source
branch if they are ever needed:

- `packages/wbraid/legacy/`
- `packages/wbraid/crates/braid/vs_lift/`

## Stateright / model-checking work

The stateright / model-checking work that once lived separately on
`exp/braid-stateright/main` has been folded back into the 0.6.x line: that
branch's tip `4404cee077c1cad0584bc59ca9d86b9ccf705bd0` is an ancestor of the
imported commit, so the work is part of this import (see `STATERIGHT.md` and
`crates/braid/tests/model_check*.rs`).

## `crates/vsc`

`crates/vsc` (crate version `0.6.2`, `[lib] name = "cryptography"`) is a vendored
fork of https://github.com/FreeAndFair/VoteSecure, licensed Apache-2.0,
© Free & Fair. It was taken verbatim from `feat/braid-0.6.3/main` @ `9b159b2582`;
no delta against upstream is measured or recorded here. The relationship to
upstream — including whether changes are contributed back or the fork is
maintained independently — is tracked by David.

Upstream licence headers under `crates/vsc/` and
`crates/macros/custom_warning_macro/` are left untouched; files there that carry
no header are covered by an Apache-2.0 / Free & Fair annotation in the
repository-root `REUSE.toml`.

## Local modifications made at import

- **Exact-pinned the six pre-release dependencies** in `crates/vsc/Cargo.toml`,
  so that a pre-release bump upstream cannot silently change what we build:
  - `curve25519-dalek` `=5.0.0-pre.6`
  - `ed25519-dalek` `=3.0.0-pre.6`
  - `elliptic-curve` `=0.14.0-rc.31`
  - `p256` `=0.14.0-rc.9`
  - `chacha20poly1305` `=0.11.0-rc.3`
  - `aead` `=0.6.0-rc.10`
- **Renamed the `b4` binary to `b4v6`** in `crates/b4/Cargo.toml` (the crate name
  is unchanged), to avoid colliding with the existing `b4` binary in the main
  workspace. `b4.ps1` was updated to match.
- **Committed `packages/wbraid/Cargo.lock`**, generated with cargo 1.96.0. The
  `Cargo.lock` entries were removed from `packages/wbraid/.gitignore` and
  `packages/wbraid/crates/braid/.gitignore`.
- **Added AGPL-3.0-only headers to `crates/rnk/`**, which carried no licence
  markers at all, and added `license = "AGPL-3.0-only"` to its `[package]`
  section.
- **Added AGPL-3.0-only headers to `crates/braid/fuzz/`** (`Cargo.toml` and
  `.gitignore`), which carried no licence markers. The fuzz crate is its own
  cargo workspace and keeps no lockfile, matching the source branch.

## Local modifications for building on stable Rust

The source branch assumes a nightly toolchain; the following changes make the
workspace build with stable Rust (1.96.0), with upstream behaviour restored on
nightly by enabling the named features:

- **Gated `crates/vsc`'s nightly feature gates** (`stmt_expr_attributes`,
  `proc_macro_hygiene`) behind `cfg_attr(feature = "custom-warnings", ...)`,
  and wrapped every `#[crate::warning(...)]` in statement, expression, or
  file-module position the same way — those positions reject proc-macro
  attributes on stable even though `custom_warning_macro` expands to a no-op
  pass-through when its `on` feature is off. Item-position uses are unchanged.
- **Gated the libtest bench** `crates/vsc/benches/shuffle.rs`
  (`#![feature(test)]`, a hard error on stable) behind a new empty
  `nightly-benches` feature via `required-features`, so `--all-targets` builds
  skip it on stable.
- **Pinned `primefield` to `0.14.0-rc.9` in `Cargo.lock`**: cargo's pre-release
  semver rules resolve `p256 0.14.0-rc.9`'s `primefield 0.14.0-rc.9`
  requirement to the API-incompatible `0.14.0` final release, which does not
  compile against p256 rc.9.
- The `[[patch.unused]]` entry for `auto_generate_cdp` in `Cargo.lock` is
  written by cargo because `packages/.cargo/config.toml` (an ancestor config)
  declares that patch for the main workspace; it is inert here.

`cargo clippy --workspace` passes on stable. `cargo clippy -p vsc
--all-targets` fails inside `crates/vsc`'s test modules and the
`shuffle_scaling` example (mostly `unwrap_used` and pedantic lints in test
code, identical on nightly); that upstream state is left untouched.

## Local modifications for the devcontainer dev loop and CI

- **Added bash twins of the five PowerShell scripts** (`build-wasm.sh`,
  `serve.sh`, `test-wasm.sh`, `b4.sh`, `localstack.sh`). The `.ps1` files are
  unchanged and remain the Windows dev loop. Differences beyond syntax:
  `build-wasm.sh` stays on the stable toolchain and uses `RUSTC_BOOTSTRAP=1`
  instead of a nightly override (the approach of
  `packages/braid/scripts/build-wasm.sh`); both wasm scripts verify that the
  `wasm-bindgen` CLI on `PATH` matches the version pinned in `Cargo.lock`
  before building; in the devcontainer, `localstack.sh` runs LocalStack the
  way every other dev service runs — as the `localstack` compose service
  (opt-in `wbraid` profile in `.devcontainer/docker-compose-base.yml`, with a
  `configure-localstack` one-shot creating the bucket and CORS, mirroring
  `configure-minio`), addressed as `http://localstack:4566` on the project
  network — while outside a compose project it keeps `localstack.ps1`'s
  standalone docker-run flow; the image is pinned to `localstack/localstack:4`
  everywhere (2026-era `latest` exits at startup without an auth token);
  `b4.sh` defaults `AWS_ENDPOINT_URL` per environment (a pre-set value wins)
  and falls back to the `amazon/aws-cli` image joined to the project network
  when no AWS CLI is installed; and
  `build-wasm.sh`/`serve.sh` clear an inherited `RUSTFLAGS`, which would
  otherwise override the atomics rustflags in `crates/braid/.cargo/config.toml`
  entirely (the devcontainer's devenv exports `RUSTFLAGS=-Awarnings`).
- **`server.py` honours a `PORT` environment variable** (default 8080,
  unchanged); in the devcontainer 8080 is taken by Hasura.
- **Ran `cargo fmt`** (rustfmt 1.96.0) over the workspace — the tree was
  imported unformatted — so CI can gate on `cargo fmt -- --check`.
- **Fixed the warn-level clippy findings in `braid`, `rnk` and `v2v`** so that
  `cargo clippy --workspace --exclude vsc --all-targets --no-deps -- -D
  warnings` passes (the CI invocation; `--no-deps` keeps `-D warnings` from
  leaking into `vsc`, which every workspace clippy run otherwise compiles with
  the same flags). Mechanical changes: removed clones of `Copy` types and
  same-type casts, `is_multiple_of`/`filter_map`/needless-borrow cleanups, a
  `&PathBuf` parameter became `&Path`, a boxed-closure type alias in a test,
  and a duplicated `allow(dead_code)` removed. Three lints are allowed rather
  than refactored: `too_many_arguments` on two protocol functions and one
  spec-shaped RO helper, `items_after_test_module` in `v2v::wire::protinfo`,
  and `inherent_to_string` in `rnk` (its `to_string` is JSON serialization
  paired with `from_string`). `crates/vsc` is untouched: its lib passes clippy
  at upstream's own lint levels (`cargo clippy -p vsc --no-deps`), and its test
  modules/example still fail those levels as noted above.
- **Added `.github/workflows/wbraid.yml`**: fmt + clippy (the two invocations
  above), `cargo test --release`, and a `wasm-core` build for
  `wasm32-unknown-unknown`, on pushes/PRs touching `packages/wbraid/`. The
  shared `setup-rust-tests` action gained optional `components`/`targets`
  inputs for this (defaults unchanged).

## Local modifications for clippy

The tree was imported with warn-level clippy findings in `braid`, `rnk` and
`v2v`, so a `-D warnings` gate failed. The following changes make

```
cargo clippy --workspace --exclude vsc --all-targets --no-deps -- -D warnings
cargo clippy -p vsc --no-deps
```

pass on stable 1.96.0. `crates/vsc` is linted separately at upstream's own
levels (`--no-deps` keeps `-D warnings` from reaching it through the workspace
run) and is untouched: its lib passes, with upstream's warn-level
`indexing_slicing` findings.

- **`crates/braid`**: dropped the same-type casts of `PROTOCOL_MANAGER_INDEX`
  and a `clone()` of the `Copy` type `MessageType`; `AccumulatorSet::extract`
  is `flatten().cloned()`. Two functions were over the argument limit:
  `Trustee::sign_mix` lost its unused `_self_index` parameter;
  `compute_partial_decryptions_inner` keeps its eight under
  `#[expect(clippy::too_many_arguments)]`, since it exists as a monomorphized
  call target for the dispatching function and takes exactly that function's
  locals. In `tests/model_check*.rs`: two `clone()`s of `Copy` group elements
  and a redundant closure.
- **`crates/rnk`**: the seven value types' inherent `to_string()` methods
  (`inherent_to_string`) became `fmt::Display` impls producing the same JSON.
  `.to_string()` callers are unaffected; the `unwrap()` on serialization is
  gone.
- **`crates/v2v`**: `is_multiple_of` for the `% 2` checks, a `clone()` of a
  `Copy` scalar, and the test module of `wire/protinfo.rs` moved to the end of
  the file (`items_after_test_module`, it had sat between the parse and emit
  halves). `wire::crypto::pos_seed` keeps its eight parameters under
  `#[expect(clippy::too_many_arguments)]`: they map one-to-one onto the node
  the spec hashes. In `tests/`: needless borrows, a `&PathBuf` parameter that is
  now `&Path`, a duplicated `allow(dead_code)`, and a boxed-closure table that
  is now fn pointers behind a type alias.

## Local modifications for the devcontainer dev loop and CI

- **Ran `cargo fmt`** (rustfmt 1.96.0) over the workspace; the tree was imported
  unformatted.
- **Added bash twins of the five PowerShell scripts** (`build-wasm.sh`,
  `serve.sh`, `test-wasm.sh`, `b4.sh`, `localstack.sh`). The `.ps1` files are
  unchanged and remain the Windows dev loop; flags map one-to-one
  (`.\b4.ps1 -Reset -NoRun` ⇄ `./b4.sh --reset --no-run`). Differences beyond
  syntax:
  - `build-wasm.sh` stays on the stable toolchain and uses `RUSTC_BOOTSTRAP=1`
    instead of a nightly override (the approach of
    `packages/braid/scripts/build-wasm.sh`). It and `serve.sh` clear an
    inherited `RUSTFLAGS`, which would otherwise replace the atomics rustflags
    in `crates/braid/.cargo/config.toml` entirely (the devcontainer's devenv
    exports `RUSTFLAGS=-Awarnings`).
  - `build-wasm.sh` and `test-wasm.sh` verify that the `wasm-bindgen` CLI on
    `PATH` matches the `Cargo.lock` pin before building. The main workspace
    (`strand`, `braid`, `sequent-core`) was moved to this workspace's
    `=0.2.123` pin, so the single CLI in the repository's `devenv.nix` and
    `flake.nix` files serves both.
  - `test-wasm.sh` accepts `geckodriver` as well as `chromedriver`
    (`wasm-bindgen-test-runner` drives either); the devcontainer ships
    geckodriver and Firefox.
  - In the devcontainer, `localstack.sh` runs LocalStack the way every other
    dev service runs: as the `localstack` compose service (opt-in `wbraid`
    profile in `.devcontainer/docker-compose-base.yml`, with a
    `configure-localstack` one-shot creating the bucket and applying CORS,
    mirroring `configure-minio`), addressed as `http://localstack:4566` on the
    project network. Outside a compose project it keeps `localstack.ps1`'s
    standalone docker-run flow. The image is pinned to `localstack/localstack:4`
    in both paths: from the 2026 releases on, `latest` exits at startup without
    an auth token.
  - `b4.sh` defaults `AWS_ENDPOINT_URL` per environment (a pre-set value wins)
    and, when no AWS CLI is installed, runs the `amazon/aws-cli` image on the
    project network. Linux unlinks open files without complaint, so the reset
    refuses to run while a `b4v6` process exists instead of relying on a
    locked-file error.
- **`server.py` honours a `PORT` environment variable** (default 8080,
  unchanged); `serve.sh` falls back to `WBRAID_SERVE_PORT` for it.
- **Made the b4 listen address and the live tests' b4 URL configurable**:
  `crates/b4/src/main.rs` honours `WBRAID_B4_BIND` and the two `#[ignore]`d
  live-b4 tests (`protocol_test_http*.rs`) read `WBRAID_B4_URL`, both keeping
  the upstream `127.0.0.1:3000` default when unset. The step devcontainer sets
  them to port 3005 in `.devcontainer/.env.development`, with
  `WBRAID_SERVE_PORT=8085` and `WBRAID_S3_ENDPOINT_URL=http://localstack:4566`
  alongside: 3000 is the voting portal's, the host's 8080 is forwarded to
  Hasura, and S3 is the `localstack` compose service. `emulator.html` keeps its
  `http://127.0.0.1:3000` default; the URL field is edited by hand.
- **Added `.github/workflows/wbraid.yml`**, scoped to changes under
  `packages/wbraid/`: `cargo fmt -- --check`; clippy as the two invocations
  listed under "Local modifications for clippy"; `cargo test --release`, with
  the live-b4 tests still `#[ignore]`d until CI has a b4 and LocalStack to run
  them against; and a `wasm-core` build for `wasm32-unknown-unknown` from the
  workspace root, where `crates/braid/.cargo/config.toml` does not apply, so
  plain cargo suffices and no wasm-bindgen CLI is involved. The shared
  `setup-rust-tests` action gained optional `components`/`targets` inputs for
  this (defaults unchanged).
