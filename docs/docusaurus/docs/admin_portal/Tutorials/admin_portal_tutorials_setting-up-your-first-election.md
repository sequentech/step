---
id: admin_portal_tutorials_setting-up-your-first-election
title: Setting Up Your First Election
description: "This tutorial runs a small test election from the start to the results. Use it to learn the admin portal and to check your tenant before a real election."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This tutorial runs a small test election from the start to the results. Use it to learn the
admin portal and to check your tenant before a real election. Each part links to the
procedure with the full steps, and tells you what to check in the test.

## The test election

| Item | Value |
| --- | --- |
| Election event | "Test Election Event" |
| Election | "Election 1", **External ID** `election-1` |
| Contest | "Contest 1", **Min votes** 0, **Max votes** 1, **Winning candidates num** 1 |
| Candidates | "Candidate A", "Candidate B" |
| Area | "Area North", with "Contest 1" |
| Voters | Two test voters in "Area North", with a password |
| Trustees | Two trustees, threshold 2 |

Use test accounts only. Do not use the data of real voters.

## 1. Check the tenant

Do [Set Up the Tenant](../procedures/01-tenant.md) if it is not done.

Check:

- **Settings** > **LANGUAGES** has the languages of the test.
- **Settings** > **TRUSTEES** has two or more trustees.
- Two trustee accounts exist, each with its own **Act as Trustee**.

## 2. Create the election event

Do [Create the Election Event](../procedures/02-event.md) with the values of the table.

Check:

- The menu shows the tree: "Test Election Event" > "Election 1" > "Contest 1" > "Candidate A"
  and "Candidate B".
- On the **Areas** tab, "Area North" has "Contest 1" in **Contests**.
- On the **Data** tab of "Election 1", **Allow Tally** is **Requires Voting Period End**. Use
  this value in the test, so that you see how the tally waits for the end of the voting period.

## 3. Add the voters

Do [Add the Voters](../procedures/03-voters.md). For two voters, use **Add** on the **Voters**
tab instead of an import.

Check:

- **Eligible Voters** on the **Dashboard** tab is 2.
- The **Area** column of both voters shows "Area North".

## 4. Run the key ceremony

Do [Run the Key Ceremony](../procedures/04-keys.md) with threshold 2 and both trustees. Leave
**Election** empty, so the ceremony is for **All Elections**.

Check:

- Each trustee has two copies of the key file.
- The ceremony status is `SUCCESS`.
- The **Dashboard** tab marks the stage **Keys**.

## 5. Publish and vote

Do [Publish and Manage the Voting Period](../procedures/05-publish.md):

1. Generate the publication, preview the ballot of "Area North", and publish it.
2. Click **Start Voting**.
3. Open **Voter Login URL** from the **Dashboard** tab. Sign in as each test voter and vote.
4. Click **Stop Voting**.

Check:

- **Actual Voters** on the **Dashboard** tab of "Election 1" is 2.
- The **Voted** column on the **Voters** tab shows both voters.

## 6. Run the tally and get the results

Do [Run the Tally Ceremony](../procedures/06-tally.md), then
[Get the Results](../procedures/07-results.md).

Check:

- **Total Votes Counted** is 2.
- The votes of "Candidate A" and "Candidate B" are the votes that you cast.
- You can download the results as **PDF**.

## 7. Clean up

When the test is complete, archive the election event: click the three dots next to it in the
menu, then click **Archive this Election Event**. The election event moves to **Archived** in
the menu.

:::note
A test election event uses the same key ceremony rules as a real one. Make a new election event
for the real election. Do not reuse the test election event.
:::

## Related pages

- [Before You Start](../00-before-you-start.md)
- [Election Management](../election_management.md)
- [Basic Navigation](../Reference/basic_navigation.md)
