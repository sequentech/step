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

Measured on one aarch64 host (16 cores, 62 GiB). Final UI, WASM,
focused-test and backend/UI-only warm series ran sequentially at `d5ec0a32bf`
(UI focused uses the identical tree at `990549ab821`). Earlier concurrent
runs retain their qualifications below. Every result records sample counts
and host load. Unless specified, before = `ovcs` 679c3ea181. Historical cold
stacks used fresh task-owned Docker daemons and fresh Nix/application stores;
the full and UI+Keycloak attempts explicitly seeded two MinIO images after
an initial registry failure. Different service sets, readiness conditions
and cache states preclude a general startup-speedup comparison.

| Loop | Before | After |
| --- | --- | --- |
| Historical full-mode cold readiness, Beat optional | 1532.528 s (n=1) | 1480.829 s (n=1) |
| Historical full-mode warm readiness, Windmill required | 0/10 successful; all 10 failed the Windmill exit guard | — |
| Historical warm readiness with Windmill optional | 59.045 s median, 48.232–66.628 (n=10 under the weaker rule); Windmill ready in 5/10 and exited in 5/10 | — |
| Historical UI-only cold devcontainer CLI completion | — | 420.362 s median, 320.039–622.610 (n=3); no UI server/browser readiness probe |
| Historical UI+Keycloak cold CLI and tenant OIDC readiness | — | 670.247 s (n=1); no UI server started |
| Historical UI-only devcontainer recreation, CLI completion | — | Retained Nix store: 31.725 s median, 24.591–61.303 (n=10); empty Nix store: 365.442 s, 319.147–411.737 (n=2); images warm in both |
| Final backend warm command-to-ready, strict required probes | No matched baseline | 47.941 s median, 19.241–48.444 (n=10, zero failures; one excluded warmup) |
| Final UI-only warm command-to-ready, CLI + Storybook HTTP | No matched baseline | 3.1885 s median, 3.132–3.248 (n=10, zero failures; one excluded warmup) |
| Shared UI component edit → visible in a portal | Voting 15.721 s; verifier 15.519; results 14.822; admin 18.330 | Voting 2.648 s; verifier 2.448; results 1.905; admin 4.873 (n=10 per portal/version) |
| Voting screen edit → visible | 2.4235 s, 2.324–2.468 | 0.781 s, 0.779–0.782 (n=10 each) |
| Shared Header edit → visible in Storybook | 1.283 s, 1.281–1.286 | 1.2825 s, 1.281–1.287 (n=10 each; unchanged) |
| sequent-core edit → visible WASM result | 80.084 s, 78.001–82.670 | 6.1155 s, 6.024–6.245 (n=10 each); no-change command 0.164 s, 0.164–0.165 (n=10) |
| Direct test → focused selector, same selected tests | Voting 2.3175 s; WarnBox 1.316; Header story 4.246; core SQLite 4.3715 | 2.417 / 1.516 / 4.9725 / 4.596 s respectively (n=10 each; selector overhead, not speedup) |
| Keycloak template, message or CSS edit → visible | 191 s image rebuild and recreate (n=3) | 0.12–0.19 s live mount (n=10 each); 27 s for a provider jar (n=3) |

Final backend warm command-to-ready measures `devcontainer up` after the
harness has stopped the stack. Shutdown, image preparation, initial Nix and
CLI cache preparation, and one warmup are outside the timer. All ten measured
starts passed at `d5ec0a32bf`; median 1-minute host load was 9.325
(6.96–10.89). The required scope is CLI completion, every configured Docker
healthcheck, Hasura, tenant OIDC, Harvest readiness, Windmill main-running
and readiness, and required nonzero-exit guards. No frontend server is selected.
Beat is observed separately: readiness passed in 10/10 starts and no nonzero
Beat exit was observed. This is a final backend workflow result, with no
matched baseline for a speedup claim.

The four application/realm HTTP endpoints were all ready at median 19.068 s
(16.108–20.494). In eight of ten starts, Hasura's last Docker health transition
extended completion by 26.735–29.869 s beyond the endpoint probes; in the other
two, CLI completion and health polling were the final gate. The required
Docker health policy was unchanged.

