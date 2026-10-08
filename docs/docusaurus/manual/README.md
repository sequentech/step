<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Election administrator manual: how to write and maintain it

This folder is the source of the **Election Administrator Manual** that is published at
`/manual/` on the documentation site. This file is for authors. It is not published.

## Structure

| Folder or file | Content |
| --- | --- |
| `index.md` | Manual home: purpose, audience, the procedure table and the PDF link. |
| `00-before-you-start.md` | Roles, concepts, conventions and the safety notices. Read first. |
| `01-procedures/` | The main procedures, numbered in the order of an election: `01-TENANT`, `02-EVENT`, `03-KEYS`, `04-PUBLISH`, `05-TALLY`, `06-RESULTS`. |
| `02-more-procedures/` | Procedures for optional features (voter import, communication, login methods and more). |
| `03-reference/` | One page for each screen of the admin portal, with all fields and options. |

Each procedure page has the same parts, in this order:

1. A title with the procedure code: `03-KEYS: Run the key ceremony`.
2. One paragraph: what the procedure does and who does it.
3. **Before you start**: the role, the preconditions and what to have.
4. Numbered steps, grouped under `##` headings, one action in each step.
5. **Expected result** after each group of steps: what the user sees when the step is correct.
6. **If there is a problem**: a table with the problem and the action.
7. **Next**: the link to the next procedure.

## Versions

The manual has a version for each release line that is in service. `manual/` is the
*Next* version (the `main` branch). The released versions are in
`manual_versioned_docs/version-<X.Y>/`, with their sidebars in `manual_versioned_sidebars/`
and the list in `manual_versions.json`.

- Write a change that applies to a released version in the folder of that version too.
- Make a new version when a release branch is cut:
  `yarn docusaurus docs:version:manual <X.Y>`, then set `lastVersion` in
  `docusaurus.config.js`.
- Describe only what the admin portal of that release line shows. Check labels in
  `packages/admin-portal/src/translations/en.ts` on the release branch.

## Languages

English is the source language. A translated page goes to
`i18n/<locale>/docusaurus-plugin-content-docs-manual/<version>/` with the same path as
the English page (`current` for *Next*, `version-10.0` for 10.0). A page without a
translation shows the English text. Run `yarn write-translations --locale es` to update the
labels of the navigation and the sidebar.

## PDF

`yarn build` followed by `yarn pdf` prints one PDF for each version and language to
`build/[<locale>/]pdf/`. The page order of the PDF is the order of the sidebar. The
`<ManualPdfLink />` component links to the PDF of the version and language on screen.
The **Documentation Preview** workflow runs both commands.

## Writing rules (ASD-STE100, adapted)

The manual follows the ASD-STE100 Simplified Technical English rules where they make the
text clearer for election staff. We apply about 70 % of the standard: we keep the rules on
sentences, voice and structure, but we permit technical words and product labels that are
not in the STE dictionary.

### Words

- Use one word for one meaning. Use the terms in the glossary of
  [00-START](00-before-you-start.md) and nothing else: *election event*, not *event* in one
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
