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
| `ui-only` | `.devcontainer/ui-only/devcontainer.json` | `devcontainer` | Storybook 6006–6010 (default `ui-essentials`), portals 3000–3004 | Stories and screens on fixtures |
| `ui-keycloak` | `.devcontainer/ui-keycloak/devcontainer.json` | adds `postgres-keycloak` and `keycloak` (8090), which starts without Harvest | Storybook 6006–6010 | Login and account themes |
| `backend` | `.devcontainer/backend/devcontainer.json` | the `base` profile: databases, MinIO, RabbitMQ, ImmuDB, Keycloak, Hasura, Harvest, Windmill, beat and B4 | none | Rust services, Hasura, step-cli |
| `full` | `.devcontainer/devcontainer.json` | as `backend` | portals 3000–3004 (default voting 3000 and admin 3002), Storybook | End-to-end work in the portals |

```sh
scripts/dev/step-dev mode list
scripts/dev/step-dev mode status
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

Every checkout gets its own Compose project, `<folder>_devcontainer`. The one in
a folder named `step` keeps `step_devcontainer` and unprefixed container names;
any other prefixes them with `<folder>-`, so its `ui-only` mode runs next to
another checkout's full stack. The other modes publish the same host ports, so
only one checkout runs them at a time. The devcontainer mounts the checkout's
parent at `/workspaces` and again at its host path, where git worktree links
resolve. Service containers build into the checkout's own `packages/target`;
local Cargo builds use `rust-local-target`.

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

Production builds and journeys still use the packages' `dist` entry points: run
`yarn --cwd packages build:ui-core` and `build:ui-essentials` before
`build:<portal>`. `STEP_SHARED_UI=dist` makes a dev server use those builds too,
for example to reproduce a production-only difference. The shared settings are in
`packages/ui-essentials/webpack.portal.cjs`.

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

The workbench dev server automatically loads the artifact published by
`step-dev wasm`, falling back to the installed package when none exists. Production
builds use the installed package. `WORKBENCH_SEQUENT_CORE=<wasm-pack web output>`
explicitly selects another build for either mode. No reinstall is needed; the page
reloads when the artifact changes and the inspector shows the binary's hash. `WORKBENCH_TEST_CHROME_PATH` selects a local Chromium for
`test:smoke`. Stories render one production route with its action; the workbench mounts
the production event routes. The only preview UI inside the portal frame is the error
shown when the portal loader rejects a snapshot.

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
  --target voting --target admin --target verifier --target results \
  --rebuild-cmd 'yarn --cwd packages build:ui-essentials'
$B ui-update --label before --checkout . --edit voting-screen --target voting
$B ui-update --label before --checkout . --edit shared-header --target storybook
$B test --label before --checkout . --suite cargo-harvest
$B rust --label before --checkout . --edit windmill-service --build windmill --build harvest
$B wasm --label before --checkout . --edit sequent-core-wasm
$B summarize ~/.cache/step-bench/results --phases
```

Edits insert a unique marker line and restore the file afterwards. `ui-update`
starts its own dev servers from `--port-base` and times until headless Chromium,
authenticated through the `ui-test-kit` mocks, shows the marker. `test` suites
cover Jest, a Storybook story and Cargo. `wasm` restarts the dev server after
`--build-cmd` and `--install-cmd` unless `--server-restart never`; `--no-change`
repeats the workflow without an edit.

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
