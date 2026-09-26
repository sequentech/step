<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Fast development feedback loops

Design record for making the devcontainer loop **edit → see the result → run the
relevant test** fast. It keeps the detailed requirements, the decisions taken, how
they were measured and how to continue. Commands live in the developer guide
[Fast feedback loops](../docusaurus/docs/07-developers/03-development-environment/fast-feedback.md);
status and progress live in the tracking issue,
https://github.com/sequentech/meta/issues/13610.

## Requirements

Make the normal development loop **edit → see the result → run the relevant test** fast on `sequentech/step` branch `ovcs`, using the existing devcontainer workflow. Prioritize less waiting and setup over expanding coverage. Reuse the Storybook, Playwright, fixtures, and ports/adapters work from https://github.com/sequentech/meta/issues/13571.

This is an implementation programme for the ten recommendations below. Ship the immediate improvements first; architecture/tool migrations are measured experiments, not prerequisites. Do not create ten separate PRs or rewrite the whole frontend.

Related work (reuse and cross-reference; do not silently supersede or close):
- https://github.com/sequentech/meta/issues/12244 — browser-only Election Workbench; reference implementation https://github.com/sequentech/step/pull/2719 (open/unmerged when inspected; `packages/workbench` absent from `ovcs`). Inspect its current implementation before reuse; do not merge that PR or assume it is already available.
- https://github.com/sequentech/meta/issues/12291 — Rust compilation time.
- https://github.com/sequentech/meta/issues/2127 — frontend Rust feedback loop.
- https://github.com/sequentech/meta/issues/5796 — devcontainer startup and footprint.
- https://github.com/sequentech/meta/issues/12409 — build artifact ownership.
- https://github.com/sequentech/meta/issues/13010 — stale packaged WASM.
- https://github.com/sequentech/meta/issues/12507 — GraphQL initialization readiness.

### Baseline and measurement contract

Before changing build configuration, add a small reproducible benchmark command using the existing task runner/scripts. Record commit, tool versions, machine load, enabled services, cache state, commands and raw samples. Separate cold startup from warm iteration. Do not clear shared caches to simulate cold runs: use isolated task-owned directories. For warm operations collect at least 10 samples and report median and range; reserve p95 claims for a sufficiently larger sample. Record at least three cold-start samples when their cost permits, with sample count explicit.

Measure: (1) command-to-ready workspace time, (2) save-to-visible UI update, (3) save-to-focused-test result, (4) push-to-first-actionable CI result. Also record peak memory, disk footprint and Rust build critical path where relevant. Measure browser-visible updates, not just bundler completion. Representative edits: shared UI component, portal screen, Rust-backed frontend function, Windmill service and Harvest consumer. Record initial numbers before setting realistic numerical budgets; do not invent speedups or make host timing a flaky required CI gate.

### Cross-cutting requirement — incremental builds AND test runs

Incrementality is mandatory both locally and in GitHub Actions, not merely optional dependency caching. Distinguish dependency download caches, compiler/bundler caches, reusable build outputs, and affected-test selection: restoring dependencies alone does not satisfy this requirement. Apply this across frontend/Storybook, Rust/WASM and applicable integration tests.

