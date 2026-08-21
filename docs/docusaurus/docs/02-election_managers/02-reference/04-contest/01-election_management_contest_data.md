---
id: election_management_contest_data
title: Data
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


# Contest Data Tab

The Contest Data tab allows administrators to manage core settings and configurations for a specific contest within an Election. Many elements mirror those in the Election Event Data tab, adapted for contest-specific context. Below are the primary sections and details.

## Sections Overview
- **General**: Basic details and multilingual naming.
- **Ballot Voting System**: Configuration for different voting systems (work in progress).
- **Ballot Design**: Display and behavior settings for ballot presentation.
- **Page Name**: Controls grouping of contests on voting pages.
- **Image**: Upload an image/icon for this contest.
- **Advanced Configuration**: Import custom configuration files for specialized behavior.

---

## General
Configure fundamental details and multilingual names for this contest.

- **Contest Name**  
  - Enter the official name.  
  - Support for multiple languages if Election is multilingual.
- **Alias (optional)**  
  - System-only display alias in side menus or internal references.
- **Description (optional)**  
  - Brief description or instructions shown to administrators; may appear in tooltips or help texts if implemented.
- **Languages Tabs**  
  - If the Election supports multiple languages, configure how this contest’s name/description appears per language.

---

## Ballot Voting System
Voting System configuration will affect the tally and the Voting portal interface (e.g. whether the system is preferential or not).

### Counting Algorithm
- **Plurality at Large**
  - Voters select up to a specified number of candidates. Candidates with the most votes win.
  - Best suited for non-preferential elections where voters choose their top choices without ranking.
- **Instant Runoff or IRV**
  - Preferential voting system where voters rank candidates in order of preference.
  - Candidates with the fewest first-choice votes are eliminated and their votes redistributed until winners are determined.
  - When selecting IRV, set Max votes parameter (in Ballot Design) to at least the number of candidates available to rank.


---

## Ballot Design
Manage how the ballot for this contest is displayed and how voters interact with it.

### Display Settings & Alerts
- **Is acclaimed**
  - Indicates that the winner is already determined.
  - Voters can view the result but cannot cast a vote.
- **Min Votes**
  - The minimum number of selections a voter must make before the ballot is considered valid.
  - Setting this to 0 allows voters to submit without selecting any candidate.
- **Max Votes**
  - The meaning depends on the counting algorithm:
    - **Non-preferential (Plurality at Large)**: the maximum number of candidates a voter may select.
    - **Preferential (Instant Runoff / IRV)**: the highest ranking position available to voters. For example, a value of `5` means voters can assign ranks 1 through 5. Must be at least the number of candidates available to rank.
  - Works in conjunction with the Over Vote Policy to determine what happens when a voter exceeds this limit.
- **Under-Vote Alert**
  - Show a warning if a voter selects fewer than the maximum required options.
- **Over-Vote Alert**
  - Show a warning if a voter selects more than the allowed number of options.
- **Winning Candidates Number**
  - Specify how many candidates can win this contest.
- **Candidate Order**
  - Determine display order:
    - Random
    - Alphabetical
    - Custom (when selected, enable manual reordering).

### Edit Lists & Selection Types
- **Enable Checkable Lists**  
  - Allow grouping candidates into lists or combined list-candidate selections.  
  - When enabled, configure list-based voting behavior.
- **Max Selections Per Type**  
  - Define maximum selections for each candidate/list type.
  - Example: Maximum of 2 individual candidates and 1 list selection.

### Policies
Define voter behavior rules and system responses in various voting scenarios. For each policy type below, choose one of the options:  
- **Allowed**: Voter may submit without warning.  
- **Warn**: Voter may submit but receives warning(s).  
- **Warn in Review** (if available): Warning appears only during review phase.  
- **Warn and Alert**: Warning appears and a confirmation dialog forces acknowledgement.  
- **Not Allowed**: Voter cannot submit; may include warning/alert before blocking.

#### Under Vote Policy
When voter selects fewer options than the minimum required:
- **Allowed**: Submit without warning.
- **Warn**: Warning during ballot and review phases.
- **Warn in Review**: Warning only in review phase.
- **Warn and Alert**: Warning during ballot; confirmation required to proceed.

