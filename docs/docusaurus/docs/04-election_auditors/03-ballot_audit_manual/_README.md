<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Ballot audit manual: how to write and maintain it

This folder is the source of the **Ballot Audit Manual**, for election auditors who run the
offline ballot audit tools (`packages/ballot-audit`) and the Braid verifier. This file is for
authors. Docusaurus does not publish files whose name starts with `_`.

The separate explanation is `../04-ballot_audit_explanation.md` (**What the Ballot Audit
Verifies**). Keep its links from the auditor introduction and manual home. When the tools
change, check its implementation pin, assurance limits and outcome interpretation against
the code. Keep the explanation separate from this manual's procedural PDF guide.

## Structure

| Folder or file | Content | Sidebar |
| --- | --- | --- |
| `00-ballot_audit_manual.md` | Manual home: purpose, what the audit checks, the procedure table and the PDF link. | Section page |
| `01-before-you-start.md` | Order of the work, roles, terms, versions, conventions, notices, safety rules and limits. | 1 |
| `02-procedures/` | The audit in order, from the workstation to the clean-up, with the audit image (`packages/ballot-audit/Dockerfile.prod` and `packages/ballot-audit/image/run-audit`): get the image, prepare the backups, one `docker run` command. | 2 |
| `03-read-the-results.md` | Exit statuses, the step statuses of `audit-tally`, lookup outcomes and exclusion reasons. | 3 |
| `04-troubleshooting.md` | Each message of the commands, with its cause and action. The messages of `audit-tally` come first. | 4 |
| `05-command-reference.md` | All options of each command and the fields of `scope.json` and `connections.json`. Copy them from the `Args` structs of this branch. | 5 |
| `06-file-reference.md` | The content of each file that the commands write, and which files contain personal data. Copy the fields from the code that writes each file. | 6 |
| `07-run-each-step/` | Appendix: the setup without the audit image, and each command of `audit-tally` run by hand for one scope, to repeat or examine one step. | 7 |

When the audit image changes, update the procedures, the audit image sections of
**Troubleshooting**, **Command Reference** and **File Reference**, and the table of the appendix
home page. Copy the messages from `packages/ballot-audit/image/run-audit`.

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

`yarn pdf` prints the **Ballot Audit Guide** to
`build/pdf/sequent-ballot-audit-guide-<version>-en.pdf`, and the download button of the
manual home links to it. The guide has the pages of `pages` in `manuals.js` (Restore the Database
Copies and Run the Audit), after its first page `scripts/pdf-intros/ballot-audit-guide.en.html`,
which sends the reader to the online manual for everything else. When you add, move or rename a
page that the first page links to, update its links. Add a locale to `locales` in `manuals.js`
only when the translation of all pages exists.