The initial unprepared final-backend series is retained separately: its
warmup and all ten measured attempts failed before application readiness
because the original Compose-created stack lacked the local devcontainer
feature image. Excluded preparation then built the configured feature and
UID-mapping images, applied the frozen RabbitMQ healthcheck to its retained
container, and completed the configured devenv/onCreate/postCreate work.
Original service images, the Nix volume and the two existing Cargo targets
were retained. This prepared-cache success does not establish fresh-machine
or cold-start readiness.

The historical full cold rows require CLI, Docker health, Hasura, tenant OIDC,
Harvest, Windmill and Storybook readiness; Beat was optional and never passed
its probe. Historical UI-only cold and recreation rows stop at CLI completion.
The earlier 59.045 s warm baseline uses a weaker condition than strict backend
readiness and cannot serve as its before value. Its preceding strict series
failed all ten starts. Retained logs correlate those failures and all five
optional-rule Windmill exits with AMQP connection refusal before RabbitMQ's
listener opened. The final configuration waits for the RabbitMQ healthcheck.

The final UI-only warm series uses the same frozen source and original Nix
volume, with competing task previews stopped. CLI completion takes 1.213 s
median; Storybook HTTP readiness completes the command-to-ready measurement
at 3.1885 s (3.132–3.248, n=10). The excluded warmup takes 20.9 s. Median host
load is 7.87 (7.14–10.01). Shutdown and prior dependency preparation are outside
the timer. This checks the devcontainer and selected Storybook endpoint; it
does not time a browser render or compare against a matched earlier UI-only run.

The final backend footprint is a snapshot after the prepared warm series;
it includes earlier native-test and paired-build outputs. Startup memory is
the maximum observed over ten measured starts with five-second sampling,
which can miss brief peaks. Inner-container and whole-DinD measurements
describe overlapping scopes and must not be added. The Nix store is included
in daemon storage; host-bound Cargo outputs are reported separately. These
results do not establish a footprint reduction.

| Final backend resource observation | GiB |
| --- | ---: |
| Maximum sampled simultaneous inner-container memory | 3.092 |
| Maximum sampled whole task-daemon memory | 12.760 |
| Retained daemon storage, including images, cache and volumes | 38.494 |
| Nix store, apparent bytes; included in daemon storage | 18.782 |
| Existing `packages/target` | 56.097 |
| Existing `packages/rust-local-target` | 35.837 |
| New configured CLI `packages/step-cli/rust-local-target` | 3.289 |

The configured postCreate `init-cli` created the additional 3.289 GiB CLI
target during excluded preparation: `CARGO_TARGET_DIR=rust-local-target`
is relative to the CLI working directory, and the configured devenv PATH
expects its release binary there. This deviated from the intent to reuse
only the two existing targets. The additional cache is retained for a working
CLI and reported explicitly; the two original targets remain intact.

In the final UI-only warm series, sampled whole-daemon peak memory is
0.760 GiB median (0.704–0.773). Each measured start finishes before the
five-second inner-container sampler, so its empty measurements mean unobserved,
not zero usage. The excluded warmup samples 0.981 GiB inner and 1.803 GiB
whole-daemon peaks. Retained UI-daemon storage is 42.172 GiB, including other
stopped task stacks and caches; the Nix store is 18.785 GiB within that total.
The checkout has 0.916 GiB of node_modules. This is not an isolated incremental
UI footprint and cannot be added to the overlapping daemon/Nix figures.

Historical full cold disk snapshots were 42.282 GiB before and 42.719 GiB
after for daemon storage, with the same approximately 16.893 GiB main target,
3.266 GiB CLI target and 0.867 GiB node_modules. Persistent volumes move
storage out of writable containers; these results do not show lower total
full-stack disk use. Historical UI-only cold daemon storage was 22.617 GiB,
with no application outputs created, so its smaller footprint also reflects
less work. These historical cold snapshots and the final accumulated warm
backend snapshot are separate observations, not a before/after comparison.