#### Over Vote Policy
When voter selects more options than allowed:
- **Allowed**: Submit without warning.
- **Warn**: Warning during ballot and review phases.
- **Warn and Alert**: Warning during ballot; confirmation required to proceed.
- **Not Allowed with Warn and Alert**: Voter cannot submit; warning appears.
- **Not Allowed with Warning and Selection Lock**: Warning appears and prevents extra selections.

#### Invalid Vote Policy
When voter selection is invalid (e.g., null vote or too many selections):
- **Allowed**: Submit without warning.
- **Warn**: Warning during ballot and review phases.
- **Warn Implicit and Explicit**: Warning for explicitly invalid (null/spoiled) and implicitly invalid (e.g., too many).
- **Not Allowed**: Cannot submit if invalid options selected.
- **Allowed with Exclusive Explicit**: Same as Allowed, except selecting the explicit-invalid option is mutually exclusive with any other selection — picking it clears other selected candidates, and picking a candidate clears it, mirroring how blank vote already behaves. Combining an explicit-invalid selection with other candidates (e.g. via an older client) is still tallied like Allowed, since that combination remains an intentionally supported ballot shape for clients that rely on it.

##### Preferential systems (e.g. Instant Runoff)

###### What is considered Implicit invalid error?
- **Duplicated Position**: Whether casting the vote is allowed or not depends on the policy. Either case ranking 2 candidates
  with the same position will invalidate the vote.

###### What will cause an alert but it´s still valid?
- **Preference order with gaps**: Missing one of the positions in between, e.g. 1,2,4,5.

#### Blank Vote Policy
When voter selects blank vote option:
- **Allowed**: Submit without warning.
- **Warn**: Warning during ballot and review phases.
- **Warn in Review**: Warning only in review phase.
- **Not Allowed**: Cannot submit blank ballot.

#### Invalid Vote - Duplicate Rank Policy
_Applies to preferential voting (e.g. Instant Runoff) only._

When a voter assigns the same ranking position to two or more candidates, the vote is considered invalid. This policy controls the UX response:
- **Show Warning and Dialog (voter can proceed)**: A warning dialog is shown, but the voter is allowed to submit the ballot as-is.
- **Show Warning and Dialog (voter not allowed to proceed)**: A warning dialog is shown and the voter is blocked from submitting until the duplicated rank is resolved.

#### Invalid Vote - Skipped Ranks Policy
_Applies to preferential voting (e.g. Instant Runoff) only._

When a voter's ranking has gaps (e.g. positions 1, 2, 4, 5 — skipping 3), this policy controls the UX response:
- **Show Warning and Dialog (voter can proceed)**: A warning dialog is shown, but the voter is allowed to submit the ballot with the gap.
- **Show Warning and Dialog (voter not allowed to proceed)**: A warning dialog is shown and the voter is blocked from submitting until the gap is filled.

### Page Name
- **Define Page Name**  
  - Contests sharing the same page name will be grouped and displayed on the same voting page.
  - Contests without a page name appear on their own page by default.
- **Usage**  
  - Helps organize multi-contest ballots to reduce navigation steps for voters.

---

## Image
Upload an image or icon representing this contest (e.g., logo, symbol).

- **Upload**  
  - Drag and drop a PNG (or supported format) file.
- **Preview**  
  - The system displays a preview after upload.
- **Usage**  
  - May appear in voter-facing ballot UI or administration screens.

---

## Advanced Configuration
Import or upload a custom configuration file for specialized contest behavior.

- **Upload Configuration**  
  - Drag and drop a configuration file (JSON/YAML or system-specific format).
  - Applies settings such as custom validation rules, extended metadata, or integrations.
- **Example Use Cases**  
  - Custom ballot behavior not covered by default options.
  - Integration with external systems or specialized auditing tools.

---

## Notes & Tips
- Many settings mirror those in the Election Event Data tab; apply similar workflows where applicable.  
- Changes take effect immediately or on next publication, depending on system design.  
- Validate policy combinations to avoid conflicting rules (e.g., a policy cannot be both “Allowed” and “Not Allowed”).  
- Ensure that “Is acclaimed” is only enabled once the contest outcome is determined, to prevent accidental closure of voting.  
- For multilingual elections, verify translations of contest names and alerts.  

---

