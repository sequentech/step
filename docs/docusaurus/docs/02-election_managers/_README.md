<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Election administrator manual: how to write and maintain it

This folder is the source of the **Election Administrator Manual**. This file is for authors.
Docusaurus does not publish files whose name starts with `_`.

## Structure

The manual is the **Election Managers** section of the documentation (`docs/02-election_managers/`).

| Folder or file | Content | Sidebar |
| --- | --- | --- |
| `01-election_management.md` | Manual home: purpose, audience, the procedure table and the PDF link. | Section page |
| `00-before-you-start.md` | **Before You Start**: roles, terms, conventions and the safety notices. Read first. | 1 |
| `03-procedures/` | The main path of an election, in order: **Set Up the Tenant**, **Create the Election Event**, **Run the Key Ceremony**, **Publish and Manage the Voting Period**, **Run the Tally Ceremony**, **Get the Results**. | 2 |
| `01-tutorials/` | One task in depth: all options, statuses, edge cases and errors. A tutorial on a topic of a procedure links to the procedure and does not repeat its steps. | 3 |
| `02-reference/` | One page for each screen of the admin portal, with all fields and options. | 4 |
| `02-results-website.md`, `03-support-materials.md` | Optional features. | 5, 6 |

The folder and file names keep the URLs of earlier versions of the documentation. Do not rename
them: the number prefix does not set the order of the sidebar (the `position` in
`_category_.yml` and `sidebar_position` do), and a new name changes the URL of each page.

Each procedure page has the same parts, in this order:

1. A title in Title Case that starts with a verb, like the other pages of the documentation:
   `Run the Key Ceremony`. Reference pages use the name of the screen (`Trustees`), tutorials
   the name of the task (`Tally Ceremony`, `Import Voters`).
2. One paragraph: what the procedure does and who does it.
3. **Before you start**: the role, the preconditions and what to have.
4. Numbered steps, grouped under `##` headings, one action in each step.
5. **Expected result** after each group of steps: what the user sees when the step is correct.
6. **If there is a problem**: a table with the problem and the action.
7. **Next**: the link to the next procedure.

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
