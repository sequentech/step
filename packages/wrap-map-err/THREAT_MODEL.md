<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# wrap-map-err threat model

wrap-map-err is a first-party proc-macro crate with one export, the attribute `#[wrap_map_err(E)]` in `src/lib.rs`. It runs only inside rustc when windmill is built (harvest links windmill), on developer hosts, CI and image builds, and has no runtime code, I/O or secrets of its own. windmill puts `#[wrap_map_err::wrap_map_err(TaskError)]` on all 58 Celery tasks in `../windmill/src/tasks/`, always between `#[instrument]` and `#[celery::task]`. The code it generates decides how every task result, including key and tally ceremonies, voter credential letters, imports, exports and publication, reaches Celery as success, failure or retryable failure. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Generated task code**: the rewritten signature and body of each windmill task. Integrity: an `Err` stays an `Err` with the same error class, and the `Ok` value does not change.
- **Build hosts and the windmill and harvest images**. Integrity: code that runs at compile time can change what ships.
- **Error content passed through the mapping**. Confidentiality: task errors reach worker logs and Celery task results.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/lib.rs` `wrap_map_err` attribute (`attr` = target error type, `item` = function) | Developers who write windmill tasks; rustc during builds | operator | `wrap_map_err_impl` parses with `syn::parse2` into `ItemFn` and `Type`; a parse failure panics inside the macro, which fails the build |
| `Cargo.toml` dependencies `syn` (2.0, `full`), `quote`, `proc-macro2` | crates.io publishers, through `../Cargo.lock` | operator | Versions and checksums pinned in `../Cargo.lock`; production images build with `--locked` |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| wrap-map-err-T1 | Tampering | The expansion changes a task's outcome: an error becomes a success or is dropped, the `Ok` value changes, or a retryable error loses its class, so a failed ceremony, tally or import is reported to Celery as done or retried wrongly | `src/lib.rs` `wrap_map_err_impl` binds the original body to `let result: <original return type>` and returns `result.map_err(::std::convert::Into::into)`; only the error is converted into the target type. A `?` or `return` that escapes the block exits the outer function and either converts the same way or fails to compile | Mitigated |
| wrap-map-err-T2 | Tampering | A compromised release of `syn`, `quote` or `proc-macro2`, or a malicious change to this crate, runs code on build hosts and injects code into every windmill task | Three dependencies only (`Cargo.toml`); exact versions and checksums in `../Cargo.lock`; `--locked` production builds. Wider supply-chain controls are SYS-T20 and SYS-T21 in the system model | Partial |
| wrap-map-err-T3 | Information disclosure | Secrets, credentials or voter data in a task error reach worker logs and Celery task results through the mapped error | The macro adds no data of its own. Error content is covered by windmill's model | Not verified |
| wrap-map-err-T4 | Denial of service | A function shape the macro does not handle breaks the windmill build | An unsupported shape fails to compile (a panic in `wrap_map_err_impl` or a type error), so no task with a changed outcome is produced | Accepted (build-time failure only; fails closed) |

## Assumptions

- windmill's threat model covers what task errors and logs contain.
- `#[instrument]` stays the outermost attribute, so `?` and `return` in a task body exit the async block it generates and the result then passes through `map_err`.
- Builds use `../Cargo.lock` with `--locked`, and the system CI and image controls (SYS-T20, SYS-T21) protect build hosts and published images.
- The rusty-celery fork behind `celery::task` records an `Err(TaskError)` as a failed task and applies `max_retries` and `retry_for_unexpected` by `TaskError` variant.

## Review focus

- Expansion semantics stacked with `tracing::instrument` and `celery::task`: attribute order, early `return`, `?` and async bodies, which decide whether every failure reaches Celery as a failure of the right class.
- What task errors carry into worker logs and task results.
- Unit tests for `wrap_map_err_impl` over supported and rejected return shapes.
- Dependency pinning and advisory scanning for the proc-macro dependencies, which run with build-host privileges.
