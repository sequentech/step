<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

## ✨ Automatically create super tenant

This change requires new env vars:  `AWS_S3_JWKS_CERTS_PATH`=certs.json and 
`ENV_SLUG`. `ENV_SLUG` should be the short name of the environment.

Also keycloak no longer needs to import realms at
 `/opt/keycloak/data/import`. Furthermore the file `certs.json` doesn't
 need to exist initially, as windmill-beat will automatically create it
 along with the first tenant.

## 🐞 Uncategorized error while casting ballot

Improve error handling on the Voting Portal when casting a vote. This
includes handling a Timeout, Excess Allowed Revotes, Voting in another
Area, Internal Server Error.

## 🐞 service-account-realm-management shouldn't appear as a voter

This fixes the issue where a service account appears in the voters list.
In order to deploy this in production, the configmap for the default
election event configuration needs to be changed.

## ✨ Add support retrieving master secret in an env variable

A new environment variable `MASTER_SECRET` has been added to use in DEV evironment instead of hashicorp.
`SECRETS_BACKEND` was updated to `SECRETS_BACKEND=EnvVarMasterSecret` accordingly.

This change should not affect production, there the value should be `SECRETS_BACKEND=AwsSecretManager`, more info in `.devcontainer/.env.development`.

The Braid Trustee service and its initialization script (`trustee.sh`) have been updated also support the env vars secrets backends.

## ✨ Read tally in frontend from Sqlite3

With this change, the admin portal starts reading the results directly
from the Sqlite3 file produced by the Tally. This makes it faster and
more scalable.

## ✨ Improve demo mode

With this change, the DEMO tiled background and the Demo warning dialog
will appear when entering the voting portal from the preview screen in the
admin portal. Also, the warning dialog will appear on the election start
screen rather than in the election chooser. This includes a fix so that
the demo background/dialog will only appear for elections that don't have
generated keys when voters login to the voting portal. Also, css classes
are added to the demo background and dialog to help custom styling.

## 🐞 Accessing tenant url after logging out does not show tenant selection page.

Previously, if you're logged in to the Admin Portal, and you logged out,
and then went to the /tenant page to select the tenant, the page didn't load
correctly the first time. This change fixes the issue.

## 🐞 Intermitten errors loading preview

Fix a race condition for calling WASM code when loading the voting portal that
was sometimes causing an error.

## 🐞 Voter actions are not logged

Voter actions were not being logged because they were published to a message queue
that didn't include the environment prefix.

## ✨ Voting Portal > Start Screen: Allow Showing Election Event Title instead of Election Title

The title of the Start Screen (Voting Portal) can be to either the election title or the Election Event Title. 
The default value is the Election title, so there is no action required by the admin.

This an be changed at election level > Data > Advanced Configuration.

## ✨ Tally - Add decoded ballot json to SQLite results database

With this change, it is possible now to include all the raw decoded ballots 
inside the sqlite database. It also moves part of the database generation 
inside velvet. This can be set at advanced config at the election event.

## ✨ Voting Booth: Security confirmation checkbox support

Add a security confirmation checkbox to the election Start Screen. Enable it from
the Election > Data > Advanced Configurations in the Admin Portal, then configure
it from  Election > Data > General translations section.

## 🐞 Can't cast vote

When an Election was created manually through the Admin Portal, the voting channels
column was left empty. This means voters couldn't cast their vote as the online
channel was not set active.

## 🐞 Tenant/Event keycloak configs have static secrets 

When a new tenant or event is created, some clients have secrets and they are 
being imported as-is. When creating/importing a new tenant/event, now the secrets are 
stripped from the config to be regenerated. 

## 🐞 Default language in the voting portal is not honored in preview mode

Previously the default language was not being selected when loading the Voting
Portal, now it is.

## ✨ Add automatic keys/tally ceremonies

Add a new Ceremonies Policy at the election event level.
This policy provides the option for a user to enable automatic key ceremonies
for a specific election or all elections. With this enabled, the tally will 
also be performed automatically, eliminating the need for trustee involvement.

## 🐞 Voters can't login to election events in new tenants

For security, secrets/certificates are generated randomly when creating a new
election event/tenant. However the secret for the service account of the tenant
should be set by the system as it is used internally. This is now set by
environment variables  `KEYCLOAK_CLIENT_ID` and `KEYCLOAK_CLIENT_SECRET`.

## ✨ Voting Portal Immutable Logs table

