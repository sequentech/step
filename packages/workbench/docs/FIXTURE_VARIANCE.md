# VARIANCE.md — Election/Ballot Fixture Variance Catalogue

## Fixture-location convention

Two directories hold JSON, with very different roles:

- [`packages/workbench/app/src/fixtures/snapshots/`](packages/workbench/app/src/fixtures/snapshots) — **bundled snapshots**. Each file is a full `PersistedSnapshot` and is eagerly imported at build time by [`bundledSnapshots.ts`](packages/workbench/app/src/fixtures/bundledSnapshots.ts#L36) (`import.meta.glob("./snapshots/*.json", { eager: true })`). The filename (minus `.json`) becomes the snapshot id surfaced in the inspector's Snapshots table. To add coverage, drop a JSON + matching `.license` sidecar here.
- [`packages/workbench/app/src/fixtures/velvet/`](packages/workbench/app/src/fixtures/velvet) — **reference election-config blobs**. Not loaded by any code path. They exist purely so an operator can copy them into a form field (or hand-promote one into `snapshots/`). Calling them "fixtures" in coverage tables is misleading; they exercise nothing on their own.

The coverage assessments below use the term **bundled fixture** for `snapshots/*.json` and **reference blob** for `velvet/*.json`.

---

## Executive Summary Table

| Dimension | Value Space Size | Coverage Gap | Notes |
|-----------|------------------|--------------|-------|
| Voters & assignments | Unbounded | Yes | Workbench-only; `activeVoterId` + `assignments` pool |
| Ballot-style count per election | 1..N | Yes | Bundled fixtures use 1 ballot style each |
| Contest-sharing across ballot styles | 3 classes | Yes | No bundled multi-ballot-style fixture; only reference blobs cover it |
| Contests per ballot style | 1..N | Yes | All bundled fixtures use exactly 1 contest |
| CountingAlgType | 10 variants | Significant | Only PluralityAtLarge + IRV bundled; 8 others unimplemented in velvet |
| min_votes / max_votes | [0..$maxint$] | Yes | Range coverage thin; bundled covers (1,1) and (0,3) |
| winning_candidates_num | [0..$maxint$] | Yes | Not systematically varied; always 1 in bundled fixtures |
| Candidates per contest | 1..N | Partial | Bundled fixtures use 2-3; reference blobs go up to 5 |
| allow_writeins | { true, false } | Yes | No bundled fixture sets allow_writeins (default true); only reference blobs set false |
| base32_writeins | { true, false } | Yes | Never exercised as false |
| InvalidVotePolicy | 4 variants | Yes | All defaults; no bundled fixture sets it |
| UnderVotePolicy | 4 variants | Yes | All defaults; no bundled fixture sets it |
| OverVotePolicy | 5 variants | Yes | All defaults; NOT_ALLOWED_WITH_MSG_AND_DISABLE missing everywhere |
| BlankVotePolicy | 4 variants | Yes | All defaults; no bundled fixture sets it |
| DuplicatedRankPolicy | 2 variants | Yes | Untested; first IRV bundled fixture (`instant-runoff-3cand`) leaves it default |
| PreferenceGapsPolicy | 2 variants | Yes | Untested; same as DuplicatedRankPolicy |
| CandidatesOrder | 3 variants | Yes | Random/Custom never exercised; Alphabetical default |
| CandidatesSelectionPolicy | 2 variants | Yes | CUMULATIVE default; RADIO untested |
| CandidatesIconCheckboxPolicy | 2 variants | Yes | SQUARE_CHECKBOX default; ROUND_CHECKBOX untested |
| EnableCheckableLists | 4 variants | Yes | Presentation code present but not in fixtures |
| CollapsibleLists | 3 variants | Yes | Never exercised |
| PaginationPolicy | String | Yes | No values tested |
| Columns | Numeric | Yes | Never set; defaults to 1 |
| CumulativeNumberOfCheckboxes | Numeric | Yes | Never exercised |
| ShuffleCategories | { true, false } | Yes | Only set in a reference blob; no bundled fixture exercises it |
| ShowPoints | { true, false } | Yes | Never exercised; always false or default |
| Tie-breaking-policy (Contest) | 2 variants | Yes | RANDOM default; EXTERNAL_PROCEDURE untested |
| WeightedVotingPolicy | 2 variants | Yes | Election-event level; DISABLED_WEIGHTED_VOTING default; AREAS_WEIGHTED_VOTING untested |
| DelegatedVotingPolicy | 2 variants | Yes | DISABLED default; ENABLED untested |
| Multi-ballot encoding (capacity) | Up to 30 bytes | Yes | Encoding limits not stress-tested |

---

## Dimension Details

### 1. Voters & Assignments (workbench-only overlay)

- **Field / type**: [`packages/workbench/app/src/workbenchStore.ts`](packages/workbench/app/src/workbenchStore.ts) (`Voter`, `activeVoterId: string | null`, `assignments: Record<string, string[]>`, `ballotStylePool: Record<string, unknown[]>`)
- **Value space**:
  - `Voter[]` — workbench-generated personas; displayName is free text; stable across reloads.
  - `activeVoterId` — `null` (anonymous/default) or a voter id; when set, cast votes are attributed to that voter.
  - `assignments: Record<voterId, ballotStyleIds[]>` — per-voter eligibility map; which ballot styles voter may receive; controls eligibility-overlay swap on voter change.
  - `ballotStylePool: Record<electionId, BallotStyle[]>` — full pool of ballot styles per election; portal's `ballotStyles` slice only ever holds one at a time.
- **Branching sites**:
  - [`packages/workbench/app/src/persistence.ts`](packages/workbench/app/src/persistence.ts) — `setActiveVoter` listener uses `assignments[voterId]` to rewrite portal slice from `ballotStylePool`.
  - [`packages/workbench/app/src/workbenchStore.ts:L163-L200`](packages/workbench/app/src/workbenchStore.ts#L163) — `setActiveVoter` mutation branches on `assignments` presence to pick which ballot styles to dispatch.
  - [`packages/workbench/app/src/workbenchStore.ts:L91-L120`](packages/workbench/app/src/workbenchStore.ts#L91) — `attributeCastVote` maps `castVote.id` to `activeVoterId` in `castBy` ledger.
- **Current fixture coverage**: 
  - `default.json` snapshot: voters array has two personas (Alice, Bob); `activeVoterId` initialized to `null`; `assignments` absent (pre-Phase-1).
  - `instant-runoff-3cand.json` snapshot: same workbench overlay shape as default; `assignments` absent.
  - Velvet reference blobs (`sample-election-config.json`, `velvet-*.json`): no workbench-extra state — these are not bundled snapshots, just election-config templates for paste-into-form use.
- **Velvet upstream variants**: Velvet fixture generators do not produce workbench-extra state; pure election configs only.
- **Coverage gap assessment**: No fixtures exercise `activeVoterId` swaps with multi-ballot-style eligibility (`assignments`). Multi-voter scenarios with cast-vote attribution only minimally tested in workbench manual testing, not in bundled snapshots.

---

### 2. Ballot-Style Count per Election

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L2405`](packages/sequent-core/src/ballot.rs#L2405) (`Election.contests: Vec<Contest>`) paired with [`packages/sequent-core/src/ballot.rs:L802`](packages/sequent-core/src/ballot.rs#L802) (`Election.id` → ballot_styles array in ElectionConfig)
- **Value space**: 1 to N ballot styles per election; each ballot style has one `election_id` reference.
- **Branching sites**:
  - [`packages/voting-portal/src/store/slices/ballotStyles.ts`](packages/voting-portal/src/store/slices/ballotStyles.ts) — Redux slice holds one ballot style per election at a time; booth shows one.
  - [`packages/workbench/app/src/workbenchStore.ts:L32`](packages/workbench/app/src/workbenchStore.ts#L32) — `ballotStylePool` indexed by election_id, holds all ballot styles for that election.
  - Tally aggregation (area-vs-contest operations) loops over all ballot styles per election.
- **Current fixture coverage**: 
  - Bundled snapshots (`snapshots/*.json`):
    - `default.json`: one election, one ballot style.
    - `instant-runoff-3cand.json`: one election, one ballot style.
  - Velvet reference blobs (`velvet/*.json`, not bundled — paste-into-form only):
    - `sample-election-config.json`: two ballot styles (different areas, different contests), one election.
    - `velvet-plurality-5cand.json`: one ballot style, one election.
    - `velvet-approval.json`: one ballot style, one election.
    - `velvet-multi-bs.json`: **two ballot styles**, two areas, **same contest id** shared between them (disjoint candidate pool) — most complex reference blob.
- **Velvet upstream variants**: 
  - [`packages/velvet/src/fixtures/elections.rs:L48`](packages/velvet/src/fixtures/elections.rs#L48) (`get_election_config_1`) — one ballot style.
  - [`packages/velvet/src/fixtures/elections.rs:L60`](packages/velvet/src/fixtures/elections.rs#L60) (`get_election_config_2`) — **two ballot styles**, different areas, same contest.
  - [`packages/velvet/src/fixtures/elections.rs:L100`](packages/velvet/src/fixtures/elections.rs#L100) (`get_election_config_3`) — one ballot style, hierarchical areas (parent_id set).
- **Coverage gap assessment**: Three or more ballot styles per election not exercised. Cross-ballot-style contest aggregation only marginally tested.

---

### 4. Contest-Sharing Across Ballot Styles (Equivalence Classes)

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1482`](packages/sequent-core/src/ballot.rs#L1482) (`Contest.id`) referenced by multiple ballot styles' `contests` arrays.
- **Value space**: Three equivalence classes per election:
  1. **Disjoint**: Each ballot style has unique contests (no id overlap).
  2. **Fully shared**: All ballot styles carry the same contest(s).
  3. **Partial**: Some ballot styles share a contest, others differ.
- **Branching sites**:
  - [`packages/voting-portal/src/store/slices/ballotSelections.ts`](packages/voting-portal/src/store/slices/ballotSelections.ts) — Redux slice maintains one `ballotSelections` state across all contests visible to active ballot style; booth UI renders contests from active style.
  - Tally: [`packages/velvet/src/pipes/do_tally/do_tally.rs`](packages/velvet/src/pipes/do_tally/do_tally.rs) — aggregates results per contest across all areas/ballot styles carrying that contest.
  - Area-contest matching (workbench): determines which contests are visible per area during tally.
- **Current fixture coverage**: 
  - Bundled: `default.json` and `instant-runoff-3cand.json` each have one ballot style → trivially "fully shared" (one style).
  - Reference blobs (not bundled):
    - `sample-election-config.json`: two ballot styles, **disjoint** contests (colour vs shape).
    - `velvet-multi-bs.json`: two ballot styles, **fully shared** contest id ("44444444-4444-4444-4444-4444444400c1") but **disjoint** candidate pools (Area A vs B have different candidate ids even though contest id is same). This is a **partial shared** scenario discovered in recent velvet-multi-bs audit.
- **Velvet upstream variants**: 
  - `get_election_config_1` — disjoint (one ballot style).
  - `get_election_config_2` — disjoint (two areas, different contests per style).
  - `get_election_config_3` — disjoint (one ballot style, hierarchical areas).
- **Coverage gap assessment**: 
  - Fully shared contests (same contest id, same candidates) across multiple ballot styles not explicitly tested.
  - Partial sharing (same contest id, different subsets of candidates per area) discovered but not yet fully exercised in tally pipeline.

---

### 5. Contests Per Ballot Style

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L2405`](packages/sequent-core/src/ballot.rs#L2405) (`BallotStyle.contests: Vec<Contest>`)
- **Value space**: 1 to N contests per ballot style; no fixed upper limit in type.
- **Branching sites**:
  - [`packages/voting-portal/src/components/BoothLayout.tsx`](packages/voting-portal/src/components/BoothLayout.tsx) — renders all contests from `state.ballotSelections` (indexed by contest_id).
  - [`packages/voting-portal/src/store/slices/ballotSelections.ts`](packages/voting-portal/src/store/slices/ballotSelections.ts) — initializes one selection entry per contest in ballot style.
  - Ballot encoding: [`packages/sequent-core/src/ballot_codec/multi_ballot.rs`](packages/sequent-core/src/ballot_codec/multi_ballot.rs) — encodes multiple contests' selections into fixed-size 30-byte payload.
- **Current fixture coverage**: All bundled fixtures and all reference blobs use exactly 1 contest per ballot style.
- **Velvet upstream variants**: All generators produce 1 contest per ballot style.
- **Coverage gap assessment**: Multi-contest ballot styles (N ≥ 2) never tested in fixtures; encoding capacity with multiple contests untested.

---

### 6. CountingAlgType (Enumeration & Tally Branching)

- **Field / type**: [`packages/sequent-core/src/types/ceremonies.rs:L323`](packages/sequent-core/src/types/ceremonies.rs#L323) (`pub enum CountingAlgType`); [`packages/sequent-core/src/ballot.rs:L1497`](packages/sequent-core/src/ballot.rs#L1497) (`Contest.counting_algorithm: Option<CountingAlgType>`)
- **Value space**: 10 variants
  - `PluralityAtLarge` (default)
  - `InstantRunoff`
  - `BordaNauru`
  - `Borda`
  - `BordaMasMadrid`
  - `PairwiseBeta`
  - `Desborda3`
  - `Desborda2`
  - `Desborda`
  - `Cumulative`
  
  **UI support** (TypeScript mirrors; see [`packages/ui-core/src/types/CoreTypes.ts:L15`](packages/ui-core/src/types/CoreTypes.ts#L15)): Only `PluralityAtLarge` and `InstantRunoff` are uncommented; others commented out until velvet tally support extends.

- **Branching sites** (Tally dispatch):
  - [`packages/velvet/src/pipes/do_tally/tally.rs:L110-L111`](packages/velvet/src/pipes/do_tally/tally.rs#L110) — `create_tally()` match on `CountingAlgType`: dispatches to `PluralityAtLarge::new()` or `InstantRunoff::new()`, errors on others.
  - [`packages/sequent-core/src/ballot_codec/bases.rs:L23-L26`](packages/sequent-core/src/ballot_codec/bases.rs#L23) — `get_bases()` match: `PluralityAtLarge` → base 2; `Cumulative` → `cumulative_number_of_checkboxes + 1`; others (preferential) → `max_votes + 1`.
  - [`packages/sequent-core/src/interpret_plaintext.rs:L64-L86`](packages/sequent-core/src/interpret_plaintext.rs#L64) — `get_contest_layout_properties()` match: distinct layout for each algorithm.
  - Voting booth (UI): [`packages/voting-portal/src/components/Answer/Answer.tsx:L82-L84`](packages/voting-portal/src/components/Answer/Answer.tsx#L82) — `isPreferential()` check (InstantRunoff only) switches answer rendering to ranked-choice style.

- **Current fixture coverage**: 
  - Bundled: `default.json` uses `PluralityAtLarge`; `instant-runoff-3cand.json` uses `InstantRunoff` (3 candidates, min=0, max=3).
  - Reference blobs: all use `PluralityAtLarge`.
  - No bundled Borda, Desborda, Cumulative, or Pairwise fixture exists — see velvet `create_tally()` limitation below.

- **Velvet upstream variants**: 
  - `get_contest_1()` — `PluralityAtLarge`.
  - `get_contest_min_max_votes()` — `PluralityAtLarge`.
  - No velvet generators produce non-default algorithms.

- **Coverage gap assessment**: **Severe gap**. Eight of ten counting algorithm variants untested anywhere; tally implementations for Borda, Desborda, Cumulative, Pairwise do not exist in velvet (`create_tally()` only dispatches PluralityAtLarge and InstantRunoff). The new `instant-runoff-3cand.json` is the first bundled fixture exercising the IRV path; preference-related policies remain at defaults.

---

### 7. min_votes / max_votes / winning_candidates_num on Contest

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1482`](packages/sequent-core/src/ballot.rs#L1482) (`Contest.min_votes: i64`, `max_votes: i64`, `winning_candidates_num: i64`)
- **Value space**: 
  - `min_votes`: [0, $\infty$); semantically ≤ `max_votes`.
  - `max_votes`: [0, $\infty$); semantically ≤ candidate count.
  - `winning_candidates_num`: [0, $\infty$); semantically ≤ candidate count.
  - Typical ranges in fixtures: min_votes ∈ {0, 1}, max_votes ∈ {1, 2, 3}, winning_candidates_num ∈ {1}.

- **Branching sites** (Validation):
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L37`](packages/sequent-core/src/ballot_codec/checker.rs#L37) — `check_max_min_votes_policy()`: validates max/min are convertible to usize; returns error if not.
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L80`](packages/sequent-core/src/ballot_codec/checker.rs#L80) — `check_min_vote_policy()`: if num_selected < min_votes, error.
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L137`](packages/sequent-core/src/ballot_codec/checker.rs#L137) — `check_over_vote_policy()`: if num_selected > max_votes, errors; alerts depend on `over_vote_policy`.
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L197`](packages/sequent-core/src/ballot_codec/checker.rs#L197) — `check_under_vote_policy()`: if num_selected < max_votes (and ≥ min_votes), alert depends on `under_vote_policy`.
  - Ballot encoding (bases): [`packages/sequent-core/src/ballot_codec/bases.rs:L23`](packages/sequent-core/src/ballot_codec/bases.rs#L23) — base computed as `max_votes + 1` for preferential; dimension of choice space.
  - UI (voting portal): [`packages/voting-portal/src/components/Question/Question.tsx`](packages/voting-portal/src/components/Question/Question.tsx) — contest rendering and validation depend on max/min for checkbox limit enforcement.

- **Current fixture coverage**: 
  - Bundled:
    - `default.json`: max_votes=1, min_votes=1, winning_candidates_num=1.
    - `instant-runoff-3cand.json`: max_votes=3, min_votes=0, winning_candidates_num=1.
  - Reference blobs (not bundled):
    - `sample-election-config.json`: max_votes=1, min_votes=1, winning_candidates_num=1.
    - `velvet-plurality-5cand.json`: max_votes=1, min_votes=0, winning_candidates_num=1 (under-vote allowed).
    - `velvet-approval.json`: min_votes=1, max_votes=3, winning_candidates_num=1 (only blob with min>0 and max>1).
    - `velvet-multi-bs.json`: max_votes=1, min_votes=0 (per-area contest, disjoint candidate ids).

- **Velvet upstream variants**: 
  - `get_contest_1()` — max=1, min=0.
  - `get_contest_min_max_votes(min, max)` — parameterizable; used to generate velvet-approval (min=1, max=3).

- **Coverage gap assessment**: 
  - **Range**: Only tested min ∈ {0,1}, max ∈ {1,3}. Missing: min ≥ 2, max ≥ 4, winning_candidates_num > 1.
  - **Edge cases**: max=0 (impossible vote), min > max (invalid), negative values — not tested.
  - **Interaction with under/over/blank vote policies**: Validation logic is dense; boundary conditions (num_selected exactly at min/max) under-exercised.

---

### 8. Candidates Per Contest & Write-In Support

- **Field / type**: 
  - [`packages/sequent-core/src/ballot.rs:L1482`](packages/sequent-core/src/ballot.rs#L1482) (`Contest.candidates: Vec<Candidate>`)
  - [`packages/sequent-core/src/ballot.rs:L1408-L1439`](packages/sequent-core/src/ballot.rs#L1408) (`ContestPresentation.allow_writeins: Option<bool>`, `base32_writeins: Option<bool>`)
  - [`packages/sequent-core/src/ballot.rs:L2048`](packages/sequent-core/src/ballot.rs#L2048) (`Candidate.presentation.is_write_in: Option<bool>`)

- **Value space**: 
  - Candidate count: 1 to N; typical 2–5 in fixtures.
  - `allow_writeins`: { true, false }; default true.
  - `base32_writeins`: { true, false }; default true (base32 encoding for write-in text).
  - `is_write_in` (candidate marker): true for write-in candidate entries (placeholder).

- **Branching sites**:
  - Voting-portal UI: [`packages/voting-portal/src/services/ElectionConfigService.ts:L35-L36`](packages/voting-portal/src/services/ElectionConfigService.ts#L35) — `checkAllowWriteIns()` checks `presentation?.allow_writeins` to show/hide write-in input fields.
  - [`packages/voting-portal/src/components/Answer/Answer.tsx`](packages/voting-portal/src/components/Answer/Answer.tsx) — renders write-in candidates distinctly if `is_write_in` true.
  - Ballot encoding: [`packages/sequent-core/src/ballot_codec/bases.rs:L45-L54`](packages/sequent-core/src/ballot_codec/bases.rs#L45) — if `allow_writeins()`, adds bases for each write-in candidate's character map.
  - Character set: [`packages/sequent-core/src/ballot_codec/character_map.rs`](packages/sequent-core/src/ballot_codec/character_map.rs) — if `base32_writeins`, base 32; else base (ASCII).

- **Current fixture coverage**: 
  - Bundled:
    - `default.json`: 2 candidates; `allow_writeins` not set (default true).
    - `instant-runoff-3cand.json`: 3 candidates; `allow_writeins` not set (default true).
  - Reference blobs:
    - `sample-election-config.json`: 2 candidates per contest; `allow_writeins` absent.
    - `velvet-plurality-5cand.json`, `velvet-approval.json`: 5 candidates each; `allow_writeins=false` (only blobs setting false).
    - `velvet-multi-bs.json`: 5 candidates per area-specific contest; `allow_writeins` absent (default true).
  - **No fixture (bundled or blob) actually tests write-in submission or encoding.**

- **Velvet upstream variants**: All generators set `allow_writeins=false`.

- **Coverage gap assessment**: 
  - Write-in encoding path never exercised in actual ballots (templates exist, no test ballots).
  - `base32_writeins=false` never tested; fallback character encoding untested.
  - Edge case: candidate with `is_write_in=true` but `allow_writeins=false` — undefined behavior not tested.

---

### 10. Per-Contest Presentation Policies

#### 10.1 InvalidVotePolicy

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L833`](packages/sequent-core/src/ballot.rs#L833) (`pub enum InvalidVotePolicy`)
- **Value space**: 4 variants
  - `ALLOWED` (default) — explicitly invalid candidates are allowed.
  - `WARN` — warn if user selects explicit invalid.
  - `WARN_INVALID_IMPLICIT_AND_EXPLICIT` — warn on both implicit and explicit invalidity.
  - `NOT_ALLOWED` — reject ballots with explicit invalid selections.
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L281`](packages/sequent-core/src/ballot_codec/checker.rs#L281) — `check_invalid_vote_policy()`: if explicit invalid selected and policy != ALLOWED, add error or alert.
  - [`packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx:L75-L76`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx#L75) — retrieves policy; displays warnings/errors.
- **Current fixture coverage**: `invalid_vote_policy: ALLOWED` set in `velvet-plurality-5cand.json` reference blob; no bundled fixture sets it (defaults to ALLOWED).
- **Velvet upstream variants**: `get_contest_1()` sets ALLOWED.
- **Coverage gap assessment**: WARN, WARN_INVALID_IMPLICIT_AND_EXPLICIT, NOT_ALLOWED never tested.

#### 10.2 UnderVotePolicy

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1128`](packages/sequent-core/src/ballot.rs#L1128) (`pub enum EUnderVotePolicy`)
- **Value space**: 4 variants
  - `ALLOWED` (default) — under-voting (selecting fewer than max_votes) is fine.
  - `WARN` — warn if num_selected < max_votes (but ≥ min_votes).
  - `WARN_ONLY_IN_REVIEW` — warn only on review screen, not during voting.
  - `WARN_AND_ALERT` — warn + popup alert.
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L197`](packages/sequent-core/src/ballot_codec/checker.rs#L197) — `check_under_vote_policy()`: checks if num_selected < max_votes and policy != ALLOWED, adds alert.
  - Voting-portal: [`packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx:L71-L72`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx#L71) — reads policy; display logic.
- **Current fixture coverage**: `under_vote_policy: ALLOWED` set in `velvet-plurality-5cand.json` reference blob; no bundled fixture sets it (defaults ALLOWED).
- **Velvet upstream variants**: `get_contest_1()` sets ALLOWED.
- **Coverage gap assessment**: WARN, WARN_ONLY_IN_REVIEW, WARN_AND_ALERT never tested.

#### 10.3 OverVotePolicy

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1192`](packages/sequent-core/src/ballot.rs#L1192) (`pub enum EOverVotePolicy`)
- **Value space**: 5 variants
  - `ALLOWED` — over-voting is silently allowed (extra votes ignored).
  - `ALLOWED_WITH_MSG` — allow with message.
  - `ALLOWED_WITH_MSG_AND_ALERT` (default) — allow with popup alert.
  - `NOT_ALLOWED_WITH_MSG_AND_ALERT` — reject with message + alert.
  - `NOT_ALLOWED_WITH_MSG_AND_DISABLE` — disable checkboxes when max reached (strict UX).
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L137`](packages/sequent-core/src/ballot_codec/checker.rs#L137) — `check_over_vote_policy()`: if num_selected > max_votes, error added; alerts vary by policy.
  - Voting-portal: [`packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx:L77-L78`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx#L77) — reads policy; checkbox disabling logic in Answer component.
- **Current fixture coverage**: `over_vote_policy: ALLOWED_WITH_MSG_AND_ALERT` set in `velvet-plurality-5cand.json` reference blob; no bundled fixture sets it (defaults ALLOWED_WITH_MSG_AND_ALERT).
- **Velvet upstream variants**: `get_contest_1()` sets ALLOWED_WITH_MSG_AND_ALERT.
- **Coverage gap assessment**: ALLOWED, ALLOWED_WITH_MSG, NOT_ALLOWED_WITH_MSG_AND_ALERT, NOT_ALLOWED_WITH_MSG_AND_DISABLE rarely tested; strict checkbox-disable UX (NOT_ALLOWED_WITH_MSG_AND_DISABLE) never exercised.

#### 10.4 BlankVotePolicy

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1160`](packages/sequent-core/src/ballot.rs#L1160) (`pub enum EBlankVotePolicy`)
- **Value space**: 4 variants
  - `ALLOWED` (default) — blank ballots OK.
  - `WARN` — warn if no selections made.
  - `WARN_ONLY_IN_REVIEW` — warn on review only.
  - `NOT_ALLOWED` — reject blank ballots (enforce min_votes ≥ 1).
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L103`](packages/sequent-core/src/ballot_codec/checker.rs#L103) — `check_blank_vote_policy()`: if num_selected == 0 and policy != ALLOWED, add alert or error.
  - Voting-portal: [`packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx:L73-L74`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx#L73) — display logic.
- **Current fixture coverage**: `blank_vote_policy` absent in all fixtures (defaults ALLOWED).
- **Velvet upstream variants**: Not set in generators.
- **Coverage gap assessment**: WARN, WARN_ONLY_IN_REVIEW, NOT_ALLOWED never tested.

#### 10.5 DuplicatedRankPolicy (Preferential Voting)

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1243`](packages/sequent-core/src/ballot.rs#L1243) (`pub enum EDuplicatedRankPolicy`)
- **Value space**: 2 variants
  - `ALLOWED_WARN_AND_DIALOG` (default) — allow duplicate ranks but warn with dialog.
  - `NOT_ALLOWED_WARN_AND_DIALOG` — reject duplicates with warning dialog.
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L235`](packages/sequent-core/src/ballot_codec/checker.rs#L235) — `check_duplicated_rank_policy()`: validates ranked votes; error/alert if duplicates and policy rejects.
  - Applies only to preferential (InstantRunoff, Borda*) contests.
- **Current fixture coverage**: `duplicated_rank_policy` not set anywhere. The new `instant-runoff-3cand.json` bundled fixture is the first preferential contest — leaves this at default.
- **Velvet upstream variants**: Not set; no IRV fixtures to test.
- **Coverage gap assessment**: Both values untested; dependence on preferential-only semantics untested.

#### 10.6 PreferenceGapsPolicy (Preferential Voting)

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1267`](packages/sequent-core/src/ballot.rs#L1267) (`pub enum EPreferenceGapsPolicy`)
- **Value space**: 2 variants
  - `ALLOWED_WARN_AND_DIALOG` (default) — gaps in rankings allowed (e.g., rank 1, rank 3, skip rank 2).
  - `NOT_ALLOWED_WARN_AND_DIALOG` — enforce contiguous ranking.
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L258`](packages/sequent-core/src/ballot_codec/checker.rs#L258) — `check_preference_gaps_policy()`: validates no gaps if policy requires contiguous ranks.
  - Preferential-only.
- **Current fixture coverage**: Not set anywhere. The new `instant-runoff-3cand.json` bundled fixture is the first preferential contest — leaves this at default.
- **Velvet upstream variants**: Not set.
- **Coverage gap assessment**: Both variants untested; gap validation untested.

#### 10.7 CandidatesOrder

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L575`](packages/sequent-core/src/ballot.rs#L575) (`pub enum CandidatesOrder`)
- **Value space**: 3 variants
  - `Random` — shuffle candidates on each ballot.
  - `Custom` — display in fixture-defined order.
  - `Alphabetical` (default) — sort by name.
- **Branching sites**:
  - Voting-portal: [`packages/voting-portal/src/components/AnswersList/AnswersList.tsx:L98`](packages/voting-portal/src/components/AnswersList/AnswersList.tsx#L98) — `sortCandidatesInContest()` dispatches on `candidatesOrderType`.
  - [`packages/voting-portal/src/components/Question/Question.tsx:L178`](packages/voting-portal/src/components/Question/Question.tsx#L178) — sets `candidatesOrderType`; used for rendering.
- **Current fixture coverage**: `candidates_order` absent in all fixtures (defaults Alphabetical).
- **Velvet upstream variants**: Not set.
- **Coverage gap assessment**: Random and Custom never exercised; only Alphabetical (default) present.

#### 10.8 CandidatesSelectionPolicy

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1001`](packages/sequent-core/src/ballot.rs#L1001) (`pub enum CandidatesSelectionPolicy`)
- **Value space**: 2 variants
  - `Radio` — single selection; deselects previous when new one clicked.
  - `Cumulative` (default) — multiple independent selections (checkboxes).
- **Branching sites**:
  - Voting-portal: [`packages/voting-portal/src/components/Question/Question.tsx`](packages/voting-portal/src/components/Question/Question.tsx) — toggle button or checkbox based on policy (not explicitly branching, but UI adapts).
  - Redux ballot selections: deselection logic in reducer depends on this.
- **Current fixture coverage**: Absent in all fixtures (defaults Cumulative).
- **Velvet upstream variants**: Not set.
- **Coverage gap assessment**: Radio selection (single-choice) never tested.

#### 10.9 CandidatesIconCheckboxPolicy

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1055`](packages/sequent-core/src/ballot.rs#L1055) (`pub enum CandidatesIconCheckboxPolicy`)
- **Value space**: 2 variants
  - `SquareCheckbox` (default) — standard checkbox icon.
  - `RoundCheckbox` — radio-button icon (visual only, regardless of selection policy).
- **Branching sites**:
  - Voting-portal UI: Icon selection in answer component.
  - Largely presentational; no logic branching.
- **Current fixture coverage**: Absent (defaults SquareCheckbox).
- **Velvet upstream variants**: Not set.
- **Coverage gap assessment**: RoundCheckbox never tested.

#### 10.10 EnableCheckableLists, CollapsibleLists, ShuffleCategories, Columns, Pagination, Show/CumulativeCheckboxes

- **Fields**: [`packages/sequent-core/src/ballot.rs:L1408-L1432`](packages/sequent-core/src/ballot.rs#L1408)
- **Value spaces**:
  - `enable_checkable_lists: Option<String>` — { "disabled", "allow-selecting-candidates-and-lists", "allow-selecting-candidates", "allow-selecting-lists" }; default undefined (disabled).
  - `collapsible_lists: Option<String>` — { "disabled", "enabled-expanded", "enabled-collapsed" }; default undefined.
  - `shuffle_categories: Option<bool>` — default false.
  - `columns: Option<u64>` — numeric; default undefined (1 column).
  - `pagination_policy: Option<String>` — string; no defined values tested.
  - `cumulative_number_of_checkboxes: Option<u64>` — numeric; used for Cumulative voting algorithm base.
  - `show_points: Option<bool>` — display point tallies; default false.

- **Branching sites**:
  - Voting-portal: [`packages/voting-portal/src/components/Question/Question.tsx:L151-L214`](packages/voting-portal/src/components/Question/Question.tsx#L151) — `getCheckableOptions()`, `collapsibleListsPolicy`, `columnCount` determine category presentation.
  - Cumulative: [`packages/sequent-core/src/ballot_codec/bases.rs:L24`](packages/sequent-core/src/ballot_codec/bases.rs#L24) — base = cumulative_number_of_checkboxes + 1.

- **Current fixture coverage**: 
  - `shuffle_categories=true` only in `velvet-plurality-5cand.json` reference blob (not bundled).
  - All other fixtures absent (defaults).
  - Checkable-lists, collapsible-lists, columns, pagination, show_points never set anywhere.

- **Velvet upstream variants**: `get_contest_1()` sets `shuffle_categories=true`.

- **Coverage gap assessment**: 
  - List categories with checkable/collapsible UI never exercised; code present but untested in fixtures.
  - Columnar layouts never set (always 1 column).
  - Pagination policy never exercised.
  - Show_points (display vote counts) never exercised.

#### 10.11 TieBreakingPolicy (on Contest, not Presentation)

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1482`](packages/sequent-core/src/ballot.rs#L1482) (`Contest.tie_breaking_policy: Option<TieBreakingPolicy>`)
- **Value space**: 2 variants
  - `Random` (default) — break ties by random draw.
  - `ExternalProcedure` — defer to external (human) tie-breaking.
- **Branching sites**:
  - Tally: [`packages/velvet/src/pipes/do_tally/tally.rs`](packages/velvet/src/pipes/do_tally/tally.rs) — when instantaneous runoff reaches a tie, invokes tie-breaking based on policy. (Implementation not fully visible; depends on velvet-core.)
  - Workbench: [`packages/workbench/app/src/contestDetail/TallyPage.tsx`](packages/workbench/app/src/contestDetail/TallyPage.tsx) — displays pending tie-break resolutions if policy is ExternalProcedure.
- **Current fixture coverage**: Absent in all fixtures (defaults Random).
- **Velvet upstream variants**: Not set.
- **Coverage gap assessment**: ExternalProcedure tie-breaking (manual resolution UI) never tested.

---

### 11. Election-Event-Level Policies

#### 11.1 WeightedVotingPolicy

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L2554`](packages/sequent-core/src/ballot.rs#L2554) (`pub enum WeightedVotingPolicy`)
- **Value space**: 2 variants
  - `DisabledWeightedVoting` (default) — all votes weighted 1:1.
  - `AreasWeightedVoting` — votes per area weighted by area annotation (`AreaAnnotations.weight`).
- **Branching sites**:
  - Tally: [`packages/velvet/src/pipes/do_tally/tally.rs:L30-L44`](packages/velvet/src/pipes/do_tally/tally.rs#L30) — `get_ballots()` pairs each decoded vote with a `Weight` from ballot file metadata or area annotations; weighted votes passed to tally.
  - Tally aggregation: [`packages/velvet/src/pipes/do_tally/do_tally.rs:L244`](packages/velvet/src/pipes/do_tally/do_tally.rs#L244) — children areas' weights accumulated.
- **Current fixture coverage**: WeightedVotingPolicy not set in any fixture; no area weights in annotations.
- **Velvet upstream variants**: Not set; election_event_annotations absent.
- **Coverage gap assessment**: AreasWeightedVoting never tested; weight accumulation and normalization untested.

#### 11.2 DelegatedVotingPolicy & other ElectionEventPresentation fields

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L966`](packages/sequent-core/src/ballot.rs#L966) (`pub struct ElectionEventPresentation`)
- **Value space**: Multiple policies (DelegatedVotingPolicy, ConsolidatedReportPolicy, Enrollment, Otp, VoterSigningPolicy, etc.); see source.
- **Branching sites**: Primarily at API/backend authorization level (outside booth/tally scope); not branched in voting-portal fixture code.
- **Current fixture coverage**: Not set; election_event_presentation typically absent.
- **Coverage gap assessment**: Election-event policies outside voting booth scope; minimal portal/tally dependence tested in fixtures.

---

### 17. Multi-Ballot Encoding Capacity & Limits

- **Field / type**: [`packages/sequent-core/src/ballot_codec/multi_ballot.rs:L202`](packages/sequent-core/src/ballot_codec/multi_ballot.rs#L202) — fixed 30-byte encoding size.
- **Value space**: Single encoded ballot must fit in 30 bytes; capacity depends on contest count and max_votes per contest.
  - Capacity formula (approximate): sum(log2(candidate_count) * max_votes for all contests) < 30*8 bits.
  - Multi-ballot codec only supports Plurality-at-large; comment at line 742 enforces this.
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/multi_ballot.rs:L719-L742`](packages/sequent-core/src/ballot_codec/multi_ballot.rs#L719) — `get_bases()` validates no non-PluralityAtLarge contests; errors if mixed.
  - Encoding: [`packages/sequent-core/src/ballot_codec/multi_ballot.rs:L240-L250`](packages/sequent-core/src/ballot_codec/multi_ballot.rs#L240) — comment documents capacity constraint.
- **Current fixture coverage**: 
  - All fixtures: 1 contest per ballot style → capacity never stressed.
  - No multi-contest ballots tested.
- **Velvet upstream variants**: Single-contest only.
- **Coverage gap assessment**: Multi-contest encoding capacity (near-limit scenarios) never tested; capacity overflow behavior untested.

---

## Discovered (Bonus) Dimensions

### D.1 Tally Operation Scope & Aggregation

- **Field / type**: [`packages/sequent-core/src/types/ceremonies.rs:L290`](packages/sequent-core/src/types/ceremonies.rs#L290) (`pub enum TallyOperation`)
- **Value space**: 3 variants
  - `ProcessBallotsAll` — count votes per candidate; report participation.
  - `AggregateResults` — sum area/contest results (no per-candidate detail).
  - `SkipCandidateResults` — participation only, no results.
- **Branching sites**:
  - Tally orchestration: [`packages/velvet/src/pipes/do_tally/do_tally.rs`](packages/velvet/src/pipes/do_tally/do_tally.rs) — dispatches tally per operation scope.
  - Default per algorithm: [`packages/sequent-core/src/types/ceremonies.rs:L362-L371`](packages/sequent-core/src/types/ceremonies.rs#L362) — `CountingAlgType::get_default_tally_operation_for_contest()` assigns ProcessBallotsAll for preferential, AggregateResults for Plurality.
- **Current fixture coverage**: Operation type implicit in default algorithm; not explicitly tested as a varying dimension.
- **Coverage gap assessment**: AggregateResults and SkipCandidateResults behavior rarely exercised; preferential-specific defaults untested.


---

## Summary of Coverage Gaps (High-Priority)

| Dimension | Gap | Impact | Priority |
|-----------|-----|--------|----------|
| CountingAlgType — 9/10 unsupported | Only Plurality-at-Large + minimal IRV tested | Tally dispatch, ballot encoding, validation | Critical |
| Contest-sharing (disjoint/shared/partial) | Only disjoint and partial-with-disjoint-candidates tested | Multi-ballot-style aggregation incomplete | High |
| Preference policies (Dup/Gap/etc.) | Untested; depends on IRV | Validation of preferential votes incomplete | High |
| Vote constraint policies (under/over/blank/invalid) | Only ALLOWED tested | Validation policy branching incomplete | High |
| Multi-contest ballot styles | Never tested | 30-byte encoding capacity untested | Medium |
| Write-in encoding (allow_writeins=true, text submission) | Never exercised end-to-end | Write-in text encoding/decoding untested | Medium |
| UI policies (shuffle, columns, checkable-lists, etc.) | Not in fixtures | UI path testing requires browser/visual tests | Low |

---

## Recommendations for Extended Fixture Coverage

1. **Priority 1: CountingAlgType variants**
   - ✅ `instant-runoff-3cand.json` bundled: min_votes=0, max_votes=3, 3 candidates (minimal IRV) — first preferential bundled fixture.
   - Borda/Desborda/Cumulative/Pairwise fixtures are blocked: velvet's `create_tally()` only dispatches PluralityAtLarge and InstantRunoff and errors on the rest (see [packages/velvet/src/pipes/do_tally/tally.rs](packages/velvet/src/pipes/do_tally/tally.rs#L109-L115)). Defer until velvet tally support lands.

2. **Priority 2: Vote validation policies**
   - Extend velvet fixtures to exercise all combinations of (invalid_vote_policy, under_vote_policy, over_vote_policy, blank_vote_policy) in a small matrix.

3. **Priority 3: Multi-ballot-style scenarios**
   - Create fixture with 3+ ballot styles (fully shared contest, disjoint contests, partial).
   - Test contest aggregation across 3+ areas.


---

End of variance catalogue.
