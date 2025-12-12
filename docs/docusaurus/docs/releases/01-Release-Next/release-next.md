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

## ✨ Instant-runoff Voting (IRV/RCV) System support

Support for Instant-runoff elections. Adaptations and implementations were added:
- Velvet tally and make the tally operation configurable.
- Admin portal, selectable counting algorithm at contest level, UI tally results and results report.
- Voting portal and ballot verifier for preferential order.
- Ballot codec for preferential order.
- Layout documents in docusaurus of the Velvet crate for developers.
- Description on how to configure the tally operation in annotations.
- Document in election management at contest level.

- Issue: [#8214](https://github.com/sequentech/meta/issues/8214)

## ✨ Delegate voting with imports

Implemented a simple version of delegate voting. Voters can delegate their vote 
on other voters, and initially the state of delegation will be imported. 
Then at the tally level if a voter is a delegate with N weight, we'll insert the 
ballot N times.

- Issue: [#7683](https://github.com/sequentech/meta/issues/7683)

## 🐞 Keys Ceremony > State not cleared when switching Election Events

Clear keys ceremony state when switching events.

- Issue [#8675](https://github.com/sequentech/meta/issues/8675)

## 🐞 Can't filter voter logs by username

Fixed an issue that prevented to search logs by username in the Admin portal.

- Issue: [#7751](https://github.com/sequentech/meta/issues/7751)

## 🐞 Tally > Election aliases not used

Use election alias in all places in tally results.
Fix showing 'event' or 'election' instead of the actual election event or election
 name/alias.

- Issue: [#8426](https://github.com/sequentech/meta/issues/8426)


## 🐞 Error with tenants and templates in Admin portal.

Fixed issues that prevented tenant creation and template creation and deletion 
in the Admin portal.

- Issue: [#9539](https://github.com/sequentech/meta/issues/9539)

## 🐞 Tally UI shows manual and executes automatic after policy switch

Now tally view checks if it's an automatic ceremony based on only the keys ceremony policy.

- Issue: [#8472](https://github.com/sequentech/meta/issues/8472)
