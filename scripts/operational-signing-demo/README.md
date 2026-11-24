<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Server-managed signatures: isolated Initialize Voting rehearsal

For **one disposable synthetic installation**, two existing authorized SBEIs
review one pending request for one Post and its exact published ballot. A user
gets **Set up signing** on first use, completes the required fresh identity
check and waits for certificate attachment. Setup is never an approval. They
then review the request again and explicitly **Approve**. The second distinct
officer follows the same path. The operator selects the device; users never
choose providers, upload P12s, handle device secrets or use personal YubiKeys.
The software-token demo is not evidence of physical YubiHSM operation.

## Build and use

* [Install the dedicated API, worker, optional Hasura actions, and fixture realm](HARVEST.md).
  Use only new fixture sockets, queues, object storage, Keycloak, bulletin board and token store.
* [Provision the two-person roster, separate disposable issuer, and current CRL](../signing/disposable_initialize_authority.md).
  Only for a SoftHSM rehearsal the operator may start the bounded
  `serve-preauthorized-pair` before either participant opens the dialog.
* [Run the existing Admin Portal dialog in Chromium](../../packages/admin-portal/test/manual/managed-signing.md).
  Its UI-adapter test is not a substitute for the guarded two-user live harness.
* Before first use, after setup, after the first approval, and after the second
  approval **and worker completion**, run `observe-initialize.sql` with a
  read-only fixture operator session on its local PostgreSQL socket. Pass
  `-v tenant=… -v event=… -v request=… -v post=… -v run=… -v backend=softhsm-test
  -v hash=… -v first=… -v second=… -v expected=N` to `psql -X
  -d operational_demo_14990 -f scripts/operational-signing-demo/observe-initialize.sql`.
  Pin IDs and the reviewed SHA-256 before signing. Expect **0, 0, 1, 2**
  applied receipts, with exactly zero, zero, zero, then one linked
  initialization-report tally. Repeat the last observation after a refresh;
  never initiate a second request to simulate a retry. Keep the fixture idle
  otherwise: a second initialization of the same Post/country is intentionally
  flagged.

Host defaults remain `legacy-p12`. Only explicit operator provisioning of
the named demo database promotes the chosen tenant and makes new tenants
there HSM-required. It does **not** register unrelated events automatically:
each event needs separate operator admission and an identical sealed
installation on its dedicated processes. Entries are not portable between
installations or providers. In HSM mode a missing required rule or an
unavailable signer must refuse; do not make an unsupported workflow appear
to work by switching back to P12.

## Start with a real disposable Post

Do not populate a publication ID or a successful key ceremony by hand.
The normal [backend journey](../e2e/journeys/test_journeys.py) and its
[browser-fixture seed](../e2e/ui/seed.py) show the smallest existing sequence:
bootstrap a new isolated tenant, import the synthetic election and census,
complete the two-trustee DKG and verify its bulletin-board transcript, then
publish ballot styles. Stop there, before voting. These are genuine app
operations, not a replacement SQL seed.

Run those operations against the intended **fresh dedicated installation**
*before* HSM promotion. Record the actual IDs returned by import and
publication. Bind the installation manifest, scope, SBEI roster, Post label
and pending Initialize request to those values. Do not copy a running test
database, use another installation's IDs or assume the ordinary backend
journey installs the signing worker. Completing the provisioning journey is
not evidence that the subsequent two-signer HSM flow ran.

## Lost setup delivery

If the signing device did not claim the request, the same official can
explicitly **Retry setup** after a fresh identity check. This redelivers the
original attempt only; it does not extend its deadline or create another
key. Once the device claims it, refresh the status instead of retrying. If
the device, certificate or original request can no longer be reconciled,
contact the fixture operator. Do not clear ledger rows, generate a
replacement key, or use a P12 to complete the waiting request. For this
disposable rehearsal the safe operator fallback is a new isolated fixture,
not overriding the old one.

Once an independent officer attaches a valid certificate, another eligible
pending request on the same Post can use it without repeating enrollment.
The protected action still needs its own reviewed request, fresh approval
ceremony and configured quorum. A certificate restricted to one Post does
not authorize a different Post. A missing or conflicting installed provider
is a refusal, including for a pre-enrolled official.

The socket-only PostgreSQL checks are
`scripts/postgres/operational_signing_contract/run_initialize_self_setup.sh`
(first use and reconciliation),
`scripts/postgres/demo_signing_installation/test.sh` (provider and defaults),
and `scripts/postgres/demo_signing_installation/test-canonical-reload.sh`
(real request creation, commit and typed reload). None of those checks
substitutes for observing the whole paired-service run above.

## Coverage boundary

| Protected action or delivery path | What the isolated signing flow covers |
| --- | --- |
| Initialize Voting | One Post; optional one country, immutable published-ballot ID. The existing worker checks publication again and creates its normal initialization-report tally. The scope is not every tally creation or every PDF that may follow. |
| Open Voting; Close Voting | Not implemented here. Manual channel changes and scheduled transition/expiry handling in `actions/voting.rs` are separate entry points; the scheduler is not a human signer. |
| Approve Configuration; Approve Voter | Not implemented here. Existing protected ballot publication and per-voter approval have their own payloads and permission checks. |
| Generate Election Returns; Generate Reports | Not implemented here. Tally-triggered, scheduled and on-demand reports require exact PDF bytes, PAdES revisions and separately guarded release/delivery. The signing flow does not cover arbitrary downloads or mail. |
| Transmit Results | Not implemented here. EML artifacts, Post transmission configuration/MIRU admission and their actual release are separate; downloading or signing a payload alone is not transmission approval. |
| Key ceremony; Tally key | Trustee custody proof workflows are separate from these SBEI approvals. Not implemented or claimed. |
| Electoral audit log | Existing signing audit/outbox remains in use. An electoral-log signature is not an SBEI approval and cannot satisfy this quorum. |

Do not enable this profile for a multi-action demonstration until the exact
workflows it needs have working guards, signers and negative tests. Do not
claim browser-to-effect or hardware acceptance from adapter, UI, or SQL tests
run separately.
