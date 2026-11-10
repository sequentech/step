---
id: states_and_problems
title: Election States and Problems for Voters
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## States of an election

An election goes through these states, in the phases named in
[About the Handbook for Voters](01-about.md). The voting portal shows them to
you as follows.

| State (phase) | What you see | What you can do |
|---|---|---|
| Before voting starts (preparation) | The voting portal cannot load the screen "Ballot list" yet and shows the message "There was a problem fetching the data. Please try again later or contact support for assistance." | Wait until voting starts. You can already log in, but no ballots are shown. |
| Voting is open (execution) | The ballot shows the status "Open" and the button "Click to Vote". A ballot that opens later than the others shows the status "Closed" until then, with its start after the label "Open:" and the time left after the label "Election Begins in". | Vote, audit, check your cast ballot and copy your encrypted ballot. |
| Voting is paused (execution) | The ballot shows the status "Closed", or the voting portal cannot load the screen "Ballot list" and shows the message "There was a problem fetching the data. Please try again later or contact support for assistance." | Wait. Nobody can start or continue voting while voting is paused. The election organizer announces every interruption to the voters. Votes already cast stay stored. |
| Voting has ended (evaluation) | The voting portal cannot load the screen "Ballot list" any more and shows the message "There was a problem fetching the data. Please try again later or contact support for assistance." | You cannot vote any more: voting cannot be reopened. The ballot locator no longer finds your cast ballot: it shows the message "Something went wrong." or the text "Checks ended on" followed by the date. |
| Counting and result (evaluation, then post-processing) | The election board counts the votes and the result is determined. In the post-processing phase the election organizer publishes the verification data. | After the election organizer has published the verification data, check that your vote was counted, or ask an observer you trust to do it (see [Checking That Your Vote Was Stored and Counted](07-checking_storage.md)). |

## If a failure happens during voting

If the voting system detects a failure it can recover from while voting is
open, it puts the election into maintenance mode. In maintenance mode:

- voting is paused for everyone: no voter can start or continue voting;
- nobody can read or change the stored votes through the voting portal;
- a screen "Ballot list" already shown in your browser may still show the status
  "Open", but the voting system refuses what you send;
- when you load the screen "Ballot list" again, you see the message below, with no
  button "Click to Vote";
- the ballot locator is not available, which does not mean that your stored vote
  has been lost.

Only the election board can resume voting, and the voting system first checks
itself before it allows that.

![Voting portal refusing to load ballots during maintenance](assets/voter-maintenance-unavailable.png)

*Illustration: the voting portal during maintenance mode, with test data. The
voter has logged in, but cannot load a ballot or start voting.*

If you see the message "There was a problem fetching the data. Please try again
later or contact support for assistance.", wait and try again later. If it
continues, tell the election organizer the message and time. Do not try another
address or link.

The same can happen at the start of the election period: the voting system
checks itself before it opens voting. If that check fails, voting does not
open, and the screen "Ballot list" stays unavailable until the problem is solved.

For you this means:

- A ballot you had not cast is not cast. You keep your right to vote.
- A vote you had already cast stays stored.
- When voting is open again, log in, check the status "Voted" or "Not voted" on
  the screen "Ballot list", and vote if you have not voted yet.

If the failure cannot be repaired, voting in the election stops for good. The
election organizer then tells voters what happens next under the electoral
regulation.

## Logins refused to protect the records

The voting system keeps records of what happens during the election. If it can
no longer keep them safely (for example, because the storage for these records
is almost full), it refuses new voter logins and voter requests until the
problem is solved. Voting can then stay paused until the election board resumes
it. The voting portal can show the message "The voting portal could not start.
Please try again." with the button "Try again". Wait before trying again; if it
continues, contact the election organizer. The message alone does not identify
the cause or change whether a previous vote was cast.

![The voting portal refusing to start while voter logins are refused](assets/voter-storage-unavailable.png)

*Illustration: the voting portal while voter logins are refused to protect the
records, in a test. Votes already cast stay stored. After the problem is solved,
voting can stay paused until the election board resumes it.*

## Messages and what to do

When a message says "contact support", contact your election organizer. Always
tell them the message and the time.

