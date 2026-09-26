<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Fast development feedback loops

Design record for making the devcontainer loop **edit → see the result → run the
relevant test** fast. It keeps the detailed requirements, the decisions taken, how
they were measured and how to continue. Commands live in the developer guide
[Fast feedback loops](../docusaurus/docs/07-developers/03-development-environment/fast-feedback.md);
status and progress live in the tracking issue.

## Requirements

Less waiting and setup matters more than wider coverage. Reuse the Storybook,
Playwright, fixture and ports/adapters infrastructure already in the repository.
Architecture and tool migrations are measured experiments, not prerequisites.

### Measurement contract

- A reproducible benchmark command records commit, tool versions, machine load,
  enabled services, cache state, commands and raw samples.
- Cold startup is measured separately from warm iteration. Cold runs never clear
  shared caches: they use isolated, task-owned directories or Docker daemons.
- Warm operations take at least 10 samples (median and range); p95 claims need a
  larger sample. Cold starts take at least three samples when the cost permits, and
  the sample count is always stated.
- Four loops are measured: command to ready workspace, save to visible UI update,
  save to focused test result and push to first actionable CI result. Peak memory,
  disk footprint and the Rust build critical path are recorded where relevant.
  Browser-visible updates count, not bundler completion.
- Representative edits: a shared UI component, a portal screen, a Rust-backed
  frontend function, a Windmill service and a Harvest consumer.
- Budgets come from measured baselines. Host timing never becomes a required CI gate.

### Incremental builds and test runs

Locally and in GitHub Actions, incrementality means more than dependency download
caches:

- One shared, inspectable change/dependency model, built on existing workspace
  tooling (Cargo metadata, Yarn workspaces), decides what a change affects. It
  compares against the correct merge base and includes transitive consumers, shared
  UI, Cargo path dependencies and features, GraphQL schemas and generated clients,
  WASM consumers, fixtures, configuration, lockfiles and toolchains. Unknown impact
  or missing history selects the broader validation. Documentation-only changes do
  not compile unrelated applications.
- Only affected outputs rebuild. Task invalidation is separate from reusable
  compiler-cache keys, so a source edit still reuses unchanged compilation units.
  Output fingerprints cover dependency, toolchain, target, profile, feature and
  configuration inputs. A cache hit never proves an output or test result current.
- Affected tests run with their dependency closure. Rust filtering accounts for
  shared dependencies; per-test impact is not claimed where it is not known.
  Integration selection includes shared schemas, services and fixtures.
- Test results are cached only when the complete input and environment identity is
  reproducible; live-service results are never reused from source hashes alone.
  Required full validation stays; broader validation is scheduled after the first
  focused result, and skipped checks never look like passing tests.
- CI publishes an actionable summary (affected packages, selected and skipped checks
  with reasons, cache hits and misses, rebuilt and reused outputs, durations) and a
  stable required-check aggregator distinguishes a legitimate no-op from a failure
  or cancellation. Superseded runs are cancelled; independent work runs in parallel
  with bounded resources; WASM and UI artifacts build once per revision and are
  shared with downstream jobs; caches are optional and never cross trust boundaries.
- Validation covers a cold cache, a warm no-change run, leaf and shared UI edits, a
  Rust/WASM edit, a fixture/schema change, a lockfile/toolchain change and a missing
  or invalid cache. Local and Actions results are compared, with queue, setup, build
  and test times and the time to first useful result reported separately.

### Phase 1 — immediate local improvements

- **1. Shared UI hot reload.** In development the portals resolve `ui-core` and
  `ui-essentials` to source (assets, styles, TypeScript paths, a single React
  runtime); production keeps the package `dist` entry points. A shared component
  edit appears in every consuming portal without a library build or server restart.
- **2. Storybook as the everyday UI workspace.** Extend the existing Storybook and
  `ui-test-kit`: reusable globals for tenant/theme, locale, permissions and workflow
  state; directly linkable complete screens (voting/kiosk, admin ceremony/tally,
  results, verifier) with loading, empty, populated and error states; no login or
  provisioning; explicit, deterministic mocked network; one focused interaction-test
  command for a selected story.
- **2a. Browser-only workbench.** Reuse the Election Workbench approach (browser-only
  snapshots, booth screens, policy overrides, diagnostics, WASM ballot pipeline)
  without merging or depending on its PR. Storybook and the workbench share typed
  synthetic snapshots, scenario definitions, providers and WASM adapters, free of
  Storybook runtime imports, and open equivalent screens through stable IDs. The
  minimal slice selects, imports, exports and resets a snapshot, opens a production
  voting screen, changes supported presentation/validation policies, inspects state
  and runs a real supported ballot/WASM operation, with stories and a focused
  Playwright smoke flow.
- **3. Devcontainer modes.** UI-only, UI + Keycloak, backend and full-stack modes use
  the existing Compose/devcontainer mechanisms and start only their dependencies.
  The full-stack entry point, readiness checks, port overrides, file ownership and
  persistent caches are preserved; existing listeners are detected; one checkout
  switches modes without destructive cleanup.
- **4. Incremental WASM.** The full rebuild remains the explicit recovery and release
  operation. The development build keeps Yarn caches, `node_modules` and unrelated
  `dist` trees; it keys on Rust sources, transitive path dependencies, manifests,
  the lockfile, build flags and toolchain/wasm-bindgen versions; a no-change run does
  nothing; one task-owned artifact is published atomically after a successful build;
  failures are reported and cannot look like a successful rebuild. Production
  packages stay correct and source freshness is checked, not only archive hashes.

### Phase 2 — focused execution and reproducible states

