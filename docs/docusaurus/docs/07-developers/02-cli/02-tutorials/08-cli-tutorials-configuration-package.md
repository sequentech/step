---
id: cli_tutorials_configuration_package
title: Applying a Configuration Package
position: 8
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

A configuration package is the delivery zip Election Architect exports: the
election event with its census, and the tenant's configuration, realm settings,
role mappings, administrator accounts and trustees' public keys. Applying it
configures each of those on one tenant, in that order, through the platform's
existing importers. Importing only the event zip through the Admin Portal is
unchanged and never applies the other sections.

Configure the tenant with `step-cli step config` as described in the
[getting-started guide](./01-cli-tutorials-getting-started.md). The account
needs every permission the sections use: `tenant-write`, `user-write`,
`role-write`, `trustee-create`, `trustee-write` and `election-event-create`.
The same set is required to preview, since a preview reads what an application
would change.

## Preview

```sh
step-cli step apply-configuration-package --tenant-id <tenant> \
  --file delivery.zip --mode preview \
  --require tenant-configuration --require tenant-settings \
  --require role-permissions --require admin-accounts \
  --require trustees --require election-event --census required
```

`--file` uploads the zip; `--document-id` reuses one uploaded before. Each
`--require` names a section the package must carry, as the package's delivery
profile does; one that is absent is reported missing and the package is
refused. The command waits for the task and prints every section's state:

| State | Meaning |
| --- | --- |
| `previewed` | What would change, nothing changed yet |
| `unchanged` | The tenant already matches |
| `blocked` | Something prevents it: an unknown or reserved role, an existing account outside its group, a trustee registered with other keys, an event realm left by an interrupted import, a census that differs |
| `missing` / `not-included` | Required and absent / neither present nor required |

Nothing is changed by a preview.

## Apply

Run the same command with `--mode apply`. Every section is previewed again
first and a single blocker refuses the run before anything changes. Sections
are then applied in order, each through its own import task, and a section
counts as applied only when the tenant matches the package afterwards.
Existing administrator accounts are never re-created or given credentials;
trustee keys are never replaced; the event is imported once.

If the run stops, run it again with the same file. It continues from what is
already in place: a section whose import is still running is waited for, not
submitted again, and the sections already applied keep the task ids that
applied them. Only one application of a package to a tenant runs at a time.

The outcome is `applied`, `refused` (this run changed nothing), `failed` (it
stopped at a section; earlier sections may be applied, as their states say) or
`unknown` (a section could not be settled; nothing further was submitted).

## How it is wired

The command calls the Hasura action `apply_configuration_package`
(`hasura/metadata/actions.yaml`), handled by harvest's
`POST /apply-configuration-package`, which checks the permissions above on the
caller's tenant and starts windmill's `apply_configuration_package` task. Its
progress and each section's record are the task row's annotations, which the
command reads back.
