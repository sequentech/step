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
dev servers (`--servers none`, or a comma-separated list) and prints the URLs. A
server already listening on its port is reused rather than started again; the
ones `step-dev` starts log to `.cache/dev-mode/`. `switch` first stops the
services and `step-dev` servers the target mode does not use; `stop` stops all
of them except the devcontainer. Nothing removes containers, volumes or caches,
and `up` reuses existing containers as they are: rebuild the devcontainer to
apply Compose changes. `up`, `switch` and the devcontainer's initialize command
fail before touching a container when another Compose project publishes a port
or holds a container name the mode needs.

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

`docker compose down --volumes` leaves them alone. To reset one, stop the
devcontainers using it (`docker ps --filter volume=<name>`) and
`docker volume rm <name>`; the next start recreates it. Garbage-collecting the
Nix store only keeps what the checkouts visible in that container use, so run
`nix-collect-garbage` while no other devcontainer builds.

## Shared UI hot reload

## Screens, workbench and scenarios

## Incremental WASM

## Focused tests

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
