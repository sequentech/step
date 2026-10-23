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
| `01-before-you-start.md` | Order of the work, roles, terms, versions, conventions, notices, safety rules and limits. | 1 |
| `02-procedures/` | The audit in order, from the workstation to the clean-up, with the audit kit (`packages/ballot-audit/kit`): the audit image, `audit-env up`, `audit-env tally` and `audit-env down`. | 2 |
| `03-read-the-results.md` | Exit statuses, the step statuses of `audit-tally`, lookup outcomes and exclusion reasons. | 3 |
| `04-troubleshooting.md` | Each message of the commands, with its cause and action. The messages of `audit-tally` come first. | 4 |
| `05-command-reference.md` | All options of each command and the fields of `scope.json` and `connections.json`. Copy them from the `Args` structs of this branch. | 5 |
| `06-file-reference.md` | The content of each file that the commands write, and which files contain personal data. Copy the fields from the code that writes each file. | 6 |
| `07-run-each-step/` | Appendix: the setup without the audit kit, and each command of `audit-tally` run by hand for one scope, to repeat or examine one step. | 7 |

When `audit-env` changes, update the procedures, the `audit-env` sections of **Troubleshooting**,
**Command Reference** and **File Reference**, and the table of the appendix home page. Copy the
messages from `packages/ballot-audit/kit/audit-env`.

When `audit-tally` changes, update **Run the Audit**, the `audit-tally` sections of **Read the
Results**, **Troubleshooting**, **Command Reference** and **File Reference**, and the appendix
pages of the commands that it runs.

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