Final portal shared-update ranges, before → after in seconds (n=10 each):
voting 15.308–15.867 → 2.512–2.877; verifier 15.376–15.808 → 2.401–2.682;
results 14.379–14.990 → 1.861–1.995; admin 17.931–18.840 → 4.698–5.005.

The final browser runs record zero page exceptions and mock violations through
all edits and restoration, with source/package hashes unchanged afterward. Portal
shared baselines include manual library builds; the new path does not. Storybook
already loaded source: its previous 0.16-second result was a different workbench
story and is not the final Header comparison. The WASM baseline wrapper keeps
installed dependencies rather than deleting them, while retaining archive build,
Yarn install and server restart; this is a conservative warm comparison. The
incremental build itself completes in 3.695 s median (3.669–3.771, n=10). No-change
measures command latency, not a browser update. Raw phase markers are cumulative
milestones, not independent stage durations.

Focused comparisons preserve identical selected scope in each pair: 13 voting,
10 WarnBox, five Header and 16 core SQLite tests per invocation. All 88 invocations,
including excluded warmups, pass. UI direct/selector ranges are 2.267–2.368 /
2.318–2.417, 1.266–1.316 / 1.466–1.567, and 4.171–4.321 / 4.971–5.123 seconds;
core ranges are 4.271–4.471 / 4.471–5.072. Header host load is higher after
(4.145 versus 2.095), so the difference is not solely wrapper overhead. The core
pair uses the same final source and existing target; it is not an original-to-final
application comparison.

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
- **Keycloak** (adopted for development): live theme folders plus the retained
  opt-in `keycloak-ui` React workspace. Login and message OTP use React refresh;
  unported or complex login widgets inherit the original templates with automatic
  reload. The explicit server context carries login policies, OTP courier and
  localized messages, including realm overrides. The earlier implementation
  passed eleven real authentication/policy checks, 12 browser stories and 23
  original theme tests. Its warm visible edits on real Keycloak: login 0.077 s median, 0.076–0.077 (n=10);
  OTP 0.076 s, 0.075–0.087 (n=10). Typed input and the authentication session survive
  without navigation; the OTP exchange completes after the edits. Server message
  edits also reload automatically (n=1). The earlier packaged pilot took 8.4 s
  (n=10). Production adoption remains deferred; SMS delivery, one-time links,
  CAPTCHA and external identity providers need their configured integration
  environments. Commands and fixture limits are in the Keycloak developer guide.
- **Keycloak visual and accessibility refinement**: the login and custom OTP
  pages use a scoped Sequent theme inspired by Election Architect, local fonts,
  a responsive card and visible keyboard focus. The final 30 stories pass, including two failing-before missing-courier cases.
  Fifteen real authentication checks and all 137 provider/62 theme tests also
  pass after the final review corrections at `46c91b4a8d`. Real-page hot edits measured at `d5ec0a32bf`
  take 0.07533 s median (0.07449–0.07674) for login and 0.07535 s
  (0.07472–0.07716) for OTP, n=10 each plus excluded warmups, with input/session
  preserved and exact source restoration.
  The header correction at `aee7b16d63` places the native language selector and
  inherited application version/hash above the card, retaining server-translated
  labels. All 30 stories and 15 real-authentication checks pass again, including
  locale-link navigation, server build values and backward keyboard access from
  login/OTP fields. Eight desktop/320px and 100%/200% text combinations have no
  overflow or browser errors; types, lint, formatting and the theme JAR build pass.
  The earlier hot-reload measurements retain their original source attribution.
  Both React and original FreeMarker complete password/whole-code OTP login and
  redeem the OIDC code. The original OTP's missing names and truncated whole-code input
  fail before the fix and pass after it. New checks exercise credential errors,
  native locale/recovery/registration links, realm options, provider fallbacks,
  keyboard order, 320px reflow and 200% text. Provider 137 and theme 62 tests,
  TypeScript, lint, formatting and the theme JAR build pass. Source palette
  contrast is 5.30:1 for the main button and 3.43:1 for field borders; browser
  accessibility scans and visual review supplement the interaction checks.
  Concurrent theme preparation uses a per-checkout lock and atomic files, with
  two failing-before regressions. Automated checks do not establish formal WCAG
  conformance: native screen readers, browser zoom, OS clipboard integration,
  native autofill, password-manager extensions and inherited flows retain
  explicit verification limits. The replacement walkthrough shows the final design and keyboard behavior.
