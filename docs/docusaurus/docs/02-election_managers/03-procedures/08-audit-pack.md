---
title: Prepare the Audit Pack
sidebar_position: 8
description: "This procedure collects the documents that show that the election was secret and correct, and selects the documents that you can give to candidates, observers and the public. The election administrator does this procedure."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Prepare the Audit Pack

This procedure collects the documents that show that the election was secret and correct. It
also tells you which documents you can give to candidates, observers and the public. The
election administrator does this procedure after the results are approved.

## Before you start

- The tally status is `SUCCESS` and the results are approved. See
  [Get the Results](07-results.md).
- You have the records that you wrote during the key ceremony and the tally ceremony: the date,
  the time, the trustees present and the result. See
  [Run the Key Ceremony](04-keys.md#3-check-that-the-ceremony-is-complete-election-administrator)
  and [Run the Tally Ceremony](06-tally.md#3-start-the-tally-election-administrator).
- Your account has these permissions: **Read Tally Results** and **Export Ceremony** for the
  results, **Export Logs** for the electoral log, and the permission to generate reports.

## 1. Collect the documents

Collect each document of the table. Keep all of them together, in a safe location.

| Document | Where to get it | What it shows |
| --- | --- | --- |
| Initialization report | **Tally** tab, tally of the type **Initialization Results**. See [Initialization report](../01-tutorials/11-admin_portal_tutorials_initialization-report.md). | The count of the ballot box before the voting period opened. If you made the report before the first vote, the count is zero. |
| Results document (PDF) | **Tally** tab, results of the tally of the type **Electoral Results**. See [Get the Results](07-results.md#2-download-the-result-documents). | The participation and the votes of each candidate, for each contest and area. |
| **Participation Report** | **Reports** tab, **Generate**. | The number of voters, the number of voters that voted and the participation, for the election event or one election. |
| Electoral log (CSV or PDF) | **Logs** tab, export button. See [Election logs](../01-tutorials/19-admin_portal_tutorials_election-logs.md#export-the-log). | Each important action with its date and time: the publication, the opening and closing of each channel, the key ceremony, the upload of each key fragment, the start and the end of the tally, and each vote cast. |
| Ceremony records | Your own records, and the **Keys** and **Tally** tabs. | Who the trustees were, when each ceremony occurred and its result. |
| Results archive (**TAR_GZ**) | **Tally** tab, actions menu of the election event in the results. | All the files of the tally, including the decrypted ballots of each area. |
| Ballot images | **Tally** tab, actions menu of an election in the results: **Generate Ballot Images**. This item shows only if your platform has the ballot images export. | One image of each decrypted ballot, for each contest and area, for a count by hand. |

The platform has no report of the key ceremony or the tally ceremony. The electoral log has a
record of each step, with the names of the trustees that uploaded a key fragment. Use the log and
your own records together.

**Expected result:** you have all the documents of the table that your election uses.

## 2. Select the documents to share

:::danger WARNING
Do not give the electoral log, the results archive or the ballot images to persons outside the
election organization. The electoral log contains the identifier, the username, the IP address
and the country of each voter that voted. The results archive and the ballot images contain each
decrypted ballot of an area. In an area with few voters, a person can find how a voter voted.
:::

| Document | Candidates, observers and the public | Election organization only |
| --- | --- | --- |
| Initialization report | Yes | |
| Results document (PDF) | Yes | |
| **Participation Report** | Yes | |
| Ceremony records (date, time, trustees, result) | Yes | |
| The technical documents of the platform. See [Technology](#4-give-the-technical-documents). | Yes | |
| Electoral log | Only a summary that you write from it. Do not include voter data. | Yes |
| Results archive (**TAR_GZ**) | No | Yes |
| Ballot images | No. Show them only in a supervised count by hand. | Yes |
| **Credentials** and **Manual Verification** documents | No. They contain voter passwords or sign-in links. | Only the persons that send them to voters |

1. Make a copy of the documents that you can share.
2. Remove nothing from the PDF documents. If you change a document, it is not the original.
3. For the electoral log, write a summary with the times of the main actions: publication, opening
   and closing of each channel, key ceremony, tally start and tally end. Do not copy voter rows.

**Expected result:** you have a set of documents that you can give outside the election
organization.

## 3. Answer the questions

Use this table when a candidate or an observer asks for evidence.

| Question | Document | What to say |
| --- | --- | --- |
| Was the ballot box empty at the start? | Initialization report | The count before the first vote was zero. |
| Can a person see how I voted? | Ceremony records, technical documents | The vote is encrypted on the device of the voter. The tally shuffles the encrypted votes before it decrypts them, so no vote can be linked to a voter. Decryption needs a minimum number of trustees. One person alone cannot decrypt the votes. |
| Who could decrypt the votes? | Ceremony records, electoral log | The trustees of the key ceremony. The log shows which trustees uploaded a key fragment for the tally, and when. |
| When did the voting open and close? | Summary of the electoral log | The times of the records `ElectionVotingPeriodOpen` and `ElectionVotingPeriodClose`, or `ElectionEventVotingPeriodOpen` and `ElectionEventVotingPeriodClose` for the election event. |
| Do the results agree with the number of voters? | Results document, **Participation Report** | Compare the votes counted with the number of voters that voted. |
| Could a voter check the ballot? | Technical documents | Before a voter casts the ballot, the voter can audit it with the ballot verifier. The audit shows that the encrypted ballot contains the choices of the voter. An audited ballot is not cast; the voter makes a new one. |

:::note
The electoral log is kept in a tamper-evident database, and the platform signs each record. In
version 10.0, the admin portal has no tool to check these signatures. If an auditor must check
them, contact Sequent support.
:::

## 4. Give the technical documents

The [Technology](../../06-technology/01-threat-model.md) part of this documentation describes
the security of the platform. You can give its pages to candidates and observers:

- [Threat model](../../06-technology/01-threat-model.md)
- [White paper](../../06-technology/02-white-paper.md)
- [Cryptographic protocol specification](../../06-technology/03-cryptographic-protocol-spec.md)
- [Cyber security](../../06-technology/04-cyber-security.md)

The source code of the platform is public, with the AGPL license. An independent expert can
read it. See [Source code review](../../04-election_auditors/02-tutorials/05-auditor_source-code-review.md).

## If there is a problem

| Problem | Action |
| --- | --- |
| There is no initialization report. | You cannot make it after the voting period opened. Use the summary of the electoral log to show when the voting opened. |
| The initialization report does not show zero. | The report was made after votes were cast. Contact Sequent support before you share it. |
| **Generate Ballot Images** is not in the menu. | Your platform does not have the ballot images export. Use the results document. |
| A person asks for the full electoral log. | Do not give it. Give the summary, or show the log in a supervised session. |
| The export of the log failed. | Look at the **Tasks** tab. Try again, then contact Sequent support. |

## More information

- [Election logs](../01-tutorials/19-admin_portal_tutorials_election-logs.md)
- [Initialization report](../01-tutorials/11-admin_portal_tutorials_initialization-report.md)
- [Tally ceremony](../01-tutorials/16-admin_portal_tutorials_tally-ceremony.md)
- [Reports and templates](../01-tutorials/18-reports_and_templates.md)
- [Election auditors](../../04-election_auditors/01-introduction/auditor_introduction.md)
