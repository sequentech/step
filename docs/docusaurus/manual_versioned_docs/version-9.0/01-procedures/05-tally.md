---
title: "05-TALLY: Run the tally ceremony"
sidebar_label: "05-TALLY: Run the tally ceremony"
sidebar_position: 5
slug: /procedures/tally
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# 05-TALLY: Run the tally ceremony

The tally ceremony decrypts and counts the votes. The election administrator creates the tally.
The trustees upload their key fragments. When enough trustees have uploaded their fragment, the
election administrator starts the tally.

## Before you start

- The key ceremony status is `SUCCESS`. See [03-KEYS](03-keys.md).
- The election event has a publication. See [04-PUBLISH](04-publish.md).
- The voting period is closed, unless you make an initialization report.
- **Allow Tally** of each election permits the tally. See [02-EVENT](02-event.md#3-create-the-elections).
- A number of trustees equal to or more than the threshold are available, with one copy of
  their key fragment.
- You have agreed a date and a time with the trustees.

:::danger WARNING
Start the tally of the election results only after the official end of the voting period. If
**Allow Tally** is **Allowed**, the admin portal lets you start the tally while voters can
still vote.
:::

## 1. Create the tally (election administrator)

1. Open the election event.
2. Click the **Tally** tab.
3. Click **Start Tally Ceremony**.
4. In **Elections to Tally**, select the elections to count.
5. In **Keys Ceremony**, select the key ceremony of these elections. If there is only one key
   ceremony, the admin portal selects it.
6. Click **Start Tally Ceremony**.
7. Read the message "This action will notify the trustees to import their key fragments."
8. Click **Ok**.

**Expected result:** the message "Tally created" shows. The tally shows in the list of the
**Tally** tab with the status `STARTED`.

9. Tell the trustees that the tally ceremony has started.

## 2. Upload the key fragment (each trustee)

Each trustee does these steps with their own account. You need one copy of your key fragment
from your storage device.

1. Sign in to the admin portal.
2. Open the election event.
3. Click the **Tally** tab. The message "You have been invited to participate in a Tally
   ceremony." shows.
4. In the row of the tally, click **Add Tally Key** in the **Actions** column.
5. Read the list of **Elections to Tally**. Make sure that it is the list that you expect.
6. Drop your key fragment file into the upload area.
7. Make sure that the message "Backup verified successfully." shows.
8. Click **Next**.
9. Remove the storage device and keep it in its safe location.

**Expected result:** the trustee has uploaded the key fragment.

## 3. Start the tally (election administrator)

1. On the **Tally** tab, click the view icon of the tally.
2. Look at the **Trustees** section. It shows how many trustees have imported their key and how
   many are necessary.
3. Wait until the status is `CONNECTED`. This status shows that enough trustees have uploaded
   their key fragment.
4. Click **Start Tally**.
5. Read the message "Do you want to start the Tally?".
6. Click **Start Tally**.

**Expected result:** the message "Tally started" shows. The status changes to `IN_PROGRESS`.
The **Elections Tally Progress** table shows the progress of each election.

7. Wait until the status is `SUCCESS`. The time depends on the number of votes.
8. Click **Results**. See [06-RESULTS](06-results.md).
9. Record the date, the time, the trustees present and the result of the ceremony.

## Tally status

| Status | Meaning |
| --- | --- |
| `STARTED` | The tally is created. The trustees can upload their key fragments. |
| `CONNECTED` | Enough trustees have uploaded their key fragment. You can start the tally. |
| `IN_PROGRESS` | The platform mixes, decrypts and counts the votes. |
| `SUCCESS` | The tally is complete. The results are available. |
| `CANCELLED` | The tally is cancelled. |

The **Elections Tally Progress** table shows these steps for each election: `WAITING`,
`MIXING`, `DECRYPTING`, `SUCCESS` or `ERROR`.

## Cancel a tally

You can cancel a tally before it starts, when its status is `STARTED` or `CONNECTED`.

:::caution CAUTION
You cannot undo a cancellation. The trustees must upload their key fragment again for a new
tally.
:::

1. On the **Tally** tab, click **Cancel Tally Ceremony** in the row of the tally.
2. Click **Cancel Tally**.

## If there is a problem

| Problem | Action |
| --- | --- |
| "The Tally Ceremony cannot start until the Keys Ceremony has been successfully completed." | Complete the key ceremony. See [03-KEYS](03-keys.md). |
| "The Tally Ceremony cannot start until you create one publication in the Publish tab." | Publish the ballot. See [04-PUBLISH](04-publish.md). |
| "You cannot continue the ceremony because no elections are selected or the elections are not published." | Select one or more published elections. |
| "Elections have different keys ceremonies" | Make one tally for each key ceremony. |
| "You cannot continue the ceremony because the tally session is not connected or the start of the ceremony is not allowed." | Wait for more trustees. If the status is `CONNECTED`, check **Allow Tally** of each election and close the voting period. |
| "Trustee not part of the keys ceremony or has invalid state" | The trustee is not in the key ceremony of these elections, or has already uploaded the fragment. |
| A trustee's file is not accepted. | Use the copy from the other storage device. Make sure it is the file of this election event. |
| An election shows `ERROR`. | Do not start a new tally. Contact Sequent support and give the name of the election event. |

**Next:** [06-RESULTS: Get the results](06-results.md).
