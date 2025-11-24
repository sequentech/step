---
id: voter_locate_ballot
title: Locating a Ballot
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

It is possible to verify that your ballot has been correctly submitted.
In the Voting Portal's landing page `/election-chooser` click on the button 
"Locate Your Ballot" to go to `/ballot-locator`.

## Tabs in the Ballot Locator

### BALLOT LOCATOR

Enter the ballot ID and hit the button to Locate your ballot.

### LOGS

The immutable logs of the type `CastVote` are displayed in the Logs tab. 
To see the immutable logs, it's policy must be enabled in the Admin Portal at 
Election Event level: Data > Ballot Design > Show Cast Vote Logs Tab.

Enter the ballot ID to confirm you ballot is in the logs.

The table shows the user name, the ballot ID, timestamp of the ballot and message.
The immutable log's message contains the user's signature among other things. It's 
content is in Json format and you can press the copy button to visualize it better 
on an editor.
