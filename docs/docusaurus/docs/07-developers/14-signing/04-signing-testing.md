---
id: signing_testing
title: Testing Signing and Known Gaps
sidebar_label: Testing and known gaps
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Test suites

Run Cargo from `packages/` with `CARGO_TARGET_DIR` set to the checkout's
`packages/rust-local-target`, or use `scripts/dev/step-dev test <file>`
([fast feedback](../03-development-environment/fast-feedback.md)).

| Layer | Command | Covers |
|---|---|---|
| sequent-core | `cargo test -p sequent-core --features default_features --lib signing` | Enums and rule validation, canonical JSON and the signing code against fixed byte vectors computed outside Rust. |
| Windmill, no database | `cargo test -p windmill --test signing_certificates`, `--test signing_pades`, `--test signing_configuration` | Every certificate check against a PKI generated in the test (root, intermediate, RSA and EC signers, expired, not yet valid, no key usage, revoked with a list, foreign root); PDF signature pages, revisions, CMS verification and their refusals; import mapping. |
| Windmill, PostgreSQL | `cargo test -p windmill --test postgres_signing_<name>` for `schema`, `rules`, `requests`, `approve`, `approve_pki`, `certificates`, `log_outbox`, `log_outbox_unreadable`, `pdf`, `actions`, `results`, `key_shares`, `permissions`, `configuration` | Constraints, quorum and once-only execution, concurrency, cancellation, expiry, the claim and the sweeper, registration and revocation, the outbox worker against a board, each action's integration. Most run under two organizations' configurations. |
| Harvest | `cargo test -p harvest -- signing route_permissions request_boundaries` | Every route's permission (403), the minimum permissions per route, error bodies, the routes answering `signing_request`, the Hasura select permissions of the signing tables. |
| Admin Portal | `yarn --cwd packages/admin-portal test src/lib/signing src/components/signing src/resources/ElectionEvent/Signatures` | Opening every `.p12` variant, signing, CMS, the API client, the panel and dialog states, settings validation and permissions. |
| Admin Portal stories | `yarn --cwd packages/admin-portal test:stories src/components/signing src/resources/ElectionEvent/Signatures` | The panel, dialog and Signatures tab with the rule `Required` and `NotRequired`, read-only roles and a tenant's label overrides. |
| Scripts | `python3 -m unittest scripts.dev.test_organizations scripts.dev.test_janitor_signing_preset` | The organization fixtures and the sample tenant template's preset. |

### A private PostgreSQL

The PostgreSQL tests create a private `windmill_schema_*` database with every migration on
the server that `HASURA_DB__*` names. Point them at a throwaway server, never the dev
database or another checkout's:

```sh
docker run -d --rm --name signing-test-pg --network container:devcontainer \
  -e POSTGRES_PASSWORD=postgres -e PGPORT=55440 postgres:16-bookworm
devenv shell -- bash -c 'cd packages && export CARGO_TARGET_DIR="$PWD/rust-local-target" \
  HASURA_DB__HOST=127.0.0.1 HASURA_DB__PORT=55440 HASURA_DB__USER=postgres \
  HASURA_DB__PASSWORD=postgres HASURA_DB__DBNAME=postgres && \
  for t in schema rules requests approve approve_pki certificates log_outbox; do
    cargo test -p windmill --test postgres_signing_$t || exit 1
  done'
docker stop signing-test-pg
```

Use a port of your own when several checkouts run tests at once.

## Fixtures

- **Certificate files.** `scripts/signing/make-test-p12.sh [OUT_DIR]` builds a synthetic
  test PKI, a foreign chain and `.p12` files in every encryption the browser module must
  open: PBES2/AES-256 with a SHA-256 MAC, 3DES, RC2-40 with a SHA-1 MAC (OpenSSL 3 with
  the legacy provider), RSA and EC P-256 keys, non-ASCII passwords, an expired, a not yet
  valid, a revoked, a no-key-usage and a foreign certificate, and a file without a key. The
  default password is `Demo-2028`. Its output is committed under
  `packages/admin-portal/src/lib/signing/__fixtures__/` with `fixtures.json`, the oracle
  computed by the `openssl` command line (fingerprints, key hashes, names, validity, key
  usage). Every run makes new keys, so regenerate the whole directory, never one file.
- **Organizations.** `scripts/dev/scenario/signing-organizations/` holds two organizations
  as data: one with Posts of three board members each, trustees and the settings roles of
  the sample preset, and a student council with faculties as Posts, its own groups, rules,
  translation overrides and a test-only issuer (its key is kept for tests; never trust it
  elsewhere). [Fast feedback](../03-development-environment/fast-feedback.md#signing-organizations)
  shows how to write their files and load them on a running stack.
- **PDF validation.** Besides the tests, check signed PDFs with `pdfsig` (poppler),
  pyHanko and `qpdf --check`. Adobe Reader is a manual check.

## Manual check on a running stack

1. Load an organization's files (see above) and import its issuer in **Signatures** >
   **Certificates**.
2. Issue staff certificates under the issuer, or use the `__fixtures__` files after
   importing `test-pnpki-chain.pem` as a trusted issuer.
3. Switch a rule on, start the action as one signer and sign; use **Next member signs in**
   for the next signer.
4. Check the request in **Signatures** > **Requests** and the USER and SYSTEM entries in
   **Logs**.

## Known gaps

- **Activity logs** are generated as a zip of PDFs, which isn't signed with PAdES, so
  Generate other election reports doesn't cover them.
- **Election returns from the tally pipeline** (Velvet's report generation after a tally)
  don't pass through the report hook, so they aren't held for signatures; only those
  generated from Reports are.
- **Event-level start and stop, and scheduled events,** open and close voting without the
  Post rule. Whether a Post's signed close and the common close interact is an open
  decision with the organization; until it is taken they aren't gated.
- **Group membership changes** (adding a user to a group that holds a `sign-<action>`
  permission, or removing them) are not written to the election event's log; changes of a
  role's permissions are.
- **The admin's public key** reaches the electoral log only through the first event board
  where `ElectoralLog::for_admin_user` creates the key; USER entries in other events are
  signed with it but don't publish it again.
- **Browsers and real files.** Opening certificate files is tested in Chromium. Safari,
  and real files from national or commercial PKIs (their algorithms, key sizes, key usage
  and policies), are untested. A smart-card token would need a local signing program.
- **Open-failure reasons** are `wrong-password`, `unreadable` and `no-key`; other file
  errors (unsupported encryption or key) are logged as `unreadable`.
- **The configuration package's signature** by the organization's HSM or KMS key after a
  configuration version is approved is a hook for its owner; signing approves and
  publishes the version.