- Inventory existing workflows, package dependencies, code generation, build outputs and test inputs. Implement a shared, inspectable change/dependency model so local commands and CI agree about what is affected. Prefer existing workspace tooling over adding a second build orchestrator.
- Compare PR changes against the correct merge base with sufficient fetched history. Include transitive consumers, shared UI, Cargo path dependencies/features, GraphQL schemas/generated clients, WASM consumers, fixtures, configuration, lockfiles and toolchains. Unknown or missing history/metadata triggers conservative broader validation. Documentation-only changes should run their relevant checks without compiling unrelated applications.
- Rebuild only affected outputs; reuse unchanged compatible outputs and restore compiler caches. Separate task invalidation from reusable compiler-cache keys so a source edit can reuse unchanged compilation units. Fingerprint all output inputs, including dependency/toolchain/target/profile/feature/configuration dimensions. Never treat a successful cache restore as proof that an output or test result is current.
- Run tests affected by the change and their dependency closure, using explicit package/story/test selection where sound. Rust test filtering must account for shared dependencies; avoid claiming precise per-test impact analysis where unavailable. Integration selection must include shared schemas, services and fixtures. Unknown impact runs the broader relevant suite.
- Cache test results only when the complete input/environment identity is reproducible; otherwise reuse builds and rerun selected tests. Do not reuse live-service or mutable-environment test results based solely on source hashes. Existing required full validation stays intact unless an explicitly sound equivalent is established; schedule broader validation separately from the earliest focused result without disguising skipped checks as passing tests.
- Emit an actionable CI summary: affected packages, selected/skipped checks and why, cache hit/miss reasons, rebuilt/reused outputs and durations. Keep a stable required-check aggregator that distinguishes legitimate no-op selection from failure/cancellation; avoid path filters leaving required checks permanently pending.
- Cancel obsolete runs on superseded commits using appropriately scoped concurrency groups. Parallelize independent work with bounded resources; shard long affected suites where useful. Do not rebuild the same WASM/UI artifact in every job: build once and share the exact revision-compatible output with downstream jobs. Treat cache availability as optional and restrict cache/artifact publishing across trust boundaries.
- Validate cold cache, warm no-change invocation, leaf UI edit, shared UI edit, Rust/WASM edit, fixture/schema change, lockfile/toolchain change and missing/invalid cache. Prove unchanged work is skipped/reused and affected work still runs. A failure must remain visible. Compare equivalent local and Actions runs; report queue/setup/build/test times separately and time to the first useful result as well as total completion time.

### Phase 1 — immediate local feedback improvements

**1. Hot-reload shared UI source.**
- Inspect portal dev bundlers and resolve `ui-core` / `ui-essentials` directly to source during development. Today their package entry points refer to `dist/index.js`.
- Watch/transpile linked workspace source; handle assets, styles and TypeScript paths; ensure a single React instance. Preserve production package/build entry points.
- Demonstrate a shared-component edit appearing in each consuming portal without running a separate library build or restarting the server. Check production build parity and HMR behavior for an interactive screen.

**2. Make Storybook the everyday UI workspace.**
- Extend the existing Storybook 10 setup and `ui-test-kit`; do not install a parallel story framework or duplicate fixtures.
- Add reusable decorators/globals for tenant/theme, locale, permissions and workflow state, plus direct links to representative complete screens. Include loading, empty, populated and error states where meaningful.
- Start with a voting/kiosk screen, admin ceremony/tally screen, results screen and verifier screen. Reuse existing stories and fill only the gaps needed for these development workflows.
- These stories must work without real login or provisioning; mocked network behavior must be explicit and deterministic. Provide one focused interaction-test command for a selected story.

**2a. Reuse the Election Workbench approach within this issue.**
- Review https://github.com/sequentech/meta/issues/12244 and the current code of https://github.com/sequentech/step/pull/2719. Its proposed browser-only snapshots, booth screens, policy overrides, diagnostics and WASM ballot pipeline are inspiration and reusable work, not a prerequisite merge or a second competing programme.
- Add a lightweight developer workbench entry point on `ovcs`, or integrate the existing one if available by implementation time. It must run in UI-only mode without external services. Reuse production components directly wherever possible; avoid copied screens and silent source drift. Any temporary adapter must have a documented boundary.
- Make Storybook and the workbench consume the same typed synthetic election snapshots, scenario definitions, context providers and WASM adapters. Keep fixtures free of Storybook-specific runtime imports so production-component previews and tests can share them. A selected fixture/scenario should open an equivalent screen in either surface through stable documented links/IDs.
- Deliver a minimal useful vertical slice: select/import/export/reset a synthetic snapshot; open a production voting screen; change supported presentation/validation policies locally; inspect state and run an existing supported ballot/WASM operation. Reuse existing pipeline functionality if available; implementing a new cryptographic/tally engine or reproducing all workbench features is outside scope.
- Provide Storybook stories for the reused production screens and the workbench controls, plus a focused Playwright smoke flow for the vertical slice. Keep global browser persistence and network behavior isolated between tests. Mocked story computation must be labeled; the workbench's real WASM path must exercise the actual implementation.
- Verify shared-component HMR in both surfaces and WASM updates without reinstalling dependencies. Include workbench/Storybook tasks in affected-build/test selection and CI artifact reuse. Publish commands and entry links in developer docs, and cross-reference progress in this issue without closing or merging #12244's work.

