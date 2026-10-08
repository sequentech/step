---
id: admin_portal_tutorials_key-ceremony
title: Key Ceremony
description: "The Key Ceremony is a vital security procedure that ensures the integrity and secrecy of an election."
---

import GoogleVideo from '@site/src/components/GoogleVideo';

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

<GoogleVideo id="1F7IVr_xcTzLdUQ3K7CkRtoObeVpakzuV" />

The Key Ceremony is a vital security procedure that ensures the integrity and secrecy of an election. It generates a fragmented cryptographic private key, ensuring that no single individual or organization can decrypt votes or manipulate the tally independently.

## Core Concepts: Threshold Cryptography

The security of the Sequent system relies on **Threshold Decryption**. 

* **Fragmentation**: The private key is never generated as a single file. It is split into multiple fragments, each assigned to a different **Trustee**.
* **The Threshold**: This is the minimum number of Trustees required to combine their fragments and decrypt the results. 
* **Security Guarantee**: A threshold (e.g., 2 out of 3) ensures that even if one fragment is lost or stolen, the election remains secure and recoverable—but only if the remaining Trustees cooperate.


## Step 1: Administrator Initiation

The Platform Administrator acts as the coordinator for the ceremony.

1.  Log in with **Administrator** permissions.
2.  Select the **Electoral Event** and navigate to the **Keys** menu.
3.  Click **Create Keys Ceremony**. If the election event already has a key ceremony, click **Add**.
4.  **Configure the Threshold**: Set the minimum number of members needed to tally. Sequent recommends that the threshold be lower than the total number of Trustees (e.g., a threshold of 2 for 3 Trustees).
5.  **Assign Trustees**: In **Trustees**, select the trustees of the ceremony. Select at least as many trustees as the threshold.
6.  In **Election**, select one election, or leave it empty for **All Elections**.
7.  Click **Create Keys Ceremony**, then click **Yes, Create Keys Ceremony**.

![Ceremony Configuration Panel](./assets/keys_config_panel.png)

## Step 2: Trustee Participation

Once the ceremony is created, each assigned Trustee must perform their individual security steps.

1.  **Login**: Each Trustee must log in with their own unique credentials.
2.  **Access Keys**: Navigate to the **Keys** menu of the event. The message "You have been invited to participate in a Keys ceremony." shows. In the row of the ceremony, click the key icon **Participate in Keys Ceremony**.

![Trustee Key Actions](./assets/keys_trustee_actions.png)

3.  **Generate & Download**: Click **Download your Encrypted Private Key** to download the private key fragment.

![Trustee Key Download](./assets/keys_download_key.png)


4.  **Secure Backups**: Trustees are required to confirm they have saved the fragment in at least two different secure locations, typically encrypted USB devices.

![Trustee Secure Backups](./assets/keys_secure_backups.png)

5.  **Integrity Check**: The Trustee must upload the file back into the "Check" box to verify that the download was successful and the file is valid.

![Key Backup Verification](./assets/keys_backup_verification.png)

## Step 3: Monitoring and Success

The Administrator can monitor the **Keys Ceremony Progress** page to track completions. On the **Keys** tab, click the view icon of the ceremony.

![Key Ceremony Monitoring](./assets/keys_ceremony_status.png)


* **Green Checkmarks**: Indicate that a Trustee has successfully generated, downloaded, and verified their fragment.
* **Logs**: The **Logs** table at the bottom of the screen records every step of the ceremony for auditing purposes.

Once all Trustees complete their tasks, the status will change to `SUCCESS`.

The admin portal cannot cancel or restart a key ceremony. For the full procedure, see
[Run the Key Ceremony](../03-procedures/04-keys.md).

:::danger **Irrecoverable Data Warning**
If too many Trustees lose their fragments (dropping the total below the set threshold), the election results **cannot be decrypted by anyone**, including Sequent technical support. Secure storage of these fragments is the most critical responsibility of the Trustees.
:::
