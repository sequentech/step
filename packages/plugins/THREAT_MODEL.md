<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# plugins threat model

`packages/plugins/` holds the WASM components that the windmill plugin manager loads at process start inside the harvest server container and the windmill worker containers, and runs in wasmtime. It has one plugin, `miru/`: a `wasm32-wasip2` component (`miru/.cargo/config.toml`) built from `miru/wit/world.wit` against the interfaces in sequent-core `src/plugins_wit/`. It exports `get-manifest` and `create-transmission-package`. Election managers can reach its route `/miru/create-transmission-package` through the Hasura mutation `call_plugin_route` (the admin-portal defines the mutation in `src/queries/CallPluginRoute.ts` but no page calls it on this branch); the handler returns a fixed message. It does not build, sign or send a transmission package; that is native code in windmill `src/services/consolidation/` and harvest `src/routes/miru_plugin.rs`. The package matters because plugin code runs inside the two services that hold the platform's data, and because plugin routes are reachable by admins. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Plugin binaries**: `plugins/<name>.wasm` objects in object storage, built from this directory. Integrity: they run inside harvest and windmill.
- **Host capabilities given to a plugin**: the `transaction` (database) and `authorization` host interfaces, and WASI. Confidentiality and integrity of platform data.
- **Caller claims**: the serialized `JwtClaims` that harvest puts into the route data as `claims`. Integrity: the plugin's authorization decision depends on them. Confidentiality: they carry the caller's name, email, tenant and roles.
- **Plugin manifest and route registry**: plugin name, hooks, routes and `process_as_task` returned by `get-manifest`. Integrity: they decide which code a route path runs.
- **Plugin runtime**: the plugins loaded in each harvest and windmill process. Availability, and confidentiality of anything a plugin keeps between calls.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `miru/src/lib.rs` `Component::create_transmission_package` (export `create-transmission-package`), synchronous path | Election managers: Hasura mutation `call_plugin_route` (role `admin-user`, body `{path, data}`) to harvest `POST /plugin`, then windmill `PluginManager::call_route` for path `/miru/create-transmission-package` | authenticated admin | Hasura role; harvest replaces `data.claims` with the caller's claims; checks in the handler |
| Same export, task path (the manifest sets `process_as_task`): harvest `POST /plugin` with `task_execution`, windmill Celery task `execute_plugin_task`, `PluginManager::execute_task` | Callers of harvest that set `task_execution`; the Hasura action forwards only `path` and `data` | internal service | Same handler checks |
| `miru/src/lib.rs` `PluginCommonGuest::get_manifest` | The windmill plugin manager, when harvest and windmill start | operator (plugin author) | Read once, at process start |
| Hook `create-transmission-package` through windmill `src/services/plugins_manager/plugins_hooks.rs` `PluginHooks::create_transmission_package` | windmill code; no caller on this branch | internal service | Same handler checks |
| Plugin binary under the `plugins/` prefix in object storage, read by the windmill plugin manager (`src/services/plugins_manager/`) at process start; uploaded in development by the repository's `.devcontainer/scripts/upload_plugins_to_s3.sh` | Whoever can write that prefix | operator | Write access limited to operators (deployment) |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| plugins-T1 | Spoofing | Forged or caller-supplied claims make the plugin authorize the wrong identity | harvest replaces any caller-supplied `claims` in the route data; Hasura validates the token; network isolation (deployment) | Partial |
| plugins-T2 | Elevation of privilege | A plugin route lets an admin act beyond their permissions or tenant | The miru handler changes nothing and returns a fixed message | Partial |
| plugins-T3 | Tampering | A replaced, added or modified binary under `plugins/` runs inside harvest and windmill with the host functions | Binaries are read only from the `plugins/` prefix, at process start; write access to that prefix is limited to operators (deployment, see Assumptions) | Partial |
| plugins-T4 | Elevation of privilege | A plugin uses the host database functions to read or change data it should not reach | `miru/src/lib.rs` calls no `transaction` function; only operators deploy plugins (deployment) | Partial |
| plugins-T5 | Tampering | A plugin registers routes or hooks that belong to another plugin, so a route runs other code | `miru/src/lib.rs` `get_manifest` declares only `/miru/create-transmission-package` and the hook of the same name; only operators can add plugins (deployment) | Partial |
| plugins-T6 | Elevation of privilege | Plugin code reads the service's files, environment secrets or network | windmill gives the plugin's WASI context no preopened directories, environment variables or arguments. Network access not verified | Partial |
| plugins-T7 | Denial of service | A plugin call exhausts CPU or memory, or fails, and degrades harvest or windmill for other callers | The miru handler does bounded work: one `serde_json::from_str` under serde_json's recursion limit, and errors returned as `Err` rather than panics | Partial |
| plugins-T8 | Denial of service | Malformed route data (invalid JSON, missing or non-string `claims`) crashes the plugin | `miru/src/lib.rs` `create_transmission_package` returns `Err` for each case | Mitigated |
| plugins-T9 | Denial of service | Route data that is not a JSON object panics the host before the plugin runs: harvest `POST /plugin` (`src/routes/plugins.rs` `plugin_routes`) and windmill `execute_plugin_task` (`src/tasks/plugins_tasks.rs`) | None on this branch | Open on release/10.0 (fix in sequentech/step#3522) |
| plugins-T10 | Information disclosure | Caller claims (personal data and roles) passed to the plugin leak through logs or other outputs | `miru/src/lib.rs` prints nothing | Partial |
| plugins-T11 | Repudiation | A plugin route call cannot be attributed to an admin afterwards | The task path records a `tasks_execution` row with the executor's name (harvest `plugin_routes`, windmill `src/services/tasks_execution.rs` `post`); the synchronous path relies on Hasura and harvest request logs | Partial |
| plugins-T12 | Tampering | A compromised dependency or regenerated binding ends up in a deployed plugin binary | `miru` is a member of the `packages/` Cargo workspace (`plugins/*`), so `packages/Cargo.lock` fixes `wit-bindgen-rt` at 0.42.1 (`miru/Cargo.toml` asks for `^0.42.1`); `miru/src/bindings.rs` is generated by wit-bindgen 0.41.0 and committed. No CI job in this repository builds or publishes plugin binaries, and the SonarQube workflow excludes `miru` from its build and clippy runs | Not verified |
| plugins-T13 | Denial of service | A faulty or unexpected object under `plugins/` disrupts plugin loading in harvest or windmill | Only operators can write the prefix (deployment) | Partial |
| plugins-T14 | Information disclosure | State a plugin keeps between calls leaks from one caller or tenant to the next | `miru/src/lib.rs` keeps no global or static state: each call parses its input and returns | Mitigated |

## Assumptions

- Only operators can write the `plugins/` prefix, and they deploy only binaries built from reviewed source in this directory (object storage, deployment).
- Hasura validates the Keycloak JWT before it forwards `call_plugin_route`, and only Hasura and platform services can reach harvest (Hasura, ingress).
- Only platform services can publish to RabbitMQ (RabbitMQ, deployment).
- The windmill plugin manager, not the plugin, decides what a plugin can reach (windmill `src/services/plugins_manager/`).
- Keycloak grants `admin-user` only to administrators (Keycloak).
- The database roles harvest and windmill use have only the privileges those services need (deployment).

## Review focus

1. Host capability surface: what the host interfaces give a plugin.
2. Plugin binary provenance: how binaries are built and reach `plugins/`.
3. Plugin route authorization: how plugin routes authorize their callers.
4. Route and hook registration in the plugin manager.
5. Resource use and failure handling of plugins in harvest and windmill.
6. Data that crosses into and out of a plugin, including caller claims.
