<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Error conversion macro tests

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

Do not interpret the lack of a valid coverage report as a measured zero. LLVM records both the expansion helpers exercised
by runtime unit tests and the public entry point exercised while compiling
consumer fixtures. The package percentage combines those two phases.

No source files or positive counters are excluded. The downstream Harvest build
also compiles Windmill's real Celery consumers. This caught renamed `TaskResult`
and `WrapResult` aliases, which have dedicated compiled regressions here.
The macro recognizes the conventional `Result` suffix; it cannot resolve
arbitrarily named Rust aliases. Branch coverage and successful distributed task
execution need separate evidence.

Cancellation controls drop an owned resource exactly once both before first polling
and while a task is suspended. Successful and error completions provide matching
controls; these tests execute the public generated future without a task broker.

The native CI profile runs each revision's existing Windmill consumers as
well as the macro's own tests. Start synthetic PostgreSQL 16 on loopback port
3322, with database/user/password all `test`; the profile supplies those public
settings. Standalone `cargo test -p wrap-map-err` still needs no database.

Apply the same consumer configuration to both actual PR revisions. Compiler
counters can provide a valid macro baseline even if the original macro has no
unit tests. Consumer source counters never inflate macro totals. Use each
revision's own source and tests; an empty or failed consumer run cannot pass.
Incompatible consumer sets fail comparison. Full distributed Celery execution
requires a separate integration fixture.

The original body finishes, including cleanup of captured arguments, before
`Into::into` converts its returned error. Error converters must not depend on
those captured arguments remaining alive. Cancellation tests cover cleanup of
unpolled and suspended futures; they do not promise a resource lifetime across
the later conversion.
