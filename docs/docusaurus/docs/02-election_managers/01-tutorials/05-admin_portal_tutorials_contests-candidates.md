---
id: admin_portal_tutorials_contests-candidates
title: Create Contests and Candidates
description: "The main path is in Create the Election Event. This page gives the details: all the options of a contest and of a candidate, their default values and their effect on the ballot and on the count."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The main path is in [Create the Election Event](../03-procedures/02-event.md#4-create-the-contests).
This page gives the details: all the options of a contest and of a candidate, their default
values and their effect on the ballot and on the count.

## Before you start

- The election of the contest exists. See [Create the Election Event](../03-procedures/02-event.md#3-create-the-elections).
- Your account has the permissions to create and to edit contests and candidates. Without the
  permission to edit a contest, the **Data** tab of the contest has no save button.
- You know the counting method, the number of selections and the number of winners of each
  contest.

:::caution CAUTION
Set the contest options before you publish the ballot. A change after publication applies only
after a new publication. A change to **Decided by acclamation** after voters cast ballots
invalidates the ballots that are already cast.
:::

## Create a contest

1. In the menu on the left, click the three dots next to the election.
2. Click **Create a Contest**.
3. Type the **Name**, the **Description** and the **External ID**.
4. Click the save button.

**Expected result:** the admin portal opens the **Data** tab of the new contest.

A new contest has these values. Change them on the **Data** tab.

| Option | Value of a new contest |
| --- | --- |
| Counting algorithm | **Plurality at Large** |
| Minimum number of selections | `0` |
| Maximum number of selections | `1` |
| Number of winning candidates | `1` |
| Order of the candidates | **Alphabetical** |
| **Allow Write-Ins** | On |
| **Decided by acclamation** | Off |

The **Data** tab of a contest has these sections: **General**, **Ballot Voting System**,
**Ballot Design**, **Image** and **Advanced Configuration**.

## General

| Field | Use |
| --- | --- |
| **External ID** | The identifier of the contest in your other systems. |
| **Name** | The name of the contest on the ballot. Type it on the tab of each language. |
| **Alias** | A short name. Where the admin portal shows the alias, for example in the results, it uses the name when there is no alias. |
| **Description** | The description of the contest. Type it on the tab of each language. |
| **IVR prompt** | The text that telephone voting reads for the contest. |

The language tabs are the languages of the election. If the election has no language settings,
the tabs are the languages of the election event.

## Ballot Voting System

The counting algorithm sets how voters select and how the platform counts. Version 10.1 has two
counting algorithms:

| Counting algorithm | How the voter votes | How the platform counts |
| --- | --- | --- |
| **Plurality at Large** | The voter selects one or more candidates. | The candidates with the most votes win. |
| **Instant Runoff** | The voter ranks the candidates in order of preference. | The candidate with the fewest votes is eliminated in each round. The votes of that candidate go to the next preference. |

For **Instant Runoff**, the section also shows **Tie-Breaking Policy**:

| **Tie-Breaking Policy** | Effect |
| --- | --- |
| **Random** (default) | The platform breaks a tie at random. |
| **External Procedure** | The tally stops at the tie. An administrator enters the decision. See [Resolve a tie](16-admin_portal_tutorials_tally-ceremony.md#resolve-a-tie). |

## Ballot Design

### Number of selections and winners

| Field | Use |
| --- | --- |
| **Decided by acclamation** | The candidates are elected without a vote. Voters see the contest but cannot select a candidate. All the candidates are winners with zero votes. See [Acclaimed contests](../02-reference/11-acclaimed-contests.md). |
| **Allow Write-Ins** | Permits write-in candidates. Voters can type a name only if the contest also has a candidate with **Write-in** on. |
| Minimum number of selections | The fewest candidates that a voter must select. Type `0` if the voter can leave the contest blank. |
| Maximum number of selections | For **Plurality at Large**: the most candidates that a voter can select. For **Instant Runoff**: the lowest rank that a voter can give, for example `5` for ranks 1 to 5. |
| Number of columns | The number of columns of candidates on the ballot. |
| Number of winning candidates | The number of candidates that win the contest. |

:::caution CAUTION
The admin portal does not check that the minimum is lower than the maximum. Check the values
before you save. For **Plurality at Large**, make sure that the maximum is not higher than the
number of candidates.
:::

### Order of the candidates

| Value | Effect |
| --- | --- |
| **Alphabetical** | The ballot shows the candidates in alphabetical order. |
| **Random** | The ballot shows the candidates in a random order. |
| **Custom** | The ballot shows the candidates in the order that you set. Drag the candidates in **Reorder candidates**. |

### Lists of candidates

The candidates of a contest can be in lists. The **Type** of a candidate is the name of its list.
For each list, the **Ballot Design** section shows a **List Name** field for each language.

| Field | Values |
| --- | --- |
| Checkable lists | **Candidates And Lists** (default), **Candidates Only**, **Lists Only** or **Disabled**. It sets what a voter can select: the candidates, the list or both. |
| **Collapsible Lists** | **Disabled** (default), **Enabled (starts expanded)** or **Enabled (starts collapsed)**. |
| Maximum selections for each list | The most candidates that a voter can select in one list. Leave it empty for no limit. |

### Policies

The policies set what the voting portal does when the selection of a voter is not normal.

| Policy | Values | Default |
| --- | --- | --- |
| **Under Vote Policy**: fewer selections than the maximum | **Allowed**, **Warn in Review**, **Warn**, **Warn and Alert** | **Allowed** |
| **Invalid Vote Policy**: an invalid selection | **Allowed**, **Warn**, **Warn Invalid Implicit And Explicit**, **Not Allowed**, **Allowed With Exclusive Explicit** | **Allowed** |
| **Blank Vote Policy**: no selection | **Allowed**, **Warn in Review**, **Warn**, **Not Allowed** | **Allowed** |
| **Over Vote Policy**: more selections than the maximum | **Allowed**, **Allowed with Warning Message**, **Allowed with Warning message and Alert**, **Not Allowed with Warning message and Alert**, **Not Allowed with Warning message and Disable further selections** | **Allowed** |
| **Invalid Vote - Duplicate Rank Policy**: two candidates with the same rank. Only for **Instant Runoff**. | **Show Warning and Dialog (voter can proceed)**, **Show Warning and Dialog (voter not allowed to proceed)** | Voter can proceed |
| **Invalid Vote - Skipped Ranks Policy**: a rank is missing, for example 1, 2, 4. Only for **Instant Runoff**. | **Show Warning and Dialog (voter can proceed)**, **Show Warning and Dialog (voter not allowed to proceed)** | Voter can proceed |

Two other fields change the look of the contest:

- **Candidates checkbox icon shape**: **Square Checkbox** (default) or **Round Checkbox**.
- **Page Name**: contests with the same page name show on the same page of the ballot.

For the details of the policies, see [Contest: Data](../02-reference/04-contest/01-election_management_contest_data.md).

## Image and Advanced Configuration

- **Image**: drop an image file. The admin portal shows the image in this section.
- **Advanced Configuration**: the configuration of the contest as a JSON file. Use it only with
  the help of Sequent support.

## Create a candidate

1. In the menu on the left, click the three dots next to the contest.
2. Click **Create a Candidate**.
3. Type the **Name**, the **Description** and the **External ID**.
4. Click the save button.

**Expected result:** the admin portal opens the new candidate. The candidate shows under the
contest in the menu.

## Candidate options

The **Data** tab of a candidate has the sections **General**, **Type** and **Image**.

### General

| Field | Use |
| --- | --- |
| **External ID** | The identifier of the candidate in your other systems. |
| **Name**, **Alias**, **Description** | The texts of the candidate on the ballot. Type them on the tab of each language. |
| **IVR prompt** | The text that telephone voting reads for the candidate. |
| **Disabled** | The ballot does not show the candidate. |

### Type

| Field | Use |
| --- | --- |
| **Type** | The name of the list of the candidate. Candidates with the same type are in the same list. |
| **Subtype** | A group inside the list. |
| **Invalid Vote** | The candidate is an option to cast an explicit invalid vote. |
| **Blank Vote** | The candidate is an option to cast an explicit blank vote. |
| **Category List** | The candidate is the header of its list, which is the list of its **Type**. |
| **Write-in** | The candidate is a field where the voter types a name. The contest must have **Allow Write-Ins** on. |
| **Invalid Vote Position** | For an invalid or blank vote option: **None (Default)**, **Top** or **Bottom** of the list of candidates. |

### Image

Drop an image file, for example a photo or a logo. The admin portal shows the image in this section.

## If there is a problem

| Problem | Action |
| --- | --- |
| The **Data** tab of the contest has no save button. | Your account does not have the permission to edit contests. Ask your tenant administrator. |
| **Create a Contest** or **Create a Candidate** is not in the menu. | Your account does not have the permission to create them. Ask your tenant administrator. |
| **Tie-Breaking Policy** is not shown. | Select **Instant Runoff**. The field is only for this counting algorithm. |
| A candidate is not on the ballot preview. | Make sure that **Disabled** is off. Then make a new publication. |
| Voters cannot type a write-in name. | Turn on **Allow Write-Ins** in the contest and **Write-in** in a candidate. |
| Voters do not see the contest. | Add the contest to the area of the voters. See [Create the areas](../03-procedures/02-event.md#6-create-the-areas). |

## More information

- [Create the Election Event](../03-procedures/02-event.md)
- [Contest: Data](../02-reference/04-contest/01-election_management_contest_data.md)
- [Candidate: Data](../02-reference/05-candidate/01-election_management_candidate_data.md)
- [Acclaimed contests](../02-reference/11-acclaimed-contests.md)
- [Blank ballots](../02-reference/09-blank-ballots.md)
- [Tally ceremony](16-admin_portal_tutorials_tally-ceremony.md)
