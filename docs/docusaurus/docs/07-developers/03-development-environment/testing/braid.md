---
id: braid
title: Braid protocol and persistence tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

From `packages/`, with the repository's pinned Rust toolchain:

```sh
CARGO_PROFILE_TEST_OPT_LEVEL=2 RUST_TEST_THREADS=2 RAYON_NUM_THREADS=2 cargo test --locked -p braid --tests
cargo test --locked -p braid --lib coverage_contracts
cargo test --locked -p braid --test util_contracts
```

The persistence tests own temporary SQLite databases and blob directories. They
verify local ordering independently of remote IDs, restart reads, exclusive
pagination, duplicate rejection, transaction rollback and malformed or missing
blob errors. Trustee cases verify configuration bootstrap counts and rejected
messages followed by valid controls. Utility cases cover canonical encoding,
filesystem errors and preservation of typed protocol errors.

Private v10 storage/trustee APIs are exercised by unit modules whose test bodies
live under `tests/unit/`; no production visibility is widened for tests. The
existing full in-memory protocol test remains part of the suite.

From the repository root:

```sh
python3 scripts/coverage/run.py braid --baseline
```

The native profile optimizes test builds to bound cryptographic execution time.
Inline helpers and generated protocol logic remain counted. CI compares lines,
functions and LLVM regions separately against the actual PR base; regions are
not branches. The full protocol test chooses trustee counts randomly, so inspect
per-file differences if measurement varies.

Live gRPC/PostgreSQL transports, long-running sessions and optional cryptographic
backend configurations need separate execution. The old `local.rs` implementation
is not declared by any module; tests exercise the compiled `local2.rs` backend.

Blob-batch regressions check that ordinary SQL/decode failures remove newly
created files and preserve committed rows/files. The writer lock covers metadata
and file creation; a retry must read its own bytes. This cleanup does not make
SQLite and the filesystem one crash-atomic transaction: process termination or a
filesystem refusing deletion needs separate recovery testing.