**3. Provide lightweight devcontainer modes.**
- Expose documented UI-only, UI+Keycloak, backend and full-stack modes using existing Compose/devcontainer configuration mechanisms.
- Declare each mode's minimum dependencies and start only its necessary services/watchers. UI-only must not boot Rust workers, database, queues or Keycloak merely to render fixture-backed stories.
- Preserve the existing full-stack entry point, readiness checks, port overrides, file ownership and persistent dependency caches. Detect existing listeners rather than launching duplicate servers.
- Measure mode startup and resource footprint. Verify that the same checkout can switch modes without destructive cleanup.

**4. Add an incremental WASM path.**
- Keep the full rebuild as an explicit recovery/release operation. Add a normal development build that does not delete Yarn caches, all `node_modules`, or unrelated `dist` trees.
- Key rebuild decisions on relevant Rust sources, transitive workspace/path dependencies, manifests/lockfile, build flags and toolchain/wasm-bindgen versions. A no-change invocation should do no compilation or reinstall.
- Produce one task-owned dev artifact for consuming portals; update consumers atomically only after a successful build. Ensure dev-server invalidation and explicitly report build failures so stale output cannot look like a successful rebuild.
- Preserve and verify production packaging for all consumers. Address the stale-artifact concern in #13010; a hash comparison between committed archives alone does not establish source freshness.

### Phase 2 — focused execution and reproducible states

**5. Continuous focused tests.**
- Offer discoverable commands for a selected package, story, Playwright test and backend test. Use existing Vitest/Playwright/Cargo infrastructure and existing fixtures.
- Add watch mode where supported, or a narrow file-triggered runner; keep coverage instrumentation and full-stack startup out of the default edit loop.
- Show the executed scope clearly. Provide a separate explicit broader-validation command and preserve existing required CI gates. Use conservative fallback when affected-test selection is uncertain.

**6. Give Keycloak a frontend development loop.**
- First enable live-mounted theme resources with theme/static caching disabled only in the development mode. Demonstrate an edit becoming visible without Maven packaging/image rebuild for changes that do not require Java recompilation.
- Then pilot React/Keycloakify with Storybook for login plus one custom OTP page. Reuse design tokens/components where compatible; exercise real Keycloak with the existing Java authenticator.
- Preserve authentication semantics, redirects, localization, accessibility, User Profile metadata and the sanitized template/context contract. Keep credentials and sensitive context out of fixtures.
- Record compatibility, development latency and migration cost. Adopt the pilot only if it improves the loop without auth/packaging regressions; otherwise retain the live-theme improvement and document the result. A full auth UI rewrite is outside this issue.

**7. One-command repeatable scenarios.**
- Reuse existing E2E seed/helpers to expose named synthetic scenarios: `kiosk-voter`, `completed-ceremony`, and `published-results` (names illustrative; follow existing command conventions).
- A command creates or reuses its own isolated resources, waits on actual readiness, prints direct URLs and fixture credentials where appropriate, and offers a targeted reset. Avoid fixed startup sleeps.
- Support repeated invocation and actionable errors. Never reset a shared database, import production data, or delete another task's resources. Use real backend scenarios only when stories cannot answer the development question.

### Phase 3 — measured experiments and shared infrastructure

**8. Benchmark Vite on one portal.**
- Pilot the smallest suitable portal; compare equivalent warm/cold workloads with the existing Webpack path. Verify shared source HMR, WASM, routes/base paths, runtime settings, assets and production packaging.
- Keep the current path available during evaluation. Predeclare a worthwhile improvement threshold after baseline collection (suggested: at least 20% median improvement in the limiting startup/edit workload, with no material regression elsewhere).
- Expand only if the measured benefit justifies migration and maintenance costs. A documented negative experiment is a valid outcome; no mandatory all-portal migration.

**9. Coordinate and shorten Rust builds.**
- Inspect existing watchers before starting Cargo; eliminate duplicate builds for the same developer workflow. Coordinate concurrent Cargo processes using the destination environment's available mechanism and use separate target directories per worktree.
- Use Cargo timings on representative Windmill/Harvest edits to identify recompilation and linking bottlenecks. Check profiles, feature sets and cache compatibility before restructuring crates.
- Pilot a smaller pure-logic boundary only if measurements identify a useful boundary. Validate public APIs/features and remeasure downstream rebuilds. Keep an extraction only when the measured benefit outweighs complexity; record rejected experiments.

