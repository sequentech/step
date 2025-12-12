---
id: release-next
title: Release Notes next
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 🐞 Tally > Export option can't be read correctly if title is too long

Modify the tally export translations to show the format before the document name.

- Issue: [#8676](https://github.com/sequentech/meta/issues/8676)

## 🐞 Tally > "No Results" while loading the results
1. Fix Tally results show "No results" while loading for it.
2. Fix Starting new tally after review other tally results shows the previous tally results while processing the tally ceremony.

- Issue: [#8677](https://github.com/sequentech/meta/issues/8677)

## 🐞 Tally > Export option can't be read correctly if title is too long

Modify the tally export translations to show the format before the document name.

- Issue: [#8676](https://github.com/sequentech/meta/issues/8676)

## 🐞 Tally > "No Results" while loading the results
1. Fix Tally results show "No results" while loading for it.
2. Fix Starting new tally after review other tally results shows the previous tally results while processing the tally ceremony.

- Issue: [#8677](https://github.com/sequentech/meta/issues/8677)

## 🐞 Tally > Election aliases not used

Use election alias in all places in tally results.
Fix showing 'event' or 'election' instead of the actual election event or election
 name/alias.

- Issue: [#8426](https://github.com/sequentech/meta/issues/8426)

## 🐞 Fix Graphql Typescript issues

Update Graphql definitions in the admin-portal, which is required after a
bad merge from main.

- Issue: [#9540](https://github.com/sequentech/meta/issues/9540)

## 🐞 Keys Ceremony > State not cleared when switching Election Events

Fix keys ceremony state is not cleared when switching election events.

- Issue: [#8675](https://github.com/sequentech/meta/issues/8675)

## 🐞 Tally > State not cleared when switching events

Fix tally state is not cleared when switching election events on the tally tab.

- Issue: [#8674](https://github.com/sequentech/meta/issues/8674)
  
## 🐞 Username is shown after an attempted login with a valid username

When the Keycloak login flow used the step `Username Password Form - Allowing password expiration`,
the email was being shown at the top of the login page after a failed login
if the user existed, leaking the information that the user did exist.

- Issue: [#6476](https://github.com/sequentech/meta/issues/6476)

## 🐞 Can't filter voter logs by username

Fixed an issue that prevented to search logs by username in the Admin portal.

- Issue: [#7751](https://github.com/sequentech/meta/issues/7751)

## 🐞 Contest result extended metrics are 0

Fixes the extended metrics calculation that is visible in the json file of the 
tally result files in `velvet-generate-reports`.
It contains the value of some election metrics.

- Issue: [#8573](https://github.com/sequentech/meta/issues/8573)
