---
id: release-next
title: Release Notes next
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

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