| Message | What it means | What to do |
|---|---|---|
| "The voting portal could not start. Please try again." | The voting portal could not complete login or startup. This can happen when the voting system refuses voter access to protect its records. | Wait and select the button "Try again". If it continues, tell the election organizer the message and time. |
| "There was a problem fetching the data. Please try again later or contact support for assistance." | The voting portal could not load your ballots. This happens before voting starts, after it has ended, during maintenance, or with a connection problem; the message alone does not identify the cause. | Wait and try again later. If it continues, contact the election organizer. The message does not confirm whether an earlier vote was cast. |
| "This election does not allow casting a vote. It may be closed, archived, or outside the allowed voting period" | Voting is not open for this ballot: it has not started, is paused or has ended. Your vote was not cast. | Look at the screen "Ballot list". If voting is paused, wait and try again when the ballot shows the status "Open". |
| "A network problem occurred. Please try again later or contact support" | The connection to the voting system failed. | Check your internet connection. Check the status "Voted" or "Not voted" as in [If casting fails](05-casting_your_vote.md#if-casting-fails) before you try again. |
| "A timeout occurred while casting your vote. Please try again later or contact support" | The voting system did not answer in time. Your vote may or may not have been cast. | Check the status "Voted" or "Not voted" as in [If casting fails](05-casting_your_vote.md#if-casting-fails) before you try again. |
| "An unknown error occurred while casting your vote. Please try again later or contact support" | Casting failed for an unknown reason. Your vote may or may not have been cast. | As above. |
| "An internal error occurred while casting your vote. Please try again later or contact support" | An error in the voting system. | As above. |
| "There was an internal error while casting the vote. Please try again later or contact support for assistance." | The voting system refused your request, for example because voting is paused or in maintenance, or because of an error. Your vote may or may not have been cast. | Wait. When the screen "Ballot list" loads again, check the status "Voted" or "Not voted" before you try again. |
| "Failed to validate your vote. Please try again later or contact support" | The voting system refused your encrypted ballot because it did not pass its checks, for example of its encryption, its signatures or its ballot style, or because it was audited. | Check the status "Voted" or "Not voted". If it shows "Not voted", open the ballot again with the button "Click to Vote" and vote with a new ballot, on a device you trust. If it happens again, tell the election organizer. |
| "The ballot box could not confirm the audit of this ballot. Please try again later or contact support" | You chose to audit a ballot, but the intermediate ballot box did not confirm that it recorded the audit. The voting portal revealed nothing, and nothing was cast. | Select the button "Audit ballot" again later, or select the button "Edit ballot", then the button "Next", to continue with a new ballot. If it happens again, tell the election organizer. |
| "The ballot id does not match with the cast vote." | The Ballot ID does not belong to the ballot sent. Your vote was not cast. | Tell the election organizer. Try again on a device you trust. |
| "There was an error related to the ballot hashing process." followed by "Please report this issue to support.", or a page "Oops! Unexpected Error" with the code "INCONSISTENT_HASH" | A confirmation of the intermediate ballot box, or the Ballot ID of your encrypted ballot, did not pass the check of your browser. | Do not cast a ballot on this device. Tell the election organizer (see [Checking That Your Vote Was Stored and Counted](07-checking_storage.md)). |
| "You are not authorized to cast a vote. Please contact support." | The voting system does not let you cast a vote in this ballot. | Contact the election organizer. |
| "Your session has expired. Please start again" | You took more than 5 minutes to log in again before casting. Your vote was not cast. | Open the ballot again with the button "Click to Vote". |
| "Login failed. Your username or password may be incorrect. Please try again or contact support" | Logging in again before casting failed. Your vote was not cast. | Open the ballot again and try again. |
| "Session storage is not available. Please try again or contact support." | Your browser did not keep the data needed to cast your vote. Your vote was not cast. | Try again. If it happens again, use another browser you trust. |
| "We couldn’t check whether you have more elections to vote in. Please retry." | On the confirmation screen: the voting portal could not load your other ballots. Your vote has been cast. | Select the button "Retry". |
| "The election event hasn’t been published yet. Please try again later or contact support for assistance." | The election is not ready yet. | Try again later. |
| "There are no elections you can vote for. This could be because the area doesn’t have any contest associated. Please try again later or contact support for assistance." | You have no ballot to vote on. | Contact the election organizer if you expected one. |
| Any other message | The voting portal could not complete the step. Your vote may or may not have been cast. | Check the status "Voted" or "Not voted" as in [If casting fails](05-casting_your_vote.md#if-casting-fails) before you try again. Tell the election organizer the message and the time. |

Messages about voting again are in [Voting Again](08-voting_again.md). Messages of
the ballot verifier are in [Auditing Your Ballot](06-auditing_your_ballot.md).
Messages about logging in are in [Logging In to Vote](04-logging_in.md).

## Signs that something is wrong

:::warning

**Stop and tell the election organizer** if you notice any of these:

- the browser shows a certificate or security warning (the real voting portal
  never causes one), or the address is not the one the election organizer gave
  you; do not try another address or link instead;
- the login page looks different from what the election organizer described,
  or someone asks you for your login details;
- your account is locked, or a ballot shows the status "Voted", although you have
  not logged in or voted;
- the ballot shows questions or options you did not expect, or the review
  screen shows selections you did not make;
- an audit fails (see [Auditing Your Ballot](06-auditing_your_ballot.md));
- your cast ballot is not found with the button "Locate your ballot", or the cast
  time is wrong;
- someone watches you vote, pressures you, or asks you to show how you voted.

Do not cast your vote while something looks wrong. Note what you saw, the
message and the time, and tell the election organizer. If you think you have
found a security flaw in the voting system, also report it as described in [Reporting a security problem](#reporting-a-security-problem).

:::

## Reporting a security problem

If you think you have found a security flaw in the voting system, report it to
the security contact of Sequent, the company that makes the voting system,
[security@sequentech.io](mailto:security@sequentech.io). Send questions about
suspected security flaws, including about a report you have made, to the same
address. You may also tell your election organizer, who forwards the report to
Sequent.

Encrypting your report is recommended. Use Sequent's security-contact OpenPGP
key, fingerprint `DB6B035E872AB5DA90C7194A60D3CBE30BAC91ED`. It is published on
Sequent's
[Vulnerability Disclosure Policy page](https://docs.sequentech.io/docusaurus/main/docs/technology/vulnerability-disclosure-policy).
Unencrypted reports are accepted and handled in the same way.

Your report should contain:

- the name of the election and the address of the voting portal;
- what happened, and what effect it had;
- the date, the time and your time zone;
- the steps that led to it;
- how Sequent can contact you.

:::warning

**Never put a password, a one-time code, a private key, a key share or how you
voted in a report.** Do not try to exploit the problem any further on a live
election.

:::

Sequent acknowledges your report within five business days and gives you a
reference for it. Use this reference when you ask about the status of your
report.
