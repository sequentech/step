---
id: velvet_instant_runoff
title: Instant Runoff Algorithm
sidebar_position: 5
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Instant Runoff Voting Algorithm

The Instant Runoff Voting (IRV) algorithm, also known as Ranked-Choice Voting (RCV) or Alternative Vote, is a single-winner electoral system where voters rank candidates in order of preference.

## Algorithm Overview

In IRV, if no candidate receives a majority of first-preference votes, the candidate with the fewest votes is eliminated. Votes for the eliminated candidate are then redistributed to each voter's next preference. This process continues until a candidate achieves a majority or a tie condition is reached.

## Key Features

- **Ranked ballots** - Voters rank candidates in order of preference
- **Majority requirement** - A candidate must receive more than 50% of active votes to win
- **Elimination rounds** - Candidates with the fewest votes are eliminated sequentially
- **Vote redistribution** - When a candidate is eliminated, their votes transfer to the next-ranked active candidate
- **Exhausted ballots** - Ballots with no remaining valid choices become exhausted and are removed from the count
- **Tie-breaking** - Look-back rule attempts to break ties by examining previous rounds

## Implementation Details

### Location

**File**: `/packages/velvet/src/pipes/do_tally/counting_algorithm/instant_runoff.rs`

### Core Structures

#### ECandidateStatus

Tracks whether a candidate is still active or has been eliminated:

```rust
pub enum ECandidateStatus {
    Active,
    Eliminated,
}
```

#### BallotStatus

Tracks the state of each ballot throughout the counting process:

```rust
enum BallotStatus {
    Valid,      // Ballot is still counting
    Exhausted,  // No more valid choices remain
    Invalid,    // Ballot is invalid
    Blank,      // Ballot has no selections
}
```

#### Round

Represents the results of a single elimination round:

```rust
pub struct Round {
    pub winner: Option<String>,
    pub candidates_wins: CandidatesWins,  // Vote count for each candidate
    pub eliminated_candidates: Option<Vec<String>>,
    pub active_candidates_count: u64,
    pub active_ballots_count: u64,
}
```

#### RunoffStatus

Manages the overall state of the IRV process:

```rust
pub struct RunoffStatus {
    pub candidates_status: CandidatesStatus,
    pub round_count: u64,
    pub rounds: Vec<Round>,
    pub max_rounds: u64,
}
```

### Algorithm Flow

#### 1. Initialization

```rust
fn initialize_statuses(votes: &Vec<(DecodedVoteContest, Weight)>, contest: &Contest) -> BallotsStatus
```

- Classify each ballot as valid, invalid, or blank
- Count explicit and implicit invalid votes
- Calculate initial metrics
- All candidates start with `Active` status

#### 2. Round Execution

```rust
fn run_next_round(&mut self, ballots_status: &mut BallotsStatus) -> bool
```

For each round:

1. **Count first preferences** - For each active ballot, find the highest-ranked candidate who hasn't been eliminated
2. **Check for winner** - If any candidate has > 50% of active votes, they win
3. **Eliminate candidates** - If no winner, eliminate the candidate(s) with the fewest votes
4. **Update ballot status** - Mark ballots as exhausted if they have no more valid preferences
5. **Record round** - Store results for this round

#### 3. Vote Redistribution

```rust
fn find_first_active_choice(&self, choices: &Vec<DecodedVoteChoice>, active_candidates: &Vec<String>) -> Option<String>
```

When counting votes in a round:

- Examine each ballot's ranked choices in order
- Skip eliminated candidates
- Assign the vote to the first active candidate in the ranking
- If no active candidates remain on the ballot, mark it as exhausted

#### 4. Elimination Logic

```rust
fn do_round_eliminations(&mut self, candidates_wins: &CandidatesWins, candidates_to_eliminate: &Vec<String>) -> Option<Vec<String>>
```

When eliminating candidates:

- Find the candidate(s) with the fewest votes
- If there's a tie for fewest votes, apply the look-back rule
- If all remaining candidates are tied, no elimination occurs (tie for winner)
- Simultaneous elimination can occur when multiple candidates have the same fewest votes and can't be broken by look-back

#### 5. Tie-Breaking (Look-Back Rule)

