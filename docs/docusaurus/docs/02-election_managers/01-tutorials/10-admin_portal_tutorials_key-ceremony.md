---
id: admin_portal_tutorials_key-ceremony
title: Key Ceremony
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

A key ceremony uses the native wbraid distributed key generation protocol to
create an election public key. Each trustee keeps its private identity and
protocol state on its own machine. The platform receives public keys, signed
protocol messages, and public custody proofs.

## Prepare trustees

Each trustee generates its identity with the native trustee tool:

```sh
trustee generate --trustee-config trustee.toml
trustee public-keys --trustee-config trustee.toml
```

Register both the signing public key and share encryption public key in
**Settings → Trustees**. Assign a trustee account whose `trustee` attribute
matches the registered trustee name. Complete that account's required profile
and MFA enrollment, configure the runner, and start it:

```sh
trustee start --trustee-config trustee.toml
```

See [Wbraid trustee configuration](../../07-developers/09-wbraid/trustee-configuration.md)
for runner authentication and service settings. Securely back up `trustee.toml`
and the runner's `message_store` directory. The protocol state contains the
trustee's election shares; preserving just its identity file is insufficient
for recovery.

## Start a manual ceremony

1. Sign in as an administrator and select the election event's **Keys** tab.
2. Choose **Create Key Ceremony**, select the trustees and election scope,
   and set a threshold between two and the number of selected trustees.
3. Leave **Automatic Ceremony** disabled and confirm creation.

The runners exchange and verify native DKG messages. **SHARES_POSTED** means
that a trustee has posted shares and is still generating its key. The custody
steps become available only after the joint public key is verified and all
trustees have generated their keys. A manual ceremony remains **IN_PROGRESS**
until every participating trustee confirms custody, even if its threshold is
lower than the number of trustees.

## Confirm local custody

Each participating trustee signs in to the administration portal, selects the
ceremony's key action, and completes these steps:

1. Choose **Download Ceremony File**. The downloaded `<trustee>-custody-request.json`
   contains public identifiers and the trustee's registered public keys.
2. Save two copies of this public file and confirm that you have saved them.
3. Upload the same file at **Confirm Trustee Custody**. A running server trustee
   creates the signed custody proof automatically. Wait for it to respond.
4. If the tenant requires an additional approval signature, complete the signing
   dialog. The proof alone does not complete that approval.

Keep the ceremony file for future manual tallies. No command or access to the
trustee machine is needed for these portal steps. If the trustee server has not
responded, check that it is running with the correct identity and upload the file
again. Failed proof requests can also be retried by uploading the file again.

The ceremony file contains no private key or recoverable protocol state. The
server operator must separately back up `trustee.toml` and the runner's private
protocol state as described above. Saving the public file does not prove that
these private backups exist.

Offline trustees can still create a proof locally and upload it in the same step:

```sh
trustee attest --request trustee1-custody-request.json --trustee-config trustee.toml --output custody-proof.json
```

A proof is scoped to its tenant, election event, ceremony, and trustee. A proof
from another trustee or ceremony is rejected. Confirming custody changes that
trustee to **CUSTODY_CONFIRMED**. Once every trustee has confirmed custody and any
required approval signatures are complete, the ceremony becomes **SUCCESS**.
The administrator can then publish ballot styles and open voting.

A trustee whose custody is confirmed sees **Custody confirmed** and a message
that no further action is required from them. If other trustees are still pending,
the ceremony also explains that it is waiting for their custody confirmation and
any required approval. **More actions → Reconfirm custody (optional)** opens the
confirmation step again; using it is not required to complete the ceremony.
Until custody and any required approval are recorded, the wizard continues to
show **Custody confirmation required** and blocks advancing past that step.
Completed custody cannot be downgraded by downloading another request. Retain the public
file for future tally ceremonies. Upload that same file to authorize a manual
tally; the server creates a new proof bound to that tally session. Offline
trustees instead create a new proof with `--tally-session-id <tally UUID>`. A key
ceremony proof cannot be replayed as a tally proof.

If generation fails, the ceremony displays **FAILED** and its failure reason.
Resolve the runner or protocol failure before arranging a replacement ceremony.


Browser trustees can generate or import their identity and participate from the
portal's `/trustee` screen. Start the browser trustee and keep its page open.
In another tab, upload the saved public ceremony file to the ceremony wizard;
the browser generates the proof automatically. For a manual proof file, stop the
runner and choose **Advanced → Generate proof file manually** with the saved
public request, then upload the result to the ceremony wizard.
Keep private backups of both the keys and browser
sessions. See [browser trustee setup](../../07-developers/09-wbraid/trustee-configuration.md#browser-trustees)
for key registration, browser storage recovery, and tally restore instructions.
