---
id: about
title: About the Handbook for Voters
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## This handbook

| Item | Value |
|---|---|
| Document ID | AGD-01 |
| Title | Handbook for Voters |
| TOE reference (TOE: target of evaluation, the product as evaluated) | `Sequent Voting Platform - uniWAHL Version, v10.0.0` |
| Document version | 1.0 |
| Software release | v10.0.0 |
| Role | Voter |

This handbook is for voters. It tells you how to vote safely with the
Sequent Voting Platform - uniWAHL Version, v10.0.0, in its evaluated
configuration. It covers:

- what you need and what you must do before you vote;
- how to log in, cast your vote and log out;
- how to check that your vote was encrypted as you intended, that it was
  stored and that it was counted;
- how to vote again, if your election allows it;
- what the different states of an election mean for you, what can go wrong,
  and what to do then;
- how to report a security problem.

## The voting system and the parts you use

In this handbook, **the voting system** means the Sequent Voting Platform -
uniWAHL Version, v10.0.0. You use it through these parts:

- **The voting portal.** The web application you open in your browser to vote.
  It is not itself part of the voting system. It brings the voting system's
  encryption code to your browser, so your vote is encrypted on your device
  before it is sent.
- **The ballot verifier.** A page of the voting system for checking a ballot
  you have audited (see [Auditing Your Ballot](06-auditing_your_ballot.md)).
- **The election verifier.** A program of the voting system that checks the
  verification data of an election after the count. Your election organizer
  tells you where to get it and how to use it (see
  [Checking That Your Vote Was Stored and Counted](07-checking_storage.md)).
- **The authentication service your election organizer tells you about.** You
  log in there. It is not part of the voting system. The voting system does not
  hold your password.

## Who does what

- **The election organizer** runs your election. It gives you your login
  details and the address of the voting portal, decides the election dates and
  rules (for example, whether you may vote more than once), and is your contact
  for any question or problem.
- **The election board** is responsible for voting and the count. Voting opens
  at the start of the election period after the voting system has checked
  itself, or when the election board starts it after that time. The election
  board can pause, resume and end voting, and it counts the votes after voting
  has ended.
- **You, the voter,** are in the voters' register of the election. You can vote
  only in the ballots the voting portal shows you, and only while voting is
  open.

## Other handbooks

You need only this handbook to vote. You do not need the other handbooks; they
are for the people who run the election:

- the Handbook for Election Organizers (AGD-03): what the election organizer
  and the election board do, including the instructions they give you;
- the Handbook for Election Administrators (AGD-02): how administrators set up
  and run the voting system;
- the Deployment Guide (AGD-04): how the voting system is installed.

## Conventions

- Text in double quotes, such as "Cast ballot", is exactly what the screen shows
  in English. If you use the voting portal in another language, the same items
  appear in that language.
- Numbered steps must be done in the order given.
- Boxes marked **warning** or **danger** are security warnings. Follow them.
- These terms have a fixed meaning in this handbook:
  - **election**: the whole vote your election organizer runs. It can have one
    or more ballots.
  - **ballot**: one of the votes of an election, with its questions (contests)
    and options (candidates or answers). The voting portal lists your ballots in
    the "Ballot list".
  - **election period**: the time from the start to the end of voting that the
    election organizer announces.
  - **phase**: the stage an election is in. Preparation: before voting.
    Execution: from when voting opens until it ends, including any pause.
    Evaluation: after voting has ended, while the votes are counted.
    Post-processing: after the result has been determined. You vote and check
    your cast ballot only in the execution phase.
  - **intermediate ballot box**: where the voting system stores the encrypted
    votes during the election period. Where a screen of the voting portal says
    "ballot box", it means the intermediate ballot box.
  - **ballot box**: the encrypted votes that are counted. When counting starts,
    the voting system takes the last valid vote of each voter from the
    intermediate ballot box, without the voter's name, into the ballot box,
    which is then mixed and decrypted.
  - **Ballot ID**: the code that identifies your encrypted ballot. You use it to
    check your ballot.
  - **cast**: to send your vote so that it counts. Until you cast it, your
    vote does not count.

## Document history

| Release | Changes |
|---|---|
| v10.0.0 | First edition for the evaluated configuration. |
