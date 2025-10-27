---
id: release-9.0.1
title: Release Notes v9.0.1
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 🐞 Can't see Election Lists

Due to a recent change, a bug was introduced that hid Candidate Lists in the
Voting Portal.

- Issue [#8735](https://github.com/sequentech/meta/issues/8735)

## 🐞 Admin Portal > "Something went wrong" error when switching between diferent elections/questions

Prevent error when switching between elections on the "Data" tab by safely
 handling an undefined record.

- Issue: [#8725](https://github.com/sequentech/meta/issues/8725)

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

## 🐞 Further translation issues

In the Voting Portal, the lang HTML tag is set to English/en and it doesn't
change even when changing the language. This fixes the issue, which was
triggering unwanted automatic translations, for example translating to German
pages that were already in German.

- Issue: [#8470](https://github.com/sequentech/meta/issues/8470)

## 🐞 Velvet test errors

A failing velvet test was identified due to a recent change: ballots exceeding 
the maximum allowed votes are now classified as invalid. Since this behavior was
not previously enforced, the corresponding test required an update.

- Issue: [#8526](https://github.com/sequentech/meta/issues/8526)

## 🐞 Tally shows as an Admin 1 election but as a Trustee it shows 2 elections

At the trustees tally ceremony, all elections were fetched instead of only those
selected to participate in the tally.

- Issue: [#7584](https://github.com/sequentech/meta/issues/7584)

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

## 🐞 Admin Portal > Sidebar: Fix left and right margins in tenant & election event actions

- Issue: [#8527](https://github.com/sequentech/meta/issues/8527)