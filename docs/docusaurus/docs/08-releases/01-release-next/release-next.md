---
id: release-next
title: Release Notes next
---
<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## ✨ Implement happy path for CLI
- Modify step CLI to run an election event from start to finish.
  Key commands:
    - Import election event
    - Create Publication (for both Election Event and Election)
    - Start/Stop/Pause (both Election Event and Election)
    - Create Keys Ceremony (including trustees part)
    - Create Tally Ceremony (including trustees part)
    - Download tally results 
    - Export election event
- Update the api-key-client to use the same mappers as the admin-portal client, 
  and fix gold authentication for api-key-client by falling back to the iat claim
  when auth_time is missing.

- Issue: [#6680](https://github.com/sequentech/meta/issues/6680)

## 🐞 Multi-Tenant login doesn't work

A loop blocked the multi-tenant login.

- Issue: [#9993](https://github.com/sequentech/meta/issues/9993)

## ✨ ✨ Investigating costs increase in infra cluster (GHA)

- Added ECR caching for docker artifacts
- Added cargo caching for efficient test builds
- Added 'sudo' to release-it installation

- Issue: [#9293](https://github.com/sequentech/meta/issues/9293)

## 🐞 Admin Portal > Can't send message to voters

Going to the Admin Portal > Election Event > Voters > Send generated an
unexpected error.

- Issue: [#9721](https://github.com/sequentech/meta/issues/9721)

## ✨ Reports > Add pagination to the electoral results report

Added pagination to the electoral results reports in the footer.

- Issue: [#9535](https://github.com/sequentech/meta/issues/9535)

## 🐞 Keycloak's custom event listener is not working

Electoral logs from keycloak, for example when a voter logs in/off, are not being
recorded. This happened because after the dependency updates, the inetum
authenticator keycloak extension was removed, but it contained the custom event
listener. This fix adds it back, only the custom event listener, not the inetum
part.

- Issue: [#9574](https://github.com/sequentech/meta/issues/9574)

## 🐞 Errors editing forms

After the dependency updates, the Election Data form had stopped working.

- Issue: [#9572](https://github.com/sequentech/meta/issues/9572)

## 🐞 Tally > "No Results" while loading the results

1. Fix Tally results show "No results" while loading for it.
2. Fix Starting new tally after review other tally results shows the previous tally results while processing the tally ceremony.

- Issue: [#8677](https://github.com/sequentech/meta/issues/8677)

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

## 🐞 Tally > Export option can't be read correctly if title is too long

Modify the tally export translations to show the format before the document name.

- Issue: [#8676](https://github.com/sequentech/meta/issues/8676)

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

## ✨ Documentation: Structural Changes

Documentation sidebar order has been restructured

- Issue: [#8672](https://github.com/sequentech/meta/issues/8672)

## 🐞 Voting Portal > Grace Period not applied if no scheduled event

Fix to allow voting if grace period applies when the status is checked.

- Issue: [#9091](https://github.com/sequentech/meta/issues/9091)
