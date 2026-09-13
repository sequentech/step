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

## Run a package

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
The raw LLVM JSON and LCOV files come from the same test execution.

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
- Native stable LLVM coverage includes inline unit-test code and fixture helpers.
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

The **Rust package coverage** workflow tests the coverage tooling automatically on
relevant pull requests. Its own tests must reach 95% combined line/branch coverage.

Package coverage is initially **on demand**, since existing PR checks already run
the Rust suites. This avoids duplicating every Rust build while the package
baselines are being established:

1. Open **Actions → Rust package coverage → Run workflow**.
2. Choose the branch and package profile.
3. Open the **Core package coverage baseline** job summary.
4. Download its `coverage-<profile>` artifact for HTML, LCOV and detailed logs.

No coverage-service account or secret is needed. GitHub's normal Actions usage
still applies. Artifacts expire after seven days. A green baseline job means the
measurement completed; its summary explicitly says **TARGET NOT MET** when the
95% requirement or scope check fails. Promote a package to an automatic strict
check after its package issue's acceptance criteria are satisfied.

## Add tests and profiles

Start from the uncovered lines and the behavior they represent. Cover valid use,
boundaries, invalid inputs, and controlled failures; do not add assertions that
only repeat the implementation. For a regression, preserve the failing observation
before fixing it, then rerun the full package suite and relevant consumers.

Add a profile only after checking its Cargo features and required services. The
profiles state their dependencies explicitly:

| Profile | Local requirements and scope |
| --- | --- |
| `sequent-core` | Native application features; synthetic fixtures. |
| `strand` | Default native cryptographic backend. |
| `velvet` | Chrome for in-place PDF tests; `DOC_RENDERER_BACKEND=inplace`. |
| `wrap-map-err` | Compiler expansion and runtime parser tests. |
| `harvest` | Local Rocket client and public SQL configuration supplied by the profile; service workers are not started. |
| `windmill` | PostgreSQL 16 with synthetic fixture credentials, plus native default FIPS dependencies. |
| `step-cli` | File conversion, import validation and subprocess tests; full election services remain a separate fixture. |

For example, run `python3 scripts/coverage/run.py harvest --baseline --offline`.
A `test_environment` table contains only public fixture settings and overrides
matching inherited environment variables. Never put deployment secrets in it.
Every measurement clears workspace binaries and raw counters. Keeping binaries
from an earlier feature profile can contaminate the next report even when its
counters are fresh. External dependencies stay cached. For quick development
feedback, run focused Cargo tests; reserve a complete coverage measurement for
a finished change. Compiler-time macro coverage is rebuilt as part of that run.

Windmill also has a native profile backed by the same PostgreSQL 16 fixture used
in Rust CI: loopback port `3322`, with database, user and password all `test`.
These are synthetic development credentials. Its SQL tests use real transactions
and PostgreSQL's parser; they must never point at a production database.

```bash
python3 scripts/coverage/run.py windmill --baseline --offline
```

The native run keeps existing ignored tests visible. The voter-channel database
regression can be run separately with the fixture environment and `--ignored
--exact`; the activity-log case needs additional local services. See
`packages/windmill/tests/README.md` for the tested boundaries and remaining scope.

The package READMEs describe their test boundaries. Successful database, identity,
broker and storage workflows still need their corresponding local fixtures.
Package progress remains tracked separately:

| Work | Meta issue |
| --- | --- |
| Coverage tooling | [#13291](https://github.com/sequentech/meta/issues/13291) |
| Sequent Core | [#13292](https://github.com/sequentech/meta/issues/13292) |
| Strand | [#13293](https://github.com/sequentech/meta/issues/13293) |
| Velvet | [#13294](https://github.com/sequentech/meta/issues/13294) |
| wrap-map-err | [#13299](https://github.com/sequentech/meta/issues/13299) |
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
COVERAGE_FILE=coverage/tooling/.coverage python3 -m coverage report --fail-under=95
```

The tests cover threshold boundaries, missing files, invalid counters, source
changes, failed commands, interrupted processes and concurrent-run rejection.
