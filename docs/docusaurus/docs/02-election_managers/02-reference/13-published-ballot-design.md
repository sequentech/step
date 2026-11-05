---
id: published_ballot_design
title: Published Ballot Design
sidebar_position: 13
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The ballot a voter sees is the ballot design of the last **publication** of the
Election Event, not the settings the Admin Portal holds at that moment. This
page says what a publication keeps, what the Voting Portal draws from it, and
how to show that the two match.

## What a publication keeps

**Publish Changes** generates a publication: a copy of the Election Event, of
each Election and of the ballot of every Area, taken when it is generated. The
copy is stored in private storage under the publication's own identifier and is
never rewritten. Publishing it makes it the one voters are served and withdraws
the previous one.

Editing the Election Event, an Election, a Contest or a Candidate afterwards
changes nothing for voters. The edit reaches them only when a new publication
is generated and published.

A voter is given the ballot of their own Area, for the Elections they may vote
in, and only from the published publication.

## What the Voting Portal draws from it

| Part of the design | Set on | Where it shows |
| --- | --- | --- |
| Names and descriptions, in every enabled language | Election Event, Election, Contest, Candidate | Ballot list, start screen, ballot, review |
| Languages offered and the default language | Election Event | Header |
| Logo (**Ballot Design → Logo URL**) | Election Event | Header, on every screen |
| Stylesheet (**Ballot Design → Custom CSS**) | Election Event | Every screen, from the ballot list on |
| Order of the Contests: custom, alphabetical or random | Election | Ballot, review |
| Contests on one page or several | Contest | Ballot |
| Order of the Candidates: custom, alphabetical or random | Contest | Ballot, review |
| Columns of Candidates | Contest | Ballot, on wide screens. Narrow screens use one column |
| Candidate pictures and links | Candidate | Ballot, review |
| Voting rules: seats, write-ins, blank, invalid, under-vote and over-vote policies | Contest | Ballot, review |

A random order of Contests is drawn once for each voter when they open the
ballot. It stays the same while they mark it, and the review screen repeats it.

### What is not part of the publication

These are read when the voter opens the portal, so changing them does not need
a new publication:

- Whether voting is open, paused or closed, per voting channel.
- The number of times a voter may vote again.
- Support materials.

Pictures and the logo are published by their address. Replacing the file kept
at that address changes the picture voters see without a new publication, so
upload a changed picture as a new file and publish again.

## Preview

**Preview** in the Publish tab opens the Voting Portal on one Area's ballot of
that publication. It shows the Election Event and Elections as the publication
was generated with them, also when they were edited later, so the preview of a
publication is what voters get from it. The preview has voting open so the
whole ballot can be walked through, and it casts no vote.

## Showing that the ballot is the published design

1. In the Admin Portal, open the Election Event and set the design: logo,
   stylesheet, languages, Contest order, Candidate order, columns and pictures.
2. In **Publish**, select **Publish Changes**, review the changes and open
   **Preview** for an Area. Check the ballot list, the ballot and the review
   screen against the design.
3. Select **Publish Changes** again to publish.
4. Sign in to the Voting Portal as a voter of that Area. The ballot list, the
   ballot and the review screen show the same design as the preview, in each
   enabled language, on a computer and on a phone.
5. Change something visible in the Admin Portal, such as a Candidate's name,
   and do not publish. Reload the Voting Portal: the ballot is unchanged.
6. Publish the change and reload the Voting Portal: the ballot shows it.

Steps 4 to 6 are the demonstration that the Voting Portal renders the approved
ballot design from the election management system (OVCS post-qualification,
PQRI Annex A D.4.5.2).

The same behaviour is checked automatically by the Voting Portal's
`ballot-design` journey and by Windmill's publication tests; see
[UI browser tests](../../07-developers/03-development-environment/testing/ui-browser-tests.md).