- **Authentication secret disclosure and related errors**: the follow-up at
  `3ca7601b69` removes OTPs, signed login links, delivery credentials and CAPTCHA
  secrets from authenticator, email/SMS-provider and bridge logs/events. Message
  delivery still receives the original payload. Communication-event masking uses
  literal values and the signed key separately, including URLs normalized by
  Keycloak's actual sanitizer. Missing-code and smart-link null-user errors,
  empty test-code acceptance, optional-OTL fallthrough and incorrect success
  feedback are corrected. CAPTCHA validation now requires the provider's success
  flag, preserves the configured score threshold, closes interrupted responses
  and fails closed; no attacker exploitability claim is made for the old flag
  handling. The same 33 focused cases produced 21 assertion failures and five
  null-pointer errors on original sources, with seven controls passing; all pass
  after correction. Six CAPTCHA cases independently fail before/pass after, and
  the sanitizer regression reproduces the review finding before its correction.
  The full Java reactor passes 367 tests with zero failures/errors/skips; scoped
  formatting and independent candidate review pass. Hosted Java checks exposed
  Maven-dependent discovery: the older runner omitted 43 existing Jupiter tests,
  while the pinned-Maven job ran all 367. Surefire 3.5.5 is now pinned for the
  parent and standalone bridge. Local clean verification and both hosted Java
  jobs at `88cb1ae4a7` now pass all 367 tests with zero failures/errors/skips. Four verified provider JARs
  are installed in the owned Keycloak environment. All 15 live browser checks
  pass in both themes with OIDC redemption, localization, errors and reflow.
  Those browser checks now use a random per-run test code only in their disposable
  realm, with no provider-log access; Java tests cover actual stored-code
  verification, expiry, rejection, delivery and feedback. External services are
  mocked in the Java checks. Scenario output and tutorials no longer direct users
  to OTP logs; all 85 scenario tests and scoped Ruff checks pass. The optional HMR
  timing check was not rerun; earlier timing samples retain their source revision.
- **Workbench** (adopted): shared scenarios and snapshots in `ui-test-kit`, one
  preview provider for Storybook and the workbench, production routes and loaders,
  typed policy overrides and the real sequent-core pipeline. The dev server now
  picks up `step-dev wasm`'s versioned artifact automatically; the inspector tracks
  the active binary through publication/reload. Unit tests (33), smoke flows (3),
  an actual versioned artifact switch with the five-step ballot pipeline (n=1),
  and the production build pass. Production WASM exactly matches the installed
  package. Voting Storybook exposes tenant/workflow controls and all eight
  locales; all 80 story tests pass (n=1 run).

Current validation: the voting Jest suite passes locally (30 suites, 264 tests,
one run), as do verifier stories (14 tests, including one retained expected
accessibility failure) and results stories (19 tests), one browser run each.
The hosted regressions were a missing story CSS hook, virtual mocks for a now-real
shared module and obsolete expected-failure markers after upstream accessibility
fixes. Frontend lint and formatting pass across all seven packages. Paired voting
coverage against `ovcs` 833385f62396 passes (one base/head pair), with all four
metrics increasing. Current `ovcs` b8f2a5c69d is merged through all three phases;
the updated voting suite, types, lint and formatting pass. Admin stories pass
(110 tests, one run), as do the six upstream load-replay regressions. Hosted
`1d25f2b73e` passes all 51 Tests jobs, including frontend journeys, safety-net
coverage and the final aggregate; its notification-only job is intentionally
skipped. Independent backend E2E validation has its own check status. Inline review corrections include recoverable cache snapshots, isolated worker
ports, impossible CI timestamps and nonpersistent checkout credentials; replies
and current check status are recorded in the issue and PR discussions.
The WASM benchmark wrapper preserves its checkout path, and CI verification
reports planning failures before decoding a selection. The repeated failed-plan
finding is covered by GitHub's default status guard and seven passing CLI
regressions. Phase-two review fixes cover SELinux mounts, matching English messages,
absent OTP courier context, strict generated-template identifiers and redirect-derived
origins. The repeated depth option and comprehension-bound `glob` names work
as verified by actual CLI commands and their regression tests; both findings
have no-change replies.
Copilot could not review phase 1 because its diff exceeds 20,000 lines; phase 2/3
received Lite reviews after runner timeouts. CodeRabbit reviewed phases 2 and 3; phase 1 requests remain rate-limited.

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

