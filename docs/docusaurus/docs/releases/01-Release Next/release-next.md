---
id: release-next
title: Release Notes next
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
## 🐞 Tally > Election aliases not used

Use election alias in all places in tally results.
Fix showing 'event' or 'election' instead of the actual election event or election
 name/alias.

- Issue: [#8426](https://github.com/sequentech/meta/issues/8426)

## 🐞 Tally > State not cleared when switching events

Fix tally state is not cleared when switching election events on the tally tab.

- Issue: [#8674](https://github.com/sequentech/meta/issues/8674)
  
## 🐞 Username is shown after an attempted login with a valid username

When the Keycloak login flow used the step `Username Password Form - Allowing password expiration`,
the email was being shown at the top of the login page after a failed login
if the user existed, leaking the information that the user did exist.

- Issue: [#6476](https://github.com/sequentech/meta/issues/6476)

