<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Vitrine integration review

## Frontend interface decisions

The three apps use the verifier and UI packed from Vitrine commit `723316eb09992811da5c891fe9b04d44683bd713`. The archive-based vendor script runs upstream builds and tests before packing and has no failed-test bypass. Repacking the installed verifier and UI produces byte-identical tarballs; their SHA-256 digests are recorded in `packages/vendor-npm/SHA256SUMS`.

New policy admission uses the SDK's `pinPolicy`, including authenticated profile meanings and election minimums. Structural parsing remains available for unsigned Board drafts. Election fixtures use `vitrine/election/ballots/v1` and `vitrine/election/events/v1` with their published meaning hashes, two app hubs, an independent hub subset and two monitor parties. Private fixtures name separate blinded subjects for both logs.

Private successor discovery adds `route: "policies"` to `vitrine_voter_forward`. Its `body` is the empty string and `hub` is omitted. Harvest authenticates the voter and event for this read; only `upgrade` and `opening` require ownership of the receipt or promise in their body. The returned `vitrine/policy-proofs/v1` document is verified before policy history changes. Missing or forged proofs leave discovery pending. Upgrade blocks may name another policy only after that policy is accepted in this history; their log and entry expectation must still match.

Config observations follow the current `problem` line and receipt/key/styles/style block format. They contain configuration material, never the voter's encrypted ballot, own ballot ID or submission receipt. Public context failures can be reported using a separately admitted public policy; an unpinned private context supplies no report destinations. All monitor requests omit credentials and use independent random delays and bounded retries.

Receipt requests stop reading streamed bodies above 4 MiB. Retention requires an authenticated account and a live, non-kiosk opt-in. An opt-out or unmount during an IndexedDB write is checked again before retention completes. The chooser binds its displayed receipt list to the current account, so a changed or logged-out account does not briefly see the previous account's list. Downloads, printing and QR generation remain local actions.

JSONb statement contexts accept canonical chain strings and legacy objects carrying a `text` or `chain` string. Other object forms are refused as missing configuration; no JavaScript or HTML from server text or uploaded files is executed. Hand-written frontend action types remain next to their operations until the coordinator regenerates the combined schema.

## Validation

The Vitrine-focused Jest suites pass: voting portal 80, admin portal 96, ballot verifier 103. They exercise all four cast-answer kinds, kiosk retention, opt-out during a pending write, file and reference verification, all admin mode/policy states, pre-encryption mismatches and observations, private policy proofs, untrusted text and bounded network bodies. All three complete TypeScript projects pass `tsc --noEmit`. Scoped ESLint checks pass. All three production webpack builds pass, with existing asset-size warnings only.

Commands run from each app directory:

```sh
../node_modules/.bin/jest --runInBand --runTestsByPath <Vitrine test files>
../node_modules/.bin/tsc --noEmit --pretty false
../node_modules/.bin/eslint <changed frontend paths>
../node_modules/.bin/webpack --config webpack.config.cjs --mode production
```


## Backend admission and private infrastructure

Step generates and accepts only its reviewed `vitrine/election/ballots/v1` and `vitrine/election/events/v1` bindings. Upload requires `required on` before either log is modified. The sidecar remains responsible for complete policy parsing, authority signatures, immutable profile metadata and infrastructure validation. The deployment parser additionally enforces a subset of two pairwise independent app hubs, one independent of the operator, two distinct monitor parties and monitor quorum at least two, including private events. Optional weaker settings cannot remove these floors.

Private policy drafting provisions one blinded-anchor key for each log using authenticated, idempotent `POST /admin/logs/<name>/blinded-key` with JSON `{ "pseudo": "<64 lowercase hex>" }`. The pseudo-origin is SHA-256 of `vitrine/origin/v1`, the origin bytes, and the 32 raw salt bytes. The response key must be Ed25519 named by that pseudo-origin. Secret keys and durable mappings belong to the sidecar; Step stores no additional secrets. Missing, reused, wrongly typed or salt-mismatched public keys refuse drafting. Existing signed policies supply their public blinded keys for a refreshed draft without another provisioning request; an offline sidecar leaves an unprovisioned draft unavailable while the administrator can still read the event state. Both private logs retain hubs and monitor requirements.

Policy IDs are stored and emitted as `sha256:` plus lowercase hex. Bare sidecar/legacy IDs are normalized at the client and database read boundaries. The install response must equal SHA-256 of the signed note text, excluding signatures; equivalent formatting cannot split two logs into different pins and a mismatched response is refused. Statement-context JSONb retains string-valued receipt chains. Frontends accept the explicitly documented legacy wrappers while rejecting arbitrary objects.

The opt-in Step compose profile supplies only the sidecar and two log witnesses. Its strong example deployment requires separately running the declared hubs, hub witnesses and monitors, with their actual verifier keys and URLs, or an explicitly supplied `HARVEST_VITRINE_CONFIG_FILE`. The development documentation states this prerequisite, including authorization of the sidecar enrolment signer by hubs and explicit event-policy subscriptions for monitors; an answering monitor listener alone does not establish monitoring. No community keys or production independence are assumed. A full Step UI deployment has not been run in this recovery.


Backend verification commands (from the repository root, with `CARGO_TARGET_DIR` pointing to the Step build cache):

```sh
cargo test --offline --manifest-path packages/Cargo.toml -p windmill --lib services::vitrine
cargo test --offline --manifest-path packages/Cargo.toml -p electoral-log --lib vitrine
cargo test --offline --manifest-path packages/Cargo.toml -p electoral-log --test vitrine_vectors
cargo test --offline --manifest-path packages/Cargo.toml -p electoral-log --test ballot_box -- --ignored
cargo test --offline --manifest-path packages/Cargo.toml -p electoral-log --test vitrine_grants -- --ignored
cargo test --offline --manifest-path packages/Cargo.toml -p windmill --test vitrine_sidecar -- --ignored
```

The first five pass: 61, 20, 4, 14 and 1 tests respectively. The PostgreSQL runs use a newly created, task-owned `vitrine-step-backend-audit` container, isolated from existing application databases. The ballot tests cover acceptance and refusal, revotes, retries, a queue insert failure, byte-for-byte queue entries and application-role permissions. The grants test creates and removes its own event database and role. Shared election vectors are copied from the current Vitrine profile packages under their new namespaces. Existing unrelated Rust warnings remain visible.

The external sidecar test additionally accepts `WINDMILL_VITRINE_TEST_CONFIG_FILE` for an explicit deployment using the running services' verifier keys and URLs. Its authorities remain Step's development Board/operator keys. The public scenario exercises policy signatures and installation, witness enrolment, an actual queued ballot, receipt retrieval, voter ownership checks and config/opened statement receipts. The private scenario checks two separate blinded keys, idempotent provisioning, both policy installs and the authenticated policy-history endpoint. These scenarios do not establish full UI deployment or independent monitor execution.