- **Incremental CI**: the Tests workflow uses the local dependency model, runs
  selected suites and calls the reusable frontend workflow. Selected tests rerun;
  a strict aggregate check rejects missing, cancelled or unexpectedly skipped
  jobs. Production journey shards share one current portal build. Dependency
  and shared-output caches keep their complete input identity in the restore
  prefix and add run/attempt generations to saved keys. Rejected snapshots are
  repaired by a normal install or build and a new immutable generation; actual
  Yarn and output-checksum fault probes cover corrupted caches and retries.
  Shared UI reuse verifies its complete input identity and output checksums: rebuilding
  locally took 19.3 s median, 19.1–20.7 (n=3), versus 0.26 s, 0.26–0.42 (n=10)
  for verified reuse. This native aarch64 result excludes hosted transfers.
  At hosted head `1d25f2b73e`, checksum verification accepts the shared output
  and both library-build and cache-save steps are skipped. All 32 observed
  dependency restores pass Yarn checks and skip installation: restore median
  15.2715 s (8.056–19.135), verification 22.5655 s (12.183–24.157). These are
  observations of the original cache keys, not tests of the generation repair
  or a matched speedup comparison.
  Hosted validation found a composite-action expression rejected before Rust
  setup: the repository cache epoch now enters through workflow inputs in all
  ten callers. Toolchain-only jobs count as an initial check, not a product-code
  result. Queue and execution metrics require an assigned runner; GitHub's
  placeholder start timestamps on queued/skipped jobs and negative intervals
  are excluded. A completion before its recorded start cannot become the first
  result or the all-done timestamp; two regressions cover invalid and absent starts. The observed queue delay remains separate from execution time.
  At `acebe8e3d2`, the first actionable product result is seven dependency-free
  package contracts: 273 s after the push proxy, comprising 2 s dispatch,
  153 s queue and 118 s execution (57 s contract step). At `1d25f2b73e`, six
  ECIES tests finished first in 46 s (2 s dispatch, 7 s queue, 37 s execution;
  18 s contract step). The same six ECIES tests at `acebe8e3d2` take 667 s,
  including 630 s queued and 35 s executing (17 s contract step).
  At `d62c84055b`, those six ECIES tests finish first in 103 s: 2 s dispatch,
  71 s queue and 30 s execution (14 s contract step).
  Historical median is 565.5 s, range 42–3242, across 34
  actionable pushes out of 35 observed. The proxy is the earliest eligible
  workflow creation time, including the CLA pull-request-target event. This
  set of uncontrolled n=1 observations does not establish a speedup. A historical ECIES
  exemplar also executes in 35 s but waits 535 s for its runner.
  Fresh hosted runs exposed missing Python coverage, Node fixture dependencies,
  Compose defaults and container Git trust; the selected setup now supplies these
  prerequisites. All 463 developer-tool tests, 97 Python coverage tests and five real Node
  coverage fixtures pass in hosted CI at `1d25f2b73e`. The verifier catalog has its own Vite configuration,
  preserving shared Storybook settings and excluding the application HTML plugin;
  its actual 14-story static build, config typecheck and formatting pass.
