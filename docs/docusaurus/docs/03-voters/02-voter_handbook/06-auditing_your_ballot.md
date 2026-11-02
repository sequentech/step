---
id: auditing_your_ballot
title: Auditing Your Ballot
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## What an audit checks

Your browser encrypts your ballot on your device. An audit lets you check that
it encrypted exactly your selections, and nothing else. This is called checking
that your vote is cast as intended.

It works like this. On the review screen you can either cast the encrypted
ballot or audit it. If you audit it, the voting portal reveals the secret values
it used to encrypt that ballot, and the ballot verifier checks with them that
the encrypted ballot contains your selections. A ballot whose secret values have
been revealed is spoiled: it must never be cast, because these values show how
it was filled in. The voting portal discards it and does not let you cast it.
The voting system itself cannot recognise an audited ballot, because the audit
takes place only on your device. To vote, you start again, and your next ballot
is encrypted anew, with new secret values and a new Ballot ID. Because your device
cannot know in advance whether you will audit or cast a ballot, a device that
encrypts wrongly can be caught by an audit.

Auditing is optional. You can audit as many times as you like before you cast.

:::warning

- **Never cast an audited ballot.** Before you cast the new ballot, check on
  the review screen that its full Ballot ID is different from the full Ballot
  ID of the audited ballot. If it is the same, do not cast, and tell the
  election organizer. Compare full Ballot IDs, not short codes (see
  [Your Ballot ID](07-checking_storage.md#your-ballot-id)).
- **An audit shows your selections.** The file you download and the ballot
  verifier show the selections of the audited ballot. Audit only where nobody
  can see your screen, keep the file to yourself, and delete it when you have
  finished the check.

:::

## Start the audit

![Audit confirmation warning and the explicit discard-to-audit button](./assets/voter-audit-confirmation.png)

*Current voting interface with synthetic example data. Auditing spoils this
ballot; cancel if you want to return to the review screen.*

To start the audit:

1. On "Review your ballot", select the copy icon next to "Your Ballot ID:" and
   paste the full Ballot ID into a text file. It identifies the ballot you
   audit; you need it for the ballot verifier and to check your next ballot.
2. Select "Audit ballot".
3. The voting portal asks "Would you like to audit your ballot" and explains:
   "Auditing your ballot will void it and restart the voting process. Continue
   only if you’re comfortable with the advanced steps. Otherwise, click Cancel to
   go back."
4. Select "Yes, discard my ballot to audit". To return to the review screen
   instead, select "Cancel".

## Save the audit details

The voting portal shows "Check your ballot" with "To check your ballot, follow
the steps below:". To save the audit details:

1. Under "1. Save the following details:", the screen refers to the Ballot ID
   at its top, after "Your Ballot ID:". This is the full Ballot ID of the
   audited ballot. Check that it is exactly the full Ballot ID you copied on
   the review screen. If it is not, stop and tell the election organizer.
2. Select "Download" (on a small screen, the download icon). Your browser saves your encrypted ballot, with the secret
   values used to encrypt it, as a text file. The screen also shows the same
   data as text below the button.

## Open the ballot verifier

To open the ballot verifier:

1. Under "2. Check your ballot", select the link "Check your ballot code". The
   ballot verifier opens in a new tab of your browser.
2. Check that it opened from the voting portal and that the browser shows a
   secure connection without warnings.
3. If the ballot verifier asks you to log in, log in in the same way as for the
   voting portal.

## Check your ballot in the ballot verifier

![Ballot verifier import screen with Browse, an empty Ballot ID field and disabled Next](assets/ballot-verifier-current.png)

*Illustration: the v10.0.0 verifier before a file is loaded. Use your own audit
file and Ballot ID, not “Use a sample ballot”. This empty screen does not show
a successful verification.*

To check your ballot:

1. Under "Step 1: Import your ballot", drag the file you downloaded onto the area
   that says "Drag & drop files or", or select "Browse" and choose the file. The
   supported format is txt. If the ballot verifier shows "Could not import this
   file. Please try again.", or "Error" with "There was a problem importing the
   auditable ballot. Did you choose the right file?", choose the file you
   downloaded in section 6.3.
2. Under "Step 2: Insert your ballot ID", paste the full Ballot ID you copied
   on the review screen into the field "Ballot ID".
3. Select "Next".
4. The ballot verifier shows "Ballot ID" with two values: "Decoded Ballot Id"
   (read from your file) and "The Ballot Id you provided". They must be the
   same. If they are not, the ballot verifier shows "Does’t match the decoded
   ballot ID".
5. Under "Verify your ballot selections", the ballot verifier shows the
   selections it has decoded from your encrypted ballot. Check that they are
   exactly the selections you made in the voting portal.

The check is successful if both Ballot ID values are the same and the decoded
selections are exactly yours.

:::warning

**Check only the file you downloaded.** Do not select "Use a sample ballot" in
the ballot verifier. It loads an example ballot, and a check of it says nothing
about your ballot.

:::

## Vote

To vote after the audit:

1. Go back to the tab of the voting portal. At the bottom of "Check your ballot"
   it says: "For security reasons, when you audit your ballot, it needs to be
   spoiled. To continue with the voting process, you need to click ‘Start
   Voting’ below."
2. Select "Start Voting". The voting portal returns to the ballot list.
3. For the same election, select "Click to Vote", then "Start Voting".
   Make your selections again and select "Next". Your browser encrypts a new
   ballot.
4. On "Review your ballot", copy the full Ballot ID with the copy icon and
   check that it is different from the full Ballot ID of the audited ballot. If
   it is the same, do not cast. Tell the election organizer.
5. Cast the new ballot as described in
   [Casting Your Vote](05-casting_your_vote.md), or audit it, too.
6. When the voting portal shows "Your vote has been cast", copy the full Ballot
   ID with the copy icon and check that it is the one of the new review screen.
   If it is the full Ballot ID of the audited ballot, tell the election
   organizer.
7. Delete the file you downloaded, and close the tab of the ballot verifier.

## If the check fails

If any of the following happens, do not cast a ballot on this device. Note the
message and the time, and tell the election organizer. If you want to vote,
use another device you trust.

| What the ballot verifier shows | What it means |
|---|---|
| "Ballot verification failed" with "The ciphertext in this auditable ballot is not the encryption of the plaintext and randomness it contains. The ballot cannot be trusted." | The encrypted ballot does not contain what the file says. |
| "Ballot verification failed" with "The ballot style in this auditable ballot is not the one published for the election: its public key or its contests and candidates differ, or it is not published. The ballot cannot be trusted." | The ballot was not encrypted for the published ballot of your election. |
| "This auditable ballot does not match any ballot published for this election event, so it cannot be verified." | The file does not belong to a ballot of this election. Check that you chose the file of this audit. If you did, report it. |
| "This auditable ballot uses an older ballot format whose Ballot ID does not cover the ballot style. Its Ballot ID cannot confirm that the ballot was encrypted with the published election key." | The check cannot confirm the encryption key. |
| "Does’t match the decoded ballot ID" | The Ballot ID you entered is not the one of this file. Check that you pasted the full Ballot ID you copied on the review screen. If it was correct, report it. |
| The decoded selections are not the ones you made | Your device did not encrypt your selections. |

If the ballot verifier shows "The ballots published for this election event
could not be loaded, so this auditable ballot cannot be verified yet. Please
try again.", the check could not be done. Try again later.

If the voting portal shows "There is no ballot available for audit. Please
contact support", go back to the "Ballot list", open the ballot again with
"Click to Vote", and contact the election organizer if it happens again.
