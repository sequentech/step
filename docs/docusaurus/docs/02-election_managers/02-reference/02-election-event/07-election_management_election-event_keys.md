---
id: election_management_election_event_keys
title: Keys
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


The key ceremony runs wbraid distributed key generation and publishes the joint
public key used to encrypt ballots. Each trustee's native runner or browser runner
generates and retains its own election shares locally. Trustees later publish
verifiable partial decryptions; the platform does not reconstruct the private key.

Before creating a ceremony, register each trustee's signing public key and share
encryption public key in **Settings → Trustees**. Start the registered trustees'
runners and preserve their local identities and protocol state. See the
[key ceremony tutorial](../../01-tutorials/10-admin_portal_tutorials_key-ceremony.md)
and [native and browser trustee setup](../../../07-developers/09-braid/braid_trustees_configuration.md).


## Opening Key Distribution Ceremony

During the cryptographic key ceremony for an Election Event, trustees gather to generate essential cryptographic keys used to secure the voting process. This ceremony typically involves:

- **Key Generation**: Creating new keys according to the system’s security requirements.
- **Backup Procedures**: Backing up each key to ensure redundancy and prevent loss.
- **Verification Steps**: Performing cryptographic tests and audits to validate key integrity and functionality.
- **Secure Storage**: Storing keys safely so they are ready for deployment during election events to encrypt and decrypt sensitive voting data.
- **Confidentiality & Authenticity**: Ensuring that all keys remain confidential and unaltered, maintaining trust in the election process.

---

## Creating Cryptographic Keys

To initiate the Key Ceremony in the Sequent Online Voting System (OVS):

1. **Select Election Event**
   - In the Administration Portal, choose the relevant Election Event.

2. **Navigate to Keys Tab**
   - Go to the “Keys” section for that Election Event.

3. **Start Key Ceremony**
   - Click **Create Key Ceremony** (or equivalent button/screen).
  
4. **Automatic Key Ceremony (Optional)**
   - Only available for events with an 'Allow Automatic Ceremonies' policy.
   - Enabling this setting skips manual custody confirmations. The selected trustees'
     native or browser runners must still participate in distributed key generation.

5. **Configure Threshold & Trustees**
   - **Input Threshold**: Specify the minimum number of trustees required to tally,
     between two and the number of selected trustees. A ceremony supports up to eight trustees.
   - **Select Trustees**: Check the trustees who will participate in this ceremony for the Election Event.

6. **Create Keys Ceremony**
   - Confirm and submit to generate the keys.
   - Approve any confirmation prompt if prompted.

7. **Trustee Actions**
   - Each trustee must:
     - **Download** the public custody request after distributed key generation completes.
     - **Secure** multiple backups of their local identity and protocol state.
     - **Confirm custody** with a locally generated signed proof (see next section).

> _Note_: In the UI, you may see indicators (icons or status) showing which trustees have completed download/backup/verification. For instance, a green icon indicates completion.

---

## Confirm Trustee Custody

For a manual ceremony, every participating trustee confirms local custody. The
threshold controls tally participation; it does not reduce the number of custody
confirmations required to complete key generation.

1. **Log In & Select Election Event**
   - Log in to the Administration Portal.
   - Select the relevant Election Event.

2. **Access Key Ceremony Invitation**
   - Click on the ceremony’s key action invitation, which may be highlighted (e.g., an orange box) or via a key icon.

3. **Follow On-Screen Instructions**
   - Wait until the runners have verified the joint public key, then proceed through
     the custody confirmation steps.

4. **Download Custody Request**
   - Download `custody-request.json`. This public request identifies the ceremony
     and trustee and contains the registered public keys.

5. **Create Secure Backups**
   - Back up the local `trustee.toml` identity and the native runner's `message_store`
     directory. Browser trustees back up their keys and export their browser sessions.
   - Confirm backups exist before proceeding.

6. **Confirm Backup Creation**
   - Click **Next** (or equivalent) and confirm that backups have been created.

7. **Confirm Custody with the System**
   - Generate a proof on the trustee machine:

     ```sh
     trustee attest --request custody-request.json --trustee-config trustee.toml --output custody-proof.json
     ```

   - Browser trustees use **Create custody proof** on the `/trustee` screen.
   - Upload `custody-proof.json` in the ceremony wizard. Complete any required
     approval signature. Private keys and protocol state remain on the trustee machine.

8. **Completion**
   - Once all trustees confirm custody and any required approval signatures are
     complete, the key ceremony becomes **SUCCESS**.
   - Retain the public custody request for tally authorization.
   - A participating trustee can reopen the ceremony and select **Verify Custody
     Proof** to check the proof again. This does not generate or download private keys.
