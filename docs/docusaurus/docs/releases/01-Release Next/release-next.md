---
id: release-next
title: Release Notes next
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## ✨ Automatically generate tally documents after tally finishes

Added a post tally task that renders all the html reports to pdf. The pdfs are 
included into the event tar.gz file that can be downloaded from the Tally results page in Admin portal.

- Issue: [#7948](https://github.com/sequentech/meta/issues/7948)

## 🐞 Inconsistencies in Voting Portal

Removed inconsistencies and bugs when selecting candidates, explicit blank,
null votes, undervotes, overvotes and with single/multi-contest encoding.

- Issue: [#8235](https://github.com/sequentech/meta/issues/8235)

## ✨ Standarize "Overseas voters turnout"

Rename report to "Voters Turnout" and remove "Overseas", "OV" or other non standard
 terminology from the report, admin portal and source code.
Create documents and tutorials about the voters tab, adding User Attributes to keycloak,
 or how to create reports and templates.

- Issue: [#7532](https://github.com/sequentech/meta/issues/7532)

## 🐞 Tally shows as an Admin 1 election but as a Trustee it shows 2 elections

At the trustees tally ceremony, all elections were fetched instead of only those
selected to participate in the tally.

- Issue: [#7584](https://github.com/sequentech/meta/issues/7584)
