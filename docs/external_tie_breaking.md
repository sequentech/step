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

---

## Data Flow

### 1. Tie Detection

When IRV algorithm detects an unbreakable tie:

```rust
// Velvet: instant_runoff.rs
let result = runoff.run_with_policy(&mut ballots, &tie_breaking_policy);

match result {
    RunoffResult::RequiresExternalInput { state, tie_info } => {
        // Pause needed
    }
    RunoffResult::Completed(status) => {
        // Normal completion
    }
}
```

**TieBreakingState Structure:**
```json
{
  "round_number": 3,
  "tied_candidate_ids": ["candidate-a", "candidate-b"],
  "vote_counts": [150, 150],
  "method_used": "ExternalProcedure",
  "resolved_by_candidate_id": null
}
```

### 2. Pause

Windmill saves state and updates status:

```rust
// Save full RunoffStatus to annotations["paused_runoff_state"]
// Save tie info to annotations["tie_break"]
// Update execution_status to AWAITING_INPUT
// Create a tally_session_resolution row per tied contest
```

**Annotations Structure:**
```json
{
  "executer_username": "admin@example.com",
  "executer_user_id": "user-123",
  "paused_runoff_state": {
    "rounds": [...],
    "candidates_status": {...}
  },
  "tie_break": {
    "round_number": 3,
    "tied_candidates": ["cand-a-uuid", "cand-b-uuid"],
    "vote_counts": [150, 150],
    "paused_at": "2026-02-11T10:30:00Z"
  }
}
```

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
cli step submit-tally-resolution \
  --election-event-id event-123 \
  --tally-id tally-456 \
  --resolution contest-uuid:candidate-a-uuid
```

### 4. Resume

System automatically resumes tally:

1. For each resolution, creates a new row in `tally_session_resolution` (audit trail preserved)
2. Uses the latest resolution per contest (by `created_at`) to determine the decision
3. Changes status from `AWAITING_INPUT` to `IN_PROGRESS` (or `STARTED` on re-submission, to trigger re-execution)
4. Loads paused RunoffStatus from annotations
5. Applies decision: eliminates all candidates except chosen winner
6. Continues IRV algorithm to completion

---

## API Usage

### Endpoint: `POST /submit-tally-resolution`

**Authentication:** Bearer token with `ADMIN_CEREMONY` permission

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

**Response (Error):**
```json
{
  "error": "Tally session is not awaiting input. Current status: IN_PROGRESS"
}
```

**Validation Rules:**
1. ✅ Tally session must exist
2. ✅ Status must be `AWAITING_INPUT`
3. ✅ At least one resolution must be provided
4. ✅ User must have `ADMIN_CEREMONY` permission

**HTTP Status Codes:**
- `200` - Success
- `400` - Bad request (invalid status, missing resolutions)
- `401` - Unauthorized
- `404` - Tally session not found
- `500` - Server error

### Re-submission

If a resolution is submitted again for the same contest, a new row is created in `tally_session_resolution` (preserving the full audit trail) and the tally status is reset to `STARTED` to trigger re-execution with the new decision. The latest resolution (by `created_at`) is always used.

---

## CLI Usage

### Command: `cli step submit-tally-resolution`

**Installation:**
Ensure step-cli is installed and configured:
```bash
cli step config --endpoint https://api.example.com --token <your-token>
```

**Usage:**
```bash
cli step submit-tally-resolution \
  --election-event-id <EVENT_ID> \
  --tally-id <TALLY_SESSION_ID> \
  --resolution <CONTEST_ID>:<CANDIDATE_ID>
```

The `--resolution` flag can be repeated to resolve multiple contests at once:
```bash
cli step submit-tally-resolution \
  --election-event-id <EVENT_ID> \
  --tally-id <TALLY_SESSION_ID> \
  --resolution <CONTEST_ID_1>:<CANDIDATE_ID_1> \
  --resolution <CONTEST_ID_2>:<CANDIDATE_ID_2>
```

**Example:**
```bash
cli step submit-tally-resolution \
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
- ✅ `test_tie_breaking_policy_default_is_random` - Default policy validation
- ✅ `test_full_tie_with_random_policy_completes` - Random policy completes normally
- ✅ `test_full_tie_with_external_policy_pauses` - External policy pauses on tie
- ✅ `test_no_tie_with_external_policy_completes` - No pause when clear winner
- ✅ `test_apply_external_tie_decision` - Apply decision eliminates others
- ✅ `test_apply_external_tie_decision_invalid_candidate` - Validation works
- ✅ `test_resume_after_external_decision` - Resume continues correctly
- ✅ `test_backward_compatibility_with_run` - Old code still works

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
cli step start-tally --election-event-id <ID> --election-ids <ID>
```

**4. Verify Pause:**
Check tally session status:
- `execution_status` should be `AWAITING_INPUT`
- `annotations.tie_break` should contain tie information

**5. Submit Decision:**
```bash
cli step submit-tally-resolution \
  --election-event-id <EVENT_ID> \
  --tally-id <TALLY_ID> \
  --resolution <CONTEST_ID>:cand-a