- **Rust compiler caching**: CI restores bounded sccache units separately from
  downloaded dependencies. On wrap-map-err, fresh output directories with an
  edited source compiled and passed all 18 tests in 3.42 s median, 3.42–3.52
  (n=10) without compiler reuse, versus 2.02 s, 1.97–2.07 (n=10) with it.
  Deliberately corrupt cached units triggered a successful ordinary rebuild
  (n=1, 18 tests). These small-crate results do not establish hosted full-workspace
  speedups. The hosted `1d25f2b73e` run restores no Rust compiler snapshots:
  all ten jobs have cache misses, with 4,063 Rust misses and no Rust hits. Its
  411 C/C++/assembler hits occur within the sequent-core job. Successful normal PR runs now save snapshots within GitHub's isolated merge-ref
  cache scope; trusted pushes seed shared snapshots, and other events remain
  restore-only. Compiler keys add run/attempt generations under the complete
  compiler-and-lock identity so source edits can refresh stored units. Cargo
  downloads keep fixed lockfile keys. Twenty modeled runs verify refresh, retry,
  success-only saves, incompatible identities and branch/PR isolation; seven
  failures before the correction become zero afterward. This validates the
  transport policy, not an actual hosted cache hit or a speedup; current hosted
  save/restore evidence is tracked in the issue. The compiler generations saved
  by the original `acebe8e3d2` and `d62c84055b` runs were later absent from the
  cache inventory; their successful saves did not establish reuse. A controlled
  seed/replay pair at `d62c84055b` (Tests run `36272273501`, attempts 2 and 3)
  then restores the exact saved PR-scoped generation, records six Rust hits and
  zero misses, passes all 18 tests without ignored or filtered cases, and saves
  the next compatible generation. Both dependent aggregate gates pass. This is
  n=1 seed plus n=1 replay at the same SHA, not a natural-push, source-edit or
  timing-speedup result. The missing archives and earlier zero-hit runs remain
  preserved; their disappearance cause is not established.
- **Rust service linker** (adopted on aarch64 Linux): identical application sources at
  `e443270f5c` were measured before and after selecting bundled LLD on aarch64.
  All 60 measured saves succeeded, with ten per edit and linker plus excluded
  warmups. Harvest's actual ready probe improves from 36.755 s median,
  36.24–38.66, to 9.895 s, 9.66–12.18. Its separate settled observation has a
  15-second floor. Windmill edits settle in 54.760 s, 53.48–55.93, versus
  128.290 s, 124.35–131.86; shared-core edits settle in 54.310 s, 53.19–55.33,
  versus 134.570 s, 130.80–154.65. Median host loads before/after are
  5.560/8.275, 8.215/4.955 and 7.845/7.140 on 16 CPUs. The candidate also
  requires Beat readiness after recreation; the baseline observed binary start
  because its probe did not respond. That recovery is not a linker effect.
  Repeated crate names represent different profiles/features: B4 uses release,
  and Harvest's Windmill graph enables `bstr/unicode`. A shared union build would
  not replace each service's own compatible build, so that experiment is rejected.
  Direct alternating cc/LLD builds also pass all 60 measured samples, n=10 per
  arm and edit: Harvest 29.108 s (28.508–29.510) versus 8.628 s (8.627–8.878);
  Windmill 38.322 s (37.668–39.024) versus 21.776 s (20.996–22.548); shared core
  to Harvest 53.721 s (53.195–55.551) versus 27.179 s (26.606–28.760).
  Corresponding linker medians fall from 23.498/19.534/31.194 s to
  3.091/2.765/4.364 s. Both arms rebuild the same 1/2/4 Cargo units; wall time
  minus links also includes Cargo coordination and is not isolated codegen.
  The LLD Harvest binary retains full debug sections: GDB hits its source
  breakpoint and produces a Rust/Tokio backtrace. Native validation at
  `a33e1b46f2` passes Harvest 202 tests, Windmill 1,439 (six ignored), and core
  528 with `keycloak,default_features,sqlite`; scoped Clippy and formatting pass.
  The full core test suite uses the configured development shell's PostgreSQL
  binaries for private test clusters; Harvest uses its CI SQL-default settings.
  Cargo-watch already follows local dependency directories, and the baseline
  Harvest edit only rebuilds Harvest in all ten samples. Extra watch lists and
  a pure-logic crate extraction are not justified by this evidence. No profiles,
  FIPS settings or features change.
