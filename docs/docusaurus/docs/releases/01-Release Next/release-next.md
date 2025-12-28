---
id: release-next
title: Release Notes next
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
## 🐞 Voting Portal > Candidate images broken after export then import the event
Fix export/import election event with s3 files include event public files.
Add `PUBLIC_BUCKET_URL` to `global-settings.json` in ballot-verifier publuc folder
 to support displaying images from the public bucket.

- Issue: [#9087](https://github.com/sequentech/meta/issues/9087)

## 🐞 Tally > Export option can't be read correctly if title is too long

Modify the tally export translations to show the format before the document name.

- Issue: [#8676](https://github.com/sequentech/meta/issues/8676)

## 🐞 Tally > "No Results" while loading the results
1. Fix Tally results show "No results" while loading for it.
2. Fix Starting new tally after review other tally results shows the previous tally results while processing the tally ceremony.

- Issue: [#8677](https://github.com/sequentech/meta/issues/8677)

## 🐞 Fix Graphql Typescript issues

Update Graphql definitions in the admin-portal, which is required after a
bad merge from main.

- Issue: [#9540](https://github.com/sequentech/meta/issues/9540)

## 🐞 Keys Ceremony > State not cleared when switching Election Events

Clear keys ceremony state when switching events.

- Issue [#8675](https://github.com/sequentech/meta/issues/8675)

## 🐞 Tally > State not cleared when switching events

Fix tally state is not cleared when switching election events on the tally tab.

- Issue: [#8674](https://github.com/sequentech/meta/issues/8674)
  
## 🐞 Username is shown after an attempted login with a valid username

When the Keycloak login flow used the step `Username Password Form - Allowing password expiration`,
the email was being shown at the top of the login page after a failed login
if the user existed, leaking the information that the user did exist.

- Issue: [#6476](https://github.com/sequentech/meta/issues/6476)

## 🐞 Tally UI shows manual and executes automatic after policy switch

Now tally view checks if it's an automatic ceremony based on only the keys ceremony policy.

- Issue: [#8472](https://github.com/sequentech/meta/issues/8472)

## 🐞 Can't filter voter logs by username

Fixed an issue that prevented to search logs by username in the Admin portal.

- Issue: [#7751](https://github.com/sequentech/meta/issues/7751)
