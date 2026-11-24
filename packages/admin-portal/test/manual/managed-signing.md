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

Pass requires setup not to submit an approval, first officer's receipt to leave
the exact request waiting at 1 of 2, second distinct officer's receipt to reach
2 of 2 and the request to settle in the existing executed state. If the service
was not run with the supplied fixture, retain synthetic browser coverage only.
Check the fixture's signer/database audit separately for device-level
provenance and an independently counted execution effect. Never run the live
harness against a receiver or YubiHSM.
