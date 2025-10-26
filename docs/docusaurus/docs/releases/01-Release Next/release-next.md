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

##🐞 Tally > State not cleared when switching events

Fix tally state is not cleared when switching election events on the tally tab.

- Issue: [#8674](https://github.com/sequentech/meta/issues/8674)
  
## 🐞 Username is shown after an attempted login with a valid username

When the Keycloak login flow used the step `Username Password Form - Allowing password expiration`,
the email was being shown at the top of the login page after a failed login
if the user existed, leaking the information that the user did exist.

- Issue: [#6476](https://github.com/sequentech/meta/issues/6476)
