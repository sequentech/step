---
id: voter_transparency_receipt
title: Check your transparency receipt
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

After you cast in an election with transparency enabled, the confirmation screen shows a check mark next to your Tracking ID. It shows how far your browser has verified that your ballot is in the election's public record. Your vote has already been accepted. A missing receipt or a delayed check does not undo that acceptance.

Tracking IDs show in groups of four characters, for example `30ff d314 4238 6472 …`. The short ID is the first two groups, `30ff d314`. It is what you read and type to find your ballot. The full ID has 16 groups, and it is what your browser and the ballot verifier check.

The receipt separates three facts, which are the three steps of the status:

- **In the public record** (included): the log holds the entry for your Tracking ID, election and event. A promise means the log has signed a commitment to include it, with a deadline. The status stays at Checking until then.
- **Confirmed by witnesses** (unchanged): witnesses have checked the log's growth and cosigned its checkpoint.
- **Anchored publicly** (the same for everyone): independent hub and browser checks provide additional evidence that the log is showing a consistent history.

A step that is waiting or being checked still needs data, signatures or time. That is not a failure. A failure means that the available objects contradict a requirement. A receipt does not reveal your selections, and does not by itself prove that your ballot was decrypted or counted correctly.

## The check mark

The check mark appears next to the Tracking ID on the review and confirmation screens, and next to **Ballot cast** in the ballot list. It has seven states. Its shape shows progress and its colour shows trouble. An amber or red check always has a badge, so colour is never the only cue. The worst state wins: red over amber over grey. It turns blue only when every step that the election's policy uses is verified.

- **Not cast yet** (dotted grey check): on the review screen, before you cast. Nothing has been sent, and nothing is checked until you cast it.
- **Checking** (dashed grey check, moving): your cast was accepted and your browser is checking the public record.
- **In the public record** (one grey check): the entry for your ballot is in the public record, and its proof checks out.
- **Confirmed by witnesses** (two grey checks): independent witnesses signed a record that includes it.
- **Verified** (two blue checks): it is also anchored in public hubs and where browsers check.
- **Late** (amber check with a clock): a step missed its deadline. The dialog calls it **Taking longer than expected**. This usually means that a hub is slow or offline. It is not evidence of tampering, and your browser keeps checking.
- **Problem** (red check with an alert): a check failed, or the receipt is missing, damaged or could not be checked. The dialog names the cause.

The check mark and the dialog also have two states for a ballot that was audited, and so never cast. Step does not open the dialog in those two states today, because Step itself does not publish audited ballots. The ballot tracker shows an audited ballot with its own tags and cards: see [An audited ballot](./02-voter_track_your_vote.md#an-audited-ballot). The two states are:

- **Auditable** (two grey checks): the ballot was audited, and the ballot tracker decrypts it to show the selections it holds.
- **Audit failed** (red check with an alert): the ballot does not decrypt with the key the portal revealed. Do not cast from this device, and tell the election authority.

The first steps usually finish within seconds. The anchors follow within 10 minutes by default; the election's policy sets the period, and there is no need to wait for it. Elections without transparency keep the portal's usual screens, without the check mark.

## The Ballot status dialog

Hover over the check mark to see its state. Select it to open the **Ballot status** dialog, which shows:

- The election and, once you have cast, when you cast the ballot.
- Your **Tracking ID** with a tag: **Not cast** or **Cast**. It shows the short Tracking ID and the full Tracking ID. The copy button next to the full ID copies it in groups of four, and turns into a green check for a moment.
- The state as a headline, and the three steps, each with what the receipt says about it and when your browser saw it done.
- When the status is **Late** or **Problem**, what it means.
- **Open in the ballot tracker**, **Save receipt** and **OK**. Before you cast, the dialog has only **OK**.

The headline of a **Problem** says which one it is:

- **The record does not match:** the public record's entry is not the one in your receipt. Signed objects contradict each other, which is evidence. Select **Save evidence** and send the file to the election authority or an observer.
- **The receipt is damaged:** part of the receipt or the record could not be read or verified. That is a data fault, not evidence of tampering. Your vote was cast.
- **No receipt for your ballot:** the election requires a receipt, and the website did not get one. The election's observers have been told. Your vote was cast.
- **Your ballot could not be checked:** the website could not check the receipt. Your vote was cast.

## Check independently

The status is your browser's check, with code anyone can inspect, on the voting portal's page. To check the receipt in the separate ballot verifier instead, open your ballot in the ballot tracker, which is Ballot Tracking (**Open in the ballot tracker** in the dialog, or **Track your ballot** in the ballot list), and select **Check it independently**. It opens the deployment's ballot verifier with the receipt. Compare the first four groups of the policy ID with the election notice, or with the policy your election organiser supplied through an authenticated channel, before pinning it. An unsigned draft shows its draft hash in the same form, tagged **Draft**, and cannot be pinned. The policy ID is not a ballot's Tracking ID: if you type it into the ballot tracker, the tracker says so.

A receipt file and policy are data. The verifier checks them; it does not execute downloaded code. It shows the same status and three steps, with what its checks found: the entry number, how many witnesses signed and how many the policy needs, and how many hubs hold the record. Its first online round asks the policy's logs. Further checks with mirrors, hubs and monitors run after a random delay of 1 to 30 minutes to reduce timing correlation, unless you select **Check everything now** or an anchor hub or the browser layer is already late. Private-election receipts can be checked offline with the policy and proofs they contain; their upgrades remain behind the voting portal's login.

## Keep a receipt

On a personal device the portal asks the browser to persist one receipt per election. When you log in again, the ballot list shows **Ballot cast** and the check mark for each election whose receipt this browser kept for your account, and your browser checks the receipt again on each visit. Browser storage can still be cleared or refused. Select **Save receipt**, in the dialog or in the ballot tracker, for a copy of your own. It saves a `.vitrine-receipt` file named after your short Tracking ID, and makes no network request. Kiosk sessions keep no receipt on the device.

If the status shows **Problem**, save the receipt file (**Save evidence**, when the dialog offers it) and contact your election organiser through the official support channel. The file contains nothing about how you voted. Do not publish your receipt or private policy unnecessarily. If the portal refuses to encrypt before you cast, it could not establish that the key and ballot style match the witnessed configuration; no ballot was encrypted by that attempt.
