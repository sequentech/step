---
id: about
title: About the Handbook for Voters
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

| Item | Value |
|---|---|
| Document ID | AGD-01 |
| Title | Handbook for Voters |
| TOE reference (TOE: target of evaluation, the product as evaluated) | `Sequent Voting Platform - uniWAHL Version, v10.0.0` |
| Document version | 1.1 |
| Software release | v10.0.0 |
| Role | Voter |

## Who this guide is for

This handbook is for voters: the people in the voters' register of an election.
It tells you how to vote safely with the Sequent Voting Platform - uniWAHL
Version, v10.0.0, in its evaluated configuration. It covers:

- what you need and what you must do before you vote;
- how to log in, cast your vote and log out;
- how to check that your vote was encrypted as you intended, that it was
  stored and that it was counted;
- how to vote again, if your election allows it;
- what the different states of an election mean for you, what can go wrong,
  and what to do then;
- how to report a security problem.

## The voting system and what you use

In this handbook, **the voting system** means the Sequent Voting Platform -
uniWAHL Version, v10.0.0. You use it with these:

- **The voting portal.** The web application of the voting system that you
  open in your browser to vote. It brings the voting system's encryption code to
  your browser, so your vote is encrypted on your device before it is sent.
- **The ballot verifier.** A page of the voting system for checking a ballot
  you have audited (see [Auditing Your Ballot](06-auditing_your_ballot.md)).
- **The election verifier.** A program of the voting system that checks the
  verification data of an election after the count. Your election organizer
  tells you where to get it and how to use it (see
  [Checking That Your Vote Was Stored and Counted](07-checking_storage.md)).
- **The authentication service your election organizer tells you about.** You
  log in there. It is not part of the voting system. The voting system does not
  hold your password.

The voting portal leads you through the steps. A button on the screen does not
by itself allow a vote: the voting system checks each vote it receives. What the
voting portal shows does not replace the checks with the ballot verifier and the
election verifier, which this handbook describes.

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
- **An observer** is anyone who checks the result independently with the
  election verifier.

## Other handbooks

You need only this handbook to vote. You do not need the other handbooks; they
are for the people who run the election:

- the Handbook for Election Organizers (AGD-03): what the election organizer
  and the election board do, including the instructions they give you;
- the Handbook for Election Administrators (AGD-02): how administrators set up
  and run the voting system;
- the Deployment Guide (AGD-04): how the voting system is installed.

## Conventions

The screenshots show test elections with made-up data. Your election's names,
candidates, codes and dates will differ. Follow the steps in the text; do not
copy values from a screenshot.

- Text in double quotes and italics, such as the button "Cast ballot", is
  exactly what the screen shows in English. If you use the voting portal in
  another language, the same items appear in that language.
- Numbered steps are done in the order given. A step that starts with the word
  If or To applies only in that case.
- Boxes marked **warning** or **danger** are security warnings. Follow them.
  Boxes marked **info** explain something.
- These terms have a fixed meaning in this handbook:
  - **election**: the whole vote your election organizer runs. It can have one
    or more ballots.
  - **ballot**: one of the votes of an election, with its questions (contests)
    and options (candidates or answers). The voting portal lists your ballots on
    the screen "Ballot list". The word also means your filled-in ballot; your
    encrypted ballot is that ballot after your browser has encrypted it.
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
  - **Ballot ID**: the code that identifies your encrypted ballot. The review
    and confirmation screens show a short code of eight characters; the full
    Ballot ID has 64. You check your ballot with the full Ballot ID.
  - **cast**: to send your vote so that it counts. Until you cast it, your
    vote does not count.
  - **hash**: a fixed-length code calculated from a text. Any change in the
    text gives a different code.
  - **verification data**: the files the election organizer publishes after the
    count so that anyone can check it with the election verifier.
  - **electoral regulation**: the rules that apply to your election.
  - **area**: a part of an election, such as a district or unit. Each voter
    belongs to one.
- Some screens use other words: "tracker ID" and "Ballot Tracker ID" mean the
  Ballot ID; "ballot code" means the audit data of your ballot; "auditable ballot"
  means the file you download for an audit; "election" can mean a ballot;
  "election event" means the election.

## Document history

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-10-27 | First edition for the evaluated configuration of v10.0.0. |
| 1.1 | 2026-11-10 | Numbered sections; introductions to numbered steps; all sections revised; new screenshots. |