**10. Prebuild environments and implement incremental GitHub Actions.**
- Implement the cross-cutting incremental build/test requirements above in actual workflows, not only a design document. Expose the focused result before broader validation finishes, and make cache/selection behavior inspectable.
- Build on existing devcontainer/image and CI infrastructure. Separate infrequently changing toolchains/dependencies from source; provide a local fallback and explicit refresh/invalidation rules.
- Cache compatible Rust, WASM and frontend artifacts using OS/architecture, toolchain, lockfiles and relevant build configuration. Fix permissions in task-owned volumes instead of global ownership changes.
- Put fast affected checks early so a useful result arrives before long integration checks; retain all required full validation. Do not expose registry credentials to untrusted PR code.
- Measure fresh workspace readiness and CI first-result time. Verify a cache miss or stale cache rebuilds correctly and cannot ship incompatible artifacts.

### Delivery order and review size

Implement baseline plus items 1–4 first, then 5–7, then measured experiments 8–10. Keep a small number of coherent draft PRs: aim for at most three phases, not one PR per recommendation. Split further only if review size genuinely requires it, with the reason recorded in the tracking issue. The first PR targets `ovcs`; dependent PRs target their immediate predecessor and ultimately integrate into `ovcs`. No PR merging is authorized. Preserve independently useful improvements even when an experiment is rejected.

### Documentation

Extend the existing canonical guides under `docs/docusaurus/docs/07-developers/` with mode selection, screen/scenario entry points, focused test commands, incremental WASM, benchmark reproduction, incremental Actions selection/cache troubleshooting, and shared workbench/Storybook scenarios. Add verified rendered/source links to the tracking issue when the documentation exists. Update release notes if required by repository guidance; do not invent documentation URLs.

