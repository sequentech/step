---
id: package-coverage
title: Package Test Coverage
sidebar_label: Package Test Coverage
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The coverage tools run package tests, export reports and compare two Git
revisions. Configuration lives in `scripts/coverage/profiles.toml`: each native
profile names its package, Cargo features, fixture environment and source scope.
Use the profile that matches the code you changed. A feature omitted from one
profile needs an enabled-feature test run; omission does not establish coverage.

## Native Rust

Install the repository's Rust toolchain and LLVM tools, then cargo-llvm-cov:

```bash
rustup show active-toolchain
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --version 0.9.1 --locked
```

From the repository root:

```bash
python3 scripts/coverage/run.py sequent-core --baseline --offline
```

Prepare locked dependencies before using `--offline`. Omit it when dependency
fetching is needed. `--baseline` records a measurement without enforcing the
absolute line threshold in the profile; it still rejects failed tests and invalid
reports. Without `--baseline`, the runner also checks `minimum_lines` and source
inventory completeness. These command modes are separate from the CI comparison.

Reports are written under `coverage/<profile>/<run>/`. Use `--output-dir` to
choose another directory, preferably outside the checkout. Each run records the
revision, tool versions, features, test environment, exclusions, exact counters
and source inventory alongside HTML, JSON and LCOV exports.

Run tests with synthetic data in a disposable worker. Profiles do not provide
isolation themselves. Database and browser fixtures must be installed locally;
never substitute a deployment endpoint or production credentials. Use focused
Cargo tests while editing and a complete measurement after the changes stabilize.

## Interpret the reports

- Lines, functions and LLVM regions are distinct native metrics. LLVM regions
  are not branch coverage. WASM targets and optional feature combinations need
  their own runs and must not be folded into an incompatible native result.
- Stable native LLVM includes inline unit-test code. Package summaries therefore
  may mix application and test code; they are not production-only percentages.
- Generated serialization stays counted. Prefer literal wire expectations,
  rejected inputs and failure propagation to assertions that merely print a
  generated value or duplicate the implementation.
- Every Rust source file is inventoried, including files absent from LLVM output.
  An absent measurement is a gap to investigate, not a measured zero or permission
  to omit the source. Identify which enabled profile exercises feature-gated code.
- `scope_exceptions` explains files without executable regions, such as module
  declarations. These entries cannot remove measured code from the denominator;
  an exception fails if the file acquires measured executable code.
- Package summaries select the named package's `src` files. Raw exports can also
  contain measured workspace dependencies; their combined totals are not the
  named package's counters.

A run clears old LLVM counters and workspace binaries, holds a checkout lock and
checks source identity before and after measurement. Do not edit a checkout while
its measurement runs. Keep output outside both checkouts when comparing revisions.

## Compare revisions in CI or locally

The **Package coverage** workflow measures the exact PR base and the PR merged with
that base, using each revision's own tests with matching instrumentation. Base
changes a PR has not merged yet are therefore neither credited nor blamed to it. Each metric must stay the
same or increase. Comparisons use exact fractions: a line increase cannot offset
a function decrease, and rounded display percentages do not determine the result.

Prepare two clean checkouts and their locked dependencies, then run:

```bash
python3 scripts/coverage/ci.py rust sequent-core \
  --base ../step-base --head . --output ../coverage-comparison
```

Exit `0` means no decrease, `1` means a regression, and `2` means invalid or
incomplete evidence. Failed tests, empty reports, changed source, incompatible
instruments and new source-inventory gaps cannot pass. Existing inventory gaps
remain visible in both reports.

A package genuinely absent from the base is labelled **INITIALIZED**. Existing
source without a valid measurement must be measured; missing coverage is never
converted into a zero baseline. Review changes to instrumentation and exclusions
as measurement-policy changes, not as test improvements.

For a manual hosted run, select the candidate branch in **Actions → Package
coverage → Run workflow**, choose a supported profile and supply the base ref.
Download the comparison artifact to inspect both measurements and the verdict.
The workflow's dispatch choices list the profiles supported by its worker.

## Test the coverage tools

From the repository root, using Python and the pinned coverage package:

```bash
python3 -m pip install coverage==7.16.0
python3 -m unittest discover -s scripts/coverage
```

The tests exercise exact comparison boundaries, real test-removal regressions,
invalid reports, source changes, failed commands and concurrent-run rejection.
See `scripts/assurance/README.md` for lint configuration and commands.
