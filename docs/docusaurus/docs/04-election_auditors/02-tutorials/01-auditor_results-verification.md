---
id: auditor_results-verification
title: Election Results Verification
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Use this page to check that the published results agree with the evidence of the election.

## Documents you need

- The initialization report of each election, if the election administrator made one.
- The results document (PDF) of the tally.
- The **Participation Report**.
- The records of the key ceremony and the tally ceremony: date, time, trustees and result.

The election administrator gives you these documents. See
[Prepare the Audit Pack](../../02-election_managers/03-procedures/08-audit-pack.md).

## Check the start of the election

1. Open the initialization report.
2. Make sure that the ballots counted and the votes of each candidate are zero.
3. Make sure that the date of the report is before the opening of the voting period.

The platform makes the initialization report with the same tally ceremony as the results. If the
report was made after votes were cast, it does not show zero.

## Check the results

1. Open the results document.
2. For each contest, add the votes of the candidates, the blank votes and the invalid votes.
3. Make sure that the sum is equal to the **Total votes counted**.
4. Compare the **Total votes counted** with the number of voters that voted in the
   **Participation Report**.
5. Make sure that the winners in the document agree with the votes and the counting system of
   the contest.

## Check the ceremonies

1. Make sure that the number of trustees in the tally ceremony is equal to or more than the
   threshold of the key ceremony.
2. Compare the trustees in the ceremony records with the records `KeyInsertionCeremony` of the
   electoral log. See [Immutable election logs](03-auditor_election-logs.md).

## What a voter can check

Before a voter casts a ballot, the voter can audit it:

1. On the review screen, the voter selects the audit option. The audited ballot is void, and the
   voter makes a new ballot.
2. The voter downloads the audited ballot and opens it in the ballot verifier.
3. The ballot verifier encrypts the choices again and compares the result with the ballot. It
   shows the choices, so the voter can compare them with the intended choices.

This check shows that the voting portal encrypts the choices of the voter correctly. It does not
show that a cast ballot was counted.

After the voter casts the ballot, the voter gets a ballot ID. With the ballot locator, a voter
that is signed in can find the ballot ID among the cast ballots.

## Limits in version 10.0

- The **Report hash** on a report identifies the moment when the platform made the report. It
  does not cover the content of the report. Use the documents that the election administrator
  downloaded, and compare copies.
- The results archive does not include the proofs of the shuffle or of the decryption.
