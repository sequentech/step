---
id: release-next
title: Release Notes next
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 🐞 Failed scheduled event

Scheduled events and reports were being executed multiple times due to a timing 
mismatch between the beat scheduler's polling interval (10 seconds by default) 
and the look-ahead window used by the tasks (hardcoded to 60 seconds). This 
caused the same events/reports within the 60-second window to be repeatedly 
discovered and queued on each 10-second poll. 

The fix passes the configured schedule interval (schedule_events_interval and 
schedule_reports_interval) from the beat to the task functions, which now use it
as their look-ahead window instead of the hardcoded 60 seconds, ensuring each 
scheduled item is processed exactly once.

Also now admin users can schedule start/stop election events, as for those cases
selecting an election is not required.

- Issue: https://github.com/sequentech/meta/issues/8681

## ✨ Videoconference links from Admin Portal

Added a Google Meet component with a button to generate a link in EVENT > DATA to
support creating a google meeting.
Created INTEGRATIONS tab in tenant settings to add the google api credentials.
Document on how to create credentials added to docusaurus in Admin Portal Tutorials.
A new permission `google-meet-link` needs to be added manually in Keycloak, the procedure followed is:

1. Go to realm roles in the tenant realm (i.e. dev) and click on `Create role`
2. Add the role to the list
3. Then Go to `Groups` and choose `admin` group name
4. Go to `role mapping` and click on `Assign role` and add those permissions

