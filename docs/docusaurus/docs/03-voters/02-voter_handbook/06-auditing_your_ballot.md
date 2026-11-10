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

It works like this:

- On the review screen you can either cast the encrypted ballot or audit it.
- If you audit it, the voting portal first asks the intermediate ballot box to
  record the audit. Only once the ballot box has confirmed it does the voting
  portal reveal the secret values it used to encrypt that ballot, and the ballot
  verifier checks with them that the encrypted ballot contains your selections.
- A ballot whose secret values have been revealed is spoiled. It must never be
  cast, because these values show how it was filled in. The intermediate ballot
  box, which recorded the audit, refuses to cast it, and the voting portal
  discards it.
- To vote, you start again. Your next ballot is encrypted anew, with new secret
  values and a new Ballot ID.

Your device cannot know in advance whether you will audit or cast a ballot, so a
device that encrypts wrongly can be caught by an audit.

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
  finished the check. Do not use the button "Print" on the screen
  "Check your ballot" or in the ballot verifier: a printout shows the same data,
  and you cannot delete it as you delete the file.

:::

## Start the audit

![Audit confirmation warning and the explicit discard-to-audit button](./assets/voter-audit-confirmation.png)

*Illustration: the dialog that opens when you select the button "Audit ballot".
Auditing spoils this ballot; select the button "Cancel" to return to the review
screen.*

To start the audit:

1. On the review screen, select the copy icon next to the label "Your Ballot ID:"
   and paste the full Ballot ID into a text file. It identifies the ballot you
   audit; you need it for the ballot verifier and to check your next ballot.
2. Select the button "Audit ballot".
3. The voting portal opens the dialog "Would you like to audit your ballot",
   which warns that an audit voids this ballot and restarts the voting.
4. Select the button "Yes, discard my ballot to audit". To return to the review
   screen instead, select the button "Cancel".
