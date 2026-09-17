---
id: braid
title: Braid protocol and persistence tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

This guide applies to `packages/braid`. The separate `packages/wbraid` workspace
provides its own `TESTING.md` and model-checking suites.

From `packages/`, with the repository's pinned Rust toolchain:

```sh
CARGO_PROFILE_TEST_OPT_LEVEL=2 RUST_TEST_THREADS=2 RAYON_NUM_THREADS=2 cargo test --locked -p braid --tests
cargo test --locked -p braid --test storage_contracts --test trustee_contracts --test util_contracts
```

The persistence tests own temporary SQLite databases and blob directories. They
verify local insertion order independently of remote IDs, restart reads, exclusive
pagination, duplicate rejection, transaction rollback, and missing/corrupt blob
errors. Envelope metadata is deliberately different from the signed message, so
tests also verify that stored identity comes from the message itself.

Trustee tests check configuration bootstrap counts, rejected initial messages and
signature failures followed by valid controls. Utility tests check canonical
encoding, filesystem errors and preservation of typed protocol errors. The
existing full in-memory protocol test remains part of the suite.

From the repository root:

```sh
python3 scripts/coverage/run.py braid --baseline
```

The native profile optimizes test builds to keep cryptographic protocol runs bounded.
It counts inline helpers and generated protocol logic. CI compares
lines, functions and LLVM regions separately against the actual PR base. Regions
are not branches. The existing full protocol test chooses trustee counts randomly;
inspect per-file differences if that test causes measurement variance.

The HTTP protocol test needs a synthetic bulletin-board service and object store.
Browser IndexedDB/OPFS, concurrent long-running sessions, interactive commands and
recovery from partial filesystem writes require separate targeted execution.
Passing native tests does not establish coverage of those environments.
