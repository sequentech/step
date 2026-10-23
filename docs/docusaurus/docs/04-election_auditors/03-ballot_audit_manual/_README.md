<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Ballot audit manual: how to write and maintain it

This folder is the source of the **Ballot Audit Manual**, for election auditors who run the
offline ballot audit tools (`packages/ballot-audit`) and the Braid verifier. This file is for
authors. Docusaurus does not publish files whose name starts with `_`.

## Structure

| Folder or file | Content | Sidebar |
| --- | --- | --- |
| `00-ballot_audit_manual.md` | Manual home: purpose, what the audit checks, the procedure table and the PDF link. | Section page |
| `01-before-you-start.md` | Order of the work, roles, terms, conventions, notices, safety rules and limits. | 1 |
| `02-procedures/` | The audit in order, from the workstation to the clean-up. | 2 |
| `03-read-the-results.md` | Exit statuses, lookup outcomes and exclusion reasons. | 3 |
| `04-troubleshooting.md` | Each message of the commands, with its cause and action. | 4 |

Each procedure page has the parts of the procedures of the administrator manual: a title that
starts with a verb, one paragraph, **Before you start**, numbered steps under `##` headings,
**Expected result**, **If there is a problem** and **Next**.

## Writing rules

Use the writing rules of the administrator manual (ASD-STE100, about 70 %), in
`docs/02-election_managers/_README.md`. Also:

- Write command names, options, file names and messages exactly as the tools show them, in
  `monospace`. Copy the messages from the source code of this branch.
- Give the exit status of each message in brackets: `[1]`.
- Keep the manual generic: no client, country or sector names.

## PDF

`yarn pdf` prints this manual to `build/pdf/sequent-ballot-audit-manual-<version>-en.pdf`. The
list of manuals is in `manuals.js`. Add a locale to `locales` there only when the translation of
all pages exists.
