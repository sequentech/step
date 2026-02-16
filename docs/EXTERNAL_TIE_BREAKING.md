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
7. Decision is permanently recorded in annotations with timestamp and user

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

Administrator submits tie-break decision:

**Via API:**
```bash
POST /submit-tie-break-decision
Content-Type: application/json
Authorization: Bearer <token>

{
  "election_event_id": "event-123",
  "tally_session_id": "tally-456",
  "selected_candidate_id": "candidate-a-uuid"
}
```

**Via CLI:**
```bash
step-cli submit-tie-break \
  --election-event-id event-123 \
  --tally-id tally-456 \
  --candidate-id candidate-a-uuid
```

### 4. Resume

System automatically resumes tally:

1. Validates candidate is in tied candidates list
2. Updates annotations with resolution:
   ```json
   {
     "tie_break": {
       ...
       "resolution": {
         "resolved_by_candidate_id": "candidate-a-uuid",
         "resolved_at": "2026-02-11T11:00:00Z",
         "resolved_by_user": "admin-user-uuid"
       }
     }
   }
   ```
3. Changes status from `AWAITING_INPUT` to `IN_PROGRESS`
4. Loads paused RunoffStatus from annotations
5. Applies decision: eliminates all candidates except chosen winner
6. Continues IRV algorithm to completion

---

## API Usage

### Endpoint: `POST /submit-tie-break-decision`

**Authentication:** Bearer token with `ADMIN_CEREMONY` permission

**Request:**
```json
{
  "election_event_id": "uuid",
  "tally_session_id": "uuid",
  "selected_candidate_id": "uuid"
}
```

**Response (Success):**
```json
{
  "success": true,
  "tally_session_id": "uuid"
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
3. ✅ Candidate must be in `tied_candidates` list
4. ✅ User must have `ADMIN_CEREMONY` permission

**HTTP Status Codes:**
- `200` - Success
- `400` - Bad request (invalid status, invalid candidate)
- `401` - Unauthorized
- `404` - Tally session not found
- `500` - Server error

---

## CLI Usage

### Command: `step-cli submit-tie-break`

**Installation:**
Ensure step-cli is installed and configured:
```bash
step-cli config --endpoint https://api.example.com --token <your-token>
```

**Usage:**
```bash
step-cli submit-tie-break \
  --election-event-id <EVENT_ID> \
  --tally-id <TALLY_SESSION_ID> \
  --candidate-id <CANDIDATE_ID>
```

**Example:**
```bash
step-cli submit-tie-break \
  --election-event-id a1b2c3d4-e5f6-7890-abcd-ef1234567890 \
  --tally-id b2c3d4e5-f6a7-8901-bcde-f12345678901 \
  --candidate-id c3d4e5f6-a7b8-9012-cdef-123456789012

# Output:
# Success! Tie-break decision submitted. Selected candidate: c3d4e5f6-a7b8-9012-cdef-123456789012
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `--election-event-id` | Yes | UUID of the election event |
| `--tally-id` | Yes | UUID of the tally session |
| `--candidate-id` | Yes | UUID of the selected candidate |

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
step-cli start-tally --election-event-id <ID> --election-ids <ID>
```

**4. Verify Pause:**
Check tally session status:
- `execution_status` should be `AWAITING_INPUT`
- `annotations.tie_break` should contain tie information

**5. Submit Decision:**
```bash
step-cli submit-tie-break \
  --election-event-id <EVENT_ID> \
  --tally-id <TALLY_ID> \
  --candidate-id cand-a
```

**6. Verify Completion:**
- Status changes to `IN_PROGRESS` → `SUCCESS`
- Winner is "Alice"
- Annotations contain resolution with timestamp and user

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
│ POST /submit-tie-break-decision                         │
│ - Validates status (AWAITING_INPUT)                     │
│ - Validates candidate in tied list                      │
│ - Updates annotations with resolution                   │
│ - Changes status to IN_PROGRESS                         │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────┐
│ Windmill (Orchestration)                                │
│ - Detects pause needed (RequiresExternalInput)         │
│ - Saves RunoffStatus to annotations                     │
│ - Updates execution_status to AWAITING_INPUT            │
│ - On resume: loads state, applies decision              │
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
| **Harvest** | `harvest/src/routes/tally_ceremony.rs` | API endpoint |
| **CLI** | `step-cli/src/commands/submit_tie_break.rs` | CLI command |
| **Tests** | `velvet/tests/instant_runoff/irv_tie_breaking_tests.rs` | Test suite |

### State Machine

```
                    ┌─────────────┐
                    │ NOT_STARTED │
                    └──────┬──────┘
                           │
                           ▼
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
         ┌──────────│ IN_PROGRESS │◄──────────┐
         │          └──────┬──────┘           │
         │                 │                   │
         │ Tie Detected    │ No Tie           │ Decision
         │ (External       │                   │ Submitted
         │  Policy)        │                   │
         │                 │                   │
         ▼                 ▼                   │
  ┌──────────────┐  ┌─────────────┐          │
  │AWAITING_INPUT├──►   SUCCESS   │          │
  └──────┬───────┘  └─────────────┘          │
         │                                     │
         └─────────────────────────────────────┘
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

### Invalid Candidate Error

**Symptom:** API returns "Selected candidate is not in tied candidates"

**Causes:**
1. ✅ Wrong candidate ID provided
2. ✅ Typo in UUID
3. ✅ Candidate was not actually tied

**Solution:** Check tie state in annotations:
```sql
SELECT annotations->'tie_break'->'tied_candidates'
FROM sequent_backend.tally_session
WHERE id = 'your-tally-id';
```

---

## Future Enhancements

### Admin Portal UI (Pending Design)
- Visual notification when tally pauses
- Display tied candidates with names and vote counts
- Input field for external method used (dropdown or text)
- Resume button after decision entered
- Audit trail viewer

### Additional Features
- Support for multiple tie-breaking rounds
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
A: No. Once submitted and the tally completes, the decision is final and recorded in the immutable audit trail.

**Q: What if two admins submit different decisions simultaneously?**
A: The first submission wins (transaction-level database locking). The second will receive an error that status is no longer `AWAITING_INPUT`.

---

## Support

For questions or issues:
- GitHub Issues: https://github.com/sequentech/step/issues
- Documentation: https://docs.sequentech.io
- Email: support@sequentech.io
