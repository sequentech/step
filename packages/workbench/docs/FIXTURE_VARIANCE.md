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
| Voters & assignments | Unbounded | Partial | `multi-bs-shared-contest` exercises `assignments` + per-voter BS swap |
| Elections per snapshot | 1..N | Partial | `two-elections` covers N=2; N≥3 untested |
| Ballot-style count per election | 1..N | Partial | `multi-bs-shared-contest` covers N=2; N≥3 untested |
| Contest-sharing across ballot styles | 3 classes | Partial | `multi-bs-shared-contest` covers "partial" (shared+disjoint mix) |
| Contests per ballot style | 1..N | Partial | `mixed-3contests` covers N=3; capacity-near-limit untested |
| CountingAlgType | 10 variants | Significant | Only PluralityAtLarge + IRV bundled (incl. mixed on one ballot); 8 others unimplemented in velvet |
| min_votes / max_votes | [0..$maxint$] | Yes | Bundled covers (1,1), (0,3), (1,2); range still thin |
| winning_candidates_num | [0..$maxint$] | Partial | `mixed-3contests` covers winning=2; values ≥3 untested |
| Candidates per contest | 1..N | Partial | Bundled fixtures use 2-4 (max is `mixed-3contests` City council); reference blobs go up to 5 |
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
| **DeclineToVotePolicy** (§13.1) | 2 variants | Yes | New upstream; never set. Selections carry `is_decline_to_vote: false` only |
| **Explicit-blank / explicit-invalid marker candidates** (§13.2) | per candidate | Partial | `explicit-blank-invalid` bundles both markers and the mixed case; other fixtures still have none |
| **Voting channel / participation** (§13.3) | map | Yes | New upstream; workbench tallies one electronic channel by construction |
| **Tally sheets** (§13.4) | per-sheet totals | Yes | Unit-tested in velvet-core; no bundled snapshot, no UI |
| **Tie construction** (§13.5) | n/a | Yes | No fixture produces a tie, so by-lot and pending-resolution paths are dead |
| **EVoterSigningPolicy** (§13.6) | 2 variants | Yes | Signing branch of the encrypt path never exercised |

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
  - `default.json`, `instant-runoff-3cand.json`, `mixed-3contests.json`, `two-elections.json`: two personas (Alice, Bob); `activeVoterId=null`; `assignments` absent (single-BS fixtures, no swap needed).
  - `multi-bs-shared-contest.json`: two personas (Alice North, Bob South); `workbench.assignments` populated (`{alice: [bsNorth], bob: [bsSouth]}`); `workbench.ballotStylePool` populated with both BSes; clicking a voter swaps `state.ballotStyles[electionId]` to the assigned BS.
  - Velvet reference blobs (`sample-election-config.json`, `velvet-*.json`): no workbench-extra state — these are not bundled snapshots, just election-config templates for paste-into-form use.
- **Velvet upstream variants**: Velvet fixture generators do not produce workbench-extra state; pure election configs only.
- **Coverage gap assessment**: `activeVoterId` swap with `assignments` is now exercised in `multi-bs-shared-contest`. Still missing: 3+ voters, voter with >1 assigned BS, cast-vote attribution across multiple voters.

---

### 1b. Elections per Snapshot

- **Field / type**: `state.elections: Record<electionId, Election>` (portal slice) + `state.electionEvent[eventId].elections: string[]` ([`packages/sequent-core/src/ballot.rs`](packages/sequent-core/src/ballot.rs)). One event can host N elections; one snapshot can persist multiple events.
- **Value space**: 1..N elections per snapshot; bundled hydrator iterates `Object.values(state.elections)` and dispatches each.
- **Branching sites**:
  - [`packages/workbench/app/src/persistence.ts`](packages/workbench/app/src/persistence.ts) — `hydrateFromSnapshot` loops elections; one `setBallotSelection` per contest across all elections.
  - Inspector tree: [`packages/workbench/app/src/WorkbenchInspector.tsx`](packages/workbench/app/src/WorkbenchInspector.tsx) renders one subtree per election under the event.
  - Booth route `/event/<id>/election/<id>/vote` resolves a single election per render.
- **Current fixture coverage**:
  - `two-elections.json` (bundled): one event hosting two independent elections (City council + School board), each with its own BS and its own initial `ballotSelections` entry. Each booth is addressable by its own `/election/<id>/vote` URL.
  - All other fixtures: exactly one election.
- **Velvet upstream variants**: Velvet generators produce one election per config blob.
- **Coverage gap assessment**: N=2 now bundled. N≥3, multiple **events** in one snapshot, and cross-election workbench overlay (e.g. one voter holding assignments across two elections) untested.

---

