<!--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# External Hat-Procedure Tie-Breaking for IRV

## Overview

This feature adds support for externally-resolved ties in Instant Runoff Voting (IRV) tallies. When a tie occurs that cannot be resolved by the lookback rule, administrators can manually select the winner using a documented procedure (hat procedure, coin flip, etc.) instead of automatic random selection.

## Table of Contents

- [Use Case](#use-case)
- [Configuration](#configuration)
- [Data Flow](#data-flow)
- [API Usage](#api-usage)
- [CLI Usage](#cli-usage)
- [Testing](#testing)
- [Architecture](#architecture)

---

## Use Case

**Problem:** Some jurisdictions require manual, documented tie-breaking procedures rather than automatic random selection. The existing system always resolves ties randomly.

**Solution:** Contest-level tie-breaking policy that allows tallies to pause when ties occur, enabling administrators to manually select the winner and document the method used.

**Example Scenario:**
1. Three candidates (A, B, C) each receive exactly 100 votes
2. Lookback rule cannot resolve the tie
3. Instead of random selection, tally pauses with status `AWAITING_INPUT`
4. Election officials use hat procedure to select winner
5. Admin submits decision via API/CLI: "Candidate B selected via hat procedure"
6. Tally resumes and completes with Candidate B as winner
7. Decision is permanently recorded in the resolution table with timestamp and user

---

## Configuration

### Contest-Level Policy

Add the `tie_breaking_policy` field to your contest configuration:

```json
{
  "id": "contest-123",
  "name": "Mayor Election",
  "counting_algorithm": "instant-runoff",
  "tie_breaking_policy": "external-procedure",
  ...
}
```

**Available Policies:**

| Policy | Behavior | Use When |
|--------|----------|----------|
| `random` (default) | Automatic random selection | Standard elections, no special tie requirements |
| `external-procedure` | Pause for manual resolution | Jurisdictions requiring documented tie-breaking |

### Backward Compatibility

- Contests without `tie_breaking_policy` default to `random`
- Existing tallies continue to work unchanged
- No database migration required

### Tenant Permission Migration

- Existing tenant role/group configurations created before this feature will not automatically include the new `tally-resolution-submit` permission.
- In upgraded environments, update the affected admin groups manually in Keycloak or re-import the Roles & Permissions configuration so the users who handle tally tie-breaks receive `tally-resolution-submit`.
- Admin Portal visibility for pending resolutions depends on being able to read `tally_session_resolution`, which is granted through `tally-resolution-submit` or `admin-user`.

---

## Data Flow

### 1. Tie Detection

When the IRV algorithm detects an unbreakable tie with the `external-procedure` policy, it records the tie information in `RunoffStatus.pending_tie_resolution` and stops:

```rust
// Velvet: instant_runoff.rs
let mut runoff = RunoffStatus::initialize_runoff(&contest);
runoff.run(&mut ballots_status);

if let Some(tie_info) = &runoff.pending_tie_resolution {
    // Pause needed — tie_info describes the round and tied candidates
}
```

**`TallySessionResolutionData` (pending tie info):**
```json
{
  "round_number": 3,
  "tied_candidate_ids": ["candidate-a", "candidate-b"],
  "vote_count": 150,
  "method_used": "ExternalProcedure",
  "resolved_by_candidate_id": null
}
```

### 2. Pause

Windmill saves results and updates status. The pending tie info is stored in `results_contest.annotations.process_results.pending_tie_resolution`:

```json
{
  "process_results": {
    "pending_tie_resolution": {
      "round_number": 3,
      "tied_candidate_ids": ["candidate-a-uuid", "candidate-b-uuid"],
      "vote_count": 150,
      "method_used": "ExternalProcedure",
      "resolved_by_candidate_id": null
    }
  }
}
```

A `tally_session_resolution` row is created per tied contest with `status = pending`. The tally session `execution_status` is set to `AWAITING_INPUT`.

### 3. Admin Resolution

Administrator submits tally resolution(s). Multiple contests can be resolved in a single call.

**Via API:**
```bash
POST /submit-tally-resolution
Content-Type: application/json
Authorization: Bearer <token>

{
  "election_event_id": "event-123",
  "tally_session_id": "tally-456",
  "resolutions": [
    {
      "contest_id": "contest-uuid",
      "selected_candidate_id": "candidate-a-uuid"
    }
  ]
}
```

**Via CLI:**
```bash
step-cli submit-tally-resolution \
  --election-event-id event-123 \
  --tally-id tally-456 \
  --resolution contest-uuid:candidate-a-uuid
```

### 4. Resume

System processes the resolution:

1. Validates that the selected candidate is in the tied candidates list
2. Updates the `tally_session_resolution` row to `status = resolved`, recording the chosen candidate, resolver, and timestamp
3. Changes tally session status from `AWAITING_INPUT` to `IN_PROGRESS`
4. Windmill re-runs the tally; pre-loaded resolutions in `RunoffStatus.tie_resolutions` are consumed by `determine_winner_by_external_procedure`, which eliminates all candidates except the chosen winner
5. IRV algorithm continues to completion

---

## API Usage

### Endpoint: `POST /submit-tally-resolution`

**Authentication:** Bearer token with `tally-resolution-submit` permission

**Request:**
```json
{
  "election_event_id": "uuid",
  "tally_session_id": "uuid",
  "resolutions": [
    {
      "contest_id": "uuid",
      "selected_candidate_id": "uuid"
    }
  ]
}
```

Multiple contests can be resolved in one request by adding more entries to `resolutions`.

**Response (Success):**
```json
{
  "success": true,
  "tally_session_id": "uuid",
  "resolved_count": 1
}
```

**Validation Rules:**
1. ✅ Tally session must exist
2. ✅ At least one resolution must be provided
3. ✅ User must have `tally-resolution-submit` permission
4. ✅ If the tally is not in `AWAITING_INPUT`, all submitted contests must already have a resolved record (re-submission only)
5. ✅ Selected candidate must be present in the tied candidates list

**HTTP Status Codes:**
- `200` - Success
- `400` - Bad request (invalid status, missing resolutions, candidate not in tie)
- `401` - Unauthorized
- `500` - Server error

### Re-submission

If an admin changes their mind and submits a resolution for a contest that already has a resolved record, the existing `tally_session_resolution` row is updated in place (overwriting `resolution_data`, `resolved_by_user`, and `resolved_at`). The tally session status is reset to `IN_PROGRESS` to trigger re-execution with the new decision.

---

## CLI Usage

### Command: `step-cli submit-tally-resolution`

**Usage:**
```bash
step-cli submit-tally-resolution \
  --election-event-id <EVENT_ID> \
  --tally-id <TALLY_SESSION_ID> \
  --resolution <CONTEST_ID>:<CANDIDATE_ID>
```

The `--resolution` flag can be repeated to resolve multiple contests at once:
```bash
step-cli submit-tally-resolution \
  --election-event-id <EVENT_ID> \
  --tally-id <TALLY_SESSION_ID> \
  --resolution <CONTEST_ID_1>:<CANDIDATE_ID_1> \
  --resolution <CONTEST_ID_2>:<CANDIDATE_ID_2>
```

**Example:**
```bash
step-cli submit-tally-resolution \
  --election-event-id a1b2c3d4-e5f6-7890-abcd-ef1234567890 \
  --tally-id b2c3d4e5-f6a7-8901-bcde-f12345678901 \
  --resolution d4e5f6a7-b8c9-0123-def0-123456789012:c3d4e5f6-a7b8-9012-cdef-123456789012

# Output:
# Success! 1 tally resolution(s) submitted.
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `--election-event-id` | Yes | UUID of the election event |
| `--tally-id` | Yes | UUID of the tally session |
| `--resolution` | Yes (repeatable) | `CONTEST_ID:CANDIDATE_ID` pair; repeat for multiple contests |

---

## Testing

### Unit Tests

Run velvet tie-breaking tests:
```bash
cargo test -p velvet irv_tie_breaking
```

**Test Coverage:**
- ✅ `test_tie_breaking_policy_default_is_random` - Default policy is `random`
- ✅ `test_full_tie_with_random_policy_completes` - Random policy resolves tie and completes
- ✅ `test_full_tie_with_external_policy_pauses` - External policy pauses on tie
- ✅ `test_no_tie_with_external_policy_completes` - No pause when there is a clear winner
- ✅ `test_multi_round_tie_with_external_policy` - Pauses at round 2; completes with resolution
- ✅ `test_ignored_resolution_for_non_tied_candidate` - Resolution with wrong tied set is ignored
- ✅ `test_ignored_resolution_for_wrong_round` - Resolution for wrong round is ignored
- ✅ `test_tie_breaking_state_history_recorded` - Resolution history recorded for both policies

### Manual Testing

**1. Create Test Contest with Tie-Breaking Policy:**
```json
{
  "name": "Test Mayor Election",
  "counting_algorithm": "instant-runoff",
  "tie_breaking_policy": "external-procedure",
  "candidates": [
    {"id": "cand-a", "name": "Alice"},
    {"id": "cand-b", "name": "Bob"},
    {"id": "cand-c", "name": "Charlie"}
  ]
}
```

**2. Cast Tied Ballots:**
```
Ballot 1: Alice > Bob > Charlie
Ballot 2: Bob > Charlie > Alice
Ballot 3: Charlie > Alice > Bob
```

**3. Start Tally:**
```bash
step-cli start-tally --election-event-id <ID> --election-ids <ID>
```

**4. Verify Pause:**
Check tally session status:
- `execution_status` should be `AWAITING_INPUT`
- A `tally_session_resolution` row with `status = pending` should exist for the tied contest
- `results_contest.annotations.process_results.pending_tie_resolution` should contain the tie info

**5. Submit Decision:**
```bash
step-cli submit-tally-resolution \
  --election-event-id <EVENT_ID> \
  --tally-id <TALLY_ID> \
  --resolution <CONTEST_ID>:cand-a
```

**6. Verify Completion:**
- Status changes to `IN_PROGRESS` → `SUCCESS`
- Winner is "Alice"
- `tally_session_resolution` row is updated to `status = resolved` with timestamp and user

---

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────┐
│ Admin Portal UI                                         │
│ - Show tie notification                                 │
│ - Display tied candidates with names and vote counts    │
│ - Submit resolution via tally-resolution-submit role    │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────┐
│ Harvest API                                              │
│ POST /submit-tally-resolution                           │
│ - Validates status and candidate membership             │
│ - Resolves (or updates) the resolution row per contest  │
│ - Changes status to IN_PROGRESS                         │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────┐
│ Windmill (Orchestration)                                │
│ - Detects pause needed (pending_tie_resolution set)     │
│ - Stores tie info in results_contest annotations        │
│ - Creates pending tally_session_resolution rows         │
│ - Updates execution_status to AWAITING_INPUT            │
│ - On resume: loads resolved resolutions from DB,        │
│   injects into RunoffStatus.tie_resolutions             │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────┐
│ Velvet (IRV Algorithm)                                  │
│ instant_runoff.rs                                       │
│ - run() — mutates RunoffStatus in place                 │
│ - pending_tie_resolution — set when paused              │
│ - tie_resolutions — pre-loaded resolutions consumed     │
│   by determine_winner_by_external_procedure()           │
└─────────────────────────────────────────────────────────┘
```

### Key Files

| Component | File | Description |
|-----------|------|-------------|
| **Types** | `sequent-core/src/ballot.rs` | `TieBreakingPolicy` enum |
| | `sequent-core/src/types/ceremonies.rs` | `TallySessionResolutionData`, `AWAITING_INPUT` status |
| **Velvet** | `velvet/src/pipes/do_tally/counting_algorithm/instant_runoff.rs` | Core IRV logic; `RunoffStatus.pending_tie_resolution`, `tie_resolutions` |
| **Windmill** | `windmill/src/services/ceremonies/tally_resolution.rs` | Tie detection, resolution record creation, electoral log |
| | `windmill/src/postgres/tally_session_resolution.rs` | Resolution table CRUD |
| **Harvest** | `harvest/src/routes/tally_ceremony.rs` | `POST /submit-tally-resolution` endpoint |
| **CLI** | `step-cli/src/commands/submit_tally_resolution.rs` | CLI command (calls API via GraphQL) |
| **Tests** | `velvet/tests/instant_runoff/irv_tie_breaking_tests.rs` | IRV tie-breaking test suite |

### State Machine

```
                    ┌─────────────┐
                    │   STARTED   │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  CONNECTED  │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
         ┌──────────│ IN_PROGRESS │◄──────────────────┐
         │          └──────┬──────┘                   │
         │ Tie Detected    │ No Tie          Resolution submitted
         │ (External       │               (status → IN_PROGRESS,
         │  Policy)        │                tally re-runs)
         │                 │                           │
         ▼                 ▼                           │
  ┌──────────────┐  ┌─────────────┐           ┌───────┴──────┐
  │AWAITING_INPUT├──►   SUCCESS   │           │AWAITING_INPUT│
  └──────────────┘  └─────────────┘           └──────────────┘
```

---

## Troubleshooting

### Tally Won't Pause

**Symptom:** Tie is detected but tally completes with random winner

**Causes:**
1. ✅ Contest `tie_breaking_policy` not set to `external-procedure`
2. ✅ Policy is `random` (default)
3. ✅ Lookback rule resolved the tie (not a full tie)

**Solution:** Verify contest configuration:
```sql
SELECT id, name, tie_breaking_policy
FROM sequent_backend.contest
WHERE id = 'your-contest-id';
```

### Cannot Submit Decision

**Symptom:** API returns "Tally session is not awaiting input"

**Causes:**
1. ✅ Status is not `AWAITING_INPUT` (already resumed or never paused) and the contest does not yet have a resolved record
2. ✅ Multiple admins submitted simultaneously (race condition)

**Solution:** Check current status:
```sql
SELECT execution_status
FROM sequent_backend.tally_session
WHERE id = 'your-tally-id';
```

### Check Resolution History

To view the full audit trail of all resolutions submitted for a tally session:
```sql
SELECT contest_id, resolution_data, resolved_by_user, resolved_at
FROM sequent_backend.tally_session_resolution
WHERE tally_session_id = 'your-tally-id'
ORDER BY created_at DESC;
```

---

## Future Enhancements

### Admin Portal UI
- Visual notification when tally pauses
- Display tied candidates with names and vote counts
- Input field for external method used
- Resume button after decision entered
- Audit trail viewer showing all submitted resolutions

### Additional Features
- Custom tie-breaking method documentation
- Email notifications to admins when pause occurs
- Export tie-break decisions to PDF report

---

## FAQ

**Q: Can I change the policy after votes are cast?**
A: Yes, the policy is checked when the tally runs, not when votes are cast. However, changing policy mid-election is not recommended.

**Q: What happens if admin never submits a decision?**
A: The tally remains in `AWAITING_INPUT` status indefinitely. It will not time out or auto-complete.

**Q: Can I use this for non-IRV algorithms?**
A: Currently, this feature is only implemented for IRV (Instant Runoff). Other algorithms would need similar modifications.

**Q: Is the tie-break decision reversible?**
A: Yes, a resolution can be re-submitted. The existing `tally_session_resolution` row is updated in place with the new decision, and the tally status is reset to `IN_PROGRESS` to trigger re-execution.

**Q: What if two admins submit different decisions simultaneously?**
A: The last write wins — both updates succeed but only the final state is kept. The tally re-executes with whichever decision was committed last.

**Q: Can multiple tied contests be resolved in one call?**
A: Yes, the `--resolution` flag in the CLI and the `resolutions` array in the API both accept multiple entries, one per tied contest.

---

## Support

For questions or issues:
- GitHub Issues: https://github.com/sequentech/step/issues
- Documentation: https://docs.sequentech.io
