---
id: immu-board
title: Immu-board row contracts
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

From `packages/`, using the repository's pinned Rust toolchain:

```sh
cargo test --locked -p immu-board --tests
```

The tests use literal synthetic ImmuDB rows. They verify independent creation and
statement times, binary message bytes, optional voter identity, column ordering,
wrong value types, malformed names, missing values and duplicate columns. A
complete valid row accompanies rejection cases. The namespace test checks the
literal tenant/event board name derived from UUIDs.

No database or credentials are required for these mapping tests. The client RPC
suite in `immudb-rs` separately checks its transport behavior. It does not replace
an end-to-end test of immu-board's SQL construction, pagination or transaction
retry logic.

The additional package unit-test workflow runs these contracts and production
Clippy. That workflow does not publish coverage percentages. A coverage gate
must execute both revisions under the same profile; an unavailable baseline
must never be reported as zero or as a passing comparison.
