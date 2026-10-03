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

## Repeated log entries

A person's repeated step of the same statement kind on the same request within
`LOG_THROTTLE_SECONDS` (10 seconds) is answered but is not logged again. This applies
to refused steps (`SigningSignatureRefused`, one kind across refused sign, cancel,
handover and open-failure steps), handovers and certificate files that did not open.
The request's open-failure count still records every file-opening failure. No counter
records how many log entries were suppressed. Signatures, completion and execution
remain recorded with their USER and SYSTEM entries.

## Known gaps

- **Unsupported report formats and scopes.** Activity logs are a zip, per-voter manual
  verification reports have no signing integration, and event/contest-level election
  returns PDFs have no Post signing scope. When their report rule requires signatures,
  these outputs are withheld; they are never released unsigned. The tally holds the
  supported election returns per Post and country and Initialization Reports per Post.
- **Group membership changes** (adding a user to a group that holds a `sign-<action>`
  permission, or removing them) are not written to the election event's log; changes of
  a role's permissions are.
- **Requester-only access.** An account holding only an action's start permission can
  cancel its own request but has no general Hasura request list or handover role. Report
  request references also permit a report reader to discover their own report requests.
- **Trustees.** Their ceremony provides their request; the event-wide signer list omits
  trustee actions because Hasura's session does not carry the trustee identity.
- **The admin's public key** reaches the electoral log only through the first event board
  where `ElectoralLog::for_admin_user` creates the key; USER entries in other events are
  signed with it but do not publish it again.
- **Browsers, certificates and external validation.** Chromium tests use synthetic
  certificates. Safari, real national or commercial PKI files, external CCS acceptance,
  Adobe Reader validation and the offline demonstration profile still require validation.
  A smart-card token would need a local signing program. Browser tests of the legacy RC2
  and 3DES files and a standard PDF validator against product PDFs remain outstanding.
- **Open-failure reasons** are `wrong-password`, `unreadable` and `no-key`; other file
  errors (unsupported encryption or key) are logged as `unreadable`.
- **Owned integrations and policy decisions.** The configuration package's HSM/KMS
  signature is a hook for its owner. Close voting feeds closing signatures to the
  `SealRecordSink`; `NoSeal` produces no seal. The relation between a Post close, common
  close, Pause and direct Hasura changes remains an owning-feature policy decision.
- **Role composition in presets.** A rule requiring two configuration approvals counts
  two distinct people holding the permission; it does not enforce one person from each
  of two named groups. Preset group assignments and certificate/title mappings need
  confirmation with each organization. Permission changes taking effect at the next
  sign-in are checked manually.