- **5. Focused tests.** Discoverable commands for a package, story, Playwright test and
  backend test, with watch mode where supported, visible executed scope, no coverage
  or full-stack startup by default, a separate broader-validation command and a
  conservative fallback.
- **6. Keycloak frontend loop.** Live-mounted theme resources with theme caching
  disabled only in development; then a React/Keycloakify pilot for login and one
  custom OTP page against real Keycloak with the existing Java authenticator,
  preserving authentication semantics, redirects, localization, accessibility, User
  Profile metadata and the sanitized template contract. Adopt only if it improves
  the loop without regressions; a full auth UI rewrite is out of scope.
- **7. Repeatable scenarios.** Named synthetic real-backend scenarios (a kiosk voter, a
  completed ceremony, published results) built from the existing E2E helpers: own
  isolated resources, readiness waits, printed URLs and fixture credentials, targeted
  reset, repeatable invocation. Never reset a shared database or delete another
  task's resources.

### Phase 3 — measured experiments and shared infrastructure

- **8. Vite pilot** on the smallest suitable portal against the Webpack path, with a
  threshold declared after the baseline (suggested: 20% median improvement on the
  limiting workload without material regressions). A documented negative result is
  valid.
- **9. Rust builds.** No duplicate Cargo builds for one workflow, coordinated
  concurrent Cargo processes and per-worktree target directories; Cargo timings on
  representative Windmill/Harvest edits; a smaller pure-logic crate boundary only
  if the measurements justify it.
- **10. Prebuilt environments and incremental GitHub Actions** implementing the
  cross-cutting requirements in real workflows: toolchains and dependencies
  separate from source, a local fallback, explicit refresh and invalidation rules,
  compatible Rust/WASM/frontend caches keyed by platform, toolchain, lockfiles and
  configuration, permissions fixed in task-owned volumes, fast affected checks
  first, no registry credentials exposed to untrusted code.

### Delivery

Baseline and Phase 1 first, then Phase 2, then Phase 3, in at most three stacked
draft pull requests. Improvements that stand on their own are kept even when an
experiment is rejected.

## Continuing this work

Another machine can resume from the tracking issue, this record and the pushed
branches. The implementation brief used for this programme:

```text
Implement https://github.com/sequentech/meta/issues/13610 on any machine with access to the GitHub issue and sequentech/step repository. The issue (status) and docs/design/feedback-loops.md on the programme branch (requirements, decisions, measurements) are the self-contained sources; no files from the originating machine are required.

Read the complete issue and design record. Locate an existing sequentech/step clone or clone it into an appropriate writable workspace. Fetch origin/ovcs, then read CLAUDE.md, AGENTS.md and applicable package guides from that branch; use .agents/skills/implement-unit-tests/SKILL.md for test/coverage changes. Optimize edit-to-visible-result and focused-test latency, not coverage expansion. Execute the ten workstreams and phase order in the issue, including baseline measurements and evidence-based adopt/reject decisions for experiments. Incremental builds AND affected test execution are mandatory locally and in GitHub Actions: implement dependency-aware selection, compatible compiler/output reuse, sound invalidation, conservative fallback and measured CI feedback improvements. Reuse the workbench approach from meta#12244/step#2719 through the Storybook-compatible browser-only vertical slice specified here; inspect its current code, but do not merge it or depend on it landing.

Inspect repository/worktree status before making changes. Create your own clean worktree and issue-numbered feature branch from the latest origin/ovcs, for example feat/meta-13610-feedback/ovcs, using an unoccupied local directory. Do not modify, reset or push ovcs directly, or disturb another task's worktree. Reuse the repository's existing UI fixture/test infrastructure. Prior research files from the originating machine are optional and are not prerequisites; the work plan is in this issue. Do not wait for unrelated meta#13571 work.

Measure first; ship items 1–4, then 5–7, then 8–10. Implement useful improvements autonomously. A documented negative pilot is complete; an unrun pilot is not. Keep PR count small, normally no more than three coherent phases. First draft PR base ovcs; dependent drafts base their immediate predecessor. Never merge PRs, rebase, force-push or push wip/* branches. Normal merges only for updates, one branch per push, serialized. Preserve unrelated services, files, caches and worktrees.

Use repository-pinned toolchains and the documented devcontainer setup. Discover the destination machine's container runtime, available resources, ports and permissions rather than assuming sudo, a particular username, host tool installation or directory layout. Put temporary/build data in a writable location with sufficient disk space; avoid filling a RAM-backed temporary filesystem. Run Cargo from packages/ when required by repository configuration, inspect existing auto-build watchers to avoid duplicate builds, coordinate resource-heavy builds, and use a separate CARGO_TARGET_DIR per worktree. Host-specific wrappers are optional, not prerequisites. Never stop or clean up unowned containers or volumes. Record environment details with measurements; compare before/after on the same machine under comparable conditions.

Commit as Eduardo Robles <edulix@gmail.com>. End every commit message, after a blank line, with: Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Begin PR bodies with Parent issue: https://github.com/sequentech/meta/issues/13610
End with: 🤖 Generated with [Claude Code](https://claude.com/claude-code)
Request Copilot and CodeRabbit review on each draft, investigate and reply to findings in their threads, resolve only settled findings, and get applicable CI green. Do not rerun verified suites without new changes, failures or another concrete reason. Keep PR bodies and the issue current using AGENTS.md formatting. Document reusable commands in the canonical Docusaurus developer guides; publish measurements, pilot decisions and remaining work in the issue or linked repository artifacts so another machine can continue without local-only files. Finish with pushed branches, draft PR links, before/after evidence, pilot decisions and an accurate handoff for any externally blocked work. Do not ask routine questions or merge/deploy production changes.
```
