<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Vitrine integration review

## Frontend interface decisions

The three apps use the verifier and UI packed from Vitrine commit `5389ff35832380663768fe99fcfaf025dcd63ca8` (`packages/vendor-npm/VITRINE_COMMIT`). The archive-based vendor script runs upstream builds and tests before packing and has no failed-test bypass. Repacking the installed verifier and UI produces byte-identical tarballs; their SHA-256 digests are recorded in `packages/vendor-npm/SHA256SUMS`.

New policy admission uses the SDK's `pinPolicy`, including authenticated profile meanings and election minimums. Structural parsing remains available for unsigned Board drafts. Election fixtures use `vitrine/election/ballots/v1` and `vitrine/election/events/v1` with their published meaning hashes, two app hubs, an independent hub subset and two monitor parties. Private fixtures name separate blinded subjects for both logs.

Private successor discovery adds `route: "policies"` to `vitrine_voter_forward`. Its `body` is the empty string and `hub` is omitted. Harvest authenticates the voter and event for this read; only `upgrade` and `opening` require ownership of the receipt or promise in their body. The returned `vitrine/policy-proofs/v1` document supplies supplemental inclusion only. Under decision 50, selective private frames cannot establish absence, globally earliest quorum times, freshness or automatic successor adoption. Even authentic frames leave these claims pending and preserve the independently accepted history; a full private events prefix is never sent to voters. Upgrade blocks may name another policy only after that policy is independently accepted in this history; their log and entry expectation must still match.

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


## Final package and live acceptance checkpoint

The initial browser acceptance checkpoint used immutable Vitrine revision
`74f7bec3ef6a5d08791783d96d76bcce6b22adf8`; the final refresh below supersedes
its source provenance. The refresh exports normative
specs with the packages and vectors, so the UI's evidence-wording coverage
tests run instead of being skipped. Upstream SDK tests pass 489 cases with
two deliberately separate service-only areas skipped; all 172 UI tests pass.
Repacking the installed packages produces byte-identical artifacts matching
`packages/vendor-npm/SHA256SUMS`. The SDK accepts an authenticated canonical
empty checkpoint as a prefix only with an empty consistency proof, preventing
a false failure when online checking compares a fresh log's empty checkpoint
with its first ballot. The low-level proof API is unchanged.

All complete frontend Jest suites pass: voting 342, admin 419 and ballot
verifier 133, for 894 tests. All three complete TypeScript projects and scoped
ESLint pass. All three production webpack builds pass with existing asset-size
warnings. Shared UI production bundles and declarations were built from this
checkout. The optional whole ui-essentials declaration command includes
Storybook files with eight existing TS2742 inferred-type errors; excluding
those non-production stories produces the declarations used by the three
successful app type checks. No shared UI source was changed.

Both Step/native integration scenarios pass using fresh, task-owned Vitrine
services and PostgreSQL, actual randomly generated service keys and explicitly
simulated parties. The public test retains the original issued receipt from
`/wait` and separately fetches a witnessed record from the public receipt
route: an immediate issued receipt need not contain the quorum yet. The
exported policy, acknowledgement, entries and chains are the exact synthetic
queue run's bytes; no verifier result is mocked. The private scenario checks
both blinded keys, idempotent provisioning, both policy installs, enrolment
and authenticated selective policy proofs.

Chromium loads the production-built independent verifier receipt route,
checks a pasted chain at 1280 pixels and an uploaded receipt file at 390,
compares the offered policy ID before pinning, and verifies all four proof
tiers. It rejects another ballot ID and a modified signed policy. All six
screenshots pass full-page overflow checks; the two receipt results have zero
axe violations. All 40 public proof requests omit cookies, Authorization and
referrers; no unexpected request origin or page error occurs. Portable checked
chains, screenshot results and optional real browser video are produced by
`scripts/vitrine-browser.mjs`. The owned fixture and browser processes shut
down, and their container is removed.

This is a real queue/sidecar and production verifier contract rehearsal with
synthetic accepted ballots. It does not claim an authenticated Step,
Hasura/Harvest and Keycloak deployment or a complete encrypted voting ceremony,
and monitor listeners alone do not establish subscription to the new events.
The developer guide records the exact reproducible native/browser commands.

## Final WAICT conformance refresh

The final immutable package source is
`93a346021b23c6dc4a0e437ea96eb5bb4d5edd24`. The upstream native verifier had
rejected an otherwise valid WAICT prefix root when a relay appended a bad
known-witness signature line. The independent browser SDK already followed
the required normalization rule. Native Rust and compiled WASM now ignore
non-verifying lines and still require the configured number of distinct
valid witnesses. Strict key admission, profile minimums, signed text and
inclusion proofs remain required.

The archive vendor script rebuilt and tested the packages: 638 SDK cases pass
with two service-only skips, and all 172 UI cases pass. Both resulting
tarballs, their recorded digests, and the lockfile are byte-identical to the
earlier package artifacts. The three successful production builds above
therefore still contain the exact current package bytes; rebuilding them
would not exercise changed runtime code. All three complete app Jest suites
were rerun after installation (342 voting, 419 admin, 133 verifier), and all
three complete TypeScript checks pass.

`scripts/vitrine-conformance-browser.mjs` reads ten portable full receipt
chains from the exact recorded source revision and checks the installed Step
SDK in Node, then the installed SDK and compiled native counterpart in actual
Chromium. All ten cases agree: bad known-key lines before or after a valid
quorum, short/long payloads and mismatched timestamps cannot invalidate a
quorum; bad-only and duplicate-only sets cannot supply a missing distinct
witness. The browser loads only its local bundle and one WASM module, sends
no service or third-party request, and has no page errors. The temporary
listener/browser and owned `vitrine-step-waict-browser` container shut down.
This is deterministic verifier conformance, separate from the retained live
queue/sidecar rehearsal above.

## UI review against the design boards

Vitrine's UI review (`docs/ui-review.md` there) compared each screen with its
design board. On Step's side:

- The confirmation page follows the Main board: the portal's stepper, a
  headline that changes once witnesses confirm the ballot, the kit's receipt
  card naming ballots in the eight languages, Save receipt and Finish, and the
  QR code behind a row. It keeps asking the log for the hub and browser
  anchors, every 2 s at first and at most every 30 s, until those tiers are
  done or due, so the last receipt steps fill in while the voter watches.
- Kept receipts show one card per receipt with a row per tier (Revisit), and
  the failure page builds its evidence card from the tiers that failed
  (Alert).
- The Transparency tab derives its overview from the policy and the logs, laid
  out as the Admin board in the admin theme, with its label in eight
  languages.
- The voting portal's build gives each script SHA-256 integrity, so the
  extension can check the page's code against the logged release.
- `scripts/vitrine-preview` serves both portals against a local Vitrine stack,
  with a switch for the Alert scenario.

The kit from `5389ff35832380663768fe99fcfaf025dcd63ca8` says on the receipt
when a bootstrap hub stands in for the browser layer.
