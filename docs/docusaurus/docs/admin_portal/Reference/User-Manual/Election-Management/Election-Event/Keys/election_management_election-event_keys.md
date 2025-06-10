---
id: election_management_election_event_keys
title: Keys
---

The Key Ceremony establishes the collective private key used to decrypt votes and publishes a corresponding public key for voters to encrypt their ballots. During this ceremony, trustees collaboratively generate fragments of the private key, ensuring no single party holds the full secret. Each trustee downloads, securely backs up, and verifies their key fragment. Only after all trustees complete these steps can the election proceed to stages such as ballot publication, voting, and tallying. This distributed approach guarantees that votes encrypted under the shared public key remain confidential and tamper-resistant, and that the combined private key—reconstructed only when a threshold of trustees collaborates—preserves integrity and availability of decryption throughout the election lifecycle.


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

4. **Configure Threshold & Trustees**
   - **Input Threshold**: Specify the minimum number of trustees required to reconstruct/decrypt (the threshold).
   - **Select Trustees**: Check the trustees who will participate in this ceremony for the Election Event.

5. **Create Keys Ceremony**
   - Confirm and submit to generate the keys.
   - Approve any confirmation prompt if prompted.

6. **Trustee Actions**
   - Each trustee must:
     - **Download** the generated encrypted private key.
     - **Secure** multiple backups of their key.
     - **Verify** the key with the system (see next section).

> _Note_: In the UI, you may see indicators (icons or status) showing which trustees have completed download/backup/verification. For instance, a green icon indicates completion.

---

## Trustee Cryptographic Key Distribution Process

Each trustee follows this procedure to download, backup, and verify their key. Once all trustees complete these steps, the Key Ceremony is finalized and the Election can proceed.

1. **Log In & Select Election Event**
   - Log in to the Administration Portal.
   - Select the relevant Election Event.

2. **Access Key Ceremony Invitation**
   - Click on the ceremony’s key action invitation, which may be highlighted (e.g., an orange box) or via a key icon.

3. **Follow On-Screen Instructions**
   - Proceed through the guided steps for key distribution.

4. **Download Encrypted Private Key**
   - Download the encrypted private key file provided by the system.

5. **Create Secure Backups**
   - Make **multiple secure backups** of the encrypted private key (e.g., on separate secure storage/media).
   - Confirm backups exist before proceeding.

6. **Confirm Backup Creation**
   - Click **Next** (or equivalent) and confirm that backups have been created.

7. **Verify Key with System**
   - Drag and drop (or upload) the encrypted private key file to the UI to verify it matches the expected key fingerprint.
   - The system will confirm successful verification.

8. **Completion**
   - Once all trustees have downloaded, backed up, and verified their keys, the Key Ceremony is successfully completed.
   - The Election Event status will update to indicate that keys have been distributed and verified.
