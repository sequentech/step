---
id: election_management_contest_data
title: Data
description: "The Data tab of a contest holds the configuration of one contest: its texts, its voting system, the number of selections and the ballot policies."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Data** tab of a contest holds the configuration of one contest: its texts, its voting
system, the number of selections and the ballot policies.

## Open the tab

1. In the menu on the left, open the election event and the election.
2. Click the contest.
3. Click the **Data** tab.

To save changes, your role needs the **Edit Contest** permission.

To create a contest, click the arrow next to the election in the menu to show its contests, then click
**Create a Contest**. The form has **Name** and **Description**. A new contest has these
values: **Min votes** 0, **Max votes** 1, **Winning candidates num** 1, **No Preferential** and
**Plurality at Large**.

## Sections

| Section | Fields | Use |
| --- | --- | --- |
| **General** | One tab for each language with **Name**, **Alias** and **Description** | The texts of the contest. |
| **Ballot Voting System** | **Voting type**, **Counting algorithm** | Version 9.0 has only **No Preferential** and **Plurality at Large**. |
| **Ballot Design** | See the table below | The number of selections and the look of the contest. |
| **Policies** | See the table below | What the voting portal does with the selections of the voter. |
| **Image** | An image file | The image of the contest. |
| **Advanced Configuration** | A JSON file | Advanced settings of the contest. Change it only with the help of Sequent support. |

### Ballot Design

| Field | Use |
| --- | --- |
| **Is acclaimed** | Marks the contest as decided without a vote. |
| **Allow Write-Ins** | Permits candidates that the voter types. |
| **Min votes** | The minimum number of selections. Type `0` if the voter can leave the contest blank. |
| **Max votes** | The maximum number of selections. |
| **Presentation columns** | The number of columns of candidates on the ballot. |
| **Winning candidates num** | The number of winning candidates. |
| **Presentation candidates order** | **Random**, **Custom** or **Alphabetical**. With **Custom**, use **Reorder candidates**. |
| **Presentation enable checkable lists** | **Candidates And Lists**, **Candidates Only**, **Lists Only** or **Disabled**. For contests with lists of candidates. |
| **Presentation max selections per type** | The maximum number of selections for each type of candidate. |

:::caution CAUTION
The admin portal does not check that **Min votes** is lower than **Max votes**. Check the
values before you save. Make sure that **Max votes** is not higher than the number of
candidates.
:::

### Policies

| Field | Values |
| --- | --- |
| **Under Vote Policy** | **Allowed**, **Warn in Review**, **Warn**, **Warn and Alert** |
| **Invalid Vote Policy** | **Allowed**, **Warn**, **Warn Invalid Implicit And Explicit**, **Not Allowed** |
| **Blank Vote Policy** | **Allowed**, **Warn in Review**, **Warn**, **Not Allowed** |
| **Over Vote Policy** | **Allowed**, **Allowed with Warning Message**, **Allowed with Warning message and Alert**, **Not Allowed with Warning message and Alert**, **Not Allowed with Warning message and Disable further selections** |
| **Candidates checkbox icon shape** | **Square Checkbox**, **Round Checkbox** |
| **Page Name** | The name of the page of the contest in the voting portal. |

## If there is a problem

| Problem | Action |
| --- | --- |
| "Error creating candidate" when you create a contest | Version 9.0 shows this text for a contest error. Check the **Name**, then save again. |
| The **Save** button is not there. | Your role does not have the **Edit Contest** permission. |
| Voters can select too many candidates. | Check **Max votes** and **Over Vote Policy**. Then publish again. |

## Related pages

- [Create the Election Event](../../../../../procedures/02-event.md#4-create-the-contests)
- [Tally Sheets](../Tally-Sheets/election_management_contest_tally-sheets.md)
- [Candidate: Data](../../Candidate/Data/election_management_candidate_data.md)
- [Election event: Areas](../../Election-Event/Areas/election_management_election-event_areas.md)
