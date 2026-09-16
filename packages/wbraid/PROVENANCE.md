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
