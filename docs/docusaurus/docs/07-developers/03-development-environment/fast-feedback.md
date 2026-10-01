---
id: fast-feedback
title: Fast feedback loops
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The development loop is edit, see the result, run the relevant test. Commands in
this guide run from the repository root inside the devcontainer unless stated;
`scripts/dev/step-dev <command> --help` describes each one.

## Devcontainer modes

Each mode starts only the Compose services and dev servers one kind of work
needs. VS Code's **Dev Containers: Reopen in Container** lists the four
configurations; the `devcontainer` CLI selects one with `--config`.
`.devcontainer/modes.json` defines them.

| Mode | Configuration | Compose services | Dev servers | Use |
| --- | --- | --- | --- | --- |
| `ui-only` | `.devcontainer/ui-only/devcontainer.json` | `devcontainer` | Storybook 6006–6011 (default `ui-essentials`), workbench 5173, portals 3000–3004 | Stories and screens on fixtures |
| `ui-keycloak` | `.devcontainer/ui-keycloak/devcontainer.json` | adds `postgres-keycloak` and `keycloak` (8090), which starts without Harvest | Storybook 6006–6011 | Login and account themes |
| `backend` | `.devcontainer/backend/devcontainer.json` | the `base` profile: databases, MinIO, RabbitMQ, ImmuDB, Keycloak, Hasura, Harvest, Windmill, beat and B4 | none | Rust services, Hasura, step-cli |
| `full` | `.devcontainer/devcontainer.json` | as `backend` | portals 3000–3004 (default voting 3000 and admin 3002), Storybook, workbench 5173 | End-to-end work in the portals |

```sh
scripts/dev/step-dev mode list
scripts/dev/step-dev mode status
scripts/dev/step-dev mode up ui-only --servers storybook-ui-essentials,workbench
scripts/dev/step-dev mode up ui-only --servers storybook-keycloak-ui
scripts/dev/step-dev mode up ui-keycloak
scripts/dev/step-dev mode switch backend
scripts/dev/step-dev mode stop
```

They work inside the devcontainer and on the host. `up` starts the mode's
services, waits for their health checks and probes, starts the mode's default
dev servers (`--servers none`, or a comma-separated list) and prints the URLs.
Dev servers need `yarn --cwd packages install --frozen-lockfile` first. A server
already listening on its port is reused rather than started again; the ones
`step-dev` starts log to `.cache/dev-mode/`. `switch` first stops the services
and `step-dev` servers the target mode does not use; `stop` stops all of them
except the devcontainer. Nothing removes volumes or caches. `up` starts existing
containers as they are, replacing only one whose health check changed; rebuild
the devcontainer to apply other Compose changes. `up`, `switch` and the
devcontainer's initialize command fail before touching a container when another
Compose project publishes a port or holds a container name the mode needs.

Reopening the folder with another configuration attaches to the running
devcontainer without starting that mode's services: run `step-dev mode switch`,
or **Dev Containers: Rebuild Container** and pick the configuration.

### Opt-in Scanovate services

The identity verification of enrollment (Liveness Plus with its PAD server, Face
Match and OCR) belongs to the `scanovate` Compose profile, which no mode starts:
its images are private, in our ECR mirror, and need about 18 GB of memory. To
work on enrollment, start them next to `backend` or `full`, from the host, in the
checkout:

```sh
aws sso login --profile sequent-ecr
aws ecr get-login-password --profile sequent-ecr --region eu-west-1 \
  | docker login --username AWS --password-stdin 133529410358.dkr.ecr.eu-west-1.amazonaws.com
(cd .devcontainer && docker compose --profile scanovate up -d)

# PAD and OCR load their models for a minute or two
curl -s http://127.0.0.1:5050/alive http://127.0.0.1:5060/alive http://127.0.0.1:5070/alive

# Free their memory when done
(cd .devcontainer && docker compose --profile scanovate stop)
```