- **Ballot verifier Vite** (adopted as opt-in): the final alternating comparison
  at `fa670fb53b` exceeds the predeclared 20% median improvement threshold.
  Warm first render improves from 8.599 s, 8.400–9.047, to 3.265 s,
  3.211–3.309 (n=10 each); fresh compiler/optimizer caches improve from
  9.028 s, 8.557–12.808, to 3.583 s, 3.523–4.777 (n=3 each). These fresh
  caches retain installed dependencies and the host filesystem cache. Visible
  leaf edits improve from 0.784 s to 0.075 s, and shared Header edits from
  2.256 s to 1.323 s (n=10 each). Production builds improve from 19.745 s to
  7.025 s (n=3 each); output and gzip bytes decrease 5.66% and 4.19%, with
  identical WASM bytes. Median warm server memory is 1148 versus 572 MiB
  (n=10, browser excluded). Loads range from 2.00 to 4.90 on 16 CPUs.
  Every measured browser operation and restore has zero page errors or mocked
  service violations. The complete journey matrix passes: 13 Webpack production,
  13 Vite production and 14 Vite development tests, including leaf state retention
  and shared/core edit restoration. Bootstrap changes reload to avoid recreating
  the React root. Webpack remains the default release/CI path; another portal
  requires its own compatibility and performance decision.
- **Optional environment prebuilds**: native arm64/amd64 builds use a source-free
  toolchain context. PRs build without publishing; trusted branch workflows
  publish matching images. Local selection checks identity and architecture,
  falling back to the standard image when missing or incompatible. Both native
  builds and offline toolchain readiness pass at PR head `d62c84055b`, GitHub
  merge checkout `42670c443c`. Fresh-volume readiness, including copying the
  store, is 218.630 s on arm64 and 288.421 s on amd64 (n=1 each, bounded budget).
  Warm medians are 11.214 s (11.064–11.364) and 10.7195 s (10.468–11.070),
  n=10 each plus excluded warmups. Every sample verifies the baked tools and
  WASM standard library with networking disabled; cleanup leaves no owned
  resources. The preceding native passes at `acebe8e3d2` and `1d25f2b73e` remain
  recorded separately.
  Earlier fresh-start timeouts and Nix database permission failures
  remain in the raw records. Bounded creation/cleanup and an explicit daemon
  handshake address these failures; three regressions fail before the daemon
  correction. The logs establish the permission failure, not a proven daemon
  race. These measurements establish toolchain readiness, not application/service
  readiness or a hosted CI speedup.
- **Repeatable backend scenarios**: all three named states create and reuse their
  own events; targeted reset rejects foreign ownership, tenant mismatches and
  concurrent execution. Four browser checks pass against current portal sources:
  kiosk and online ballot casting, verifier upload and the exact published
  totals. The task backend reuses binaries from `728c258313`; the driver and
  portal validation use `149131a62a`. Warm reuse takes 0.786 s median,
  0.633–0.953 for kiosk, 0.625 s, 0.589–0.793 for completed ceremony, and 0.631 s,
  0.595–0.715 for published results (n=10 each, one excluded warmup). These are
  current-state timings under concurrent load, not before/after speedups.
  All 85 scenario tests pass. The final integrated developer-tool suite passes
  465 tests in the configured devcontainer with no skips; Ruff lint and formatting
  pass. Hosted CI at `1d25f2b73e` passes its earlier 463-test suite with no skips.
- **Static browser-test servers**: configured port ranges are divided among
  Playwright parallel slots, including multiple servers per slot; ephemeral ports
  remain the default. A real two-worker reproduction fails before with a bind
  collision and passes after with four simultaneous listeners. Four permanent
  contracts, all owning fixture typechecks, lint and formatting pass.
- **Browser runner preflight**: focused journey/workbench commands launch the
  suite's configured Chromium before executing tests and give an actionable
  error when the pinned runtime cannot start. An actual pinned-browser launch
  passes; the local missing-library case fails early instead of hanging.
