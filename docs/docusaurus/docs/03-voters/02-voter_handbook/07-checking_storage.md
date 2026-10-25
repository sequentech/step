---
id: checking_storage
title: Checking That Your Vote Was Stored and Counted
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## The confirmation when you cast

When you cast your vote, the intermediate ballot box stores your encrypted vote
and confirms this with its signature. Your browser checks that signature. Only
if it is correct does the voting portal show "Your vote has been cast" with
your Ballot ID (see [Casting Your Vote](05-casting_your_vote.md)).

:::warning

**Report it if a check of your browser fails.** Your browser checks each
confirmation of the intermediate ballot box and the Ballot ID of your encrypted
ballot. If one of these checks fails, the voting portal shows "There was an
error related to the ballot hashing process." followed by "Please report this
issue to support.", on the review screen or after you select "Cast ballot", or
it shows a page "Oops! Unexpected Error" with "INCONSISTENT_HASH". Do not cast
a ballot on this device. Note the time and the whole message, and tell the
election organizer. Then log in again on a device you trust and look at your
ballot in the "Ballot list" and with "Locate your ballot".

:::

## Your Ballot ID

The Ballot ID is a ballot tracker: a SHA-3 hash that identifies your
encrypted ballot. The voting portal describes it like this: "The Ballot ID is
a unique code that lets you find your ballot in the ballot box. It does not
contain any information about your selections."

- Keep the Ballot ID of your cast ballot. You can copy it, write it down,
  select "Print" on the confirmation screen to save the receipt, or scan the QR
  code.
- The Ballot ID does not show how you voted. The ballot locator finds a ballot
  only for the voter who cast it, while that voter is logged in.
- If you vote more than once (only where your election allows it), each cast
  ballot has its own Ballot ID.

## Find your ballot with the ballot locator

You can check that your cast ballot is stored in the intermediate ballot box
until the end of the election period.

1. Open the ballot locator in one of these ways:
   - select the Ballot ID on the confirmation screen;
   - scan the QR code on the confirmation screen;
   - log in to the voting portal and select "Locate your ballot" for the ballot
     in the "Ballot list".
2. The voting portal shows "Find your Ballot". Under the title it says "You can
   check your ballot until" followed by the date and time when checks end.
3. If the Ballot ID is not filled in already, type or paste it into the field
   (it shows "Confirm your ballot was cast correctly" while empty). If the
   voting portal shows "Invalid Ballot ID format", check what you typed.
4. Select "Find your Ballot".
5. The voting portal shows "Your Ballot Lookup Results" with one of the results
   below.
6. To look up another Ballot ID, select "Find another Ballot".

| Result | What it means | What to do |
|---|---|---|
| "Your ballot ID" followed by the code and "has been found" | Your ballot is stored. The voting portal also shows "Cast on" with the time it was cast, and "This is your Ballot content:" with your encrypted ballot. | Check that the time matches when you cast your vote. To check later that it was counted, copy the encrypted ballot (see below). |
| "Your ballot ID" followed by the code and "was not found" | No cast ballot of yours has this Ballot ID. | Check that you typed it correctly. If it is correct, tell the election organizer. |
| "More than one of your ballots matches" followed by "Use the full ballot ID." | The code you typed is too short to identify one ballot. | Type the full Ballot ID. |
| "Something went wrong." | The lookup could not be done, for example because the election is in maintenance mode (see [Election States and Problems for Voters](10-states_and_problems.md)). | Try again later. If it keeps happening, tell the election organizer. |
| "Checks ended on" followed by a date | The time for checking ballots is over. | Nothing can be checked any more. |

The ballot content is encrypted. It does not show your selections.

:::warning

**Tell the election organizer at once** if your ballot is not found although
you saw "Your vote has been cast" and typed the Ballot ID correctly, if the
cast time is not when you voted, or if the ballot locator finds a ballot you did
not cast.

:::

## Keep a copy of your encrypted ballot

If you want to check after the election that your vote was counted, keep a copy
of your encrypted ballot. If you voted again, use the Ballot ID of the last
vote you cast and copy that ballot: only your last vote is counted. Earlier
ballots can still be found by the locator but are absent from the counted
ballots, so checking one of them does not verify your final vote. You can copy
the final ballot only while the ballot locator shows it, and at the latest
until the end of the election period.

1. Find your ballot with the ballot locator, as described above.
2. Select all the text shown under "This is your Ballot content:" and copy it.
3. Paste it, exactly as shown and without any change, into an empty text file,
   and save the file. Put nothing else into this file. Keep your Ballot ID
   separately, for example written down.
4. Open the file again and check that it contains the whole text shown by the
   ballot locator, from its first to its last character.

The copy does not show your selections.

## After the election: was your vote counted?

After voting has closed, the election board counts the votes. Before
decrypting them, the voting system separates the votes from the voters' names
and mixes them. Every mixing step and the decryption come with mathematical
proofs (proofs of a correct shuffle and of a correct decryption).

When the result has been determined, in the post-processing phase of the
election, the election organizer:

- publishes a verification JSON file for every batch of the completed results
  tally, with each file's matching full configuration hash and Tally ID, at a
  public place it names to the voters (for example its own website);
- publishes each batch's configuration hash and Tally ID a second time on
  another channel, for example in an official notice;
- makes the election verifier available to voters and observers.

Before the election period, the election organizer tells you where this will
be published and how to run the check. Keep each batch file paired with its
own anchors. The Tally ID identifies that batch's protocol board; copy the
complete decimal value as text, not a rounded number or a tally-session ID.

The election verifier checks your copy of your encrypted ballot against the
verification data. The check succeeds only if your ballot is in the ballot box
that was mixed, and every mixing step and the decryption are proven correct.
Give the election verifier the file with your copy as the election organizer's
instructions say, and run it as they describe. If you do not want to run it
yourself, give your file to an observer you trust and ask them to run the
check for you. If there are several batch files and you do not know which one
contains your ballot, verify every file first, then search each otherwise
verified batch using its own anchors. Absence from one batch alone does not
mean your ballot is absent from the complete tally.

:::warning

**Take the verification data and the election verifier from the place the
election organizer names, and the configuration hash and the Tally ID from
their second publication.** Values taken from the same place as the file
prove nothing if that place was manipulated. If any batch fails its proof
checks, or your ballot is absent from every relevant verified batch, note what
the election verifier reported and tell the election organizer. You can also report it as a security problem (see
[Election States and Problems for Voters](10-states_and_problems.md)).

:::

The confirmation screen says "Use the code below to verify that it was
counted". The code is your Ballot ID. With it you find your ballot with the
ballot locator during the election period, and copy the encrypted ballot that
you need for the check after the election.