`step-dev mode status` lists them under `outsideModes`. The development realm
and the sample enrollment event already point to them; see
[Scanovate On-Premise Services](../../integrations/scanovate_on_premise_guide.md#running-them-in-the-development-environment)
for the ECR access, the ports and the end-to-end test.

Every checkout gets its own Compose project, `<folder>_devcontainer`. The one in
a folder named `step` keeps `step_devcontainer` and unprefixed container names;
any other prefixes them with `<folder>-`, so its `ui-only` mode runs next to
another checkout's full stack. The other modes publish the same host ports, so
only one checkout runs them at a time. The devcontainer mounts the checkout's
parent at `/workspaces` and again at its host path, where git worktree links
resolve. Service containers build into the checkout's own `packages/target`;
local Cargo builds use `rust-local-target`.

Harvest, Windmill and beat each run their own `cargo run` command. Cargo-watch
finds the current crate's local dependency directories automatically: a Harvest
leaf edit affects Harvest, while a Windmill edit also affects Harvest and beat.
Keep the per-service feature graphs separate; compiling the packages together can
enable extra dependency features and invalidate the subsequent service build.

On aarch64 Linux, the devenv shell and these services select the Rust toolchain's
bundled LLD through `.devcontainer/scripts/rust-lld-cc.sh`. The wrapper preserves
debug information and compiler arguments, falls back to `cc` when that bundled
driver is unavailable, and propagates link failures. To compare the default
linker locally, prefix Cargo with
`CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=cc`. Changing Cargo's configured
linker invalidates compilation fingerprints; warm the selected configuration
before comparing incremental rebuilds.

Dependency caches live in Docker volumes shared by all checkouts, so a new
worktree or a recreated container does not download them again:

| Volume | Mounted at | Holds | Refreshed |
| --- | --- | --- | --- |
| `step-devcontainer-nix-<image tag>` | `/nix` | Nix store with the devenv toolchains | a devcontainer image upgrade starts a new volume |
| `step-devcontainer-cache` | `~/.cache` | Yarn, Nix fetcher and Playwright caches | never stale: entries are versioned |
| `step-devcontainer-cargo` | `~/.cargo` | Cargo registry and git checkouts | never stale: entries are versioned |

`docker compose down --volumes` leaves them alone. To reset one, remove the
containers that mount it, `docker ps --all --filter volume=<name>` (the Rust
services and Hasura share the devcontainer's mounts), then
`docker volume rm <name>`; the next start recreates them. Every devcontainer
runs its own Nix daemon on the shared store. A garbage collection sees only the
roots and builds of its own container, so run `nix-collect-garbage` while no
other devcontainer is up; `.devcontainer/scripts/free-space.sh` skips it then.

### Prebuilt toolchains

On the host, `scripts/dev/step-dev prebuild pull` fetches the environment image
matching this checkout; `prebuild status` shows whether it is available. Rebuild
the devcontainer after pulling. Initialization uses only a local image whose
content label and architecture match, otherwise it uses the existing devenv
image and evaluates the shell locally. An unavailable registry never blocks
that fallback. Each prebuild gets its own shared Nix volume so an older volume
cannot hide the image's populated store.

The **Prebuild development tools** workflow builds native amd64 and arm64 images
from `devenv.nix`, `devenv.lock`, `devenv.yaml` and the locked devcontainer
features. The build context contains no application source or local `.env`.
PRs build without publishing; pushes to `main`/`ovcs` and manual runs on those
branches publish `ghcr.io/sequentech/step-devenv:env-<input hash>`. GHCR package
write permission is needed for trusted publishing and the package must be public
for anonymous pulls. Change a toolchain/feature input to get a new tag, or
manually run the workflow to refresh an existing recipe, then pull again.
`prebuild fingerprint` prints the key; `prebuild context --destination <empty-dir>`
creates the same small build context for local inspection.

After building, the workflow starts the local native image with networking
disabled and checks Node, Yarn, Rust, Cargo, wasm-pack, wasm-bindgen and the Rust
WASM standard library. It measures three fresh Nix-volume starts, then ten
new-container starts sharing the final volume after an excluded warmup. Only one
copied store exists at a time. Raw timings, tool versions, load, image identity,
logs and the sample counts are in the platform's `prebuild-smoke` artifact.
Fresh-volume seeding is included; image build/pull and application/service
readiness are separate. Each sample records container creation and toolchain
startup separately, with a combined ten-minute timeout and a twelve-minute
measurement budget. If the observed first fresh start leaves insufficient time
for the remaining fresh starts and warm series, the report states the smaller
sample count. Warm samples must complete. Insufficient disk headroom fails before
any store copy. To check an already built local image without downloading or publishing:

```sh
python3 -m scripts.dev.prebuild_smoke --image <local-image> --output-dir /tmp/prebuild-smoke
```

Pass `--docker-host unix:///path/to/owned/docker.sock` for an isolated daemon;
otherwise the helper uses `DOCKER_HOST` when set, or the default local daemon.
It removes only the containers and volumes bearing this run's UUID owner label.
A timed-out creation stays tracked while cleanup waits up to ninety seconds for
the container to become inspectable. Failure artifacts retain creation/start
logs, available container/daemon diagnostics and any resources still awaiting
cleanup. The workflow allows fifteen minutes for measurement and cleanup.

## Shared UI hot reload

Portal dev servers compile `@sequentech/ui-core` and `@sequentech/ui-essentials`
from `src`, so a shared component edit reaches every running portal without a
package build or server restart. `PORT` overrides a portal's default port and
`BROWSER=none` stops it opening a browser:

```sh
PORT=3100 BROWSER=none yarn --cwd packages/voting-portal start
```

React Refresh keeps component state when the edited module exports only
components; other edits reload the page. React, MUI, Emotion, router, i18n,
Apollo and `sequent-core` always resolve to the portal's own copy. Dev servers do
not type-check: run `test:types` in the voting portal, results portal or ballot
verifier (it resolves the shared sources), or build the admin portal.

Default webpack production builds and journeys use the packages' `dist` entry points: run
`yarn --cwd packages build:ui-core` and `build:ui-essentials` before
`build:<portal>`. `STEP_SHARED_UI=dist` makes a dev server use those builds too,
for example to reproduce a production-only difference. The shared settings are in
`packages/ui-essentials/webpack.portal.cjs`.

The ballot verifier also has an opt-in Vite server:
`yarn --cwd packages/ballot-verifier start:vite`. Its `build:vite` compiles shared
source into `dist-vite`, and `preview:vite` serves that output. Webpack remains
the default server, release build and CI build. See the
[UI browser test guide](testing/ui-browser-tests.md) for Vite production and
development journeys. Other portals require their own asset, bootstrap and
journey validation before adopting this configuration.

## Screens, workbench and scenarios

Production voter screens open against synthetic elections without login or services.
Scenarios in `packages/ui-test-kit/fixtures/scenarios.ts` build versioned snapshots:
`version`, `scenarioId`, `provenance`, `tenantId`, `areaId`, `channel` and a
publication `preview` document. `VoterPreview` in `packages/voting-portal/src/preview/`
serves Storybook and the workbench: portal theme, authentication disabled, a voter on
the snapshot's channel, a GraphQL client that rejects every operation, the production
WASM gate and store, and the portal's publication preview loader.

```sh
yarn --cwd packages/workbench dev            # 127.0.0.1:5173, or WORKBENCH_PORT
yarn --cwd packages/voting-portal storybook  # localhost:6007
yarn --cwd packages/workbench test           # policy, snapshot and storage units
yarn --cwd packages/workbench test:smoke     # Playwright flow, reuses a running dev server
```

The workbench link `#/scenario/<scenario>/<screen>` and the story
`scenarios-<scenario>--<screen>` (title `Scenarios/<scenario title>`) open the same
screen. Scenarios are `simple-plurality`, `ranked-multi-contest` and `kiosk-voter`;
screens are `chooser`, `start`, `vote`, `review` and `confirmation`. Review and
confirmation open with the first valid choices encrypted. The `Voter channel` toolbar
switches any story between online and kiosk voters.

The workbench imports, exports and resets snapshots, overrides contest policies and
vote bounds, shows the store and the voting screen's validation, and runs interpret,
Next checks, encrypt, hash and decode on the current selection with sequent-core.
Exports carry the overrides in the document and list them in `provenance.changes`.
Its local storage keys start with `sequent.workbench.v1.`; Reset removes them and the
portal's session storage. Requests to other origins and non-GET requests are refused.
Workbench controls have stories under `Workbench/`.

Admin widgets also open in isolation: `yarn --cwd packages/admin-portal stories:inventory <source file>`
prints the section, story IDs, links and focused test command of each widget in the
file; the [admin widget catalog](./testing/ui-browser-tests.md#admin-widget-catalog)
describes its fixtures and boundaries.

The workbench dev server automatically loads the artifact published by
`step-dev wasm`, falling back to the installed package when none exists. Production
builds use the installed package. `WORKBENCH_SEQUENT_CORE=<wasm-pack web output>`
explicitly selects another build for either mode. No reinstall is needed; the page
reloads when the artifact changes and the inspector shows the binary's hash. `WORKBENCH_TEST_CHROME_PATH` selects a local Chromium for
`test:smoke`. Stories render one production route with its action; the workbench mounts
the production event routes. The only preview UI inside the portal frame is the error
shown when the portal loader rejects a snapshot.

### Embedded voter preview

`packages/workbench/embed.html` is the same voter preview for other tools to frame,
such as beyond's Election Architect. `EmbeddedPreview` in
`packages/voting-portal/src/preview/` renders the production event routes inside the
portal chrome (`components/PortalChrome.tsx`: header, footer, watermark and the
event's stylesheet) in the publication preview's demo mode. It also places telephone
calls: `EmbeddedCall` runs the IVR emulator the framing tool serves through
`ui-essentials`' `IvrCall`, the component the Admin Portal's emulator uses. The framing
window talks to it with `postMessage` (`preview/embed.ts`, protocol
`sequent.voter-preview`, version 2):

| Message | Direction | Fields |
| --- | --- | --- |
| `ready` | embed to parent | sent on load; the parent then sends `show` or `call` |
| `show` | parent to embed | `document` (a publication preview document), `areaId`, `screen`, optional `electionId`, `language` and `channel`; opens a new voter session |
| `shown` | embed to parent | `screen` (when it is a preview screen) and `path`, after `show` and after each navigation by the voter |
| `call` | parent to embed | `config` (what the IVR Lambda is given: the event, the open ballot styles, the caller), `emulatorUrl` (absolute base URL of the wasm-bindgen output) and optional `labels` in the parent's language; places a new call |
| `calling` | embed to parent | `status`: `loading`, then `IvrCall`'s `Running`, `ExpectingInput` or `Disconnected`, or `absent` when nothing is served at `emulatorUrl` |
| `failed` | embed to parent | `issues`, for a malformed request, a document the portal loader rejects, or an emulator that does not start |

The embed only reads messages from its parent window and replies to that window's
origin. Its network guard admits same-origin reads and the two emulator files a `call`
names, which may live at the framing tool's origin. The demo watermark is bundled, so
it also shows when the embed is served below another path. During development a framing tool points at the dev server
(`http://127.0.0.1:5173/embed.html`), so portal edits reload inside it. `vite build`
writes both pages to `packages/workbench/dist/` with relative URLs, and the
`build_wasm` workflow uploads that directory as the `voter-preview` artifact beside
`sequent-election-config-wasm`, with `voter-wording-keys.json`: every wording key the
portal draws a string for, which the Election Architect checks its overrides against.
`yarn build` also writes `dist/problem-list/`: `ui-essentials`' `ProblemList` and
`ui-core`'s `problems.*` sentences as one ES module (`src/problemList.ts`, built by
`vite.problem-list.config.ts` with React, MUI and i18next left to the host, declarations
by `tsconfig.problem-list.json`). The Election Architect imports it to show an import's
problems as the Admin Portal does, and gets it with the voter preview.
The story `embedded-voter-preview--vote` shows a screen inside the chrome, the
`embedded-voter-preview-telephone-call--*` stories a call over a stand-in emulator
(`preview/fakeIvrEmulator.ts`), and
`yarn --cwd packages/workbench test:smoke tests/embed.spec.ts` drives the messages
from a framing page.

### Real-backend scenarios

When a story cannot answer the question, `step-dev scenario` brings a synthetic election
event of its own to a named state on the checkout's running stack (`mode up backend` or
`full`). Run it in the devcontainer:

```sh
scripts/dev/step-dev scenario list
scripts/dev/step-dev scenario up kiosk-voter         # kiosk voting open
scripts/dev/step-dev scenario up completed-ceremony  # keys ceremony, ballots, online voting open
scripts/dev/step-dev scenario up published-results   # votes cast, tallied, results published
scripts/dev/step-dev scenario urls kiosk-voter
scripts/dev/step-dev scenario status
scripts/dev/step-dev scenario reset kiosk-voter
```

`up` imports the backend journeys' fixture and census through step-cli, waits on the
task, ceremony and publication status, and prints the portal links and the synthetic
voter credentials. The event is recorded in `.cache/scenarios/<Compose project>/` and
carries owner annotations; the next `up` checks both and continues from the furthest
stage that still holds. `reset` deletes only that event. Ceremonies start `trustee1`
and `trustee2`, which no mode starts; their first start builds the braid image. On a
new stack the first `up` enrolls the tenant administrator's email code, as the journeys
do; the admin portal then asks for it, and the Keycloak container log shows it.
`VOTING_PORTAL_URL`, `BALLOT_VERIFIER_URL` and `RESULTS_PORTAL_URL` select the printed
portals, and `--step-cli` another step-cli build.

`up`, `urls`, `status` and `reset` accept `--format json`; progress goes to stderr.
An empty reset still returns a JSON outcome. Reset refuses a mismatched owner or
tenant and keeps the state file if deletion fails, so it can be retried. A second
command for the same scenario fails while the first holds its lock.

## Incremental WASM

After editing `sequent-core` or a crate it depends on, run from the devenv shell:

```sh
scripts/dev/step-dev wasm            # rebuild and publish when inputs changed
scripts/dev/step-dev wasm --status   # does the published build match the sources?
scripts/dev/step-dev wasm --clean    # drop the development package and its cache
```

The command fingerprints the path crates Cargo resolves for the wasm32 build
(their library sources, manifests and files named by `include_str!`,
`include_bytes!` or `#[path]`), the resolved dependency versions, sources and
features, the workspace profiles, Cargo configuration, `rust-toolchain.toml`,
the build recipe, the rustc, Cargo and wasm-bindgen versions, C compiler settings
and the command itself. Tests, benches, examples, other workspace members,
unrelated `Cargo.lock` entries, hidden files and editor backups are not inputs.
Without changes it only prints `up to date`. Otherwise it compiles incrementally
in `packages/rust-local-target/sequent-core-wasm/` and publishes one development
package there; node_modules, Yarn caches and dist trees are untouched.

Portal dev servers (`start:*`) and Storybook load that package instead of the
installed tgz while it exists, and reload the open page when a new build is
published; restart them after the first publish or after `--clean`. Production
builds always use the installed tgz. A failed build exits non-zero, keeps the
previous build and logs `development WASM build failed` in the browser console
until a build succeeds or the sources match the published build again.

The committed `packages/*/rust/sequent-core-0.1.0.tgz` files are the release
package. It records its source fingerprint; regenerate it with wasm-opt from the
sequent-core flake, then reinstall and commit the four tgz files and `yarn.lock`:

```sh
nix develop ./packages/sequent-core --command scripts/dev/step-dev wasm --release-package
yarn --cwd packages install --frozen-lockfile
scripts/dev/step-dev wasm --check-package
```

`--check-package` fails, naming the changed inputs, when the committed package was
not built from the checked-out sources; CI runs it in `build_wasm.yml`.
For recovery, `.devcontainer/scripts/rebuild-sequent-core-full.sh` also removes
the installed copies so that the next install extracts them again.

## Focused tests

`step-dev test` runs the narrowest existing command for a package, check, file,
directory, story or spec and prints that scope, and what it leaves out, first.
It adds no coverage and starts no services unless the selected check needs them.

```sh
S=scripts/dev/step-dev
$S test voting-portal            # the package's fast tests
$S test packages/ui-essentials/src/components/Header/Header.tsx   # Jest related tests
$S test packages/voting-portal/src/components/StartActions/StartActions.test.tsx -t 'keyboard'
$S test admin-portal --story screens-admin-tally-ceremony--populated
$S test packages/voting-portal/test/journeys/review.spec.ts 'cast confirmation'
$S test packages/sequent-core/tests/sqlite_feature_boundaries.rs
$S test windmill services::probe
$S test scripts/dev/affected/model.py
$S test voting-portal --watch
$S test --list                   # every check, its cost and command
$S test --affected               # fast checks of the current changes
$S validate                      # also slow checks; --depth full adds integration
$S affected --worktree           # what the changes affect, and why
```

A second argument, `-t`, `-g` or `-k` filters by test name (Jest and Vitest `-t`,
Playwright `-g`, the Cargo filter, unittest `-k`); arguments after `--` go to the
runner. A narrowed run that executes no test fails. A Rust source file runs its
crate's whole check, since any test may exercise it; a file under `tests/` runs
that target with the features its `#![cfg]` and `required-features` need. Cargo
builds into the checkout's `packages/rust-local-target` unless `CARGO_TARGET_DIR`
is absolute. Journeys build the production portal first only when its `dist/` is
missing, so rebuild it after source changes. `--watch` uses Jest's and Vitest's
watch modes, `cargo watch` over the crate and its path dependencies, or reruns
when files of the package or its dependencies change.

Checks cost `fast` (installed dependencies and compilers, a browser for small
suites), `slow` (story catalogues, production, release or cross-target builds,
Maven, the documentation site) or `integration` (Docker stacks or a database).
`--depth fast|broad|full` runs up to that cost; `validate` is
`test --affected --depth broad`.

`scripts/dev/affected.toml` and the workspace manifests form the one model that
local runs and CI select from. Yarn and Cargo packages and their edges come from
the manifests, including `file:` archives, path dependencies of every kind and
files compiled in with `include_str!`. The model file adds the areas outside
packages, ordered path rules, edges between ecosystems (the committed
sequent-core archives, workbench stories in the voting portal's Storybook, Hasura
migrations read by Harvest and Windmill tests) and the checks. A changed path
belongs to the first matching rule, else to the package whose directory holds
it; a package is affected when its files or inputs change or a dependency is
affected. A unit test fails for tracked files that no rule claims.

Changes count from the merge base with `--base` (default: the branch's upstream
unless it is the same branch, else `origin/ovcs`); `affected` counts commits and
`--worktree` adds staged, unstaged and untracked files, which `test --affected`
includes unless `--committed`. A missing base or merge base (deepen a shallow
clone), an unclaimed path, a devenv change or a change to the model selects every
check. `packages/yarn.lock` affects every Yarn package; `Cargo.lock` affects the
crates whose locked dependencies changed. sequent-core sources select
`wasm-freshness`; the frontends' checks follow the committed tgz.

`affected` prints each changed file's owner, the affected packages with their
file → package → consumer chain, and every check selected or skipped with the
reason. `--graph` prints the units and edges; `--json` prints a versioned
document with `base`, `fallback`, `files`, `units` and `checks` (`selected`,
`reasons`, `cost`, `cwd`, `command`, `env`, `requires`, `workflows`) for CI.

## Benchmarks

`scripts/dev/step-dev bench` times these loops for any checkout given with
`--checkout`, so two trees can be compared on the same machine. Each series is
one JSON file under `<output-dir>/<scenario>/` (default
`$STEP_BENCH_OUTPUT_DIR`, else `~/.cache/step-bench/results`) with the commit,
tool versions, load average around every sample, services, cache state,
commands and raw samples. Warm series run untimed warm-up iterations, then ten
samples by default.

```sh
B="scripts/dev/step-dev bench"
$B ui-update --label before --checkout . --edit shared-header \
  --target voting --target admin --target verifier --target results
$B ui-update --label before --checkout . --edit voting-screen --target voting
$B ui-update --label before --checkout . --edit shared-header --target storybook
$B test --label before --checkout . --suite cargo-harvest
$B rust --label before --checkout . --edit windmill-service --build windmill --build harvest
$B wasm --label before --checkout . --edit sequent-core-wasm
$B summarize ~/.cache/step-bench/results --phases
```

Portal dev servers compile shared UI source directly. For a legacy baseline
that loads shared `dist` output, select that mode and include its rebuild:

```sh
STEP_SHARED_UI=dist $B ui-update --label legacy-dist --checkout . --edit shared-header \
  --target voting --target admin --target verifier --target results \
  --rebuild-cmd 'yarn --cwd packages build:ui-essentials'
```

Edits insert a unique marker line and restore the file afterwards. `ui-update`
starts its own dev servers from `--port-base` and times until headless Chromium,
authenticated through the `ui-test-kit` mocks, shows the marker. `test` suites
cover Jest, a Storybook story and Cargo. `wasm` restarts the dev server after
`--build-cmd` and `--install-cmd` unless `--server-restart never`; `--no-change`
repeats the workflow without an edit.

`monitoring-viewers` is a load test, not a loop timing. It simulates
concurrent viewers of a monitoring dashboard against a running stack; see
[Viewer load benchmark](../13-monitoring/01-monitoring-architecture.md#viewer-load-benchmark).

`workspace` and `ci` run on the host, with the Dev Containers CLI and `gh`.
`workspace` never uses the default Docker daemon: a cold sample creates a fresh
Docker-in-Docker daemon and checkout copy under `--sandbox-root`, and `--keep`
leaves them for warm samples, which stop the stack and time the next start
until its readiness probes pass. `clean` removes a kept daemon.

```sh
$B workspace --label before --checkout . --cache cold --keep --sandbox-root ~/bench
$B workspace --label before --checkout ~/bench/envs/step-bench-dind-1/step \
  --cache warm --docker-host unix://$HOME/bench/dind/step-bench-dind-1/run/docker.sock
$B ci --label before --pr "$PR"
```

## Incremental CI

The `Tests` workflow selects checks with the same dependency model as local
commands. Pull requests compare their head with the actual base's merge base;
checkout fetches the full history. Missing history, unknown paths or changes to
the selection runner select all checks. Pushes to `main`, `ovcs` and `release/**`
run full validation. A newer PR commit cancels its older feedback run.

```sh
scripts/dev/step-dev affected --base origin/ovcs --json
python3 -m scripts.dev.ci plan --base origin/ovcs --output /tmp/ci-plan.json
```

Read the plan job's summary for affected packages, selected and skipped checks,
and reasons. The plan artifact includes the same model JSON used locally. Jest,
Rust and tooling jobs run only selected package suites; selected stories still
run interactions, accessibility, types and the catalog build. Production journeys
reuse shared-library and portal builds from the same immutable run; every
selected test reruns. The four admin journey shards use one production build.
Admin-portal stories run in three shards; the first also type-checks the stories,
checks the widget inventory and builds the catalogue.

`Required feedback checks` rejects failed, cancelled, missing or unexpectedly
skipped managed jobs. It covers `Tests` and its reusable frontend UI workflow;
existing coverage, backend integration and other workflows keep their separate
gates. Repository maintainers can add the stable feedback check to branch rules.

Frontend dependency caches require an exact OS, architecture, Node, Yarn,
workspace-manifest, lockfile and packaged-WASM match. Restored dependencies pass
Yarn's integrity and file checks before installation is skipped; a missing or
invalid cache runs a normal frozen install. Dependency and shared-output keys add
a run-and-attempt generation to the complete identity; restore prefixes retain
that complete identity. Verification decides reuse even when Actions reports
`cache-hit=false` for a compatible prefix match. An invalid restore installs or
rebuilds and saves a fresh generation, repairing reuse on later runs without
overwriting an immutable cache. Cache availability never substitutes for a
current build or test. GitHub scopes PR-written caches to that PR; base pushes
populate caches that later PRs can restore. To force a dependency cache miss,
increment `frontend-v2` in both setup-action keys, or delete the relevant cache.

Shared UI outputs also use an exact content identity: transitive workspace
sources, workspace manifests, lockfile, packaged WASM, build configuration and
recipe, OS/architecture/libc, Node/Yarn and build environment. A checksum manifest
must match the current identity and every output before compilation is skipped.
Missing, stale or damaged outputs rebuild. A portal leaf edit therefore reuses
unchanged shared libraries. Each portal production build still runs once per run.
Artifacts use the immutable run identity and output manifests; a partial job
retry can consume a successful earlier producer from that run. Tests rerun in
both cases. Before the first base cache has been populated, a new PR has a cold
cache; rerunning the PR demonstrates its own warm cache path.

### Rust compiler caches

Rust test and CLI jobs restore Cargo downloads separately from a bounded local
sccache store. The compiler key includes OS/architecture, rustc identity,
workspace manifests and Cargo configuration, the actual workspace lockfile,
profiles, features, targets and compiler flags. Source edits reuse compatible
units; compiler restore prefixes retain that full identity, including the
lockfile. Cargo still builds and runs tests every time, and sccache validates each
compilation's inputs.
A snapshot hit never skips a test.

The job summary reports the key and restore status; the sccache post-step reports
compiler hits, misses and unsupported calls. Each compiler snapshot is limited
to 512 MiB. Successful PR runs save within their merge ref, which
[GitHub excludes from the base branch and other PRs](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache);
successful pushes to `main`, `ovcs` and
`release/**` seed shared snapshots. Other events, including manual runs and
`pull_request_target`, only restore. Run-and-attempt generation keys save newly
compiled units after source edits without changing the compatibility identity.
Cargo download caches retain fixed lockfile keys and skip saving on exact hits.
Set `STEP_RUST_CACHE_EPOCH` to a new value to reset compiler compatibility.
GitHub may evict older generations within its repository cache quota. A
missing download or compiler snapshot builds normally; an unavailable sccache
installer falls back to rustc. To reproduce an identity locally:

```sh
scripts/dev/step-dev rust_cache --name sequent-core --lockfile packages/Cargo.lock \
  --target-dir packages/rust-local-target --profile dev --features default_features,keycloak
```
