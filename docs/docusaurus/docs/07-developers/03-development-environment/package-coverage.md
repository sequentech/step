---
id: package-coverage
title: Package Test Coverage
sidebar_label: Package Test Coverage
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Step's package coverage target is **95% of measured source lines**, aiming for
100% where the extra tests exercise meaningful behavior. Start with the core Rust
packages. Each package remains open until its coverage target and source-scope
review pass; a successful baseline run does not satisfy that requirement.

**The CI gate is no coverage decrease, not a 95% threshold.** Each measured metric
must stay the same or increase relative to the PR's base commit. Equal 70% passes;
99% falling to 98% fails. Compare exact fractions, not rounded display percentages.
Lines cannot compensate for lost branch coverage, and one package cannot compensate
for another. The 95% target remains the objective for the coverage work.

## Sequent Core

The `default_features,keycloak` profile has **484 passing tests**, **96.80% line
coverage**, **95.43% function coverage** and **95.65% LLVM region coverage**. Its
native aggregate includes inline test code; standalone fixtures and test files
are excluded. It is not yet a
production-only or actual branch score.

Macro-generated Borsh contract tests pin explicit policy names and bytes, reject
truncated records and propagate failures through nested ballot serializers. All
generated functions remain counted; the measured profile has no wholly uncovered
Borsh implementation. JSON exports also filter excluded fixture function records
without changing any counters.

The tests cover local Keycloak HTTP operations and token caches, real PostgreSQL
User row mapping, malformed ballot/audit boundaries, voting state, scheduling and
presentation data. PostgreSQL tests launch private temporary clusters and require
`postgresql libpq-dev` or `PG_BIN` pointing to the server binaries; they never use
an existing database or a production connection string.

Source commit `11dec93c4484e6f7876c2045521a0b554a0eeb99` measures 11,921/12,315
lines, 1,398/1,465 functions and 15,194/15,885 regions with Rust 1.96.0 and
cargo-llvm-cov 0.9.1. The native line improvement target is met. Remaining work
includes realizable failure cases, integration with a running identity provider
and separate WASM/service profiles. All 47 files missing from the LLVM report
are classified in the test guide; their outstanding measurements still prevent
the strict overall target from passing.

