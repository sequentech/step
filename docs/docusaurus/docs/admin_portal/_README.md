<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Election administrator manual: how to write and maintain it

This folder is the source of the **Election Administrator Manual**. This file is for authors.
Docusaurus does not publish files whose name starts with `_`.

## Structure

The manual is the **Election Managers** section of the documentation (`docs/admin_portal/`). The
sidebar is explicit, in the category "Election Managers" of `docs/docusaurus/sidebars.js`. In
release 9.0 the page URLs keep the 9.0 layout: do not rename or move the files.

| Folder or file | Content |
| --- | --- |
| `election_management.md` | Manual home: purpose, audience, the procedure table and the PDF link. |
| `00-before-you-start.md` | Before You Start: the order of the work, roles, terms, conventions and the safety notices. Read first. |
| `procedures/` | The main procedures, numbered in the order of an election: `01-tenant`, `02-event`, `03-voters`, `04-keys`, `05-publish`, `06-tally`, `07-results`. |
| `Tutorials/` | One task in depth. When a procedure covers the task, the tutorial links to it and gives the depth. |
| `Reference/basic_navigation.md` | The parts of the screen and the menu. |
| `Reference/User-Manual/` | One page for each screen or tab of the admin portal, with all fields, buttons and messages. One folder for each page: `Election-Management/` (election event, election, contest and candidate tabs), `Settings/`, `Users-and-Roles/` and `Templates/`. The overview pages link to the child pages. |

Each procedure page has the same parts, in this order:

1. A title: `Run the Key Ceremony`.
2. One paragraph: what the procedure does and who does it.
3. **Before you start**: the role, the preconditions and what to have.
4. Numbered steps, grouped under `##` headings, one action in each step.
5. **Expected result** after each group of steps: what the user sees when the step is correct.
6. **If there is a problem**: a table with the problem and the action.
7. **More information**: links to the Tutorials and Reference pages of the procedure.
8. **Next**: the link to the next procedure.

Each reference page has: what the screen is for, how to open it, the permissions in plain
words, the fields, columns and buttons with their exact labels, short steps for its actions,
and an **If there is a problem** table with the real messages.

## Versions

Each branch has its own manual, for the admin portal of that branch. The **Documentation**
workflow publishes `main` at `https://docs.sequentech.io/docusaurus/main/` and each release
branch at `https://docs.sequentech.io/docusaurus/release/<X.Y>/`. The version menu of the site
links these sites; the list is in `docs-version.js`.

- Write a change in each branch where it applies. Check labels in
  `packages/admin-portal/src/translations/en.ts` of that branch.
- When a release branch is cut, add it to `docs-version.js` on `main` and on the release
  branches in service.

## Languages

English is the source language. A translated page goes to
`i18n/<locale>/docusaurus-plugin-content-docs/current/` with the same path as the English
page. A page without a translation shows the English text. Run
`yarn write-translations --locale es` to update the labels of the navigation and the sidebar.

## PDF

`yarn build` followed by `yarn pdf` prints the manual for each language to
`build/[<locale>/]pdf/sequent-admin-manual-<version>-<locale>.pdf`. The page order of the PDF is
the order of the sidebar. The `<ManualPdfLink />` component links to the PDF of the language on
screen. The **Documentation** workflow runs both commands.

## Writing rules (ASD-STE100, adapted)

The manual follows the ASD-STE100 Simplified Technical English rules where they make the
text clearer for election staff. We apply about 70 % of the standard: we keep the rules on
sentences, voice and structure, but we permit technical words and product labels that are
not in the STE dictionary.

### Words

- Use one word for one meaning. Use the terms in the glossary of
  [Before You Start](00-before-you-start.md) and nothing else: *election event*, not *event* in one
  place and *election process* in another.
- Write the labels of the admin portal exactly as the screen shows them, in **bold**:
  click **Create Key Ceremony**.
- Use simple verbs: *click, select, type, open, make sure, do, start, stop, wait,
  download, keep*. Do not use *utilize, perform, leverage, ensure, initiate*.
- Do not use *-ing* forms as nouns or adjectives when another form is possible:
  "Before you publish", not "Before publishing".
- Do not use phrasal verbs when a simple verb exists: *start*, not *kick off*.
- Do not use contractions, slang or idioms.

### Sentences

- Procedural sentences (instructions): **20 words or fewer**.
- Descriptive sentences: **25 words or fewer**.
- One instruction in each step. If the user must do two things, make two steps.
- Write instructions in the imperative: "Type the threshold". Put a condition first:
  "If the status is **Failed**, click **Restart**."
- Use the active voice. Use the passive voice only when the actor is not known or not
  important.
- Use articles (*the, a*) and keep paragraphs to six sentences or fewer.

### Notices

Put a notice **before** the step it applies to.

| Notice | Use |
| --- | --- |
| `:::danger WARNING` | Risk to the secrecy, integrity or availability of the election. Example: the loss of a key. |
| `:::caution CAUTION` | Risk of data loss or of an action that you cannot reverse. |
| `:::note` | Useful information that is not a risk. |

Start a warning or a caution with the instruction, then give the reason:
"Keep two copies of the key file. If all trustees lose their files, nobody can decrypt
the votes."

### Generic text

The manual is for all clients and all types of elections. Do not write the name of a
client, a country or a sector. Use neutral examples ("Election 1", "Area North").