- Issue: [#8189](https://github.com/sequentech/meta/issues/8189)

## 🐞 Investigate rabbitmq issues

The Electoral Log Windmill maintains a RabbitMQ connection, but sometimes it
gets disconnected and Windmill didn't try reconnecting. Moreover, the probe
didn't check the connection status. This fixes the issue by checking the
connection status and reconnecting if necessary and checking the status of the
connection in the probe.

- Issue [#8626](https://github.com/sequentech/meta/issues/8626)

## 🐞 Can't export voters list

In specific cases of Election Events with hundreds of areas and elections and
millions of voters, exporting voters failed because of an issue with logging
a specific function.

- Issue [#8622](https://github.com/sequentech/meta/issues/8622)

## 🐞 Admin Portal > Tally > Actions Popup Menu doesn't close after click

Within the `Results & Participation` section of the Tally tab of the Admin
Portal, when clicking in some action item inside the Actions Popup Menu for
Elections, the Popup Menu didn't automatically close and also in some cases it
moved to the bottom right corner.

- Issue: [#8614](https://github.com/sequentech/meta/issues/8614)

## 🐞 Admin Portal > Import Election Event: Password Dialog doesn't auto focus

When importing an election event that is encrypted, a dialog pops up asking for
the password. But the password field doesn't autofocus so the admin user has to
click on it.

Additionally, when an error is shown in the import election event dialog, it
will reappear when closing and reopening the import drawer.

- Issue: [#8613](https://github.com/sequentech/meta/issues/8613)

## 🐞 Keycloak Election ids are not filtered by area

When a voter logs in and the voter is not assigned any election, keycloak adds 
all election ids to the header. However only some election ids are actually 
related to the user area and only those should be included.

- Issue: [#8593](https://github.com/sequentech/meta/issues/8593)

## 🐞 Keycloak: Redirect To Registration Authenticator doesn't work when `http-relative-path` is set

Keycloak: Redirect authenticator doesn't work when http-relative-path is set.
The reason is that the http-relative-path is set twice, `/auth` appears twice in
the URL.

- Issue: [#8574](https://github.com/sequentech/meta/issues/8574)

## 🐞 Keycloak: Deferred authenticator in Login mode ask for password confirmation

When using the Deferred Authenticator in Login mode, it was asking for password
confirmation and it was not checking that the password matches that of the user.

- Issue: [#7585](https://github.com/sequentech/meta/issues/7585)

## 🐞 Voting Portal: Invalid/BlankVote Candidates do not follow sort order

Voting Portal: Invalid/BlankVote Candidates do not follow sort order within the
top/bottom invalid candidates block.

- Issue: [#8528](https://github.com/sequentech/meta/issues/8528)

### 🐞 Invalid Vote Position was not configurable in Admin Portal > Candidate

Added Invalid Vote Position configuration in Admin Portal > Candidate. This was
already in the backend, but it was not configurable in the Admin Portal.

- Issue: [#8528](https://github.com/sequentech/meta/issues/8528)

## 🐞 Admin Portal > Sidebar: Fix left and right margins in tenant & election event actions

- Issue: [#8527](https://github.com/sequentech/meta/issues/8527)

## ✨ Automatically generate tally documents after tally finishes

Added a post tally task that renders all the html reports to pdf. The pdfs are 
included into the event tar.gz file that can be downloaded from the Tally results page in Admin portal.

- Issue: [#7948](https://github.com/sequentech/meta/issues/7948)

## 🐞 Inconsistencies in Voting Portal

Removed inconsistencies and bugs when selecting candidates, explicit blank,
null votes, undervotes, overvotes and with single/multi-contest encoding.

- Issue: [#8235](https://github.com/sequentech/meta/issues/8235)

## ✨ Standarize "Overseas voters turnout"

Rename report to "Voters Turnout" and remove "Overseas", "OV" or other non standard
 terminology from the report, admin portal and source code.
Create documents and tutorials about the voters tab, adding User Attributes to keycloak,
 or how to create reports and templates.

- Issue: [#7532](https://github.com/sequentech/meta/issues/7532)

## 🐞 Tally shows as an Admin 1 election but as a Trustee it shows 2 elections

At the trustees tally ceremony, all elections were fetched instead of only those
selected to participate in the tally.

- Issue: [#7584](https://github.com/sequentech/meta/issues/7584)

## ✨ Move release notes to Docusaurus

Moved developer release notes to Docusaurus. Updated release notes for various
existing versions, `v8.7.5`, `v8.7.6`, `v9.1.0` and `v9.1.1`.

## 🐞 Velvet test errors

A failing velvet test was identified due to a recent change: ballots exceeding 
the maximum allowed votes are now classified as invalid. Since this behavior was
not previously enforced, the corresponding test required an update.

- Issue: [#8526](https://github.com/sequentech/meta/issues/8526)

## ✨ Early voting for child areas

Add per-area Early Voting policy with UI checkbox (allowed only if the Election
Event allows the EARLY_VOTING channel) and adapt import/export/upsert support.
Backend now stores EarlyVotingPolicy and area presentation; publications use
area_presentation on BallotStyle.
EARLY_VOTING can be allowed and started at event level, its lifecycle is governed
by Online (auto-closes on Online start/close; cannot start after Online begins),
while Kiosk remains independent.
Publish UI consolidates per-channel actions in a dropdown for every start/pause/stop button;
Voting Portal and Harvest endpoints honor early voting only for voters in enabled areas
when the channel is started.

- Issue: [#7681](https://github.com/sequentech/meta/issues/7681)

## ✨ Improve Dashboard print look

Improve the election event/election dashboard so that all necessary data (statistics)
are displayed correctly in print mode.

- Issue: [#7534](https://github.com/sequentech/meta/issues/7534)

## ✨ Weighted voting for areas

Added a new election event policy at EVENT > DATA > Advanced configurations: `Weighted voting policy`.
When the policy is set to `Weighted Voting for Areas`, it allows assigning a weight
to each area. Tally results will then be calculated based on these weights, 
which are taken from the ballot style of each area defined at publication.

- Issue: [#7682](https://github.com/sequentech/meta/issues/7682)

## ✨ Electoral results charts/visualization

Added Charts in the Admin Portal's Tally Results below the data tables to display
 the General Information, Participation Results and Candidate Results.

- Issue: [#7531](https://github.com/sequentech/meta/issues/7531)

## 🐞 Tally > "No Results" while loading the results
1. Fix Tally results show "No results" while loading for it
2. Fix Starting new tally after review other tally results shows the previous tally results while process the tally ceremony

- Issue: [#8677](https://github.com/sequentech/meta/issues/8677)
