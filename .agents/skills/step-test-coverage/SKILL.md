---
name: step-test-coverage
description: Collect, combine or diagnose Step frontend and Rust unit/E2E coverage. Use for instrumentation, source mapping, profile completeness and CI coverage reports, not test-scenario design or load benchmarks.
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Read [the operating guide](../../../docs/docusaurus/docs/07-developers/e2e.md) and `packages/e2e/runner/coverage.py`. Use `scripts/e2e run --coverage e2e` or `--coverage combined` against the disposable stack. Instrumentation is not a production load-test configuration.

Frontend E2E uses Istanbul in webpack, including navigation/teardown collection. Include unvisited source files in the declared denominator. Unit and webpack statement identities differ; combine remapped source-line identities, not raw statement IDs. Separate function/branch figures if their instrumentation is not compatible. Generated GraphQL, translations, mocks, stories, tests and WASM are outside the declared frontend scope.

Rust uses the pinned compiler's LLVM tools and workspace instrumentation. Keep instrumented binaries alongside profiles; each service has a separate directory and collision-safe process/module filenames. The runner uses tested Linux continuous profiles with counter relocation. Require profiles from every expected service, even on a failed run. A killed container or missing profile is not zero coverage or success.

Both unit and E2E inputs must match the build revision and local source digest. Collect them separately, then union executable source lines. Never average percentages or add covered-line counts with duplicate file/line identities. Show frontend and Rust scopes separately; do not imply native coverage measures WASM, Java, SQL or unlinked crates.

Combined mode currently runs voting-portal unit tests and selected sequent-core/step-cli native unit binaries. Extend that scope explicitly when adding packages. Keep normal and coverage compilation caches distinct. Validate missing-profile rejection, source identity, source remapping and uncovered denominators, not only report rendering.

Actions publishes Markdown, machine-readable line maps and an HTML source view under the run's `report/`. Raw profiles and private browser data are diagnostic inputs. Report any unsupported or uncollected scope precisely.
