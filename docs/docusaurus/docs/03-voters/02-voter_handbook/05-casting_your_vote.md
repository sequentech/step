---
id: casting_your_vote
title: Casting Your Vote
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

You can vote only while the ballot shows "Open" in the "Ballot list". Your vote
counts only after you have cast it and the voting portal shows "Your vote has
been cast".

The screenshots show the voting portal with a synthetic test election and
voter, not a real election: a test ballot being reviewed, cast and located.
Your election name, candidates, branding and Ballot IDs will differ.

## Open the ballot

![Ballot list with the available synthetic test ballot](./assets/voter-chooser-current.png)

If your election offers instructions for each screen, they are in the bar at the
top of the screen: the question-mark icon shows them, and the speaker icon next
to it reads them aloud where your election has recorded them or your browser
has a voice for your language. If your election offers it, the accessibility
icon there changes how the voting portal looks on your device, for example its
text size or contrast.

To open your ballot:

1. In the "Ballot list", find the ballot you want to vote on.
2. Select "Click to Vote".

## Start voting

![How to vote screen with the Start Voting button](./assets/voter-start-current.png)

The voting portal shows "How to vote" with the three steps of voting: make your
selections, review them, and cast your ballot. To start voting:

1. Read the three steps.
2. Select "Start Voting".

If your election offers it, the screen also shows "Decline to Vote". If you
select it and confirm with "Decline to vote", the voting portal goes directly
to the review screen with a ballot that records that you declined to vote. You
cast it in the same way as any other ballot.

## Make your selections

![Candidate selection with Option A checked](./assets/voter-vote-current.png)

To make your selections:

1. For each question (contest), select the options you want. The question says
   how many options you can select.
2. To remove all your selections and start again, select "Clear selections".
3. When you have finished, select "Next".

Before it continues, the voting portal may ask you to look again at your
selections:

| Message | What it means | Your choices |
|---|---|---|
| "Your vote is invalid or blank" | Some of your answers make one or more contests invalid or blank. | "Review selections" to go back, or "Continue" |
| "Review your ballot" | Your selections may need your attention, for example because you selected fewer options than allowed. The ballot is valid. | "Back and review" to go back, or "Continue" |
| "You have not selected any candidates" | Your ballot would be cast blank. A blank ballot is valid. | "Cancel" to go back, or "Continue" |

When you continue, your browser encrypts your ballot on your device and the
voting portal opens the review screen. Nothing is cast at this step.

## Review your ballot

![Review screen with the selection, Ballot ID, edit, audit and cast buttons](./assets/voter-review-current.png)

The voting portal shows "Review your ballot" with your selections. Your ballot is
already encrypted: at the top, the screen shows "Your Ballot ID:" followed by
the short code of this encrypted ballot, such as `G5AT-12SZ`. The copy icon
next to it copies the full Ballot ID. Compare ballots only with the full Ballot
ID; the short code is not proof (see
[Your Ballot ID](07-checking_storage.md#your-ballot-id)). You can now cast this
ballot, or audit it to check that it contains your selections.

:::info

**Your vote has not been cast yet.** When this screen opens, the voting portal
sends your encrypted ballot to the intermediate ballot box. The ballot box
stores it as received, not as cast, and confirms this with its signature, which
your browser checks. The short code is calculated from your encrypted ballot
and this confirmation. A received ballot is never counted; only casting it makes it
count. Until you cast it, the ballot locator does not find this Ballot ID.

:::

To finish the review:

1. Check every selection carefully.
2. Choose the action you need:

| Button | What happens next |
|---|---|
| **Edit ballot** | Return to your selections and change them. |
| **Audit ballot** | Check this ballot’s encryption. The voting portal then discards this ballot and does not let you cast it; to vote, you start again with a new encrypted ballot and a new Ballot ID. See [Auditing Your Ballot](06-auditing_your_ballot.md). |
| **Cast ballot** | Submit the ballot. Continue with section 5.5 and wait for the confirmation. |

If you left a contest unfilled, the voting portal shows "Some selections are
unfilled". Select "Review selections" to go back, or "Continue with these
selections" to cast the ballot as it is.

If you reload the page before you cast, the voting portal may take you back to
your ballot when you select "Cast ballot", and encrypt it again. Check your
selections and continue as before.

## Cast your vote

To cast your vote:

1. Select "Cast ballot".
2. If your election asks you to confirm, the voting portal shows "Are you sure
   you want to cast your vote?" Select "Yes, I want to cast my vote". For a
   blank ballot the question is "Are you sure you want to cast a blank ballot?"
   and you select "Yes, cast my blank ballot". Select "Cancel" if you do not
   want to cast yet.
3. Your election may ask you to log in again before your vote is cast, to make
   sure it is you. Log in at the login page of the authentication service, as
   in [Logging In to Vote](04-logging_in.md). Do this within 5 minutes. If more
   time passes, the voting portal shows "Your session has expired. Please start
   again" and your vote is not cast; open the ballot again with "Click to Vote".
4. The voting portal shows "Your vote has been cast".

:::warning

**Cast only a ballot that shows exactly your selections.** If you have audited
a ballot, check that the ballot you cast has a different full Ballot ID. The
audit takes place only on your device, so the voting system cannot recognise
an audited ballot; only your check shows that you do not cast it. Once cast, a
vote cannot be changed. If your election allows
you to vote again, a new vote replaces it in the count (see
[Voting Again](08-voting_again.md)).

:::

## The confirmation

![Test-vote confirmation with Ballot ID, QR code, Print and Finish](./assets/voter-confirmation-current.png)

The confirmation screen "Your vote has been cast" appears only after the
intermediate ballot box has confirmed with its signature that it stored your
encrypted vote, and your browser has checked that signature. It shows:

- "Ballot ID" with the short code of your cast ballot. Selecting the code opens
  the ballot locator, where you can find your ballot later. The copy icon next
  to it copies the full Ballot ID.
- "Verify that your ballot was cast" with a QR code that also opens the ballot
  locator. Although the screen says "at any time", the ballot locator lets you
  check your ballot only while voting is open, at the latest until the end of
  the election period.
- "Print": saves a receipt for your vote as a PDF file. If this fails, the
  voting portal shows "Error" with "An error occurred. Please try again".
- "Finish": takes you back to the "Ballot list" if there is a ballot you can
  still vote in, including voting again where your election allows it.
  Otherwise it logs you out.

If you want to check your ballot later, copy the full Ballot ID with the copy
icon and paste it into a text file that you keep. How to check it:
[Checking That Your Vote Was Stored and Counted](07-checking_storage.md).

## If you stop before casting

You can stop at any time before you cast: close the browser, log out, or leave
the voting portal. A technical problem can also interrupt you. In all these
cases, nothing is cast, and you keep your right to vote. While voting is open,
log in again and start again from section 5.1.

## If casting fails

If the voting portal shows an error instead of "Your vote has been cast", your
vote may or may not have been cast. Do not assume either. To find out:

1. Log in again, if needed, and open the "Ballot list".
2. If the ballot shows "Not voted" and voting is still open, vote again from
   section 5.1.
3. If the ballot shows "Voted", use "Locate your ballot" to find your ballot
   (see [Checking That Your Vote Was Stored and Counted](07-checking_storage.md)).
4. If you are unsure, or the error keeps coming back, contact the election
   organizer. Tell them the message and the time.

The messages you may see are listed in
[Election States and Problems for Voters](10-states_and_problems.md).
