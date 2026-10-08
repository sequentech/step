---
id: decline_to_vote
title: Decline to Vote
sidebar_position: 7
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Decline to Vote** policy lets voters formally abstain from an entire election at once, across all of its contests, instead of casting candidate selections. This is an **election-level** action: a voter either declines for the whole ballot or votes normally on every contest.

This feature is available **only when the Election Event uses the Multiple Contests encryption policy**. It does not apply to Single Contest encryption.

> **Note on Instant Runoff (IRV):** The Instant Runoff counting algorithm is not currently supported with Multiple Contests encryption. Because Decline to Vote requires Multiple Contests encryption, it does not apply to elections that use IRV today. See [Instant Runoff Algorithm](../../07-developers/07-velvet/05-instant-runoff.md) for details on IRV limitations.

---

## Prerequisites

Before Decline to Vote can be used, both of the following must be configured:

1. **Election Event — Contest Encryption Policy:** set to **Multiple Contests** in the Election Event **Data** tab under **Advanced Configuration**. See [Election Event Data](./02-election-event/03-election_management_election-event_data.md).
2. **Election — Decline to Vote Policy:** set to **Enabled** on the individual Election **Data** tab under **Advanced Configuration**. See [Election Data](./03-election/03-election_management_election_data.md).

If the Election Event uses **Single Contests** encryption, the Decline to Vote Policy setting is hidden in the Admin Portal and the option is not offered to voters.

---

## Admin Portal

### Enabling the policy

1. Open the **Election Event** and confirm **Contest Encryption Policy** is **Multiple Contests**.
2. Open the target **Election** and go to the **Data** tab.
3. Expand **Advanced Configuration**.
4. Set **Decline to Vote Policy** to **Enabled** (the default is **Disabled**).
5. Save the Election.

The policy is configured per Election, not per Contest. All contests in that Election share the same decline-to-vote behavior.

### Tally results

After a tally completes, declined ballots appear in election-level results:

- In the **Tally** tab, the **Elections** results table includes a **Total Declined to Vote** column when at least one included Election has Decline to Vote enabled with Multiple Contests encryption.
- Generated election reports include a **Total Declined to Vote** row in the election-level participation summary.

Declined ballots are **not** broken out in contest-level result tables. Contest tallies do not show a separate declined-to-vote count alongside valid, invalid, or blank votes.

---

## Voting Portal

When Decline to Vote is enabled for an Election, voters see an additional option on the **Start Screen** (before the voting questions):

- **Decline to Vote** button — opens a confirmation dialog explaining that the voter will skip contest selection and proceed directly to review.
- **Start Voting** button — the normal flow through all contests.

### Decline to Vote flow

1. The voter clicks **Decline to Vote** and confirms in the dialog.
2. The ballot is marked as declined for **all contests** in the Election.
3. The voter is taken directly to the **Review Screen**, bypassing the contest voting screens.
4. On the Review Screen, each contest shows **Decline to vote** instead of candidate selections.
5. The voter encrypts and casts the ballot as usual (including optional audit, depending on configuration).

### Normal voting flow

If the voter clicks **Start Voting** instead, they proceed through contests normally. Choosing **Decline to Vote** after starting the normal flow is not supported — decline is only available from the Start Screen.

The destination of the **Back** button on the first voting screen is controlled by the Election-level **Voting Screen Back Button Policy**. When Decline to Vote is enabled, set that policy to **Go to the election start screen** so the decline option remains reachable; otherwise the **Back** button returns to the election selection screen (the default). See [Election Data](./03-election/03-election_management_election_data.md).

Whenever a voter arrives at the Start Screen, the ballot is reset to its initial state: all previous selections — including any previous decline selection — are cleared.

---

## Results

How declined ballots are counted differs between election-level and contest-level reporting.

### Election level

At the election level, a declined ballot:

- **Counts toward total voter participation** (included in **Total Voters** / total votes cast for the Election).
- Is reported separately as **Total Declined to Vote**, with its own count and percentage of the census.

This reflects that the voter participated in the election by formally declining, even though they did not select any candidates.

### Contest level

At the contest level, a declined ballot:

- Is **not** counted as a **valid vote**.
- Is **not** counted as an **invalid vote** (neither explicit nor implicit).
- Is **not** counted as a **blank vote**.
- Does **not** contribute to any candidate totals.

Contest-level participation and candidate result tables therefore exclude declined ballots from all vote categories. The decline is tracked only at the election level.

---

## Summary

| Aspect | Behavior |
|--------|----------|
| **Encryption policy** | Multiple Contests only |
| **Counting algorithm** | Not compatible with IRV (IRV requires Single Contest encoding) |
| **Configuration location** | Election **Data** → **Advanced Configuration** → **Decline to Vote Policy** |
| **Voter entry point** | Start Screen **Decline to Vote** button |
| **Scope** | Entire Election (all contests at once) |
| **Election-level results** | Counts as participation; shown as **Total Declined to Vote** |
| **Contest-level results** | Not counted as valid, invalid, or blank |
