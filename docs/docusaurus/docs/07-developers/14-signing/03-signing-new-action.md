---
id: signing_new_action
title: Adding a Protected Action
sidebar_label: Adding a protected action
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The catalog of protected actions is product code: each action has its own integration
point. Adding one touches every layer of the [architecture](./01-signing-architecture.md).
Write each step's test first.

1. **The action.** In `packages/sequent-core/src/signing/types.rs`, add a `SigningAction`
   variant (its kebab-case id is stored and sent everywhere) and fill its traits:
   - `sign_permission()`: a new `sign-<id>` constant in `Permissions`;
   - `scope()`: `Post`, `PostAndCountry`, `Event` or `Trustee`;
   - `mode()`: `Deferred` (the action runs when the last signature arrives) or `Gate` (the
     route runs it after checking a completed request, as the trustee steps do);
   - `document()`: `None`, `Pdf` or `Eml`;
   - `group()`: the Protected actions group it is listed under.

   The tests that pin the action ids and permission names fail until the next steps agree.
2. **The permission.** Add `sign-<id>` to `IPermissions`
   (`packages/admin-portal/src/types/keycloak.ts`) and the WASM subset, to the dev tenant
   realm under `.devcontainer/keycloak/import/` (held by the `admin` group), to the janitor
   tenant template, and to `SIGNING_PERMISSION_LABELS` in
   `packages/windmill/src/tasks/migrate_realm_permissions.rs`, so existing realms get it.
   Update the matrix in `packages/sequent-core/src/types/permissions_tests.rs`.
3. **The database.** Add a migration that extends the `signing_rule_action_known` and
   `signing_request_action_known` CHECK constraints. Keep it additive, with a `down.sql`.
4. **Texts.** In the 8 translation files: `permissions.sign-<id>` ("Sign: …") and
   `signing.actions.<id>` (`label`, `short`, `permissionName`, `object`, `appliesTo`,
   `description`), plus `signing.details.<field>` for each subject field the
   panel shows. Never name an organization: tenants override labels with their
   translation overrides. An action that signs a PDF also needs its certification
   sentence under `certify` in every language of
   `packages/windmill/src/services/signing/signature_page_texts.toml`.
5. **The subject.** Define what is signed: every field the signer must see, ids and hashes
   rather than secrets (the trustee steps sign a key share's hash, never the share). Choose
   the `subject_key` so that one waiting request exists per real-world thing (for example
   the application id).
6. **The guard.** In the existing route or service that starts the action, call
   `signing::guard` in the route's transaction before doing the work. On `Proceed`, run as
   today. On `SigningRequired`, return the existing output plus `signing_request`; add the
   optional field to the Hasura action's output type. After the commit, kick the log outbox
   (`kick_signing_log_outbox`).
7. **The executor** (`Deferred`). Implement `SigningExecutor` in
   `packages/windmill/src/services/signing/actions/` and add it to `actions::executors`
   (read by `default_registry()`):
   - if the effect stays in the database, run it in the given transaction and answer
     `Executed { result, .. }`;
   - if it reaches outside (board, S3, Keycloak, e-mail), insert a `tasks_execution` row and
     answer `Dispatched`; the task claims the execution (`claim_dispatched`), runs the effect
     and `finish_dispatched` in one transaction, and implement `redispatch` for the sweeper.

   The effect must be idempotent: a second task copy must change nothing. For a `Gate`
   action, give the route an optional `signing_request_id` and call `consume_gate` in the
   transaction that runs the step.
8. **Cancellation.** When what a waiting request signs changes, cancel it with
   `payload-changed`. A new `guard` call for the same `subject_key` does this; anything that
   changes the subject elsewhere (a regeneration, a recount) must cancel explicitly.
9. **Documents.** For `Pdf`, generate the document with the signature page (one field per
   required signature) and record its base revision; for `Eml`, serve the bytes through the
   EML document signer. Release the document only from the executor.
10. **The portal.** Where the action's route is called, open the panel when the answer has
    `signing_request` (`useSigningRequest().open(id)`, or `{sign: true}` to open the dialog
    at once). Completion actions (for example download or send) go in the panel's
    completion slot. Add stories with the rule `Required` and `NotRequired`, and one with a
    tenant's label override.
11. **Docs.** Add the action to the
    [user guide](../../02-election_managers/02-reference/02-election-event/16-signatures/01-election_management_election-event_signatures.md#protected-actions),
    its notes to
    [Signing a protected action](../../02-election_managers/02-reference/02-election-event/16-signatures/05-election_management_election-event_signatures_signing.md#notes-per-action),
    and its permission to
    [Signature permissions](../../02-election_managers/02-reference/02-election-event/16-signatures/06-election_management_election-event_signatures_permissions.md).

## Tests every action needs

With a private database ([Testing](./04-signing-testing.md)), under two organizations'
configurations:

- `NotRequired`: the route runs exactly as before and creates no request;
- `Required`: the route answers `signing_request` with the expected subject and scope;
- k−1 signatures run nothing; the k-th runs the action exactly once, also when two final
  signatures arrive together, and a second task copy does nothing;
- a change of what is signed cancels the waiting request, and its signatures don't count
  toward a new one;
- a signer without the permission, outside the Post or who already signed is refused and
  the refusal is logged;
- every step leaves its USER and SYSTEM outbox rows;
- a Harvest route test that the route answers `signing_request`, and the route permission
  inventory.