5. Wait while the voting portal asks the intermediate ballot box to record the
   audit: meanwhile the buttons "Audit ballot" and "Cast ballot" are disabled.
   Once the ballot box has confirmed it, the screen "Check your ballot" opens. If
   the review screen shows the message "The ballot box could not confirm the audit
   of this ballot. Please try again later or contact support" instead, nothing
   has been revealed or cast (see
   [Messages and what to do](10-states_and_problems.md#messages-and-what-to-do)).

## Save the audit details

The voting portal shows the screen "Check your ballot", which starts with the
line "To check your ballot, follow the steps below:". To save the audit details:

1. Under the heading "1. Save the following details:", the screen refers to the
   Ballot ID at its top, after the label "Your Ballot ID:". This is the full
   Ballot ID of the audited ballot. Check that it is exactly the full Ballot ID
   you copied on the review screen. If it is not, stop and tell the election
   organizer.
2. Select the button "Download" (on a small screen, the download icon). Your browser saves your encrypted ballot, with the secret
   values used to encrypt it, as a text file. The screen also shows the same
   data as text below the button.

## Open the ballot verifier

To open the ballot verifier:

1. Under the heading "2. Check your ballot", select the link
   "Check your ballot code". The ballot verifier opens in a new tab of your
   browser.
2. You opened it with the link of the voting portal, so you do not type an
   address. Check that the browser shows a secure connection without warnings.
3. If the ballot verifier asks you to log in, log in in the same way as for the
   voting portal.

## Check your ballot in the ballot verifier

![Ballot verifier import screen with Browse, an empty Ballot ID field and disabled Next](assets/ballot-verifier-current.png)

*Illustration: the ballot verifier before a file is loaded. Use your own audit
file and full Ballot ID, not the button "Use a sample ballot". This empty screen
does not show a successful check.*

To check your ballot:

1. Under the heading "Step 1: Import your ballot", drag the file you downloaded
   onto the area that says "Drag & drop files or", or select the word "Browse" in
   it and choose the file. The supported format is txt. If the ballot verifier
   shows the message "Could not import this file. Please try again.", or the title
   "Error" with the message "There was a problem importing the auditable ballot.
   Did you choose the right file?", choose the file you downloaded in
   [Save the audit details](#save-the-audit-details).
2. Under the heading "Step 2: Insert your ballot ID", paste the full Ballot ID
   you copied on the review screen into the field "Ballot ID".
3. Select the button "Next".
4. The ballot verifier shows the heading "Ballot ID" with two values, labelled
   "Decoded Ballot Id" (read from your file) and "The Ballot Id you provided".
   They must be the same. If they are not, the ballot verifier shows the message
   "Doesn’t match the decoded ballot ID".
5. Under the heading "Verify your ballot selections", the ballot verifier shows
   the selections it has decoded from your encrypted ballot. Check that they are
   exactly the selections you made in the voting portal.

The check is successful if both Ballot ID values are the same and the decoded
selections are exactly yours.

:::warning

**Check only the file you downloaded.** Do not select the button
"Use a sample ballot" in the ballot verifier. It loads an example ballot, and a check of it says nothing
about your ballot.

:::

## Vote

To vote after the audit:

1. Go back to the tab of the voting portal. A notice at the bottom of the screen
   "Check your ballot" says that an audited ballot is spoiled and that you
   continue with the button "Start Voting".
2. Select the button "Start Voting". The voting portal returns to the screen
   "Ballot list".
3. For the same ballot, select the button "Click to Vote", then the button
   "Start Voting". Make your selections again and select the button "Next". Your
   browser encrypts a new ballot.
4. On the review screen, select the copy icon next to the label "Your Ballot ID:"
   and paste the new full Ballot ID into your text file, on a new line under the
   full Ballot ID of the audited ballot. Compare the two lines: they must differ.
   If they are the same, do not cast. Tell the election organizer.
5. Cast the new ballot as described in
   [Casting Your Vote](05-casting_your_vote.md), or audit it, too.
6. When the voting portal shows the confirmation screen
   "Your vote has been cast", select the copy icon next to the Ballot ID and paste
   the value on a third line. It must be the same as the line of the new ballot.
   If it is not, tell the election organizer.
7. Delete the file you downloaded, and close the tab of the ballot verifier.

## If the check fails

If any of the following happens and you cannot correct it as the table says, do
not cast a ballot on this device. Note the message and the time, and tell the
election organizer. If you want to vote, use another device you trust.

| What the ballot verifier shows | What it means |
|---|---|
| The title "Ballot verification failed" with the message "The ciphertext in this auditable ballot is not the encryption of the plaintext and randomness it contains. The ballot cannot be trusted." | The encrypted ballot does not contain what the file says. |
| The title "Ballot verification failed" with the message "The ballot style in this auditable ballot is not the one published for the election: its public key or its contests and candidates differ, or it is not published. The ballot cannot be trusted." | The ballot was not encrypted for the published ballot of your election. |
| The message "This auditable ballot does not match any ballot published for this election event, so it cannot be verified." | The file does not belong to a ballot of this election. Check that you chose the file of this audit. If you did, report it. |
| The message "This auditable ballot uses an older ballot format whose Ballot ID does not cover the ballot style. Its Ballot ID cannot confirm that the ballot was encrypted with the published election key." | The check cannot confirm the encryption key. |
| The message "Doesn’t match the decoded ballot ID" | The Ballot ID you entered is not the one of this file. Check that you pasted the full Ballot ID you copied on the review screen. If it was correct, report it. |
| The decoded selections are not the ones you made | Your device did not encrypt your selections. |

If the ballot verifier shows the message "The ballots published for this election
event could not be loaded, so this auditable ballot cannot be verified yet. Please
try again.", the check could not be done. Try again later.

If the voting portal goes back to the screen "Ballot list" without showing the
screen "Check your ballot" (for example after you reloaded the page), your browser
no longer holds that ballot. Open the ballot again with the button "Click to Vote",
and contact the election organizer if it happens again.
