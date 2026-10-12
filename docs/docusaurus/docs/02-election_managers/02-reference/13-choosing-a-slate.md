---
id: choosing_a_slate
title: Choosing a Slate
sidebar_position: 13
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

When an election has slates, each slate card on the **Choose a slate** tab has a **Select slate** button. Selecting a slate is a shortcut: it selects the slate's candidates in each contest, exactly as if the voter had selected them one by one. It is not an additional vote. Nothing about the slate is encrypted, cast or counted, and a ballot filled by selecting a slate is identical to one filled by hand with the same candidates.

Elections without slates do not show the button and behave as before.

## What selecting a slate does

Selecting a slate selects all of its candidates, and only them, on the whole ballot:

- In every contest the slate has candidates for, those candidates become the voter's selection.
- Every other contest on the ballot is left without a choice. A slate with only trustee candidates, for example, selects its trustees and leaves President, Vice President and the other offices empty.
- The result never exceeds a contest's maximum number of choices, because the slate's candidates replace the current selection instead of being added to it. A contest left with fewer choices than its maximum is an undervote, which is allowed; the review screen asks the voter to confirm it when the contest's **Under Vote Policy** is **Warn and Confirm in Review**.
- Acclaimed contests are not changed.
- A slate is applied to the whole ballot at once or not at all.

After selecting, a notice above the cards says **_Slate_ selected. You can still change any candidate.** For a slate that leaves offices blank it names them: **_Slate_ selected. It has no candidate for _offices_, so those offices are blank. You can still change any candidate.** The notice stays while the ballot holds exactly that slate. The card then reads **All N selected** and offers **Edit selections**. The voter can still change any choice individually.

## Replacing existing choices

If selecting the slate would remove any choice the voter already made, in a contest the slate has candidates for or in one it has none for, the Voting Portal asks first with **Replace these selections?**:

- It says **Selecting _Slate_ will change the following offices.**
- For each office that loses a choice, it shows **Current selections**, an arrow, and the slate's candidates for that office, or **No candidate** when the office will be blank.
- For a slate that has no candidate for some offices, a note adds **_Slate_ has no candidate for some offices. Those offices will be blank.**
- **Replace selections** applies the slate. When only one office changes, the button names it, for example **Replace president**.
- **Keep my selections** closes the dialog and leaves the ballot unchanged.

Selecting a slate that removes nothing, because the ballot is empty or holds only candidates the slate keeps, does not ask.

## Clearing choices

On the **Individual candidates** tab, each office of a ballot with slates has a **Clear** button that removes the choices of that office. It is disabled while the office has no choice, and screen readers hear **Selections cleared for _office_.**

The **Clear selections** button at the bottom of the voting screen removes every choice on the ballot, on both tabs. It asks first with **Clear all your choices?**, which shows the same before and after for every office that loses a choice. **Clear choices** clears the ballot and **Keep my choices** leaves it unchanged. The button is disabled while nothing is selected. Ballots without slates keep the usual **Clear selections** behaviour.

## Selection limits

A slate never changes how many candidates a contest accepts. The limit of each contest is its **Maximum votes**, and it applies in the same way to candidates selected one by one and to candidates selected through a slate.

| Contest configuration | Selecting a candidate | Selecting a slate |
| --- | --- | --- |
| Maximum votes 1 and `candidates_selection_policy` set to `radio` | Selecting another candidate replaces the current one. | The slate's candidate replaces the current one, after the voter confirms. |
| **Over Vote Policy** set to **Not Allowed with Warning message and Disable further selections** | At the maximum, the remaining candidates are disabled and a message tells the voter to deselect one first. A selection above the maximum is refused and the earlier choices are kept. | The slate's candidates replace the current ones in that contest. The remaining candidates are then disabled in the same way. |
| Any other **Over Vote Policy** | The message, alert and blocked **Next** button of that policy, as in an election without slates. | The slate's candidates replace the current ones in that contest. |

For an election with single-seat offices and a board of three trustees, the first row is the usual configuration of each office and the second row the usual configuration of the trustees contest, with Maximum votes 3. A voter can then combine up to three trustees from any slates and independent candidates, and a fourth is never accepted.

Deselecting a candidate is always possible.

A slate with more candidates in a contest than the contest allows cannot be selected. Its button is disabled and a line next to it names the contest, the slate's number of candidates there and the maximum. No contest is changed, and the voter can still select candidates one by one.

## Custom CSS

| Class | Element |
| --- | --- |
| `slate-apply` | Container of the **Select slate** button |
| `slate-apply-button` | The **Select slate** button |
| `slate-apply-unavailable` | The line stating why a slate cannot be selected |
| `slate-applied-notice` | The notice after selecting a slate |
| `slate-replace-dialog` | **Replace these selections?** |
| `slate-clear-dialog` | **Clear all your choices?** |
| `slate-clear-status` | The line confirming that the ballot was cleared |
| `slate-changes`, `slate-replace-content` | The body of either dialog and its first sentence |
| `slate-replace-contest`, `slate-replace-contest-name` | One office that changes, and its name |
| `slate-change-columns`, `slate-change-label`, `slate-change-candidate` | The before and after columns, their headings and each candidate |
| `slate-replace-removed`, `slate-replace-added` | The current selections and what replaces them |
| `slate-replace-none` | An office that will be left with **No candidate** |
| `slate-replace-note` | The note about offices left blank |
