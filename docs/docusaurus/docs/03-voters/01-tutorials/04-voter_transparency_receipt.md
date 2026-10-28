---
id: voter_transparency_receipt
title: Check your transparency receipt
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

After you cast in an election with transparency enabled, the confirmation page shows a Vitrine receipt below your ballot ID. Your vote has already been accepted. A missing receipt or a delayed check does not undo that acceptance.

The receipt separates three facts:

- **Included:** the log holds the entry for your ballot ID, election and event. A promise means the log has signed a commitment to include it, with a deadline.
- **Unchanged:** witnesses have checked the log's growth and cosigned its checkpoint.
- **The same for everyone:** independent hub and browser checks provide additional evidence that the log is showing a consistent history.

Pending means that a check still needs data, signatures, or a deadline. A failure means that the available objects contradict a requirement. A receipt does not reveal your selections, and does not by itself prove that your ballot was decrypted or counted correctly.

## Check independently

The result on the confirmation page is labelled **Checked by this website**. Select **Check independently** to open the deployment's ballot verifier with the receipt. Compare the policy ID and its six words with the election notice, or with the policy your election organiser supplied through an authenticated channel, before pinning it. An unsigned draft has eight words and cannot be pinned.

A receipt file and policy are data. The verifier checks them; it does not execute downloaded code. The independent verifier also shows the detailed proof steps. Its first online round asks the policy's logs. Further checks with mirrors, hubs and monitors are delayed to reduce timing correlation, unless you explicitly request them or a check is already late. Private-election receipts can be checked offline with the policy and proofs they contain; their upgrades remain behind the voting portal's login.

## Keep or remove a receipt

On a personal device the portal asks the browser to persist one receipt per election. When you log in again, your election chooser can show the receipt this browser kept for your account. Browser storage can still be cleared or refused. Use the receipt's download or print actions for your own copy.

The confirmation page lets you stop keeping the receipt. Kiosk sessions keep no receipt on the device. Downloading, printing and drawing the QR code make no network request. Scanning or opening the QR reference later asks the independent verifier to retrieve the receipt from the log.

If a check fails, save the receipt file, note the displayed policy words and contact your election organiser through the official support channel. Do not publish your receipt or private policy unnecessarily. If the portal refuses to encrypt before you cast, it could not establish that the key and ballot style match the witnessed configuration; no ballot was encrypted by that attempt.
