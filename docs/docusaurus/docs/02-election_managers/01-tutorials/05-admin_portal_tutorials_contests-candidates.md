---
id: admin_portal_tutorials_contests-candidates
title: Contests and Candidates
description: "The main path is in Create the Election Event. This page gives the details: all the options of a contest, the counting algorithms, the number of selections, the policies of the voting portal and the…"
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The main path is in [Create the Election Event](../03-procedures/02-event.md#4-create-the-contests).
This page gives the details: all the options of a contest, the counting algorithms, the number
of selections, the policies of the voting portal and the fields of a candidate.

## Before you start

- The election of the contest exists. See
  [Create the Election Event](../03-procedures/02-event.md#3-create-the-elections).
- To create a contest, your account needs the permissions to create contests and elections. To
  create a candidate, it also needs the permission to create candidates.
- Set the contest and its candidates before you publish the ballot. After a change, make a new
  publication. See [Publish and Manage the Voting Period](../03-procedures/05-publish.md).

## Create a contest

1. In the menu on the left, click **Create a Contest** under the election.
2. Type the **Name**, the **Description** and the **External ID**.
3. Click the save button.

**Expected result:** the admin portal opens the new contest. The contest shows under the
election.

A new contest has these values. Change them on the **Data** tab of the contest.

| Value | Default |
| --- | --- |
| Counting algorithm | **Plurality at Large** |
| **Min votes** | `0` |
| **Max votes** | `1` |
| **Winning candidates num** | `1` |
| Candidate order | **Alphabetical** |
| **Allow Write-Ins** | On |

## The Data tab of a contest

The **Data** tab has these sections: **General**, **Ballot Voting System**, **Ballot Design**,
**Image** and **Advanced Configuration**. Click the save button after a change.

### General

- **External ID**: the identifier of the contest in your other systems.
- One tab for each language of the election event, with **Name**, **Alias**, **Description**
  and **IVR prompt**. The **IVR prompt** is the text for telephone voting.

### Ballot Voting System

**Counting algorithm** has two values in version 10.0:

| Value | How the voter votes | How the platform counts |
| --- | --- | --- |
| **Plurality at Large** | The voter selects one or more candidates. | The candidates with the most votes win. |
| **Instant Runoff** | The voter ranks the candidates. | The platform eliminates the candidate with the fewest votes in each round, and moves those votes to the next preference. |

For **Instant Runoff**, also select the **Tie-Breaking Policy**:

- **Random**: the platform breaks a tie at random. This is the default value.
- **External Procedure**: the tally stops at a tie. A person enters the decision. See
  [Resolve a tie](16-admin_portal_tutorials_tally-ceremony.md#resolve-a-tie).

### Ballot Design: number of selections

| Field | Meaning |
| --- | --- |
| **Decided by acclamation** | The contest has no vote. Voters see it but cannot select anything. Each candidate is a winner with zero votes. See [Acclaimed contests](../02-reference/11-acclaimed-contests.md). |
| **Allow Write-Ins** | The voter can type the name of a candidate. Use it with a candidate that has **Write-in** on. |
| **Min votes** | The minimum number of selections. Type `0` if the voter can leave the contest blank. |
| **Max votes** | For **Plurality at Large**, the maximum number of candidates that a voter can select. For **Instant Runoff**, the highest rank that a voter can give, for example `5` for ranks 1 to 5. |
| **Presentation columns** | The number of columns of candidates in the voting portal. The minimum is `1`. |
| **Winning candidates num** | The number of candidates that win the contest. |

:::caution CAUTION
Set **Decided by acclamation** before you publish the ballot. If you change it after voters
cast ballots, the ballots that are already cast become invalid.
:::

:::caution CAUTION
The admin portal does not check that **Min votes** is lower than **Max votes**. Check the
values before you save. For **Plurality at Large**, make sure that **Max votes** is not more
than the number of candidates.
:::

### Ballot Design: order and lists

| Field | Values |
| --- | --- |
| **Presentation candidates order** | **Alphabetical** (default), **Random** or **Custom**. With **Custom**, the section **Reorder candidates** shows. Drag the candidates into the order of the ballot. |
| **Presentation enable checkable lists** | **Candidates And Lists** (default), **Candidates Only**, **Lists Only** or **Disabled**. It tells what a voter can select when the candidates are in lists. |
| **Collapsible Lists** | **Disabled** (default), **Enabled (starts expanded)** or **Enabled (starts collapsed)**. |
| **Presentation max selections per type** | Optional. The maximum number of selections for each type of candidate. |

When the candidates have a **Type**, the section also shows each type with its **List Name**
for each language and a **Sort order**.

### Ballot Design: policies

The **Policies** part of **Ballot Design** tells the voting portal what to do when a ballot is
not complete or not valid. The default value of each policy is **Allowed**.

| Policy | When it applies | Values |
| --- | --- | --- |
| **Under Vote Policy** | The voter selects fewer candidates than possible. | **Allowed**, **Warn in Review**, **Warn**, **Warn and Alert** |
| **Invalid Vote Policy** | The ballot is not valid. | **Allowed**, **Warn**, **Warn Invalid Implicit And Explicit**, **Not Allowed**, **Allowed With Exclusive Explicit** |
| **Blank Vote Policy** | The voter selects no candidate. | **Allowed**, **Warn in Review**, **Warn**, **Not Allowed** |
| **Over Vote Policy** | The voter selects too many candidates. | **Allowed**, **Allowed with Warning Message**, **Allowed with Warning message and Alert**, **Not Allowed with Warning message and Alert**, **Not Allowed with Warning message and Disable further selections** |

For **Instant Runoff** only, two more policies show. Each has the values **Show Warning and
Dialog (voter can proceed)** and **Show Warning and Dialog (voter not allowed to proceed)**:

- **Invalid Vote - Duplicate Rank Policy**: the voter gives the same rank to two candidates.
- **Invalid Vote - Skipped Ranks Policy**: the voter skips a rank.

Two more fields change the look of the contest:

- **Candidates checkbox icon shape**: **Square Checkbox** (default) or **Round Checkbox**.
- **Page Name**: the voting portal shows the contests with the same page name on the same page.

### Image and Advanced Configuration

- **Image**: drop an image file for the contest.
- **Advanced Configuration**: import a configuration file for the contest. Use it only with
  the help of Sequent support.

## Create a candidate

1. In the menu on the left, click **Create a Candidate** under the contest.
2. Type the **Name**, the **Description** and the **External ID**.
3. Click the save button.

**Expected result:** the admin portal opens the new candidate. The candidate shows under the
contest. For the next steps, see
[Create the Election Event](../03-procedures/02-event.md#5-create-the-candidates).

## The Data tab of a candidate

The **Data** tab has these sections: **General**, **Type** and **Image**.

### General

- **External ID**: the identifier of the candidate in your other systems.
- One tab for each language, with **Name**, **Alias**, **Description** and **IVR prompt**.
- **Disabled**: a switch that marks the candidate as disabled. It shows on each language tab,
  but it is one value for the candidate.

### Type

| Field | Meaning |
| --- | --- |
| **Type** | Free text. Candidates with the same type are in the same list of the contest. |
| **Subtype** | Free text. A subgroup inside the type. |
| **Invalid Vote** | The option is an explicit invalid vote. A voter selects it to make the vote invalid. |
| **Blank Vote** | The option is an explicit blank vote. |
| **Category List** | The option is the header of the list of its **Type**, not a person in the list. |
| **Write-in** | The voter types a name in this option. The contest must have **Allow Write-Ins** on. |
| **Invalid Vote Position** | **None (Default)**, **Top** or **Bottom**. The position of the option in the ballot. |

### Image

Drop a photo or a logo of the candidate.

## If there is a problem

| Problem | Action |
| --- | --- |
| **Create a Contest** or **Create a Candidate** is not in the menu. | Your account does not have the permission to create contests or candidates. Ask your tenant administrator. |
| The save button is not shown on the **Data** tab. | Your account cannot edit the item. Ask your tenant administrator. |
| A counting algorithm that you need is not in the list. | Version 10.0 has only **Plurality at Large** and **Instant Runoff**. Contact Sequent support. |
| **Tie-Breaking Policy** is not shown. | Select **Instant Runoff** in **Counting algorithm**. |
| **Reorder candidates** is not shown. | Select **Custom** in **Presentation candidates order**. |
| Voters do not see the contest. | Add the contest to the area of the voters. Then make a new publication. See [Create the Election Event](../03-procedures/02-event.md#6-create-the-areas). |

## More information

- [Contest: Data](../02-reference/04-contest/01-election_management_contest_data.md)
- [Candidate: Data](../02-reference/05-candidate/01-election_management_candidate_data.md)
- [Acclaimed contests](../02-reference/11-acclaimed-contests.md)
- [Blank ballots](../02-reference/09-blank-ballots.md)
- [Tally ceremony](16-admin_portal_tutorials_tally-ceremony.md)