The [Sequent Core Tests guide](https://github.com/sequentech/step/blob/main/packages/sequent-core/tests/README.md)
contains the uncovered-line breakdown, tested contracts and remaining feature work.
It also inventories the remaining 67 unexecuted functions, separating useful
follow-up cases from inline test diagnostics, guards after immutable validation
and specific infeasible serialization/numeric-conversion errors. Do not force
100% by adding tests without a useful behavioral assertion.

Use the repository's development environment, Python 3.11 or newer, and the Rust
version in `rust-toolchain.toml`. Install the two coverage components once:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --version 0.9.1 --locked
```

From the Step repository root, measure the initial `sequent-core` baseline:

```bash
python3 scripts/coverage/run.py sequent-core --baseline
```

The command prints its report directory under `coverage/sequent-core/`. Read
`summary.md` first, then inspect the HTML report or `uncovered-lines.log` to choose
the next behavior to test. `summary.json` records exact counters, source revision
and hashes, enabled features, tool versions, test counts and measurement limits.
All exports come from the same test execution. `llvm.json`, HTML and LCOV omit
excluded code from both the covered and total counters. No unfiltered report is
generated.

Run the strict target check with the same command, without `--baseline`:

```bash
python3 scripts/coverage/run.py sequent-core
```

The exit codes have different meanings:

| Code | Meaning |
| --- | --- |
| `0` | Strict target passed, or a requested baseline was successfully measured. Check `mode` and `passes` in the summary. |
| `1` | Strict target not met: coverage is below 95%, or source files remain unaccounted for. |
| `2` | Measurement failed: tests/build failed, report invalid, wrong tools, source changed, or another run owns the checkout. |

For an isolated worker, fetch dependencies during its preparation stage, disable
external network access, and add `--offline` to the command. Never copy production
credentials into a test worker. The coverage command runs tests; it does not create
a sandbox or enforce network isolation itself.

## Keep production failure handling explicit

The additional assurance lints apply to production code. Unit and end-to-end
tests may use `unwrap`, `expect`, non-null assertions and similar shortcuts when
setting up fixtures or checking outcomes.

The first rollout enforces the Lightweight Assurance policy in Sequent Core's
tally-sheet validation module for non-test builds. UI Core applies stricter
TypeScript rules to production source. The URL truststore provider checks
production Java with PMD during Maven `verify`, and the Python coverage tooling
runs stricter Ruff rules on its production source. These are initial scopes,
not whole-repository compliance.

Read `scripts/assurance/README.md` for the exact commands, enforced rules and
remaining adoption work.

## Read the result honestly

The profiles in `scripts/coverage/profiles.toml` declare the package and features
being tested. `sequent-core` uses `default_features,keycloak`, matching its native
Rust CI profile. Its empty default feature set would omit much of the application
logic. Strand's initial profile measures its default native backend.

- Percentages use covered and total counters, not a rounded percentage supplied
  by LLVM. A result of 94.999% fails the 95% check.
- Package summaries count that package's `src` files. Raw HTML, JSON and LCOV may
  also show workspace dependencies; their combined total is not the package gate.
- Every Rust source file is inventoried, including files with no LLVM measurement.
  An exact file may have a reviewed explanation in `scope_exceptions`, for example
  module declarations without executable code or a disabled feature. Exceptions
  stay visible and cannot remove measured code from the denominator.
- `excluded_files` lists exact support/test filenames and a reason for each.
  Core excludes `src/fixtures/ballot_codec.rs`, `src/fixtures/encrypt.rs` and
  `src/election_config/validate_tests.rs` from JSON, HTML, LCOV and the score.
  Tests still run. Excluded paths and reasons appear in `summary.json`, but their
  counters are absent from every report. Base and head use identical exclusions.
- Native stable LLVM coverage still includes inline unit-test code.
  This can inflate the number. Prefer new tests in `tests/` or separate
  `*_tests.rs` files; account for those files explicitly in the scope review.
  Do not call the existing aggregate “production-only coverage.”
- Native coverage does not certify WASM, other feature combinations, branch
  coverage or doctests. Those configurations require separate tests and reports.
- A high percentage does not prove that election rules or security properties
  hold. Expected ballot/tally values must come from an independent rule or vector,
  and invalid inputs need assertions about the actual failure behavior.

Each run writes a fresh output directory, clears previous LLVM counters, and holds
a checkout-wide lock. Do not edit source while it runs. Both the package source
and complete checkout identity are recorded to catch changes in dependencies or
configuration too. Build dependencies can be reused locally; independent security
verification must start with a clean worker and fresh writable caches.

## Use GitHub Actions

The **Package coverage** workflow compares fresh measurements of the exact PR
base and head commits. On a push to `main`, it compares the previous commit with
the new commit. It runs each revision's own tests with matching instrumentation;
there is no editable baseline percentage or coverage-service dependency.

- Python tooling: compare lines and branches separately on relevant PRs.
- Sequent Core: compare native lines, functions and LLVM regions automatically
  on relevant PRs and pushes to main. Regions are not branch coverage.
- Strand: the same native comparison is available on demand.

For a native comparison:

1. Open **Actions → Package coverage → Run workflow**.
2. Choose the candidate branch and the native profile.
3. Enter the base branch or commit to compare with, normally the PR's target branch.
4. Read **Rust package coverage — no decrease** and download its artifact.

The hosted native worker currently supports `sequent-core` and `strand`. Other
profiles can use the same comparison command in a worker with the service fixtures
described by their package guides. The automatic native gate currently covers Sequent Core. Enabling the other
packages as required checks remains part of their individual coverage work.

```bash
python3 scripts/coverage/ci.py rust sequent-core \
  --base ../step-base --head . --output ../coverage-comparison
```

Prepare both clean checkouts and their locked dependencies first. The command
runs tests and does not provide isolation itself. Exit `0` means no decrease,
`1` means a regression, and `2` means invalid or incomplete evidence. Failed tests,
empty reports, changing source, incompatible instruments and new Rust inventory
gaps cannot pass. Existing Rust scope gaps remain visible rather than being
mistaken for a complete production measurement.

A component whose source did not exist at the base commit gets an explicitly
labelled **INITIALIZED** measurement. Existing source without coverage reports
must be measured; a missing or broken baseline is never treated as zero. Changes
to instrumentation or exclusions need review as measurement-policy changes, not
as test-coverage improvements. Keep the workflow and delegated policy scripts
under maintainer review; a PR must not approve its own coverage-policy relaxation.

Artifacts include both measurements, exact commit identities and a paired verdict.
They expire after seven days. 95% remains an improvement target even when a
comparison passes. The local native runner's optional strict target check is
separate from this CI decision.

## Add tests and profiles

Start from the uncovered lines and the behavior they represent. Cover valid use,
boundaries, invalid inputs, and controlled failures; do not add assertions that
only repeat the implementation. For a regression, preserve the failing observation
before fixing it, then rerun the full package suite and relevant consumers.

Add a profile only after checking its Cargo features and required services. The
initial profiles deliberately omit Harvest, Windmill and Step CLI until their
synthetic local service recipes are ready. Their work is tracked separately:

| Work | Meta issue |
| --- | --- |
| Coverage tooling | [#13291](https://github.com/sequentech/meta/issues/13291) |
| Sequent Core | [#13292](https://github.com/sequentech/meta/issues/13292) |
| Strand | [#13293](https://github.com/sequentech/meta/issues/13293) |
| Velvet | [#13294](https://github.com/sequentech/meta/issues/13294) |
| Harvest | [#13295](https://github.com/sequentech/meta/issues/13295) |
| Windmill | [#13296](https://github.com/sequentech/meta/issues/13296) |
| Step CLI | [#13297](https://github.com/sequentech/meta/issues/13297) |
| Integration and benchmarks | [#13298](https://github.com/sequentech/meta/issues/13298) |

To test the reporter and runner themselves:

```bash
python3 -m pip install coverage==7.16.0
mkdir -p coverage/tooling
COVERAGE_FILE=coverage/tooling/.coverage python3 -m coverage run \
  --branch --source=scripts/coverage --omit='*/test_*.py' \
  -m unittest discover -s scripts/coverage
COVERAGE_FILE=coverage/tooling/.coverage python3 -m coverage report
```

The tests cover exact comparison boundaries, a real regression caused by removing
a test, invalid reports, missing files, source changes, failed commands, interrupted
processes and concurrent-run rejection. Native local target tests remain separate.
