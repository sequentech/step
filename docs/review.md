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

The Vitrine-focused Jest suites pass: voting portal 80, admin portal 96, ballot verifier 103. They exercise all four cast-answer kinds, kiosk retention, opt-out during a pending write, file and reference verification, all admin mode/policy states, pre-encryption mismatches and observations, private policy proofs, untrusted text and bounded network bodies. All three complete TypeScript projects pass `tsc --noEmit`. Scoped ESLint checks pass. The voting portal and ballot verifier production webpack builds pass; the admin production build is checked separately during integration.

Commands run from each app directory:

```sh
../node_modules/.bin/jest --runInBand --runTestsByPath <Vitrine test files>
../node_modules/.bin/tsc --noEmit --pretty false
../node_modules/.bin/eslint <changed frontend paths>
../node_modules/.bin/webpack --config webpack.config.cjs --mode production
```
