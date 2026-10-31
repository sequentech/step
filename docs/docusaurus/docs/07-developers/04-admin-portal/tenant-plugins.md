---
id: tenant_plugins
title: Tenant Plugins
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Tenant plugins add a tenant-level screen to the Admin Portal with its own
records, permissions and history. Projects, a timeline planner, is the first.
Plugins are first-party code compiled into the release: a tenant can turn a
bundled plugin on or off, but cannot install code.

This is separate from the [Windmill plugin manager](../10-tutorials/04-plugins-manager-guide.md),
which loads WASM hooks for election processing.

## Architecture

```text
Admin Portal                         Harvest                         PostgreSQL
PluginRoute ── PluginHost ──▶ Hasura action ──▶ /plugins/execute ──▶ plugin_record
 (lazy bundle)  execute(op, data)               │                     plugin_record_event
                                                ├ verified JWT → Identity (tenant, actor, roles)
                                                ├ Registry: manifest, permission, lifecycle
                                                └ Plugin::execute(op, input, &dyn HostPorts)
```

The plugin is the hexagon's core; everything it touches is a port:

| Crate | Role |
|---|---|
| `packages/plugins/api` (`sequent-plugin-api`) | The versioned contract: `Manifest`, `Plugin`, the `RecordStore`, `Audit` and `Clock` ports, `PluginError`, `HistoryScope`. |
| `packages/plugins/host` (`sequent-plugin-host`) | `Registry`, authorization, lifecycle and the PostgreSQL adapter (`ScopedStore`). |
| `packages/plugins/projects` | The Projects plugin: domain, CSV import and export, operations. |
| `packages/harvest/src/routes/plugin_platform.rs` | The HTTP gateway and the Keycloak role provisioner. |

A plugin never receives a database connection, a tenant selector or a user.
The host binds every statement to the tenant and actor of the verified token
and to the plugin's own namespace, and rejects record kinds the manifest does
not declare.

### Versioning

`API_MAJOR` is the contract's major version. The registry refuses at startup a
plugin built for another major, and Harvest does not launch. Each manifest also
has a `data_version` for its stored records. A tenant whose stored version
differs from the bundled plugin's shows the plugin as incompatible: it cannot
be turned on and its operations are refused until a release migrates the data.

### Lifecycle

| State | Menu and route | Operations | Data, history, roles |
|---|---|---|---|
| Not set up / turned off | Hidden; the route explains how to turn it on | Refused (`plugin-disabled`) | Kept |
| Turned on | Shown to users with the read permission | Allowed per permission | Kept |

Turning a plugin on needs `tenant-write`. It creates the manifest's
permissions in the tenant realm and adds them to the `admin` group before the
state is committed; if Keycloak fails, the plugin stays off and a retry
completes the missing roles. Turning it off never removes roles or data. The
lifecycle row is locked, so turning a plugin off waits for operations in
progress, and concurrent changes are refused with `plugin-state-conflict`.

### Storage

Three tables, not tracked by Hasura, so the gateway is the only API:

- `plugin_tenant`: per tenant and plugin, its state, `data_version` and lifecycle `version`.
- `plugin_record`: JSON records with `kind`, optional `parent_id` and an optimistic `version`.
- `plugin_record_event`: append-only history of every change, with the actor,
  before and after values, and the record's parent at the time, so a parent's
  history keeps changes to children that were later deleted.

The actor is stored twice: the stable user ID (`updated_by`, `actor`) and the
username at the time (`updated_by_name`, `actor_name`), which is what screens
show.

Deleting a tenant deletes its plugin rows; otherwise history cannot be changed
or deleted.

Every change to `plugin_tenant` and `plugin_record` is also logged in the
tenant's admin audit log, like other admin changes: the gateway names the admin
with `audit::act_for`, and the `audit_change` trigger records the old and new
values.

Every operation runs in one transaction; any error rolls back all of its
writes and history. Updates and deletes name the version they replace; a
stale version is refused with `plugin-conflict` and the current record, so the
screen can offer it explicitly.

### Limits

Requests and CSV files are at most 1 MiB, records 64 KiB, pages 1000 rows.
Projects allows 1000 tasks per project, 200-character names and titles,
16,000-character notes and dates from 1900 to 2200.

## Gateway

Hasura exposes three actions to the `admin-user` role, all forwarded to
Harvest with the caller's token:

| Action | Harvest route | Needs |
|---|---|---|
| `tenant_plugins` | `GET /plugins` | `admin-user` |
| `set_tenant_plugin_state(plugin_id, state, expected_version)` | `POST /plugins/state` | `admin-user`, `tenant-write` |
| `execute_plugin_operation(plugin_id, operation, data)` | `POST /plugins/execute` | `admin-user` and the operation's permission |

The gateway checks the operation's permission, or `tenant-write` for a state
change, before it contacts any backend. The token must be a tenant realm token (not an election event realm's) whose
subject is the Hasura user. Requests with unknown fields, such as a
`tenant_id`, are refused. Errors carry a stable `extensions.code`:
`plugin-forbidden`, `plugin-disabled`, `plugin-incompatible`,
`plugin-not-found`, `plugin-conflict` (with `extensions.current`),
`plugin-state-conflict`, `plugin-invalid-input` and `plugin-unavailable`.
Storage details are logged, never returned.

## Admin Portal

`src/plugins/` holds the frontend host:

- `registry.ts` lists the bundled contributions; each screen is a lazy chunk,
  never a URL from the server.
- `PluginProvider` loads the tenant's plugins and runs operations, discarding
  responses that arrive after the tenant changed.
- `PluginRoutes` adds menu items after Templates and renders `/plugins/:id`,
  explaining a plugin that is missing, off, incompatible or forbidden.
- `PluginSettings` is the Settings → Plugins tab.
- `NavigationGuard` holds menu navigation, browser Back and Forward, and tab
  closing while a screen has unsaved changes. It needs the portal's data
  router (`createBrowserRouter` in `src/index.tsx`).

A plugin screen receives a `PluginHost` with `execute(operation, data)` and
`can(permission)`; it never sees the tenant.

## Adding a plugin

1. Create a crate under `packages/plugins/<id>` that implements `Plugin` with a
   static `Manifest`: a lowercase `id`, `api_major: API_MAJOR`, its permissions,
   record kinds and one permission per operation. Keep the domain free of
   transport and storage types; use only the ports.
2. Test the domain directly, and the operations against the host with a
   disposable database (see below).
3. Add it to `registry()` in `plugin_platform.rs` and its permissions to
   `Permissions` in `sequent-core`, `IPermissions` in the Admin Portal and the
   default tenant realm.
4. Add a contribution to `src/plugins/registry.ts` with a lazily imported
   screen, and its translations in every language.

## Testing

```sh
# Domain and policy tests
cargo test -p sequent-plugin-api -p sequent-plugin-host -p sequent-plugin-projects

# PostgreSQL contract: each test creates and drops its own database
PLUGIN_TEST_DATABASE_URL=postgres://postgres:<password>@<host>:5432/postgres \
  cargo test -p sequent-plugin-host --test postgres -- --ignored

# Gateway
cargo test -p harvest --bin harvest plugin_platform

# Admin Portal
yarn --cwd packages/admin-portal jest src/plugins
yarn --cwd packages/admin-portal test:story src/plugins/projects/Projects.stories.tsx
```

Point `PLUGIN_TEST_DATABASE_URL` at a disposable server only: the tests create
databases named `plugin_test_*` on it.