```

**6. Verify Completion:**
- Status changes to `IN_PROGRESS` → `SUCCESS`
- Winner is "Alice"
- Resolution is recorded in `tally_session_resolution` table with timestamp and user

---

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────┐
│ Admin Portal UI (Future)                                │
│ - Show tie notification                                 │
│ - Display tied candidates with vote counts              │
│ - Input decision and method used                        │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────┐
│ Harvest API                                              │
│ POST /submit-tally-resolution                           │
│ - Validates status (AWAITING_INPUT)                     │
│ - Creates new resolution row per contest (audit trail)  │
│ - Changes status to IN_PROGRESS (or STARTED on re-sub) │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────┐
│ Windmill (Orchestration)                                │
│ - Detects pause needed (RequiresExternalInput)         │
│ - Saves RunoffStatus to annotations                     │
│ - Creates pending resolution rows in DB                 │
│ - Updates execution_status to AWAITING_INPUT            │
│ - On resume: loads state, applies latest resolution    │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────┐
│ Velvet (IRV Algorithm)                                  │
│ instant_runoff.rs                                       │
│ - run_with_policy() - accepts tie-breaking policy      │
│ - Returns RunoffResult enum                             │
│ - apply_external_tie_decision() - eliminates others    │
└─────────────────────────────────────────────────────────┘
```

### Key Files

| Component | File | Description |
|-----------|------|-------------|
| **Types** | `sequent-core/src/ballot.rs` | TieBreakingPolicy enum |
| | `sequent-core/src/types/ceremonies.rs` | AWAITING_INPUT status |
| **Velvet** | `velvet/src/pipes/do_tally/counting_algorithm/instant_runoff.rs` | Core IRV logic with tie-breaking |
| **Windmill** | `windmill/src/postgres/tally_session.rs` | Annotation management |
| | `windmill/src/postgres/tally_session_resolution.rs` | Resolution table operations |
| **Harvest** | `harvest/src/routes/tally_ceremony.rs` | API endpoint |
| **CLI** | `step-cli/src/commands/submit_tally_resolution.rs` | CLI command |
| **Tests** | `velvet/tests/instant_runoff/irv_tie_breaking_tests.rs` | Test suite |

### State Machine

```
                    ┌─────────────┐
                    │ NOT_STARTED │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │   STARTED   │◄──────────────────┐
                    └──────┬──────┘                   │
                           │                           │ Re-submission
                           ▼                           │ (new resolution row)
                    ┌─────────────┐                   │
                    │  CONNECTED  │                   │
                    └──────┬──────┘                   │
                           │                           │
                           ▼                           │
                    ┌─────────────┐           ┌───────┴──────┐
         ┌──────────│ IN_PROGRESS │◄──────────│AWAITING_INPUT│
         │          └──────┬──────┘  First    └──────┬───────┘
         │                 │         submission       │
         │ Tie Detected    │ No Tie                   │ Re-submission
         │ (External       │                           │ (new resolution row
         │  Policy)        │                           │  → STARTED)
         │                 │                           │
         ▼                 ▼                           │
  ┌──────────────┐  ┌─────────────┐                  │
  │AWAITING_INPUT├──►   SUCCESS   │◄─────────────────┘
  └──────────────┘  └─────────────┘    (after re-execution)
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
1. ✅ Status is not `AWAITING_INPUT` (already resumed or never paused)
2. ✅ Multiple admins submitted simultaneously (race condition)

**Solution:** Check current status:
```sql
SELECT execution_status, annotations
FROM sequent_backend.tally_session
WHERE id = 'your-tally-id';
```

### Check Resolution History

To view the full audit trail of all resolutions submitted for a tally session:
```sql
SELECT contest_id, selected_candidate_id, resolved_by_user_id, created_at
FROM sequent_backend.tally_session_resolution
WHERE tally_session_id = 'your-tally-id'
ORDER BY created_at DESC;
```

---

## Future Enhancements

### Admin Portal UI (Pending Design)
- Visual notification when tally pauses
- Display tied candidates with names and vote counts
- Input field for external method used (dropdown or text)
- Resume button after decision entered
- Audit trail viewer showing all submitted resolutions

### Additional Features
- Custom tie-breaking method documentation
- Email notifications to admins when pause occurs
- Webhook integration for external systems
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
A: Yes, a resolution can be re-submitted. Each submission creates a new row in `tally_session_resolution`, preserving the full audit trail. The latest resolution (by `created_at`) is used, and the tally re-executes automatically.

**Q: What if two admins submit different decisions simultaneously?**
A: Both submissions succeed and are both recorded in the audit trail. The tally will use the latest one by `created_at`. Since re-submission is supported, this is not a race condition — both decisions are preserved and the tally re-executes with the most recent.

**Q: Can multiple tie contests be resolved in one call?**
A: Yes, the `--resolution` flag in the CLI and the `resolutions` array in the API both accept multiple entries, one per tied contest.

---

## Support

For questions or issues:
- GitHub Issues: https://github.com/sequentech/step/issues
- Documentation: https://docs.sequentech.io
- Email: support@sequentech.io