To enable the feature change the policy in Admin Portal at Election Event level, 
Data > Ballot Design > Show Cast Vote Logs Tab.
To see the Immutable logs of the type `CastVote` go to the Voting Portal landing page
/election-chooser > "Locate Your Ballot" button, there the tab LOGS should appear.

## 🐞 Keycloak voter logs are not recorded

Voter logs related to Keycloak (login, login error, code to token) were being 
published to the wrong rabbitmq queue. This has been fixed and now they are 
published to the queue for the respective environment.

## 🐞 Voting script for loadtesting takes screenshots when it shouldn't

The loadtesting script for voting with nightwatch was saving some screenshots
event when the screenshots option was disabled. This took a lot of space in the
tests, filling in the disk.

## 🐞 Error in Election name on Admin Portal Tally

The election name shown in the section "Results & Participation" of the Tally in
the Admin Portal is shown as 0 rather than its actual name.

## ✨ Create a task for deleting events

Event deletion is now managed as a task. It will report the result of the task if
it goes correctly or the errors if not.

## ✨ Standarize "Overseas voters turnout"

Rename report to "Voters Turnout" and remove "Overseas", "OV" or other non standard
 terminology from the report, admin portal and source code.
Create documents and tutorials about the voters tab, adding User Attributes to keycloak,
 or how to create reports and templates.

## ✨Tally - Add the option to export event tally results in xlsx format

In Results & Participation Section, Add a new action to ACTIONS button at the event level
to export results in xlsx format. This will read the data from the sqlite file
and convert it to xlsx so each table from the sqlite is a new tab at the xlsx.

## ✨ Configure election screen on closed event

Add a class name and the is-start attribute to the HTML of the select elections
 screen at the voting portal to control the CSS from the admin portal.
"is-start" attribute added to the SelectElection component in ui-essentials.

## 🐞 Multiple Finished Tally Ceremony logs

Finished Tally Ceremony logs were repeated multiple times (each time the task is
executed). Now the logs say "Updated Tally Ceremony" each time unless the
Tally is completed.

## ✨ Review UI accessibility

Using the IBM tool https://www.ibm.com/able/toolkit/tools/#develop , review the
accessibility for the Voting Portal and Ballot Verifier.

## 🐞 Text gets out of the Publish buttons in the admin portal

Fix overflow in the label's text of the Admin Portal's Publish tab when the text 
label of the Publish buttons are too long for a given language, i.e. spanish.
Also shortened the spanish translations.

## ✨ Improve tally events in the Activity Log

Improve the details reported in the activity logs by adding more information like
username, and reporting when the tally is started and completed.

## 🐞 Can't create tally

After adding indexes for the Electoral Log, the election_id field was limited to
the size of one UUID but multiple UUIDs were stored when the Keys Ceremony was
at Event level and there were multiple elections. Now only one or no election id is
saved.

## 🐞 Error message in voting portal should be a warning

Sometimes the voting portal shows an alert dialog but the ballot is a valid
ballot. In this case the dialog text should be different than in the case of
an invalid/blank ballot.

## 🐞Candidates list top border missing

On Candidate Lists for the Voting Portal, the top border was missing.

## 🐞 Issues filtering by voted/not voted

Filtering voters by whether they have voted or not wasn't working and a fix
was implemented.

## 🐞 Inconsistencies in Voting Portal

Removed inconsistencies and bugs when selecting candidates, explicit blank,
null votes, undervotes, overvotes and with single/multi-contest encoding.

## 🐞 Errors deleting Election Event

Fixed error deleting election event in specific cases. Also when an Election 
Event is created, the sidebar is automatically updated.

## ✨ Automatically generate tally documents after tally finishes

Added a post tally task that renders all the html reports to pdf. The pdfs are 
included into the event tar.gz file that can be downloaded from the Tally results page in Admin portal.

## 🐞 Further translation issues

In the Voting Portal, the lang HTML tag is set to English/en and it doesn't change even when changing the
language. This fixes the issue, which was triggering unwanted automatic translations, for example
translating to German pages that were already in German.

## 🐞 Tally shows as an Admin 1 election but as a Trustee it shows 2 elections

At the trustees tally ceremony, all elections were fetched instead of only those
selected to participate in the tally.

## ✨ Weighted voting for areas

Added a new election event policy at EVENT > DATA > Advanced configurations: `Weighted voting policy`.
When the policy is set to `Weighted Voting for Areas`, it allows assigning a weight
to each area. Tally results will then be calculated based on these weights, 
which are taken from the ballot style of each area defined at publication.
