<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Initialize Voting: managed signing browser acceptance

Run from `packages/admin-portal` with the normal locked workspace dependencies
and Playwright Chromium installed. Set `CHROMIUM_EXECUTABLE_PATH` to use an
existing Chromium binary. Nothing here starts or connects to a hosted service.

## Synthetic adapter, real dialog

```sh
node test/manual/managed-signing-fixture.cjs
```

This bundles the actual dialog, canonical review verifier and API response
parsers with the existing workspace esbuild dependency. It serves the bundle
on an ephemeral HTTPS loopback port in a disposable Chromium context. It
checks the private software label, no upload fallback, fresh Gold before
setup, separate review/approval with the exact Post and digest, changed-Post
refusal, and pending status with no duplicate approval after a failed start.
It does not run the full Portal shell, real Keycloak, database, or HSM.

## Live opt-in, disposable SoftHSM only

Supply the real isolated demo stack already started by its operator and provision
the synthetic accounts, trust, roles, exact pending Initialize request, and two
required approvals. Use separate account contexts and a genuine synthetic Gold
password + TOTP flow; the existing Portal public client must allow the
`/silent-check-sso.html` callback and the dedicated broker client must allow
`https://127.0.0.1:13443/operational-signing/return` with PAR. The Portal and
Hasura must share `https://127.0.0.1:13443`, with Hasura at `/v1/graphql`;
Keycloak must be at `https://127.0.0.1:18443`. No other network origin is allowed
by the browser fixture.

Provide a private local JSON file, outside the repo, shaped as follows. Test
credentials are disposable and must never be copied from a real account.

```json
{
  "synthetic": true,
  "provider": "softhsm-test",
  "portalUrl": "https://127.0.0.1:13443/",
  "keycloakUrl": "https://127.0.0.1:18443/",
  "requestPageUrl": "https://127.0.0.1:13443/sequent_backend_election_event/<event-uuid>/show",
  "tenantId": "<tenant-uuid>",
  "eventId": "<event-uuid>",
  "electionId": "<post-uuid>",
  "requestId": "<initialize-request-uuid>",
  "payloadSha256": "<operator-pinned lowercase SHA-256 of the canonical Initialize request>",
  "requestRow": "<unique request code displayed in the Requests table>",
  "officers": [
    {"id": "<first-subject-uuid>", "username": "hsm-fixture-first", "password": "<test password>", "totpSeed": "<test base32 seed>", "needsSetup": true},
    {"id": "<second-subject-uuid>", "username": "hsm-fixture-second", "password": "<test password>", "totpSeed": "<test base32 seed>", "needsSetup": true}
  ]
}
```

Preauthorize both officials under a separate disposable officer as described in
[`scripts/signing/disposable_initialize_authority.md`](../../../../scripts/signing/disposable_initialize_authority.md).
When the browser run prints that setup is pending, the fixture operator can use the preauthorized two-officer SoftHSM authority (see `scripts/signing/disposable_initialize_authority.md`), or else must
issue and attach the scoped test certificate in a separate terminal using
`issue-and-attach OFFICER OFFICIAL`. An already enrolled second officer can
use `"needsSetup": false`; at least one real first-use setup is required. The
harness will not load an authority private key, upload a certificate, or
grant itself access. No screenshots, traces, or credentials are persisted.

```sh
STEP_CONFIRM_SYNTHETIC_SIGNING=SOFTHSM_LOOPBACK_ONLY \
STEP_SYNTHETIC_SIGNING_FIXTURE=/absolute/private/synthetic-fixture.json \
node test/manual/managed-signing-live.cjs
```

Browser-only success requires setup not to submit an approval, the first
officer's receipt to leave the exact request waiting at 1 of 2, and the second
distinct officer's receipt to settle the same request at 2 of 2. Two reloads
must leave the completed request with no Sign button and no new signing start.
Each Gold prompt uses a fresh TOTP period for that officer, and every observed
quorum response must match the operator-pinned request digest.

For joined acceptance, set `STEP_SIGNING_WITNESS_DIR` to an empty, absolute,
owner-only 0700 directory with no symlink. This lets a separate read-only
observer verify each stage **before** the browser advances. Browser-only
success does not claim an actual device or worker effect.

1. In that directory, the browser writes `<stage>.json` atomically, containing
   only `name`, `expected` and a fresh challenge `nonce`.
2. Run the pinned
   [database observer](../../../../scripts/operational-signing-demo/observe-initialize.sql)
   against the same disposable Unix-socket database using the separately
   recorded IDs, run, software profile and request digest. Use the event's
   `expected` receipt count. Require exit 0 and retain the nonsecret count
   line; never acknowledge an observer failure.
3. Only then atomically write `<stage>.verified`, mode 0600, as an object
   with the same `name` and `nonce` and five integers from the SQL output:
   `approvals`, `distinct_officials`, `applied_device_receipts`, `post_effects`
   and `linked_effects`. A nonce echo alone is not accepted. The stages are `before` (0), `first-setup` (0, if needed),
   `first-approved` (1), `second-setup` (1, if needed), `second-approved` (2)
   and `after-refresh` (2).

Use the independently provisioned fixture operator for this gate, not any
browser credentials. Poll for each event and do not substitute the browser's
acknowledgement for the retained SQL output. The observer defaults to 60
seconds per stage; `STEP_SIGNING_WITNESS_TIMEOUT_SECONDS` accepts 5 to 180.
If a fresh rehearsal reuses a disposable officer's seed after an abort, wait
at least one TOTP period and require the separate `before` zero check again.
Never run the live harness against a receiver or YubiHSM.
