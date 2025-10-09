---
id: release-next
title: Release Notes next
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

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

## ✨ Videoconference links from Admin Portal

Added a Google Meet component with a button to generate a link in EVENT > DATA to
support creating a google meeting.
Created INTEGRATIONS tab in tenant settings to add the google api credentials.
Document on how to create credentials added to docusaurus in Admin Portal Tutorials.
