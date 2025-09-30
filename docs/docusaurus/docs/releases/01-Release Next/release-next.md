---
id: release-next
title: Release Notes next
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 🐞 Duplicate Votes is slow

Remove slowness of the duplicate votes script by disabling within the insert
sql transaction some slow constrain.

- Issue: [#8475](https://github.com/sequentech/meta/issues/8475)


## 🐞 Inconsistencies in Voting Portal

Removed inconsistencies and bugs when selecting candidates, explicit blank,
null votes, undervotes, overvotes and with single/multi-contest encoding.

- Issue: [#8235](https://github.com/sequentech/meta/issues/8235)

## 🐞 Voting Portal: Avoid uneeded google chrome automatic translations

The Voting Portal and other frontends did not specify the page language, causing
browsers with automatic translation features to sometimes apply incorrect
translations.

- Issue: [#7983](https://github.com/sequentech/meta/issues/7983)

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

## ✨ Improve Dashboard print look

Improve the election event/election dashboard so that all necessary data (statistics)
are displayed correctly in print mode.

- Issue: [#7534](https://github.com/sequentech/meta/issues/7534)
