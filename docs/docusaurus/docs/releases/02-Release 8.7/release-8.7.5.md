---
id: release-8.7.5
title: Release Notes v8.7.5
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 🐞 Can't cast vote

When an Election was created manually through the Admin Portal, the voting channels
column was left empty. This means voters couldn't cast their vote as the online
channel was not set active.

- Issue: [#7631](https://github.com/sequentech/meta/issues/7631)
- Main commit: `8d649f527`
- Cherry-picks:
  - release/8.7: `369a1513b`
  - release/9.0: `089b80eaa`
  - release/9.1: `4d296a353`

## 🐞 Default language in the voting portal is not honored in preview mode

Previously the default language was not being selected when loading the Voting
Portal, now it is.

- Issue: [#7529](https://github.com/sequentech/meta/issues/7529)
- Main commit: `d1f45b9b3`
- Cherry-picks:
  - release/8.7: `07f7c2b6c`
  - release/9.0: `4c2d7f5fd`
  - release/9.1: `98550d7a5`


## 🐞 Tenant/Event keycloak configs have static secrets 

When a new tenant or event is created, some clients have secrets and they are 
being imported as-is. When creating/importing a new tenant/event, now the secrets are 
stripped from the config to be regenerated. 

- Issue: [#7002](https://github.com/sequentech/meta/issues/7002)
- Main commit: `3a5830719`
- Cherry-picks:
  - release/8.7: `7938243a8`
  - release/9.0: `a188a68f0`
  - release/9.1: `9718dface`