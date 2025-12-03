---
id: release-next
title: Release Notes next
---
<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 🐞 Fix Graphql Typescript issues

Update Graphql definitions in the admin-portal, which is required after a
bad merge from main.

- Issue: [#9540](https://github.com/sequentech/meta/issues/9540)
- 
## 🐞 Can't filter voter logs by username

Fixed an issue that prevented to search logs by username in the Admin portal.

- Issue: [#7751](https://github.com/sequentech/meta/issues/7751)

## 🐞 Keys Ceremony > State not cleared when switching Election Events

Clear keys ceremony state when switching events.

- Issue [#8675](https://github.com/sequentech/meta/issues/8675)