- **Workspace validation limits**: the historical backend cold attempt timed
  out with Harvest and Docker health still pending (one failed sample, zero
  successful samples). CLI and Windmill probes passed at 1750 s and 1736 s;
  no cause was established before cleanup. The final prepared warm backend
  series passes all ten starts under required readiness, but does not resolve
  or reclassify that cold failure. The initial unprepared final warm series
  also retains ten failures plus its failed warmup. Historical full cold
  before/after and UI+Keycloak cold remain n=1; UI-only cold has n=3. No matched
  final full-stack warm speedup or footprint reduction is established.
- **Walkthrough and retained preview**: the [106-second narrated recording](https://github.com/user-attachments/assets/ed5491e2-51f1-4e85-8118-065ac9e314b5)
  at `381c437eec` shows the workbench, actual five-step WASM pipeline, verifier,
  redesigned desktop/mobile Keycloak pages with header language/version/hash,
  keyboard controls, plus actual
  command selection. The demonstrated runtime paths still match the current
  integration across 82 Git tree/blob comparisons; both displayed command
  outputs remain byte-identical. Later provider and admin-catalog changes are
  outside those synthetic scenes, so whole-package-tree equality is not claimed.
  All ten captured scenes pass with empty page/console-error
  ledgers and all five actual WASM steps passing. The synthetic clipboard event is explicitly labeled; this
  recording does not submit authentication or establish formal WCAG conformance.
  The first capture exposed an incomplete verifier GraphQL response and a missing
  Keycloak favicon; the fixture now supplies both queried fields and the existing
  Sequent asset is served. Actual Apollo cache validation fails before the fixture
  correction and passes afterward. Six Storybooks and the workbench remain in a
  separate UI-only project within a task-owned isolated Docker daemon.
- **Admin widget catalog** (separate stacked PR https://github.com/sequentech/step/pull/3361):
  - **Coverage.** All 330 inventoried admin-portal widgets (of 493 scanned components) have an
    `Admin/<Feature>/<Component>` section that renders the production component with typed
    synthetic fixtures, with no backend or login. The stories assert the visible result and the
    boundary calls. The 163 styled primitives, providers and render-nothing helpers are excluded
    with reasons. The admin stories job runs `stories:inventory --check`, so a new widget needs
    its section.
  - **Harness decisions.**
    - Fixtures use only schema columns (`StoryRecord<T>`).
    - The resource boundary evaluates the Hasura `where` that ra-data-hasura builds.
    - Deep MUI imports are pre-bundled, so Vite never reloads mid-run.
    - Download helpers live once, in `src/__stories__/downloads.ts`.
    - Axe defects already present in production are marked per story with their exact rules
      (strict: a listed rule that stops firing fails the story).
  - **How the catalog was built.** Up to six agents in separate worktrees wrote the sections. The
    coordinator cherry-picked their commits and gated each push on `typecheck:stories`, eslint,
    prettier and the changed stories.
  - **Production defects the isolated stories exposed.** Each was fixed with a failing story or
    test first:
    - Election, event, contest and candidate screens still searched, sorted, showed and saved the
      `name`/`alias` columns that migration `1772358027729` removed. Election search found nothing.
    - Tenant options read a missing `username`.
    - Several forms crashed or read by the wrong ID before their record loaded.
    - Create and edit failures were silent or discarded the input.
    - Menu controls were unreachable by keyboard.
    - The archived event tab read an empty ID.
    - The dashboards spun forever on error.
    - Notification dates read a missing column.
    - Reported, not fixed: the unused `CreateContestData`, ListTally's unreachable filters, and
      the notification create button, which has no create form behind it.
  - **Measurements.**
    - At `8e69814e85` the suite is 246 story files and 1355 tests. All pass locally (aarch64,
      one Vitest worker, load 4–7, n=1) in 1100 s. The three `--shard=i/3` runs took 380, 355
      and 365 s. `build-storybook` took 51 s.
    - Hosted, the job took 8m22s for 518 tests and 11m44s for about 680, so the full suite would
      pass the job's 20-minute limit.
    - A three-shard matrix like the journeys job's was proposed to the CI workstream
      (https://github.com/sequentech/step/pull/3360#issuecomment-5849901470) rather than
      changed here.

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
