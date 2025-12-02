---
id: release-next
title: Release Notes next
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 🐞 Tally > "No Results" while loading the results
1. Fix Tally results show "No results" while loading for it.
2. Fix Starting new tally after review other tally results shows the previous tally results while processing the tally ceremony.

- Issue: [#8677](https://github.com/sequentech/meta/issues/8677)

## 🐞 Tally > State not cleared when switching events

Fix tally state is not cleared when switching election events on the tally tab.

- Issue: [#8674](https://github.com/sequentech/meta/issues/8674)
  
## 🐞 Username is shown after an attempted login with a valid username

When the Keycloak login flow used the step `Username Password Form - Allowing password expiration`,
the email was being shown at the top of the login page after a failed login
if the user existed, leaking the information that the user did exist.

- Issue: [#6476](https://github.com/sequentech/meta/issues/6476)

