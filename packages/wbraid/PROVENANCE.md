<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Provenance of `packages/wbraid`

## Import

`packages/wbraid` was imported from the `sequentech/step` branch
`feat/braid-0.6.1/main` at commit `32ede628b3` (author: David Ruescas). Only the
`packages/wbraid` subtree was taken; nothing outside it was carried over.

Two subtrees were deleted at import time and are recoverable from the source
branch if they are ever needed:

- `packages/wbraid/legacy/`
- `packages/wbraid/crates/braid/vs_lift/`

## Successor branch

Work on `feat/braid-0.6.1/main` has continued on `exp/braid-stateright/main`,
tip `4cdb414941070879e1f35612eedb6e71ee9b8792` at the time of this import. That
branch is the source for the deferred stateright / model-checking work, which is
not part of this import.

## `crates/vsc`

`crates/vsc` (crate version `0.2.1`, `[lib] name = "cryptography"`) is a vendored
fork of https://github.com/FreeAndFair/VoteSecure, licensed Apache-2.0,
© Free & Fair. It was taken verbatim from `feat/braid-0.6.1/main` @ `32ede628b3`;
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
