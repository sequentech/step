---
id: voter_role
title: Your Role as a Voter
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## What you can do

As a voter you have these functions. The voting system allows each of them only
at the time shown.

| Function | Where you do it | When it is possible | Details |
|---|---|---|---|
| Log in | Login page of the authentication service your election organizer tells you about | Any time; voting itself only while voting is open | [Logging In to Vote](04-logging_in.md) |
| See your ballots | "Ballot list" in the voting portal | After you have logged in | [Logging In to Vote](04-logging_in.md) |
| Fill in, review and cast a ballot | Voting portal, "Click to Vote" | Only while the ballot shows "Open" (execution phase) | [Casting Your Vote](05-casting_your_vote.md) |
| Audit a ballot instead of casting it | "Audit ballot" on the review screen, then the ballot verifier | While voting is open, before you cast | [Auditing Your Ballot](06-auditing_your_ballot.md) |
| Get the confirmation that your vote was stored | Confirmation screen "Your vote has been cast" | Right after you cast | [Checking That Your Vote Was Stored and Counted](07-checking_storage.md) |
| Find your cast ballot and copy your encrypted ballot | "Locate your ballot" in the "Ballot list" | Until the end of the election period | [Checking That Your Vote Was Stored and Counted](07-checking_storage.md) |
| Check that your vote was counted | With the verification data and the election verifier that your election organizer makes available, and the copy of your encrypted ballot | After the result has been determined (post-processing phase) | [Checking That Your Vote Was Stored and Counted](07-checking_storage.md) |
| Vote again | "Click to Vote" for the same ballot | Only if your election allows more than one vote, and only while voting is open | [Voting Again](08-voting_again.md) |
| Log out | "Logout" in the menu under your name | Any time | [Your Voting Session](09-voting_session.md) |

## What you cannot do

- You cannot vote before the election period starts, after it ends, or while
  voting is paused.
- You cannot see other voters' ballots or find out who cast a ballot. The
  ballot locator finds only your own ballots.
- You cannot change a vote once you have cast it. If your election allows you to
  vote again, you cast a new vote, and only the vote you cast last is counted.
- You cannot register yourself, reset a password or change your account in the
  voting system. Your login details belong to the authentication service your
  election organizer tells you about.
- You cannot change any security setting of the voting system.

## How your vote is protected

- Your browser encrypts your vote before it leaves your device. The voting
  system stores only the encrypted vote.
- Before the votes are counted, the voting system separates them from the
  voters' names and mixes them. Only then are they decrypted, and only together.
  Nobody can see how you voted.
- When you cast your vote, the intermediate ballot box confirms with its
  signature that it has stored it, and your browser checks that signature before
  it shows you the confirmation.
- You can check that your vote was encrypted correctly (by auditing a ballot)
  and that it was stored (with its Ballot ID). After the result has been
  determined, you can check that it was counted, with the verification data
  that your election organizer publishes. These data contain proofs that the
  stored votes were mixed and decrypted correctly.

## Settings and choices under your control

You set no security parameter of the voting system: the voting portal offers
you none. The administrator sets them for the whole election, under the rules
of the election organizer. Those that affect you are:

- your session ends after 15 minutes without activity;
- after a number of failed logins in a row, your account is locked for a while;
- you can vote once, unless your election allows more votes; then the vote you
  cast last counts;
- you can check your cast ballot until the end of the election period;
- "Audit ballot" is always offered before you cast.

What is under your control, and the secure choice for each:

| Item | Secure choice | Where described |
|---|---|---|
| Your login details | Keep them secret; never pass them on | [Before You Vote](03-before_you_vote.md) |
| Where and how you vote | Alone, where nobody can see your screen | [Before You Vote](03-before_you_vote.md) |
| Your device and browser | A device you trust, with a current browser and no untrusted extensions | [Before You Vote](03-before_you_vote.md) |
| The address of the voting portal | Only the address your election organizer gave you, over a secure connection without warnings | [Before You Vote](03-before_you_vote.md) |
| Audit or cast | Audit if you want to check the encryption; an audited ballot can never be cast, so cast only a new ballot with a new Ballot ID | [Auditing Your Ballot](06-auditing_your_ballot.md) |
| Your Ballot ID | Keep it to check your ballot later; it does not reveal your selections | [Checking That Your Vote Was Stored and Counted](07-checking_storage.md) |
| Your encrypted ballot | Copy it from the ballot locator before the end of the election period if you want to check later that your vote was counted | [Checking That Your Vote Was Stored and Counted](07-checking_storage.md) |
| Ending your session | Log out and close the browser when you have finished | [Your Voting Session](09-voting_session.md) |
