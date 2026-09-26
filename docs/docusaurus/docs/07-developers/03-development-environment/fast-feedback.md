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
except the devcontainer. Nothing removes volumes or caches. `up` recreates a
service whose Compose configuration changed, but never the devcontainer: rebuild
that to apply its changes. `up`, `switch` and the devcontainer's initialize
command fail before touching a container when another Compose project publishes
a port or holds a container name the mode needs.

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

## Screens, workbench and scenarios

## Incremental WASM

## Focused tests

## Benchmarks

## Incremental CI
