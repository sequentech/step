---
id: auditor_ballot_election-logs
title: Immutable Election Logs
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Each election event has an electoral log. The platform adds a signed record for each important
action and stores it in a tamper-evident database (immudb). Nobody can change or delete a record
from the admin portal.

## What the log records

- The publication of the ballot.
- Each opening, pause and closing of a voting channel: `ElectionVotingPeriodOpen`,
  `ElectionVotingPeriodPause` and `ElectionVotingPeriodClose` for an election, and
  `ElectionEventVotingPeriodOpen`, `ElectionEventVotingPeriodPause` and
  `ElectionEventVotingPeriodClose` for the election event.
- The key ceremony (`KeyGeneration`) and the upload of each key fragment in a tally ceremony,
  with the name of the trustee (`KeyInsertionCeremony`).
- The start and the end of each tally (`TallyOpen`, `TallyClose`), each pause for a tie and each
  tie resolution.
- Each vote cast (`CastVote` or `CastVoteWithChannel`), and each error when a voter casts a vote
  (`CastVoteError`).

Each record has a date and time, a kind, the user that did the action and the full message.

## Who can see the log

The records of the votes contain the identifier, the username, the IP address and the country of
each voter. For this reason, the full log stays inside the election organization.

- The election administrator can show you the log in a supervised session.
- Outside the organization, you receive a summary: the times of the publication, the opening and
  closing of each channel, the key ceremony, and the start and end of the tally. See
  [Prepare the Audit Pack](../../02-election_managers/03-procedures/08-audit-pack.md#2-select-the-documents-to-share).

## Check the log

1. Make sure that the opening of the voting period is before the first vote cast.
2. Make sure that there is no vote cast after the closing of the same channel.
3. Make sure that the trustees in the records `KeyInsertionCeremony` are the trustees of the key
   ceremony.
4. Make sure that `TallyOpen` is after the closing of the voting period.

## Limits in version 10.0

Each record has a signature of the sender and a signature of the platform. The CSV export
contains them. The admin portal has no tool to check the signatures or the integrity of the
database. If you must check them, ask the election administrator to contact Sequent support.

For the export of the log, see [Election logs](../../02-election_managers/01-tutorials/19-admin_portal_tutorials_election-logs.md).