### 2. Ballot-Style Count per Election

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L2405`](packages/sequent-core/src/ballot.rs#L2405) (`Election.contests: Vec<Contest>`) paired with [`packages/sequent-core/src/ballot.rs:L802`](packages/sequent-core/src/ballot.rs#L802) (`Election.id` → ballot_styles array in ElectionConfig)
- **Value space**: 1 to N ballot styles per election; each ballot style has one `election_id` reference.
- **Branching sites**:
  - [`packages/voting-portal/src/store/ballotStyles/ballotStylesSlice.ts`](packages/voting-portal/src/store/ballotStyles/ballotStylesSlice.ts) — Redux slice holds one ballot style per election at a time; booth shows one.
  - [`packages/workbench/app/src/workbenchStore.ts:L32`](packages/workbench/app/src/workbenchStore.ts#L32) — `ballotStylePool` indexed by election_id, holds all ballot styles for that election.
  - Tally aggregation (area-vs-contest operations) loops over all ballot styles per election.
- **Current fixture coverage**: 
  - Bundled snapshots (`snapshots/*.json`):
    - `default.json`, `instant-runoff-3cand.json`, `mixed-3contests.json`, `two-elections.json` (per-election): one ballot style each.
    - `multi-bs-shared-contest.json`: **two ballot styles** (North, South) on the same election, with one shared contest + one per-area contest each.
  - Velvet reference blobs (`velvet/*.json`, not bundled — paste-into-form only):
    - `sample-election-config.json`: two ballot styles (different areas, different contests), one election.
    - `velvet-plurality-5cand.json`: one ballot style, one election.
    - `velvet-approval.json`: one ballot style, one election.
    - `velvet-multi-bs.json`: **two ballot styles**, two areas, **same contest id** shared between them (disjoint candidate pool) — most complex reference blob.
- **Velvet upstream variants**: 
  - [`packages/velvet/src/fixtures/elections.rs:L48`](packages/velvet/src/fixtures/elections.rs#L48) (`get_election_config_1`) — one ballot style.
  - [`packages/velvet/src/fixtures/elections.rs:L60`](packages/velvet/src/fixtures/elections.rs#L60) (`get_election_config_2`) — **two ballot styles**, different areas, same contest.
  - [`packages/velvet/src/fixtures/elections.rs:L100`](packages/velvet/src/fixtures/elections.rs#L100) (`get_election_config_3`) — one ballot style, hierarchical areas (parent_id set).
- **Coverage gap assessment**: Three or more ballot styles per election still untested. Cross-ballot-style contest aggregation now bundled (`multi-bs-shared-contest`) but only at N=2.

---

### 4. Contest-Sharing Across Ballot Styles (Equivalence Classes)

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1482`](packages/sequent-core/src/ballot.rs#L1482) (`Contest.id`) referenced by multiple ballot styles' `contests` arrays.
- **Value space**: Three equivalence classes per election:
  1. **Disjoint**: Each ballot style has unique contests (no id overlap).
  2. **Fully shared**: All ballot styles carry the same contest(s).
  3. **Partial**: Some ballot styles share a contest, others differ.
- **Branching sites**:
  - [`packages/voting-portal/src/store/ballotSelections/ballotSelectionsSlice.ts`](packages/voting-portal/src/store/ballotSelections/ballotSelectionsSlice.ts) — Redux slice maintains one `ballotSelections` state across all contests visible to active ballot style; booth UI renders contests from active style.
  - Tally: [`packages/velvet/src/pipes/do_tally/do_tally.rs`](packages/velvet/src/pipes/do_tally/do_tally.rs) — aggregates results per contest across all areas/ballot styles carrying that contest.
  - Area-contest matching (workbench): determines which contests are visible per area during tally.
- **Current fixture coverage**: 
  - Bundled single-BS fixtures (`default.json`, `instant-runoff-3cand.json`, `mixed-3contests.json`, `two-elections.json`): one ballot style each → trivially "fully shared" within the style.
  - `multi-bs-shared-contest.json` (bundled): two ballot styles on one election with a **partial** sharing pattern — the `Federal president` contest (`…00c1`, identical candidates Aldo/Beatriz/Cyrus) appears in both BSes, while `North district representative` (`…00c2`) and `South district representative` (`…00c3`) each appear in exactly one BS. This is the first bundled fixture exercising contest-id sharing across BSes.
  - Reference blobs (not bundled):
    - `sample-election-config.json`: two ballot styles, **disjoint** contests (colour vs shape).
    - `velvet-multi-bs.json`: two ballot styles, **fully shared** contest id ("44444444-4444-4444-4444-4444444400c1") but **disjoint** candidate pools (Area A vs B have different candidate ids even though contest id is same).
- **Velvet upstream variants**: 
  - `get_election_config_1` — disjoint (one ballot style).
  - `get_election_config_2` — disjoint (two areas, different contests per style).
  - `get_election_config_3` — disjoint (one ballot style, hierarchical areas).
- **Coverage gap assessment**: 
  - Partial sharing now bundled (`multi-bs-shared-contest`). Still missing: fully-shared identical contests across ≥2 BSes, and same-contest-id-with-disjoint-candidates (the `velvet-multi-bs` pattern) as a bundled snapshot.
  - Tally-pipeline aggregation across shared contests still untested.

---

### 5. Contests Per Ballot Style

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L2405`](packages/sequent-core/src/ballot.rs#L2405) (`BallotStyle.contests: Vec<Contest>`)
- **Value space**: 1 to N contests per ballot style; no fixed upper limit in type.
- **Branching sites**:
  - [`packages/voting-portal/src/routes/VotingScreen.tsx`](packages/voting-portal/src/routes/VotingScreen.tsx) — renders all contests from `state.ballotSelections` (indexed by contest_id). (An earlier revision of this document cited a `components/BoothLayout.tsx`; no such file exists in voting-portal — `BoothLayout` is workbench-side, in `app/src/BoothSpike.tsx`.)
  - [`packages/voting-portal/src/store/ballotSelections/ballotSelectionsSlice.ts`](packages/voting-portal/src/store/ballotSelections/ballotSelectionsSlice.ts) — initializes one selection entry per contest in ballot style.
  - Ballot encoding: [`packages/sequent-core/src/ballot_codec/multi_ballot.rs`](packages/sequent-core/src/ballot_codec/multi_ballot.rs) — encodes multiple contests' selections into fixed-size 30-byte payload.
- **Current fixture coverage**: 
  - `mixed-3contests.json` (bundled): **3 contests** in a single ballot style — Mayor (plurality, max=1), City council (plurality, max=2, winning=2), Park funding (IRV, max=3) — also exercises algorithm mixing on one ballot (see §6).
  - `multi-bs-shared-contest.json` (bundled): **2 contests per ballot style** (shared Federal president + per-area district representative).
  - `explicit-blank-invalid.json` (bundled): **2 contests** in one ballot style (§13.2).
  - All other bundled fixtures and all reference blobs: exactly 1 contest per ballot style.
- **Velvet upstream variants**: All generators produce 1 contest per ballot style.
- **Coverage gap assessment**: N=2 and N=3 now bundled. Capacity-near-limit (sum of base-bits approaching 30 bytes) still untested; multi-ballot encoding with non-plurality contests in same style not exercised at the codec level beyond `mixed-3contests`.

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
  - Bundled: `default.json` / `two-elections.json` / `multi-bs-shared-contest.json` use `PluralityAtLarge`; `instant-runoff-3cand.json` uses `InstantRunoff` (3 candidates, min=0, max=3); `mixed-3contests.json` **mixes both** on a single ballot (2 plurality contests + 1 IRV) — exercises booth dispatching `isPreferential()` per-contest within the same render.
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
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L37`](packages/sequent-core/src/ballot_codec/checker.rs#L87) — `check_max_min_votes_policy()`: validates max/min are convertible to usize; returns error if not.
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L80`](packages/sequent-core/src/ballot_codec/checker.rs#L130) — `check_min_vote_policy()`: if num_selected < min_votes, error.
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L137`](packages/sequent-core/src/ballot_codec/checker.rs#L187) — `check_over_vote_policy()`: if num_selected > max_votes, errors; alerts depend on `over_vote_policy`.
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L197`](packages/sequent-core/src/ballot_codec/checker.rs#L247) — `check_under_vote_policy()`: if num_selected < max_votes (and ≥ min_votes), alert depends on `under_vote_policy`.
  - Ballot encoding (bases): [`packages/sequent-core/src/ballot_codec/bases.rs:L23`](packages/sequent-core/src/ballot_codec/bases.rs#L23) — base computed as `max_votes + 1` for preferential; dimension of choice space.
  - UI (voting portal): [`packages/voting-portal/src/components/Question/Question.tsx`](packages/voting-portal/src/components/Question/Question.tsx) — contest rendering and validation depend on max/min for checkbox limit enforcement.

- **Current fixture coverage**: 
  - Bundled:
    - `default.json`: max=1, min=1, winning=1.
    - `instant-runoff-3cand.json`: max=3, min=0, winning=1.
    - `mixed-3contests.json`: per-contest — (max=1, min=1, winning=1), (max=2, min=1, winning=2), (max=3, min=0, winning=1) — first bundled fixture with `winning_candidates_num > 1`.
    - `multi-bs-shared-contest.json`: all contests (max=1, min=1, winning=1).
    - `two-elections.json`: both contests (max=1, min=1, winning=1).
  - Reference blobs (not bundled):
    - `sample-election-config.json`: max_votes=1, min_votes=1, winning_candidates_num=1.
    - `velvet-plurality-5cand.json`: max_votes=1, min_votes=0, winning_candidates_num=1 (under-vote allowed).
    - `velvet-approval.json`: min_votes=1, max_votes=3, winning_candidates_num=1 (only blob with min>0 and max>1).
    - `velvet-multi-bs.json`: max_votes=1, min_votes=0 (per-area contest, disjoint candidate ids).

- **Velvet upstream variants**: 
  - `get_contest_1()` — max=1, min=0.
  - `get_contest_min_max_votes(min, max)` — parameterizable; used to generate velvet-approval (min=1, max=3).

- **Coverage gap assessment**: 
  - **Range**: Bundled covers min ∈ {0,1}, max ∈ {1,2,3}, winning ∈ {1,2}. Missing: min ≥ 2, max ≥ 4, winning ≥ 3.
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
  - Bundled (all five, verified against the JSON):
    - `default.json`: 2 candidates.
    - `instant-runoff-3cand.json`: 3 candidates.
    - `mixed-3contests.json`: 3 / **4** / 3 candidates across its three contests — the
      4-candidate *City council* contest is the largest in any bundled fixture.
    - `multi-bs-shared-contest.json`: 3 (shared *Federal president*) / 2 / 2.
    - `explicit-blank-invalid.json`: 3 / 3 — each contest is 2 regular candidates plus one marker (§13.2).
    - `two-elections.json`: 2 and 2.
    - `allow_writeins` is not set anywhere (default true), and **no bundled fixture
      contains a candidate with `is_write_in`**, so the write-in code path is
      unreachable from bundled state regardless of the flag.
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

This section groups two distinct families of contest-level policy fields that both live in `ContestPresentation`:

- **10.A Vote validation policies** — policies that define what *is* (or is not) an allowed vote. They are consulted both (a) by the voting portal while the user is constructing the ballot (to surface warnings / errors / disable controls and to gate submission) and (b) by velvet's codec during the cast-and-tally pipeline (`raw_ballot::decode`, `multi_ballot::decode`, which call the per-policy checkers in [`packages/sequent-core/src/ballot_codec/checker.rs`](packages/sequent-core/src/ballot_codec/checker.rs)). The 1:1 TypeScript mirror lives in [`packages/ui-core/src/types/ContestPresentation.ts`](packages/ui-core/src/types/ContestPresentation.ts).
- **10.B Presentation / layout policies** — policies that affect rendering, ordering, or post-tally tie resolution. They never appear in `checker.rs` and `raw_ballot::decode` never branches on them; a vote is equally "allowed" or "disallowed" regardless of their value.

The split matters for fixture coverage: validation policies must be exercised through both the booth gating layer *and* the tally decode layer (and ideally with edge selections that actually trip each branch), whereas presentation policies only need rendering coverage.

#### 10.A Vote validation policies

The six policies below are the complete set of `ContestPresentation` fields that meet the criterion above. Plurality contests reach the first four; preferential contests (IRV / Borda*) reach all six.

| Policy | Checker (`ballot_codec/checker.rs`) | Booth-side gating (encode path) | Tally decode path |
|---|---|---|---|
| `InvalidVotePolicy` | [`check_invalid_vote_policy` L281](packages/sequent-core/src/ballot_codec/checker.rs#L331) | [`InvalidErrorsList.tsx`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx); [`voting_screen.rs`](packages/sequent-core/src/util/voting_screen.rs) | [`raw_ballot.rs` L343](packages/sequent-core/src/ballot_codec/raw_ballot.rs#L378); [`multi_ballot.rs` L648](packages/sequent-core/src/ballot_codec/multi_ballot.rs#L869) |
| `EOverVotePolicy` | [`check_over_vote_policy` L137](packages/sequent-core/src/ballot_codec/checker.rs#L187) | [`InvalidErrorsList.tsx`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx); [`Question.tsx`](packages/voting-portal/src/components/Question/Question.tsx) (`NOT_ALLOWED_WITH_MSG_AND_DISABLE` disables checkboxes) | [`raw_ballot.rs` L359](packages/sequent-core/src/ballot_codec/raw_ballot.rs#L407); [`multi_ballot.rs` L657](packages/sequent-core/src/ballot_codec/multi_ballot.rs#L884) |
| `EUnderVotePolicy` | [`check_under_vote_policy` L197](packages/sequent-core/src/ballot_codec/checker.rs#L247) | [`InvalidErrorsList.tsx`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx); [`voting_screen.rs`](packages/sequent-core/src/util/voting_screen.rs) | [`raw_ballot.rs` L372](packages/sequent-core/src/ballot_codec/raw_ballot.rs#L421); [`multi_ballot.rs` L670](packages/sequent-core/src/ballot_codec/multi_ballot.rs#L899) |
| `EBlankVotePolicy` | [`check_blank_vote_policy` L103](packages/sequent-core/src/ballot_codec/checker.rs#L153) | [`InvalidErrorsList.tsx`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx); [`voting_screen.rs`](packages/sequent-core/src/util/voting_screen.rs) | [`raw_ballot.rs` L381](packages/sequent-core/src/ballot_codec/raw_ballot.rs#L431); [`multi_ballot.rs` L679](packages/sequent-core/src/ballot_codec/multi_ballot.rs#L909) |
| `EDuplicatedRankPolicy` (preferential only) | [`check_duplicated_rank_policy` L235](packages/sequent-core/src/ballot_codec/checker.rs#L285) | [`voting_screen.rs`](packages/sequent-core/src/util/voting_screen.rs); default surfaced via [`getDefaultDuplicatedRankPolicy()` in ui-core/wasm.ts L425](packages/ui-core/src/services/wasm.ts#L425) | [`raw_ballot.rs` L401](packages/sequent-core/src/ballot_codec/raw_ballot.rs#L451) (preferential branch only) |
| `EPreferenceGapsPolicy` (preferential only) | [`check_preference_gaps_policy` L258](packages/sequent-core/src/ballot_codec/checker.rs#L308) | [`voting_screen.rs`](packages/sequent-core/src/util/voting_screen.rs); default surfaced via [`getDefaultPreferenceGapsPolicy()` in ui-core/wasm.ts L434](packages/ui-core/src/services/wasm.ts#L434) | [`raw_ballot.rs` L396](packages/sequent-core/src/ballot_codec/raw_ballot.rs#L446) (preferential branch only) |

Notes on the encode/decode surfaces:

- `multi_ballot::decode` invokes only the four non-preferential checkers (it rejects non-Plurality contests up-front at [`multi_ballot.rs` L125–L129](packages/sequent-core/src/ballot_codec/multi_ballot.rs#L125)); IRV / Borda* ballots therefore travel the `raw_ballot::decode` path, which is where `check_duplicated_rank_policy` and `check_preference_gaps_policy` run.
- `min_votes` / `max_votes` / `winning_candidates_num` are the numeric thresholds these six policies branch against (catalogued separately in §7); they are not themselves "policies."
- The booth's submission-gate predicate in [`voting_screen.rs::check_voting_not_allowed_next_util`](packages/sequent-core/src/util/voting_screen.rs#L14) treats `NOT_ALLOWED` / `NOT_ALLOWED_WITH_MSG_AND_ALERT` / `NOT_ALLOWED_WARN_AND_DIALOG` as hard blockers across all six policies — these are the variants where (a) and (b) can disagree (booth refuses to submit) versus the various `WARN*` and `ALLOWED*` variants (booth admits, codec decoder still annotates / errors per policy).

#### 10.A.1 InvalidVotePolicy

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L833`](packages/sequent-core/src/ballot.rs#L833) (`pub enum InvalidVotePolicy`)
- **Value space**: 4 variants
  - `ALLOWED` (default) — explicitly invalid candidates are allowed.
  - `WARN` — warn if user selects explicit invalid.
  - `WARN_INVALID_IMPLICIT_AND_EXPLICIT` — warn on both implicit and explicit invalidity.
  - `NOT_ALLOWED` — reject ballots with explicit invalid selections.
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L281`](packages/sequent-core/src/ballot_codec/checker.rs#L331) — `check_invalid_vote_policy()`: if explicit invalid selected and policy != ALLOWED, add error or alert.
  - [`packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx:L75-L76`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx#L75) — retrieves policy; displays warnings/errors.
- **Current fixture coverage**: `invalid_vote_policy: ALLOWED` set in `velvet-plurality-5cand.json` reference blob; no bundled fixture sets it (defaults to ALLOWED).
- **Velvet upstream variants**: `get_contest_1()` sets ALLOWED.
- **Coverage gap assessment**: WARN, WARN_INVALID_IMPLICIT_AND_EXPLICIT, NOT_ALLOWED never tested.
- **Precondition the policy depends on**: the *explicit* branch of this policy only
  fires when the voter selects a candidate carrying
  `presentation.is_explicit_invalid`. Setting `invalid_vote_policy` on a fixture
  is therefore *not sufficient* to exercise it — the fixture also needs a marker
  candidate. Only `explicit-blank-invalid.json` has one (§13.2); for every other
  bundled snapshot the explicit branch remains unreachable and only the implicit
  branch (validation errors from under/over-vote) can fire.

#### 10.A.2 UnderVotePolicy

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1128`](packages/sequent-core/src/ballot.rs#L1128) (`pub enum EUnderVotePolicy`)
- **Value space**: 4 variants
  - `ALLOWED` (default) — under-voting (selecting fewer than max_votes) is fine.
  - `WARN` — warn if num_selected < max_votes (but ≥ min_votes).
  - `WARN_ONLY_IN_REVIEW` — warn only on review screen, not during voting.
  - `WARN_AND_ALERT` — warn + popup alert.
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L197`](packages/sequent-core/src/ballot_codec/checker.rs#L247) — `check_under_vote_policy()`: checks if num_selected < max_votes and policy != ALLOWED, adds alert.
  - Voting-portal: [`packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx:L71-L72`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx#L71) — reads policy; display logic.
- **Current fixture coverage**: `under_vote_policy: ALLOWED` set in `velvet-plurality-5cand.json` reference blob; no bundled fixture sets it (defaults ALLOWED).
- **Velvet upstream variants**: `get_contest_1()` sets ALLOWED.
- **Coverage gap assessment**: WARN, WARN_ONLY_IN_REVIEW, WARN_AND_ALERT never tested.

#### 10.A.3 OverVotePolicy

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1192`](packages/sequent-core/src/ballot.rs#L1192) (`pub enum EOverVotePolicy`)
- **Value space**: 5 variants
  - `ALLOWED` — over-voting is silently allowed (extra votes ignored).
  - `ALLOWED_WITH_MSG` — allow with message.
  - `ALLOWED_WITH_MSG_AND_ALERT` (default) — allow with popup alert.
  - `NOT_ALLOWED_WITH_MSG_AND_ALERT` — reject with message + alert.
  - `NOT_ALLOWED_WITH_MSG_AND_DISABLE` — disable checkboxes when max reached (strict UX).
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L137`](packages/sequent-core/src/ballot_codec/checker.rs#L187) — `check_over_vote_policy()`: if num_selected > max_votes, error added; alerts vary by policy.
  - Voting-portal: [`packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx:L77-L78`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx#L77) — reads policy; checkbox disabling logic in Answer component.
- **Current fixture coverage**: `over_vote_policy: ALLOWED_WITH_MSG_AND_ALERT` set in `velvet-plurality-5cand.json` reference blob; no bundled fixture sets it (defaults ALLOWED_WITH_MSG_AND_ALERT).
- **Velvet upstream variants**: `get_contest_1()` sets ALLOWED_WITH_MSG_AND_ALERT.
- **Coverage gap assessment**: ALLOWED, ALLOWED_WITH_MSG, NOT_ALLOWED_WITH_MSG_AND_ALERT, NOT_ALLOWED_WITH_MSG_AND_DISABLE rarely tested; strict checkbox-disable UX (NOT_ALLOWED_WITH_MSG_AND_DISABLE) never exercised.

#### 10.A.4 BlankVotePolicy

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1160`](packages/sequent-core/src/ballot.rs#L1160) (`pub enum EBlankVotePolicy`)
- **Value space**: 4 variants
  - `ALLOWED` (default) — blank ballots OK.
  - `WARN` — warn if no selections made.
  - `WARN_ONLY_IN_REVIEW` — warn on review only.
  - `NOT_ALLOWED` — reject blank ballots (enforce min_votes ≥ 1).
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L103`](packages/sequent-core/src/ballot_codec/checker.rs#L153) — `check_blank_vote_policy()`: if num_selected == 0 and policy != ALLOWED, add alert or error.
  - Voting-portal: [`packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx:L73-L74`](packages/voting-portal/src/components/InvalidErrorsList/InvalidErrorsList.tsx#L73) — display logic.
- **Current fixture coverage**: `blank_vote_policy` absent in all fixtures (defaults ALLOWED).
- **Velvet upstream variants**: Not set in generators.
- **Coverage gap assessment**: WARN, WARN_ONLY_IN_REVIEW, NOT_ALLOWED never tested.
- **Explicit vs implicit blank is now a first-class distinction** (upstream #2842).
  A ballot is an *explicit* blank when the voter selects a candidate carrying
  `presentation.is_explicit_blank`, and an *implicit* blank when nothing is
  selected; selecting an explicit-blank marker *together with* a regular
  candidate is an implicit **invalid**. velvet-core reports the split as
  `blank_votes.explicit` / `blank_votes.implicit` and the workbench surfaces both
  rows. Only `explicit-blank-invalid.json` defines an explicit-blank candidate
  (§13.2); on every other bundled snapshot `blank_votes.explicit` is
  structurally `0`, so the UI row exists but can never be non-zero.

#### 10.A.5 DuplicatedRankPolicy (Preferential Voting)

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1243`](packages/sequent-core/src/ballot.rs#L1243) (`pub enum EDuplicatedRankPolicy`)
- **Value space**: 2 variants
  - `ALLOWED_WARN_AND_DIALOG` (default) — allow duplicate ranks but warn with dialog.
  - `NOT_ALLOWED_WARN_AND_DIALOG` — reject duplicates with warning dialog.
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L235`](packages/sequent-core/src/ballot_codec/checker.rs#L285) — `check_duplicated_rank_policy()`: validates ranked votes; error/alert if duplicates and policy rejects.
  - Applies only to preferential (InstantRunoff, Borda*) contests.
- **Current fixture coverage**: `duplicated_rank_policy` not set anywhere. The new `instant-runoff-3cand.json` bundled fixture is the first preferential contest — leaves this at default.
- **Velvet upstream variants**: Not set; no IRV fixtures to test.
- **Coverage gap assessment**: Both values untested; dependence on preferential-only semantics untested.

#### 10.A.6 PreferenceGapsPolicy (Preferential Voting)

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1267`](packages/sequent-core/src/ballot.rs#L1267) (`pub enum EPreferenceGapsPolicy`)
- **Value space**: 2 variants
  - `ALLOWED_WARN_AND_DIALOG` (default) — gaps in rankings allowed (e.g., rank 1, rank 3, skip rank 2).
  - `NOT_ALLOWED_WARN_AND_DIALOG` — enforce contiguous ranking.
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/checker.rs:L258`](packages/sequent-core/src/ballot_codec/checker.rs#L308) — `check_preference_gaps_policy()`: validates no gaps if policy requires contiguous ranks.
  - Preferential-only.
- **Current fixture coverage**: Not set anywhere. The new `instant-runoff-3cand.json` bundled fixture is the first preferential contest — leaves this at default.
- **Velvet upstream variants**: Not set.
- **Coverage gap assessment**: Both variants untested; gap validation untested.

#### 10.B Presentation / layout policies

The remaining `ContestPresentation` (and per-contest) fields below influence rendering, candidate ordering, list layout, or post-tally tie resolution. None of them are read by [`packages/sequent-core/src/ballot_codec/checker.rs`](packages/sequent-core/src/ballot_codec/checker.rs), and `raw_ballot::decode` / `multi_ballot::decode` never branch on them — flipping their values cannot change whether a given selection counts as a valid vote. Fixture coverage for these is a rendering-test concern, not a validation-correctness concern.

#### 10.B.1 CandidatesOrder

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

#### 10.B.2 CandidatesSelectionPolicy

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

#### 10.B.3 CandidatesIconCheckboxPolicy

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

#### 10.B.4 EnableCheckableLists, CollapsibleLists, ShuffleCategories, Columns, Pagination, Show/CumulativeCheckboxes

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

#### 10.B.5 TieBreakingPolicy (on Contest, not Presentation)

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L1482`](packages/sequent-core/src/ballot.rs#L1482) (`Contest.tie_breaking_policy: Option<TieBreakingPolicy>`)
- **Value space**: 2 variants
  - `Random` (default) — break ties by random draw.
  - `ExternalProcedure` — defer to external (human) tie-breaking.
- **Branching sites**:
  - Tally: [`packages/velvet/src/pipes/do_tally/tally.rs`](packages/velvet/src/pipes/do_tally/tally.rs) — when instantaneous runoff reaches a tie, invokes tie-breaking based on policy. (Implementation not fully visible; depends on velvet-core.)
  - Workbench: [`packages/workbench/app/src/TallyPage.tsx`](packages/workbench/app/src/TallyPage.tsx) — displays pending tie-break resolutions if policy is ExternalProcedure.
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

### 12. Multi-Ballot Encoding Capacity & Limits

- **Field / type**: [`packages/sequent-core/src/ballot_codec/multi_ballot.rs:L202`](packages/sequent-core/src/ballot_codec/multi_ballot.rs#L202) — fixed 30-byte encoding size.
- **Value space**: Single encoded ballot must fit in 30 bytes; capacity depends on contest count and max_votes per contest.
  - Capacity formula (approximate): sum(log2(candidate_count) * max_votes for all contests) < 30*8 bits.
  - Multi-ballot codec only supports Plurality-at-large; comment at line 742 enforces this.
- **Branching sites**:
  - [`packages/sequent-core/src/ballot_codec/multi_ballot.rs:L719-L742`](packages/sequent-core/src/ballot_codec/multi_ballot.rs#L719) — `get_bases()` validates no non-PluralityAtLarge contests; errors if mixed.
  - Encoding: [`packages/sequent-core/src/ballot_codec/multi_ballot.rs:L240-L250`](packages/sequent-core/src/ballot_codec/multi_ballot.rs#L240) — comment documents capacity constraint.
- **Current fixture coverage**: 
  - `mixed-3contests.json` (bundled): 3 contests per ballot style — exercises multi-contest encoding path; capacity not stressed (3 contests with ≤3 candidates each fits comfortably in 30 bytes).
  - `multi-bs-shared-contest.json` (bundled): 2 contests per ballot style.
  - All other fixtures: 1 contest per ballot style.
- **Velvet upstream variants**: Single-contest only.
- **Coverage gap assessment**: Multi-contest encoding exercised at N∈{2,3} but never near the 30-byte limit; capacity-overflow behaviour and mixed-algorithm encoding edges (e.g. plurality + IRV in `get_bases()`) under-exercised.

---

### 13. Dimensions introduced by the upstream merge (2026-08)

Five feature PRs landed between the branch point and `origin/main@0db8f855ec` —
explicit blank votes (#2842), election-level decline to vote (#2687), consistent
invalid vote policy (#2697), tally sheets input (#1929) and participation by
voting channel (#2920). They add the dimensions below. **None has bundled-fixture
coverage**, and several are not merely untested but *unreachable* from current
fixtures, which is a stronger statement.

#### 13.1 DeclineToVotePolicy (election-event presentation)

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L2863`](packages/sequent-core/src/ballot.rs#L2863) (`pub enum DeclineToVotePolicy`), carried on `ElectionEventPresentation.decline_to_vote_policy`.
- **Value space**: 2 variants — `DISABLED` (default), `ENABLED` (voter may decline at the **election** level, i.e. all contests at once).
- **Branching sites**:
  - Portal state: `declinedToVote` map plus `setDeclinedToVote` / `clearDeclinedToVoteForElection` / `isDeclineToVoteByElectionId` in `store/extra/extraSlice.ts`; `setAllBallotSelectionsDeclineToVote` in `store/ballotSelections/ballotSelectionsSlice.ts`.
  - Booth flow: `ReviewScreen` reads `isDeclineToVote` and re-points the Back button at `/start` instead of `/vote`.
  - Per-contest carrier: `IDecodedVoteContest.is_decline_to_vote` — a **required** field; sequent-core refuses to deserialise tally input without it.
  - Tally: declined ballots accumulate into `extended_metrics.total_declined_to_vote` and are **excluded from the valid total**, whereas blank ballots stay valid.
- **Note**: this policy does **not** appear in `checker.rs`. It is not a vote-validity policy in the §10.A sense; it changes booth flow and tally accounting.
- **Current fixture coverage**: no fixture sets the policy. All five bundled snapshots now carry `is_decline_to_vote: false` on every persisted selection (backfilled when the field became required), so they are schema-current but exercise only the not-declined path.
- **Coverage gap assessment**: `ENABLED` never set; no fixture produces a declined ballot; `total_declined_to_vote` is always 0; the declined-vs-blank distinction in the tally is untested.

#### 13.2 Explicit-blank and explicit-invalid marker candidates

- **Field / type**: [`packages/sequent-core/src/ballot.rs:L457`](packages/sequent-core/src/ballot.rs#L457) (`CandidatePresentation.is_explicit_blank`) and its sibling `is_explicit_invalid`.
- **Value space**: per candidate, { true, false / absent }.
- **Why this is its own dimension**: these markers are the *precondition* for the explicit branches of `InvalidVotePolicy` (§10.A.1) and `EBlankVotePolicy` (§10.A.4), and for the explicit/implicit split in the tally result. Cataloguing only the policies hides the fact that setting them changes nothing unless a marker candidate exists.
- **Branching sites**: `classify_ballot` and `get_explicit_blank_candidate_ids` in `workbench/velvet-core/src/counting/extended_metrics.rs`; `Candidate::is_explicit_blank()` / `is_explicit_invalid()`; the exclusivity rule in `setBallotSelectionVoteChoice` (choosing a regular candidate clears explicit-blank markers).
- **Current fixture coverage**: `explicit-blank-invalid.json` — the first and
  only bundled snapshot defining marker candidates. Two contests:
  - *Referendum* — Yes / No / **Blank vote** (`is_explicit_blank`),
    `max_votes: 2`. The 2 is deliberate: at `max_votes: 1` a marker-plus-candidate
    selection would also trip the over-vote checker, and the fixture could not
    separate "invalid because mixed with an explicit blank" from "invalid because
    too many selections".
  - *Council seat* — Ada / Bruno / **Null vote (invalid)** (`is_explicit_invalid`),
    `max_votes: 1`.

  The other five bundled snapshots and all reference blobs still define none.
- **Verified behaviour** — 5 ballots into *Referendum* (2×Yes, 1×marker only,
  1×nothing, 1×marker+Yes):

  | metric | result |
  |---|---|
  | total / valid | 5 / **4** — blank ballots count as valid |
  | `blank_votes` | 2 = **1 explicit** + 1 implicit |
  | `invalid_votes` | 1 = 0 explicit + **1 implicit** (the mixed ballot) |

  Selecting *Null vote (invalid)* in *Council seat* yields `invalid_votes.explicit = 1`.
- **Coverage gap assessment**: the classification path is now reachable, and the
  four `ParticipationSummary` rows that were structurally 0 render real values.
  Remaining: no fixture pairs a marker with a *non-default* `invalid_vote_policy`
  or `blank_vote_policy`, so the explicit branches are only exercised under
  `ALLOWED`; and no preferential contest carries a marker.

#### 13.3 Voting channel and participation

- **Field / type**: `sequent_core::types::participation::{ParticipationChannel, VotesByChannel}`; `ExtendedMetricsContest.votes_by_channel`.
- **Value space**: map from channel (e.g. paper vs electronic) to vote count, aggregated up an area hierarchy.
- **Branching sites**: `set_votes_by_channel` / `merge_votes_by_channel` / `validate_complete_votes_by_channel` in velvet's `do_tally.rs`; `process_tally_sheet` attaches the sheet's channel.
- **Current fixture coverage**: none — no bundled snapshot carries `votes_by_channel`, and the workbench has no UI for entering one.
- **Coverage gap assessment**: untested here. The workbench tallies a single electronic channel by construction, so this may be better exercised by velvet's own tests than by a bundled snapshot — worth a deliberate decision rather than leaving it as an open gap.

#### 13.4 Tally sheets

- **Field / type**: `sequent_core::types::hasura::core::TallySheet`, `tally_sheets::{TallySheetStatus, VotingChannel}`; consumed by `process_tally_sheet` in `workbench/velvet-core/src/counting/tally.rs`.
- **Value space**: per-sheet candidate totals, blank and invalid counts, census, channel, status.
- **Semantics worth pinning**: a sheet's blank total is recorded as **implicit**; `count_valid` falls back to candidate votes + blanks when the sheet states no valid total; and the sheet's result is folded into the contest tally by `Tally::tally()` (an omission of that fold silently dropped paper results from IRV contests until it was caught by an upstream test).
- **Current fixture coverage**: none as a bundled snapshot. velvet-core carries upstream's `process_tally_sheet_counts_blank_votes_as_valid` and `contest_tally_includes_tally_sheet_results` unit tests.
- **Coverage gap assessment**: no end-to-end coverage through the workbench; the tally sandbox has no pane for a tally sheet. Same judgement call as §13.3.

#### 13.5 Tie-breaking, revisited

§10.B.5 describes `TieBreakingPolicy` as a 2-variant field whose implementation
was "not fully visible". It now has real machinery: `RunoffStatus` carries
`tie_breaking_policy`, `tie_resolutions: Vec<TallySessionResolutionData>` and
`pending_tie_resolution`, and `determine_winner_by_lot` takes an explicit RNG.

The §10.B categorisation still holds — it does not affect *vote validity* — but
it is no longer purely presentational: it changes tally output and can leave a
contest awaiting external resolution.

- **Current fixture coverage**: absent everywhere; no bundled fixture constructs a tie.
- **Coverage gap assessment**: `ExternalProcedure` and the pending-resolution surface untested. A deliberate tie is trivial to construct (two candidates, one vote each) and would exercise both the by-lot RNG path and the workbench's rendering of a pending resolution.

#### 13.6 EVoterSigningPolicy (election-event presentation)

- **Field / type**: [`packages/ui-core/src/types/ElectionEventPresentation.ts:L18`](packages/ui-core/src/types/ElectionEventPresentation.ts#L18) — `NO_SIGNATURE` (default) / `WITH_SIGNATURE`.
- **Branching sites**: `useEncryptBallotForReview` signs the hashable ballot when the policy is `WITH_SIGNATURE`, changing what `cv.content` holds.
- **Current fixture coverage**: absent; every fixture takes the unsigned path.
- **Coverage gap assessment**: the signing branch of the encrypt path is never exercised, and the workbench's decrypt bridge has never seen a signed ballot envelope.

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
| Explicit-blank / explicit-invalid markers | Closed by `explicit-blank-invalid`. Remaining: no fixture pairs a marker with a non-default `invalid_vote_policy` / `blank_vote_policy` | Explicit policy branches still take the default variant | Medium |
| CountingAlgType — 9/10 unsupported | Only Plurality-at-Large + IRV bundled (incl. mixed on one ballot via `mixed-3contests`) | Tally dispatch, ballot encoding, validation | Critical |
| Decline to vote (§13.1) | Policy never enabled; no declined ballot produced | Declined-vs-blank tally accounting, booth back-navigation | High |
| Contest-sharing (disjoint/shared/partial) | Partial bundled (`multi-bs-shared-contest`); fully-shared-identical and disjoint-candidates-same-id still bundled-only as reference blobs | Multi-ballot-style aggregation incomplete | High |
| Preference policies (Dup/Gap/etc.) | Untested; depends on IRV; `instant-runoff-3cand` leaves both at default | Validation of preferential votes incomplete | High |
| Vote constraint policies (under/over/blank/invalid) | Only ALLOWED tested | Validation policy branching incomplete | High |
| Multi-contest ballot styles | N=3 now bundled (`mixed-3contests`); near-30-byte capacity still untested | Encoding capacity untested at limit | Medium |
| Multiple elections / events per snapshot | N=2 elections now bundled (`two-elections`); multi-event and N≥3 untested | Hydrator + workbench overlay coverage incomplete | Medium |
| Write-in encoding (allow_writeins=true, text submission) | Never exercised end-to-end | Write-in text encoding/decoding untested | Medium |
| UI policies (shuffle, columns, checkable-lists, etc.) | Not in fixtures | UI path testing requires browser/visual tests | Low |

---

## Recommendations for Extended Fixture Coverage

1. **Priority 1: CountingAlgType variants**
   - ✅ `instant-runoff-3cand.json` bundled: min_votes=0, max_votes=3, 3 candidates (minimal IRV) — first preferential bundled fixture.
   - ✅ `mixed-3contests.json` bundled: plurality + IRV on the same ballot — exercises per-contest algorithm dispatch in the booth.
   - Borda/Desborda/Cumulative/Pairwise fixtures are blocked: velvet's `create_tally()` only dispatches PluralityAtLarge and InstantRunoff and errors on the rest (see [packages/velvet/src/pipes/do_tally/tally.rs](packages/velvet/src/pipes/do_tally/tally.rs#L109-L115)). Defer until velvet tally support lands.

2. **Priority 2: Vote validation policies** (see §10.A for the canonical set and surface map)
   - Scope is the six policies that branch in [`packages/sequent-core/src/ballot_codec/checker.rs`](packages/sequent-core/src/ballot_codec/checker.rs) and are consulted by both the booth gating layer (encode path) and `raw_ballot::decode` / `multi_ballot::decode` (tally decode path): `InvalidVotePolicy`, `EOverVotePolicy`, `EUnderVotePolicy`, `EBlankVotePolicy`, plus the preferential-only `EDuplicatedRankPolicy` and `EPreferenceGapsPolicy`.
   - Plurality fixture: exercise non-default variants of the first four in a small matrix, with selections crafted to trip each checker branch (under, over, blank, and an explicit-invalid candidate).
   - Preferential fixture: extend the IRV bundle to non-default `duplicated_rank_policy` and `preference_gaps_policy`, with ranked selections that actually contain a duplicate and a gap.
   - For each policy include at least one `NOT_ALLOWED*` variant so the booth's hard-block path in [`voting_screen.rs::check_voting_not_allowed_next_util`](packages/sequent-core/src/util/voting_screen.rs#L14) is reachable.
   - Excluded from this priority (catalogued under §10.B): `CandidatesOrder`, `CandidatesSelectionPolicy`, `CandidatesIconCheckboxPolicy`, the list/layout/pagination fields in §10.B.4, and `TieBreakingPolicy` — none of them affect vote validity.

3. **Priority 3: Multi-ballot-style scenarios**
   - ✅ `multi-bs-shared-contest.json` bundled: 2 BSes with one shared contest + per-area contests; exercises `workbench.assignments` + per-voter BS swap.
   - Still pending: 3+ ballot styles per election; tally aggregation across shared contests; fully-shared-identical-candidates as a bundled fixture.

4. **Priority 4: Post-merge dimensions (§13)** — highest value first
   - ✅ **A marker-candidate fixture** — `explicit-blank-invalid.json` bundled.
     One plurality contest with an `is_explicit_blank` candidate and one with an
     `is_explicit_invalid` candidate, including the mixed-selection case
     (explicit blank *plus* a regular candidate) which classifies as implicit
     **invalid**. Verified end-to-end; see §13.2 for the numbers.
     Still open: pair a marker with a non-default `invalid_vote_policy` /
     `blank_vote_policy` so the explicit branches run under something other than
     `ALLOWED`, and add a marker to a preferential contest.
   - **A decline-to-vote fixture**: `decline_to_vote_policy: ENABLED` plus a
     snapshot whose selections carry `is_decline_to_vote: true`, to exercise
     `total_declined_to_vote` and the exclusion of declined ballots from the
     valid total.
   - **A deliberate tie** (two candidates, one vote each) to reach the by-lot
     path and any pending-resolution rendering.
   - Voting channel (§13.3) and tally sheets (§13.4) are arguably velvet-core
     unit-test territory rather than workbench snapshots — decide explicitly
     rather than carrying them as open gaps.

5. **Priority 5: Multiple elections / events per snapshot**
   - ✅ `two-elections.json` bundled: two independent elections (City council + School board) under one event.
   - Still pending: multiple events per snapshot; N≥3 elections; cross-election workbench overlay (one voter, assignments in two elections).


---

End of variance catalogue.
