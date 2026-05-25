<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

 SPDX-License-Identifier: AGPL-3.0-only
-->

`sequent-core` contains the shared Rust types, ballot-processing primitives, and service helpers used across the `Step` backend crates and frontend packages.

Most of the crate is feature-gated so consumers can keep their dependency surface small. The full API reference is most useful when documentation is generated with all features enabled.

## Module Guide

- `types`: shared domain types that are used across services, reports, and frontend bindings.
- `ballot`, `ballot_style`, `multi_ballot`, `plaintext`, `interpret_plaintext`, `mixed_radix`, and `ballot_codec`: ballot modeling, encoding, decoding, and normalization utilities.
- `encrypt`: ballot encryption helpers built on the shared cryptographic primitives.
- `serialization`: serialization helpers for the crate's public data structures.
- `services`: higher-level business logic used by the surrounding `Step` services.
- `util`: cross-cutting helpers for dates, retries, configuration, MIME types, integrity checks, and related infrastructure concerns.
- `plugins_wit`: `WIT` bindings for plugin integration.
- `signatures`: signature helpers and verification utilities.
- `sqlite`: `SQLite`-backed helpers.
- `temp_path`: report-oriented temporary file helpers.
- `wasm`: the `WebAssembly` API exported to frontend packages.

## Feature Flags

- `default_features`: enables the core ballot-processing and service modules used by the main backend and frontend flows.
- `wasm`: enables the `WebAssembly` bindings exposed under `wasm`.
- `reports`: enables report-generation support, including temporary-path helpers.
- `signatures`: enables signature-related helpers.
- `sqlite`: enables `SQLite` support.
- `plugins_wit`: enables plugin `WIT` bindings.

## Generating Docs

Build the complete local API reference with all features enabled:

```bash
cargo doc -p sequent-core --all-features --no-deps
```

`docs.rs` is configured to build this crate with all features enabled as well.