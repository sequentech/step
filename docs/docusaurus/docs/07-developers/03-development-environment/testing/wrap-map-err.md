---
id: wrap-map-err
title: Error conversion macro tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/wrap-map-err/tests/`](https://github.com/sequentech/step/blob/feat/meta-13302-ui-essentials-coverage/main/packages/wrap-map-err/tests). Commands state their working directory.

`behavior.rs` compiles functions through the public attribute macro and calls
them. These controls check actual generated Rust: default-error Result aliases,
qualified Results and renamed `TaskResult`/`WrapResult` imports, borrowed generic values, early returns, question marks,
unchanged non-Result functions, and asynchronous suspension/resumption. A leaf
error that can convert only to the original error type guards against silently
changing the meaning of `?`. A Send-but-not-Sync owned argument checks async
capture semantics; opaque success types exercise inference in the private body
annotation. Unsafe-function consumers verify that Rust’s lexical unsafe context
survives wrapping.

`support/expansion.rs` checks syntax that cannot be a compiled consumer: invalid
attributes, non-function items, unsupported Result arguments and const-function
diagnostics. It also verifies that function metadata survives expansion.

From `packages`, run `cargo test -p wrap-map-err --locked --offline`. For a full
measurement with the pinned tools, run from the repository root:

```sh
python3 scripts/coverage/run.py wrap-map-err --offline
```

The original package had no tests. Do not interpret the lack of a valid coverage
report as a measured zero. LLVM records both the expansion helpers exercised
by runtime unit tests and the public entry point exercised while compiling
consumer fixtures. The package percentage combines those two phases.

No source files or positive counters are excluded. The downstream Harvest build
also compiles Windmill's real Celery consumers. This caught renamed `TaskResult`
and `WrapResult` aliases, which now have dedicated compiled regressions here.
The macro recognizes the conventional `Result` suffix; it cannot resolve
arbitrarily named Rust aliases. Branch coverage and successful distributed task
execution remain separate evidence requirements in [Meta #13302](https://github.com/sequentech/meta/issues/13302).

Cancellation controls drop an owned resource exactly once both before first polling
and while a task is suspended. Successful and error completions provide matching
controls; these tests execute the public generated future without a task broker.

The native CI profile now runs each revision's existing Windmill consumers as
well as the macro's own tests. Start synthetic PostgreSQL 16 on loopback port
3322, with database/user/password all `test`; the profile supplies those public
settings. Standalone `cargo test -p wrap-map-err` still needs no database.

This measurement-policy change applies identically to both actual PR revisions.
The original macro has no tests of its own, but its existing 353 Windmill tests
pass and compile the real Celery consumers. Their compiler counters provide a
valid macro baseline: 25/30 lines, 2/2 functions and 52/57 LLVM regions. Consumer
source counters never inflate the macro totals, and both revisions retain their
own source and tests. No tests are injected into the base, and an empty or failed
consumer run still cannot pass. Full distributed Celery execution remains a
separate integration obligation.

The paired follow-up passes: 17 macro tests plus the same 353 existing Windmill
tests, with two existing Windmill service cases ignored on both revisions. Macro
source measures 52/52 lines, 5/5 functions and 88/89 LLVM regions. All three
fractions maintain or improve the real compiler-consumer baseline above. The
consumer configuration is recorded and incompatible consumer sets fail comparison.
