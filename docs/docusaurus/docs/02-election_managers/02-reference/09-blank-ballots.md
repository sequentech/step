---
id: blank_ballots
title: Blank Ballots
sidebar_position: 9
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Blank Ballots** policy lets a voter deliberately cast a ballot with every contest left empty, and reports that count as a distinct, verifiable figure — **Total Blank Ballots** — separate from per-contest blank votes and from [Decline to Vote](./07-decline-to-vote.md). This is an **election-level** outcome: a ballot is either blank in every contest or it is not blank at all. Blank is never offered to voters as a candidate option; it is inferred from the content of the ballot.

This feature is available **only when the Election Event uses the Multiple Contests encryption policy**. It does not apply to Single Contest encryption.

> **Note on Instant Runoff (IRV):** As with Decline to Vote, IRV is not currently supported with Multiple Contests encryption, so Blank Ballots does not apply to elections using IRV today. See [Instant Runoff Algorithm](../../07-developers/07-velvet/05-instant-runoff.md).

---

## Prerequisites

Before Blank Ballots can be used, both of the following must be configured:

1. **Election Event — Contest Encryption Policy:** set to **Multiple Contests** in the Election Event **Data** tab under **Advanced Configuration**. See [Election Event Data](./02-election-event/03-election_management_election-event_data.md).
2. **Election — Blank Ballots Policy:** set to **Enabled** on the individual Election **Data** tab under **Advanced Configuration**. See [Election Data](./03-election/03-election_management_election_data.md).

If the Election Event uses **Single Contests** encryption, the Blank Ballots Policy setting is hidden in the Admin Portal and the outcome is never reported for that election.

---

## Admin Portal

### Enabling the policy

1. Open the **Election Event** and confirm **Contest Encryption Policy** is **Multiple Contests**.
2. Open the target **Election** and go to the **Data** tab.
3. Expand **Advanced Configuration**.
4. Set **Blank Ballots Policy** to **Enabled** (the default is **Disabled**).
5. Save the Election.

The policy is configured per Election, not per Contest. Once voting has opened, treat the policy as fixed for that Election — changing it partway through a live election mixes ballots cast under different rules.

### Tally results

After a tally completes, blank ballots appear in election-level results:

- In the **Tally** tab, the tally session's **Results & Participation** table includes a **Total Blank Ballots** column when at least one included Election has Blank Ballots enabled with Multiple Contests encryption.
- The **Elections** results table (tally overview) shows the same column, populated from the underlying `blank_ballots` figure on each election's results row.

### Paper results (tally sheets)

For results arriving on paper:

- The **CSV import** pipeline accepts a per-ballot-box blank ballots figure (the same value is expected on every contest sheet of a box, since blankness is a whole-ballot property). The import validates that the figure is consistent across a box's sheets and falls within the bounds implied by that box's own per-contest figures, pre-filling the value automatically when those bounds pin it to a single number.
- **Manual tally sheet entry** (the single-sheet form in the Admin Portal) also accepts the figure, but — unlike CSV import — it does not have visibility into a ballot box's other contest sheets while editing one sheet, so it cannot cross-check or pre-fill the value the way CSV import does. Enter the same figure consistently across every contest sheet of a box by hand.
- A ballot box that never received a submitted figure is reported as **unavailable**, not as zero, and any total that includes such a box is itself reported as unavailable.

---

## Voting Portal

Unlike Decline to Vote, there is **no explicit "cast a blank ballot" button**. A ballot becomes blank purely as a consequence of the voter leaving every contest without a selection during the normal voting flow.

### Casting a blank ballot

1. The voter proceeds through the ballot normally (**Start Voting**, not Decline to Vote) and leaves every contest without a candidate selection.
2. When the voter clicks **Next** to proceed to review, a confirmation dialog explains that the ballot will be cast blank and asks the voter to confirm before continuing.
3. On the **Review Screen**, every contest shows a **Blank ballot** label instead of candidate selections.
4. If ballot casting is configured to show a confirmation dialog before the final cast, that dialog's copy is also blank-ballot specific.
5. After casting, the confirmation/receipt screen shows blank-specific copy alongside the usual Ballot ID, explaining that the ballot was cast blank as a valid, deliberate choice.

If the voter selects at least one candidate in any contest, or marks any contest explicitly invalid, the ballot is not blank — normal review and receipt copy apply, even if other contests were left empty.

### Ballot Verifier

The standalone Ballot Verifier renders the same **Blank ballot** label (distinct from the generic empty-contest label used for an ordinary undervote) whenever it decodes a ballot whose blank flag is set and the policy is enabled for that election.

---

## Results

### Election level

At the election level, a blank ballot:

- **Counts toward total voter participation** (included in **Total Voters** / total votes cast for the Election) — a blank ballot is a valid cast ballot.
- Is reported separately as **Total Blank Ballots**, with its own count and percentage. The percentage is computed over total votes cast, not over the census, since a blank ballot is a valid cast ballot rather than a non-vote.
- Is established per Area first (against the contests that Area's voters actually received), then summed up to the Election and Election Event totals.

### Contest level

At the contest level, a blank ballot does not introduce a new category — every contest of a blank ballot is simply an ordinary empty selection for that contest, and is counted as a blank vote in that contest's own existing blank-vote figures (`Blank Votes`, and its `Explicit`/`Implicit` breakdown) exactly as any other empty contest selection would be. Blank Ballots does not change how contest-level blank votes are computed; it adds a whole-ballot figure on top.

### Results website

The public Results website shows a **Total Blank Ballots** column in its election summary table whenever the published data includes it. The column is **hidden entirely** — not shown as zero — when the figure is unavailable, for example when the policy is disabled for every published election, or the underlying tally figure could not be established.

### Mutual exclusivity with Decline to Vote

A ballot cannot be both blank and declined. The two are mutually exclusive at the point of casting, and a ballot whose recorded status disagrees with its actual content is treated as invalid.

---

## Summary

| Aspect | Behavior |
|--------|----------|
| **Encryption policy** | Multiple Contests only |
| **Counting algorithm** | Not compatible with IRV (IRV requires Single Contest encoding) |
| **Configuration location** | Election **Data** → **Advanced Configuration** → **Blank Ballots Policy** |
| **Voter entry point** | None — inferred from leaving every contest empty during normal voting; confirmed via a dialog before casting |
| **Scope** | Entire ballot (all contests at once) |
| **Election-level results** | Counts as participation; shown as **Total Blank Ballots** (count + percentage of votes cast) |
| **Contest-level results** | Counted within that contest's existing blank-vote figures, not a separate category |
| **Paper results** | Per-ballot-box figure; CSV import validates and pre-fills across a box's sheets, manual entry does not |
| **Unavailable data** | Reported as unavailable (hidden or "-"), never coerced to zero |
| **Decline to Vote** | Mutually exclusive with a declined ballot |
