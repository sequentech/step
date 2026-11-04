---
name: bsi-documentation
description: Write, review or change any document of the BSI Common Criteria evaluation BSI-DSZ-CC-1291 of the uniWAHL TOE, in step or beyond. Covers the guidance AGD-01 to AGD-04 (step docs under 02-election_managers/04-administrator_handbook and 05-election_organizer_handbook, 03-voters/02-voter_handbook, 05-reference/06-deployment_guide, 05-reference/08-delivery_verification.md), the Security Target, ADV, ALC, ATE and AVA evidence and the package pages. Use for every such edit, even a one-line fix or a reply to feedback on a guide, and when a TOE change makes a document wrong. Sets the mode (new document or surgical change to a delivered one) and the rules every text must pass.
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Writing BSI documents

Choose the mode, then apply the rules. An ITSEF observation report also needs `bsi-observation-report`.

## Modes

| | New document | Improving a delivered document |
|---|---|---|
| Applies when | No version went to the ITSEF | A version went to the ITSEF |
| Start from | The work units (CEM), the PP and ST requirements, and what the reader must do | The delivered text. Change only what the request or a requirement needs |
| Structure | Design it: who it is for, tasks by role, then reference tables | Keep it, so that the evaluator's comparison stays small. Move text only if the request asks |
| Delivered state | None yet | Frozen. Never edit its package page or `assets/<class>-<version>`. Work in a new package page and assets |
| Version | Start at 1.0 | One new version per package, not per commit. Do not bump documents the package does not change |
| History row | "First edition…" | One or two lines, about 100 characters, naming sections and not their content |
| Finish with | Coverage of each work unit against the document | A list of what changed for the package page, and a search for every other place that says the same thing |

Until a package is sent, keep improving that package's version: new requests do not make a new version.

## Rules for every text

**Truth**

- Check each claim about a state, message, permission, default or actor in the code before writing it. Copy screen texts from the translations, and confirm in the component that the screen uses them: a translation key can be unused. An old document, a plan, ADV or a requirements page is not evidence for what the code does, and newer sources win over older ones.
- If the ST or PP requires what the code does not do, change the TOE. If no requirement needs the claim, remove the claim. Never weaken a required claim to match the code.
- State only what happened: a script or template is not a release, a test or an approval. Say what was not compiled or not rehearsed.

**Minimal and complete**

- Each passage answers to a requirement or to a task of the reader. Otherwise remove it.
- Check the opposite too: AGD_PRE.1 and AGD_OPE.1 per role (functions, interfaces, security-relevant events, modes, assumptions, measures), and the PP's requirements against the document.
- No process without a home. When the text says to record, confirm or keep something, it says where and who reads it. Prefer what the TOE already records, and an existing record, such as the board's minutes, over a new one. If the step is not required, remove it.

**Readers and roles**

- Each guide starts with "Who this guide is for". Roles and terms are defined once in the first chapter, and the guides agree.
- A task belongs to the role that can do it. Do not give a server check to a portal administrator, or a board function to an administrator.
- Explain what looks strange: if a reader could ask "why is this required?", answer it in the text.

**Terms**

- One term, one meaning, defined in "Terms". A new term goes there the first time it is used. Search all guides when a term changes.
- Say "the TOE", not the part that implements it ("the database enforces").
- Say "election event" in the text, in screenshots and in their fixtures, never "Post".
- A screen is named with where it is: the side menu "Electoral Log", the tab "Logs" of the election event. Never an invented name such as "console".

**Quotes and names**

- A quoted screen text says what it is: the button "Save", the heading "Trusted issuers", the message "The certificate checks were saved.". A bare quotation is not acceptable. Paraphrase a screen's description instead of quoting it.
- A technical name says what it is: the permission "Self-tests: edit settings" (`self-test-settings-write`), the role `admin`, the file `image-digests.txt`.
- A hash appears in short form, such as `DEAD-BEEF`, unless the reader compares it. A comparison uses the complete value, with a copy icon on screen.

**References**

- Refer to another section with a link. `plugins/remark-section-numbers.js` adds the number to the link's text: "3 Secure Use of the Admin Portal", "AGD-04 7.4 Configure the realms". Plain-text section names get no number, so convert them.
- The link's text is the target's title, or the plugin appends "(section 7)".

**Layout**

- Numbered sections come from the plugin: no numbers in headings.
- Use tables, not long paragraphs. Give each role or concern its own subsection, with a short lead-in.
- A numbered list follows a sentence that ends in a colon, such as "To start voting:".
- Add a screenshot where the reader must find a screen. Use the real interface with synthetic data, no pixel editing, a `.license` file and the caption "*Illustration: … synthetic …*".

**Commands on staff computers**

- Avoid them: a small feature, such as a copy icon or a comparison in the portal, beats a command.
- If a command stays, the tool is in the computer's stated requirements (AGD-04 for the administration workstation), and the command was run.

## Consistency

A change is complete only when every document says the same:

1. Search AGD-01 to AGD-04, the ST, FSP, ARC, TDS, `11_Internal_Product_Requirements.md`, the open package page and `2_alc/9_Internal_To_Do.md`.
2. A TOE change updates its guide and ADV rows in the same pull request.
3. Do not bump ADV or ALC document versions until that document's package is prepared.

## AGD package

- Build clean, export to fresh directories and validate: the PDFs name the export commit, so integrate remote changes on beyond#615 by merge, never by rebase.
- Screen texts are italic through `plugins/remark-screen-text.js`.
- Run the guidance checks in the `step` docs: `node --test plugins/*.test.js`, `node scripts/test-export-guidance.cjs`, and the helper tests in `scripts/deployment`.

## Boundaries

- step is public: neutral wording, no security findings in commits or docs.
- Do not contact the ITSEF, publish or merge without Eduardo's authorization.
