---
id: datafix_voterview_integration
title: Datafix / VoterView Integration
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Datafix / VoterView Integration

Datafix is a voter-registry provider whose **VoterView** service tracks, per
voter, whether they have already voted through any channel (in person, by
mail, online). The integration has two directions:

- **Inbound**: Datafix manages the voter roll through a REST API exposed by
  the platform (add/update/delete voters, mark/unmark voted, replace PIN).
- **Outbound**: when a voter casts an online vote, the platform notifies
  VoterView with a `SetVoted` SOAP request (and `SetNotVoted` when a voter is
  re-enabled), so the voter cannot also vote through another channel.

An election event participates in this integration when its annotations
contain the Datafix configuration (`datafix:id`, VoterView credentials and
endpoint, PIN password policy).

## Cast vote lifecycle

Every cast vote has a `status`:

| Status | Meaning |
|---|---|
| `in-progress` | Inserted, still being verified. Not counted anywhere. |
| `valid` | Verified. The only status counted by tallies, dashboards, statistics and reports. |
| `discarded` | Rejected — for Datafix events, VoterView reported the voter had already voted through another channel. Never counted. |

The flow:

1. The voting portal casts the vote through the `insert_cast_vote` action →
   harvest, which inserts it as `in-progress` and enqueues a
   `process_cast_vote` task. If enqueueing fails the vote is still accepted —
   the `review_cast_votes` beat (default every 90s) re-enqueues any
   `in-progress` vote older than a short grace window.
2. `process_cast_vote` resolves the status:
   - **Non-Datafix events**: promoted to `valid` immediately.
   - **Datafix events**: if the voter is not yet marked as having voted via
     internet, a `SetVoted` request is sent to VoterView first (see below).
3. A tally session **refuses to run** while any vote of the elections being
   tallied is still `in-progress` (see the
   [tally engine documentation](../07-developers/07-velvet/03-tally.md)), so a
   VoterView outage blocks the tally rather than under-counting.

### SetVoted semantics

- `SetVoted` is sent **once per voter**: after a successful request the
  voter's `voted_channel=internet` attribute is stored in Keycloak (with
  retries) and later votes skip the request — re-votes and votes in other
  elections of the same event do not notify VoterView again.
- VoterView answering **`HasVoted`** is disambiguated: if the voter already
  has a prior `valid` internet vote in the event, the response is treated as
  the echo of our own earlier `SetVoted` — the re-vote is **accepted** as
  `valid` and the Keycloak attribute is restored. Only when the voter has no
  prior valid internet vote is the vote **`discarded`** (they genuinely voted
  through another channel).
- Any other VoterView error or an unreachable service leaves the vote
  `in-progress`: it is retried on the next beat and, if the situation
  persists, the tally stays blocked until an operator intervenes.

All outbound requests post their outcome to the immutable electoral log
(e.g. `SetVoted Succeeded`, `SetVoted Failed`).

## Inbound API

All endpoints are `POST`, require a JWT with the `DATAFIX_ACCOUNT`
permission, and identify the election event through the caller's
`datafix_event_id` claim. `voter_id` always refers to the Keycloak
**username**.

| Endpoint | Body | Effect |
|---|---|---|
| `/add-voter` | voter info (ward/schoolboard/poll, birthdate…) | Creates the voter in the event realm, assigned to the area matching the ward/schoolboard/poll combination. |
| `/update-voter` | voter info | Updates area/birthdate/enabled state. |
| `/delete-voter` | `{voter_id}` | Disables the voter (voters are never deleted). |
| `/mark-voted` | `{voter_id, channel}` | Marks the voter as having voted through another channel and disables them. |
| `/unmark-voted` | `{voter_id}` | Clears the voted mark and re-enables the voter. |
| `/replace-pin` | `{voter_id}` | Generates a new PIN following the event's password policy and returns it. |

Every call — successful or failed — is recorded in the electoral log with its
outcome (failures include the error code, e.g.
`AddVoter Failed: voter-already-exists`).

### Responses and error codes

Success responses are `200 OK` with `{"code": 200, "message": "OK"}`
(`/replace-pin` returns `{"pin": "..."}`). Errors carry the same HTTP status
in the response body plus a stable, machine-readable `error_code`:

```json
{
    "code": 409,
    "message": "Conflict",
    "error_code": "voter-already-exists"
}
```

| HTTP status | `error_code` | Returned when |
|---|---|---|
| 409 Conflict | `voter-already-exists` | `/add-voter` for a username that already exists. Not retryable — use `/update-voter` to modify the voter. |
| 404 Not Found | `voter-not-found` | No voter matches `voter_id`. |
| 409 Conflict | `voter-not-unique` | More than one voter matches `voter_id` (data-integrity issue — contact support). |
| 422 Unprocessable Entity | `area-not-found` | No area matches the ward/schoolboard/poll combination. |
| 404 Not Found | `event-not-found` | No election event is configured for the caller's `datafix_event_id`. |
| 400 Bad Request | `invalid-request` | Invalid field values (e.g. malformed birthdate, replace-pin for a disabled voter). |
| 403 Forbidden | `forbidden` | The JWT is valid but lacks the `DATAFIX_ACCOUNT` permission. |
| 500 Internal Server Error | `internal-error` | Unexpected server-side failure. Safe to retry later. |

The `error_code` values are a **stable contract**: new codes may be added,
but existing ones will not change meaning. Clients should branch on
`error_code` (or the HTTP status) rather than parsing `message`.

## Operational notes

- The `SetVoted` / `SetNotVoted` SOAP bodies are rendered from the
  `voterview_setvoted.hbs` / `voterview_setnotvoted.hbs` Handlebars templates
  stored in the MinIO **public assets** bucket — they must be present in every
  environment.
- Workers must consume the `process_cast_vote_queue`; otherwise votes stay
  `in-progress` and tallies are blocked.
- The beat interval is configurable via the `--review-cast-votes-interval`
  flag (default 90 seconds).
- Cast votes can only be inserted through the `insert_cast_vote` action
  (harvest); there is no direct GraphQL insert, so no vote can skip the
  status pipeline.
