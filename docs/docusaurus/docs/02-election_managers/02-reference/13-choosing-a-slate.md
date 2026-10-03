---
id: choosing_a_slate
title: Choosing a Slate
sidebar_position: 13
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

When an election has slates, each slate on the voting screen has a **Choose this slate** button. Choosing a slate is a shortcut: it selects the slate's candidates in each contest, exactly as if the voter had selected them one by one. It is not an additional vote. Nothing about the slate is encrypted, cast or counted, and a ballot filled by choosing a slate is identical to one filled by hand with the same candidates.

Elections without slates do not show the button and behave as before.

## What choosing a slate does

- In every contest the slate has candidates for, those candidates become the voter's selection.
- Contests the slate has no candidates for keep the voter's current choices. A slate with only trustee candidates, for example, changes only the trustees contest.
- The result never exceeds a contest's maximum number of choices, because the slate's candidates replace the current selection of that contest instead of being added to it.
- A slate is applied to all of its contests at once or not at all.

After choosing, the contests show the selected candidates as usual, and the slate states how many candidates were selected in how many contests. The voter can still change any choice individually.

## Replacing existing choices

If the voter already selected candidates that the slate would remove, the Voting Portal asks first. The dialog lists, for each affected contest, the choices that would be removed and the candidates selected instead.

- **Replace choices** applies the slate.
- **Keep my choices** closes the dialog and leaves the ballot unchanged.

Choosing a slate that only touches empty contests, or whose candidates are already selected, does not ask.

## Custom CSS

| Class | Element |
| --- | --- |
| `slate-apply` | Container of the button and the result line |
| `slate-apply-button` | The **Choose this slate** button |
| `slate-apply-status` | The line stating what was selected |
| `slate-replace-dialog` | The confirmation dialog |
| `slate-replace-contest` | One affected contest in the dialog |
| `slate-replace-removed`, `slate-replace-added` | The removed and newly selected candidates of a contest |
