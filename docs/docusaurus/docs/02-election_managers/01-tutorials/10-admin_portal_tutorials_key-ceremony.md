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

See [Wbraid trustee configuration](../../07-developers/09-braid/braid_trustees_configuration.md)
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

1. Choose **Download Custody Request**. The downloaded `custody-request.json`
   contains public identifiers and the trustee's registered public keys.
2. Confirm two secure backups of the local trustee keys file. Preserve the
   runner's protocol state as described above.
3. On the trustee machine, produce a signed proof:

   ```sh
   trustee attest --request custody-request.json --trustee-config trustee.toml --output custody-proof.json
   ```

4. Upload `custody-proof.json` at **Confirm Trustee Custody**. If the tenant
   requires an additional approval signature, complete the signing dialog.

The request and proof may be uploaded to the platform. Keep `trustee.toml`
and the runner's private protocol state on the trustee machine.

A proof is scoped to its tenant, election event, ceremony, and trustee. A proof
from another trustee or ceremony is rejected. Confirming custody changes that
trustee to **KEY_CHECKED**. Once every trustee has confirmed custody and any
required approval signatures are complete, the ceremony becomes **SUCCESS**.
The administrator can then publish ballot styles and open voting.

A trustee may verify its proof again using **Verify Custody Proof**. Completed
custody cannot be downgraded by downloading another request. Retain the public
request for future tally ceremonies. For manual tally authorization, produce
a new proof with `--tally-session-id <tally UUID>`; a key ceremony proof cannot
be replayed as a tally proof.

If generation fails, the ceremony displays **FAILED** and its failure reason.
Resolve the runner or protocol failure before arranging a replacement ceremony.
