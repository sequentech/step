---
id: release-9.1.0
title: Release Notes 9.1.0
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 🐞 Accessing tenant url after logging out does not show tenant selection page.

Previously, if you're logged in to the Admin Portal, and you logged out,
and then went to the /tenant page to select the tenant, the page didn't load
correctly the first time. This change fixes the issue.

- Issue: [#7086](https://github.com/sequentech/meta/issues/7086)
- Main commit: `5d4a7424c`
- Cherry-picks:
  - release/8.6: `01f5ac3cf`
  - release/8.7: `439d7a0a8`
  - release/9.0: `90e5561a9`


## ✨ Add automatic keys/tally ceremonies

Add a new Ceremonies Policy at the election event level.
This policy provides the option for a user to enable automatic key ceremonies
for a specific election or all elections. With this enabled, the tally will 
also be performed automatically, eliminating the need for trustee involvement.

- Issue: [#7503](https://github.com/sequentech/meta/issues/7503)

## ✨ Add support retrieving master secret in an env variable

A new environment variable `MASTER_SECRET` has been added to use in DEV evironment instead of hashicorp.
`SECRETS_BACKEND` was updated to `SECRETS_BACKEND=EnvVarMasterSecret` accordingly.

This change should not affect production, there the value should be `SECRETS_BACKEND=AwsSecretManager`, more info in `.devcontainer/.env.development`.

The Braid Trustee service and its initialization script (`trustee.sh`) have been updated also support the env vars secrets backends.

- Issue: [#7271](https://github.com/sequentech/meta/issues/7271)

## 🐞 Can't create tally

After adding indexes for the Electoral Log, the election_id field was limited to
the size of one UUID but multiple UUIDs were stored when the Keys Ceremony was
at Event level and there were multiple elections. Now only one or no election id is
saved.

- Issue: [#8096](https://github.com/sequentech/meta/issues/8096)


## 🐞Candidates list top border missing

On Candidate Lists for the Voting Portal, the top border was missing.

- Issue: [#8190](https://github.com/sequentech/meta/issues/8190)


## ✨ Configure election screen on closed event

Add a class name and the is-start attribute to the HTML of the select elections
 screen at the voting portal to control the CSS from the admin portal.
"is-start" attribute added to the SelectElection component in ui-essentials.

- Issue: [#7984](https://github.com/sequentech/meta/issues/7984)

## ✨ Create a task for deleting events

Event deletion is now managed as a task. It will report the result of the task if
it goes correctly or the errors if not.

- Issue: https://github.com/sequentech/meta/issues/4203


## 🐞 Error in Election name on Admin Portal Tally

The election name shown in the section "Results & Participation" of the Tally in
the Admin Portal is shown as 0 rather than its actual name.

- Issue: [#7684](https://github.com/sequentech/meta/issues/7684)
- Main commit: `70feb9590`
- Cherry-picks:
  - release/9.0: `f3951c1ad`
  - release/9.1: `fb6655339`

## 🐞 Error message in voting portal should be a warning

Sometimes the voting portal shows an alert dialog but the ballot is a valid
ballot. In this case the dialog text should be different than in the case of
an invalid/blank ballot.

- Issue: [#8091](https://github.com/sequentech/meta/issues/8091)
- Main commit: `1755ef677`
- Cherry-picks:
  - release/9.0: `786b6e6ff`
  - release/9.1: `dd97b6f51`

## 🐞 Errors deleting Election Event

Fixed error deleting election event in specific cases. Also when an Election 
Event is created, the sidebar is automatically updated.

- Issue: [#8298](https://github.com/sequentech/meta/issues/8298)
- Main commit: `8bd1b0167`
- Cherry-picks:
  - release/9.0: `2a5a42e76`
  - release/9.1: `61d3e0997`

## ✨ Improve demo mode

With this change, the DEMO tiled background and the Demo warning dialog
will appear when entering the voting portal from the preview screen in the
admin portal. Also, the warning dialog will appear on the election start
screen rather than in the election chooser. This includes a fix so that
the demo background/dialog will only appear for elections that don't have
generated keys when voters login to the voting portal. Also, css classes
are added to the demo background and dialog to help custom styling.

- Issue: [#7278](https://github.com/sequentech/meta/issues/7278)

## ✨ Improve tally events in the Activity Log

Improve the details reported in the activity logs by adding more information like
username, and reporting when the tally is started and completed.

- Issue: [#6392](https://github.com/sequentech/meta/issues/6392)

## 🐞 Intermitten errors loading preview

Fix a race condition for calling WASM code when loading the voting portal that
was sometimes causing an error.

- Issue: [#7505](https://github.com/sequentech/meta/issues/7505)

## 🐞 Issues filtering by voted/not voted

Filtering voters by whether they have voted or not wasn't working and a fix
was implemented.

- Issue: [#8234](https://github.com/sequentech/meta/issues/8234)

## 🐞 Multiple Finished Tally Ceremony logs

Finished Tally Ceremony logs were repeated multiple times (each time the task is
executed). Now the logs say "Updated Tally Ceremony" each time unless the
Tally is completed.

- Issue: [#8033](https://github.com/sequentech/meta/issues/8033)

## ✨ Read tally in frontend from Sqlite3

With this change, the admin portal starts reading the results directly
from the Sqlite3 file produced by the Tally. This makes it faster and
more scalable.

- Issue: [#6721](https://github.com/sequentech/meta/issues/6721)

## ✨ Review UI accessibility

Using the IBM tool https://www.ibm.com/able/toolkit/tools/#develop , review the
accessibility for the Voting Portal and Ballot Verifier.

- Issue: [#156](https://github.com/sequentech/meta/issues/156)

## ✨ Tally - Add decoded ballot json to SQLite results database

With this change, it is possible now to include all the raw decoded ballots 
inside the sqlite database. It also moves part of the database generation 
inside velvet. This can be set at advanced config at the election event.

- Issue: [#7109](https://github.com/sequentech/meta/issues/7109) 

## ✨Tally - Add the option to export event tally results in xlsx format

In Results & Participation Section, Add a new action to ACTIONS button at the event level
to export results in xlsx format. This will read the data from the sqlite file
and convert it to xlsx so each table from the sqlite is a new tab at the xlsx.

- Issue: [#7533](https://github.com/sequentech/meta/issues/7533)

## 🐞 Text gets out of the Publish buttons in the admin portal

Fix overflow in the label's text of the Admin Portal's Publish tab when the text 
label of the Publish buttons are too long for a given language, i.e. spanish.
Also shortened the spanish translations.

- Issue: [#6806](https://github.com/sequentech/meta/issues/6806)

## 🐞 Voter actions are not logged

Voter actions were not being logged because they were published to a message queue
that didn't include the environment prefix.

- Issue: [#7561](https://github.com/sequentech/meta/issues/7561)

## ✨ Voting Booth: Security confirmation checkbox support

Add a security confirmation checkbox to the election Start Screen. Enable it from
the Election > Data > Advanced Configurations in the Admin Portal, then configure
it from  Election > Data > General translations section.

- Issue: [#7495](https://github.com/sequentech/meta/issues/7495)

## ✨ Voting Portal > Start Screen: Allow Showing Election Event Title instead of Election Title

The title of the Start Screen (Voting Portal) can be to either the election title or the Election Event Title. 
The default value is the Election title, so there is no action required by the admin.

This an be changed at election level > Data > Advanced Configuration.

- Issue: [#7252](https://github.com/sequentech/meta/issues/7252)


## ✨ Voting Portal Immutable Logs table

To enable the feature change the policy in Admin Portal at Election Event level, 
Data > Ballot Design > Show Cast Vote Logs Tab.
To see the Immutable logs of the type `CastVote` go to the Voting Portal landing page
/election-chooser > "Locate Your Ballot" button, there the tab LOGS should appear.

- Issue: [#6684](https://github.com/sequentech/meta/issues/6684)

## 🐞 Voting script for loadtesting takes screenshots when it shouldn't

The loadtesting script for voting with nightwatch was saving some screenshots
event when the screenshots option was disabled. This took a lot of space in the
tests, filling in the disk.

- Issue: [#7753](https://github.com/sequentech/meta/issues/7753)
