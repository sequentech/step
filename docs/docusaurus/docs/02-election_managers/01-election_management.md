---
id: election_management
title: Election Administrator Manual
sidebar_label: Introduction
sidebar_position: 0
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

import ManualPdfLink from '@site/src/components/ManualPdfLink';

# Election Administrator Manual

This manual tells election administrators and trustees how to prepare, run and close an
election with the Sequent Online Voting admin portal, the next version (in development).

<ManualPdfLink />

## How to use this manual

1. Read [00-START](00-before-you-start.md) first. It gives the roles, the terms and the safety rules.
2. Do the procedures in the order of the table. Each procedure tells you what you must have
   before you start.
3. Use the [Tutorials](01-tutorials/07-admin_portal_tutorials_create-voters.md) part when you need
   one task in depth: all its options, statuses and errors.
4. Use the [Reference](02-reference/02-election-event/01-election_management_election-event_dashboard.md)
   part when you need the details of a screen.

## Procedures

| Code | Procedure | Who | Result |
| --- | --- | --- | --- |
| [00-START](00-before-you-start.md) | Before you start: roles, terms and safety rules | All | You know who does each task. |
| [01-TENANT](03-procedures/01-tenant.md) | Set up the tenant: settings, users, roles and trustees | Tenant administrator | The administrators and trustees can sign in. |
| [02-EVENT](03-procedures/02-event.md) | Create the election event: elections, contests, candidates, areas and voters | Election administrator | The ballot content and the voter list are complete. |
| [03-KEYS](03-procedures/03-keys.md) | Run the key ceremony | Election administrator and trustees | The election public key exists. Each trustee keeps a key fragment. |
| [04-PUBLISH](03-procedures/04-publish.md) | Publish the ballot and open, pause and close the voting period | Election administrator | Voters can vote. Then the voting period is closed. |
| [05-TALLY](03-procedures/05-tally.md) | Run the tally ceremony | Election administrator and trustees | The votes are decrypted and counted. |
| [06-RESULTS](03-procedures/06-results.md) | Get the results, the reports, and publish the results website | Election administrator | You have the result documents. The public can see the results. |

## Versions and languages

This manual is for the version of the platform in the version menu at the top of the page. To
read the manual of another version, select it in the version menu. If you do not know the
version of your platform, ask Sequent support.

Use the language menu to select a language. A page that is not translated shows in English.