```rust
fn find_single_candidate_to_eliminate(&self, candidates_to_eliminate: &Vec<String>) -> Vec<String>
```

When multiple candidates are tied for elimination:

1. Go back to the previous round
2. Compare vote counts in that round for the tied candidates
3. Eliminate the candidate with fewer votes in that round
4. If still tied, continue looking back through earlier rounds
5. If all rounds are exhausted and tie persists, all tied candidates may be eliminated simultaneously

### Winner Determination

A candidate wins if they receive **more than 50%** of active votes in a round:

```rust
let max_wins = candidates_wins.values().max().unwrap_or(&0);
if *max_wins > act_ballots / 2 {
    // Winner found
    round.winner = Some(candidate_id);
}
```

### Result Calculation

The final `ContestResult` includes:

- **Vote counts** - Total votes for each candidate in the final/winning round
- **Percentages** - Calculated based on valid votes (excluding blanks)
- **Invalid votes** - Separated into explicit and implicit
- **Blank votes** - Counted separately
- **Extended metrics** - Total ballots, participation rates, etc.

Percentages are calculated as:

- **For regular candidates**: `(total_count / (count_valid - count_blank)) * 100`
- **For explicit blank**: `(count_blank / total_ballots) * 100`
- **For explicit invalid**: `(explicit_invalid / total_ballots) * 100`

### Results for each round

*Documentation to be added.*


### Edge Cases

#### Exhausted Ballots

When a ballot's ranked preferences are exhausted (all preferred candidates eliminated):

- The ballot is marked as `Exhausted`
- It no longer counts in the active ballot total
- The majority threshold is recalculated based on remaining active ballots

#### Simultaneous Elimination

When multiple candidates are tied for fewest votes and the tie cannot be broken:

- All tied candidates may be eliminated simultaneously
- This can create corner cases in some scenarios
- Some electoral systems handle this differently (e.g., random selection)

#### Winner Tie

If all remaining candidates have equal votes and cannot be separated:

- No eliminations occur
- The algorithm terminates
- Winner determination is left to manual tie-breaking procedures

#### Maximum Rounds

A safety limit prevents infinite loops:

```rust
let max_rounds = candidates.len() as u64 + 1;
```

At least one candidate should be eliminated per round, so rounds shouldn't exceed the number of candidates plus one.

## Testing

The IRV algorithm has comprehensive test coverage:

### Unit Tests

**File**: `/packages/velvet/tests/irv_unit_tests.rs`

Tests individual components:
- Ballot status initialization
- Vote redistribution logic
- Elimination logic
- Tie-breaking rules
- Edge cases

### Integration Tests

**File**: `/packages/velvet/tests/irv_integration_tests.rs`

Tests complete IRV tallies:
- Simple majority cases
- Multi-round eliminations
- Exhausted ballot handling
- Tie scenarios
- Complex real-world examples

Run tests with:

```bash
cargo test instant_runoff
cargo test irv
```

## Usage Example

The InstantRunoff algorithm is configured in the election's contest configuration. When the tally is executed, Velvet automatically uses the IRV algorithm based on the contest's `tally_type` setting.

```rust
pub struct InstantRunoff {
    pub tally: Tally,
}

impl CountingAlgorithm for InstantRunoff {
    fn tally(&self) -> Result<ContestResult> {
        // Initialize ballot and candidate statuses
        let mut ballots_status = BallotsStatus::initialize_statuses(votes, contest);
        let mut runoff = RunoffStatus::initialize_statuses(&contest.candidates);
        
        // Run elimination rounds until winner or tie
        runoff.run(&mut ballots_status);
        
        // Generate and return results
        // ...
    }
}
```

## Performance Considerations

- **Memory**: The algorithm stores all ballots in memory with their statuses
- **Complexity**: O(n × m × r) where n = ballots, m = candidates, r = rounds
- **Typical rounds**: Most elections conclude in 2-4 rounds
- **Worst case**: Maximum rounds equals number of candidates

## References

- IRV is used in various jurisdictions worldwide including Australia, Ireland, and many U.S. cities
- Also known as: Ranked-Choice Voting (RCV), Alternative Vote (AV), Preferential Voting
- Single-winner variant of the Single Transferable Vote (STV) system
