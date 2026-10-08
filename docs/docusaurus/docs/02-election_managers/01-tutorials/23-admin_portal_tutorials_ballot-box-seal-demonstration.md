---
id: admin_portal_tutorials_ballot-box-seal-demonstration
title: Demonstrate the ballot box seal
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

This tutorial walks through a demonstration of the
[ballot box seal](../02-reference/02-election-event/20-ballot-box-seal.md) from
start to finish: close an election, show its signed seal and a refused write,
verify the seal record without the system, then tally. It takes about
half an hour, plus the grace period of any election that has one.

| Step | What it shows |
| --- | --- |
| [3. Close and seal](#3-close-and-seal) | At the close, a signed, time-stamped SHA-512 hash of every ballot of each ballot box goes to the public bulletin board. |
| [4. Refused writes](#4-refused-writes) | After the close, no ballot can be added, changed or deleted, and voting cannot start again. |
| [5. Verify offline](#5-verify-the-record-offline) | Anyone who has the seal record can check a ballot box against it without the system, and detect tampering. |
| [6. Tally](#6-tally) | The tally counts only sealed ballot boxes and checks each one against its signed seal first. |

## Before you start

You need:

- an Admin Portal user who can configure, publish, start and stop voting, and
  tally the event;
- the `step-cli` binary on the computer used for the offline check;
- the fingerprint of the event's system key (32 hex digits), from your
  platform operator, taken before the close (see
  [Which key to trust](../../04-election_auditors/02-tutorials/06-auditor_verify-ballot-box-seal.md#which-key-to-trust));
- for the database step, a demonstration environment where you may run SQL.

## 1. Prepare the event

1. Create an election event. In **Data > Advanced Configurations**, set
   **Ballot Box Seal Policy** to **Seal at close**. Leave **Ballot Box Seal
   Record** on **Restricted** (the default) to show the records being shared
   with observers, or set it to **Public** to show anyone downloading them
   without signing in. Do it now: neither can be changed after voting has
   opened.
2. Create one election with two areas, with only the Online channel enabled.
   Every enabled channel must be stopped before the boxes are sealed, even
   one that never opened (Early Voting included), and so must any channel
   that opened and was then unchecked.
   Leave one area without voters, to show that an empty ballot box is sealed
   too.
3. Add voters, run the key ceremony and publish.
4. Start voting.

## 2. Vote

1. Cast a few ballots in the first area. Let one voter vote twice, so that the
   seal shows a replaced ballot.
2. Keep one voter's receipt: its Ballot ID is used in step 5.

## 3. Close and seal

1. On the election's **Publish** tab, select **Stop Voting**. The confirmation
   says that the ballot boxes are then sealed and that voting cannot start
   again. Confirm.
2. Open the election's **Dashboard**. On the **Ballot boxes** card, each area
   goes from **Sealing now** to **Sealed, publishing** and then
   **Sealed**, normally within seconds. (In a test run, the seal was made one
   second after Stop Voting and published half a second later.) The card names
   who closed the election, and shows the ballots in the box, the ballots
   counted, the seal time and the seal hash.
3. Open the event's **Logs** tab and filter by the statement kind
   `BallotBoxSealed`. There is one entry per area, for example "Ballot box of
   *election*, *area* sealed: 3 of 4 ballots counted." It is signed by the
   event's system key and carries the seal hash and the counts. Note the seal
   hash of each entry: step 5 compares it with the record.
4. On the card, select **Seal record** for each area and save the JSON files.
   With **Restricted**, the button downloads each record through a short-lived
   signed link, and you hand the files to the observers. With **Public**, it is
   a link to the public file, which anyone with the event's IDs can download
   without signing in.

This demonstration uses Stop Voting without signatures, so the record's
`close_request` is empty. When the close is signed by a quorum of signers, the
record also names the Close voting request, its signing code and each signer's
certificate fingerprint.

## 4. Refused writes

Show that the sealed ballot box no longer changes:

| Try | Result |
| --- | --- |
| A voter casts a ballot after the close | The voting portal shows the usual "voting is closed" error, and `CastVoteError` entries appear in the Logs. |
| **Start Voting** on the closed election | The Admin Portal doesn't offer it. Through the API, the server refuses: "Voting can't start again: with the Ballot Box Seal Policy set to Seal at close, voting that has closed stays closed and its ballot boxes are sealed." |
| Delete the election | The Admin Portal checks for seals first and refuses: "This election has sealed ballot boxes and cannot be deleted. Archive its election event instead." The database refuses it through any other path. |
| Delete the election event | The Admin Portal checks for seals first and refuses: "This election event has sealed ballot boxes and cannot be deleted. Archive it instead." The server refuses it through any other path, before anything is deleted. |
| Change the event's contest encryption, delegated or weighted voting policy | Refused by the database: it can't change after voting has opened. |
| In the demonstration environment's database, `INSERT`, `UPDATE`, `DELETE` or `TRUNCATE` on the sealed box's rows of `sequent_backend.cast_vote`, or moving a row to another area | Refused: `ERROR: ballot_box_sealed` (SQLSTATE `42501`), "A sealed ballot box takes no new ballots and no changes." The box keeps its rows. |

The database step shows the guard against a direct write. In a demonstration
environment the services may connect as a database superuser, who could disable
the guard; production needs a least-privilege role (see
[Database role for the ballot box seal](../../05-reference/05-cryptography_security/02-security/03-ballot-box-seal-database-role.md),
which also explains what a superuser could still do and how it is caught).

## 5. Verify the record offline

On a computer without access to the system, follow
[Verify a ballot box seal](../../04-election_auditors/02-tutorials/06-auditor_verify-ballot-box-seal.md):

1. Check the record against the trusted key, with the voter's Ballot ID and,
   if you have one, an `export-cast-votes` CSV:

   ```bash
   step-cli step verify-ballot-box-seal record.json \
     --expected-key '<fingerprint>' \
     --election-id <election id> --area-id <area id> \
     --ballot-id <Ballot ID from the receipt> \
     --cast-votes cast-votes.csv
   ```

   Every line is marked ✓ (the Log entry line is marked `-`: it is not
   signed), the names are "the names in the signed statement", the Ballot ID is
   "in the ballot box: counted", the CastVote comparison shows 0 differences
   both ways, and the verdict is "Seal valid".
2. Compare the full seal hash the command prints with the one in the
   `BallotBoxSealed` entry from step 3. They must be equal: this shows that the
   record is the seal on the bulletin board.
3. Check the empty area's record too: it is valid with 0 ballots.
4. Show tampering: copy a record, remove one entry or change a disposition, and
   check the copy. The verdict is "Seal NOT valid", with a non-zero exit
   status.
5. Show a wrong key: run the first command with another fingerprint. The check
   fails with "signed by key …, not the expected key".

## 6. Tally

1. In **Election Event > Tally**, start a new tally. The **Ballot boxes** column
   shows "*N* of *N* sealed" for the election, and it can be selected.
   An election whose ballot boxes are not all sealed and published cannot be
   selected. An election with a grace period becomes selectable once its grace
   period has ended and its seal is published.
2. Run the tally.
3. In the **Logs** tab, filter by `TallyBallotBoxVerified`: there is one entry
   per ballot box, for example "Ballot box of *election*, *area* matches its
   seal: 3 ballots counted." The tally counted the ballots fixed by the seal;
   the replaced ballot was not counted.
4. The seal hash of each ballot box is kept in the tally results' annotations,
   for reports.