Implementation references:
- [Storybook globals](https://storybook.js.org/docs/essentials/toolbars-and-globals)
- [Keycloak development theme configuration](https://www.keycloak.org/ui-customization/themes)
- [Keycloakify isolated previews](https://docs.keycloakify.dev/testing-your-theme/outside-of-keycloak)
- [Keycloakify custom pages](https://docs.keycloakify.dev/features/styling-a-custom-page-not-included-in-base-keycloak)
- [Cargo build timings](https://doc.rust-lang.org/cargo/reference/timings.html)
- [Vite performance guidance](https://vite.dev/guide/performance)
- [Devcontainer prebuilds](https://containers.dev/guide/prebuild)

## Decisions and measurements

Measured on one aarch64 host (16 cores, 62 GiB) while other work ran, so each row
records its sample count; load averages are in the raw `step-dev bench` results.
Before = `ovcs` 679c3ea181. Cold stacks ran in fresh, task-owned Docker daemons.

| Loop | Before | After |
| --- | --- | --- |
| Full stack from zero to ready | 1532 s (n=1) | 1481 s (n=1) |
| Full stack, warm restart | 59 s median, 48–67 (n=10) | — |
| UI-only devcontainer from zero | — (full stack only) | 420 s median, 320–623 (n=3) |
| UI + Keycloak from zero | — | 670 s (n=1) |
| Devcontainer recreate (rebuild, mode switch) | whole Nix store downloaded again | 32 s median with the shared store volume (n=10); 365 s with an empty one (n=2) |
| Shared UI component edit → visible in a portal | 16–20 s median per portal via `build:ui-essentials` (n=10 each) | 1.8–3.6 s for voting, verifier and results, 12 s for admin including react-admin reload (n=5); 0.48 s with Fast Refresh when the module exports only components |
| Shared UI edit → visible in Storybook | 1.3 s (n=10) | 0.16 s in place (n=10, workbench story) |
| sequent-core edit → WASM rebuilt | 75 s for an unchanged tree: full script, reinstall, restart (n=10) | 0.15 s no-op (n=10); 6.3 s build and 15 s until the new ballot ID shows after rerunning the voting flow (n=10) |
| Keycloak template, message or CSS edit → visible | 191 s image rebuild and recreate (n=3) | 0.12–0.19 s live mount (n=10 each); 27 s for a provider jar (n=3) |

Decisions so far:

- **Shared UI source in development** (adopted): exact-match aliases to `src`, a
  resolver plugin that keeps one copy of React and the context libraries, React
  Refresh, `eval-cheap-module-source-map` (webpack rebuild 389 ms against 689 ms,
  n=10). Production output is byte-identical except admin's embedded environment.
- **Devcontainer modes** (adopted): four configurations from one manifest,
  per-checkout Compose projects and prefixed names outside a folder named `step`, the
  checkout's parent mounted at its host path so worktrees resolve, and shared cache
  volumes (Nix store per devcontainer image tag, `~/.cache`, `~/.cargo`).
- **Incremental WASM** (adopted): content-addressed builds published by one atomic
  rename, fingerprinted sources, an incremental development profile (cargo 15 s →
  5 s on a leaf edit, +0.4% wasm size); the committed package was stale and is now
  regenerated reproducibly with a freshness check in CI.
- **Keycloak** (adopted: live theme folders; Keycloakify development continues). The pilot
  passed a real password + email OTP login with the existing authenticator (8/8) and
  its pages were lighter with no axe violations, but its real-Keycloak loop took
  8.4 s (n=10), realm localization overrides and per-event login policies did not
  reach the pages, the OTP courier enum was lost and dotted template ids needed
  workarounds; porting would cover 29 templates, 251 message keys in up to eight
  locales and a Node build stage. These results defer production adoption; they
  do not end the React development path. Keep an opt-in Keycloakify workspace
  with automatic reload and continue closing the localization, policy and custom
  OTP context gaps. Document commands and browser validation for both developers
  and agents through the shared `step-dev` entry point and `AGENTS.md`.
- **Workbench** (adopted): shared scenarios and snapshots in `ui-test-kit`, one
  preview provider for Storybook and the workbench, production routes and loaders,
  typed policy overrides and the real sequent-core pipeline.

Current validation: the voting Jest suite passes locally (30 suites, 259 tests,
one run), as do verifier stories (14 tests, including one retained expected
accessibility failure) and results stories (19 tests), one browser run each.
The hosted regressions were a missing story CSS hook, virtual mocks for a now-real
shared module and obsolete expected-failure markers after upstream accessibility
fixes. Frontend lint and formatting pass across all seven packages. Paired voting
coverage against `ovcs` 833385f62396 passes (one base/head pair), with all four
metrics increasing. Hosted reruns remain in progress.

- **Public admin build settings**: webpack defines only the four settings read
  by the application; private build environment values no longer enter the
  browser bundle. Five compiler regression tests cover private values, public
  settings, defaults, mode and reproducibility. Admin tests pass (47 suites,
  362 tests, one run); two production builds with different synthetic private
  values have identical bytes in all 309 output files.
- **Agent discovery**: `AGENTS.md` and the Claude entry point share the
  `fast-feedback` skill, the developer guide and `step-dev` commands. Explicit
  CLI help exits successfully, and shared UI edits no longer instruct agents to
  rebuild production libraries.

Keep the stack synchronized with new `ovcs` commits using normal merges into
phase 1 and then each descendant. All feedback commands must be discoverable and
usable from agent instructions as well as the developer guides.

## Continuing this work

Another machine or agent resumes from the tracking issue, this record and the pushed
programme branches (one per phase, each stacked on its predecessor). Follow
`AGENTS.md` and the developer guides; agent-specific instruction files are not
required. The brief below is tool-neutral:

```text
Continue https://github.com/sequentech/meta/issues/13610 in sequentech/step. Read
the issue (status, OVCS PRs, remaining work), docs/design/feedback-loops.md
(requirements) and AGENTS.md. Work on the phase branches listed in the issue's
OVCS PRs section: never rebase, force-push, merge PRs or push wip branches; update
branches with normal merges, one branch per push; keep each stacked PR based on its
predecessor. Commit with the repository's author identity and a Co-Authored-By
trailer naming the agent doing the work. Use isolated, task-owned Docker daemons or
Compose projects for stacks and never stop, recreate or clean up containers,
volumes, caches or worktrees you did not create. Measure before and after on the
same machine under comparable load (the step-dev bench command records it); a
documented negative experiment is complete, an unrun one is not. Request Copilot and
CodeRabbit reviews on each draft, reply to findings in their threads and resolve
only settled ones. Keep PR bodies, the issue and the developer guides current.
```
