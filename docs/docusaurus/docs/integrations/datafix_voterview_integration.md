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
  VoterView with a `SetVoted` SOAP request, so the voter cannot also vote
  through another channel; when an administrator disables a voter who has
  voted online, the platform sends `SetNotVoted`, releasing the voter to
  vote through another channel.

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
     internet, a `SetVoted` request is sent to VoterView first (see
     [Outbound requests](#outbound-requests-to-voterview)).
3. A tally session **refuses to run** while any vote of the elections being
   tallied is still `in-progress` (see the
   [tally engine documentation](../07-developers/07-velvet/03-tally.md)), so a
   VoterView outage blocks the tally rather than under-counting.

## Outbound requests to VoterView

The platform sends two SOAP requests to VoterView, both only for Datafix
election events. Every request posts its outcome to the immutable electoral
log with direction *outbound*: the log description shows the short outcome
(e.g. `Outbound request SetVoted Succeeded.`), and the failure reason is kept
in the full log message — VoterView's error message when it returns one (e.g.
`SetNotVoted Failed: The voter has not voted.`), otherwise the HTTP status or
transport error (e.g. `SetNotVoted Failed: HTTP 504 Gateway Timeout`).

### `SetVoted` — a voter casts an online vote

Sent by the `process_cast_vote` task (windmill) while resolving the status of
a newly inserted vote, to mark the voter in VoterView as having voted so they
cannot also vote through another channel.

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

### `SetNotVoted` — an administrator disables a voter

Sent by harvest's `/edit-user` endpoint (the admin-portal voter edit) when a
request on a Datafix election event sets the voter's `enabled` flag to
`false` and the voter has a prior `valid` online vote in the event.
Disabling the voter withdraws their ability to vote online, so VoterView is
told to clear their voted mark, releasing the voter to vote through another
channel — a Datafix requirement.

- The request is only sent if the voter has a prior `valid` vote in the
  event — without one, VoterView has nothing to clear and would answer
  `The voter has not voted.`. Disabling a voter who never voted online (or
  whose vote is still `in-progress` or `discarded`) sends nothing and
  records nothing in the electoral log. The voter's previous enabled state
  is not checked.
- The request is sent in the background: the edit response does not wait for
  the VoterView round-trip and does not fail if the request fails — the voter
  is disabled on the platform either way, and the outcome is only recorded in
  the electoral log (`SetNotVoted Succeeded` / `SetNotVoted Failed:
  <VoterView's message>`).
- Transport and HTTP-level failures (timeouts, gateway errors) are retried a
  few times with exponential backoff before the failure is recorded;
  `Success=false` answers are definitive and are not retried. Re-saving the
  voter with `enabled` unchecked sends the request again.
- Re-enabling a voter sends nothing. Note that `SetNotVoted` does not clear
  the voter's `voted_channel=internet` Keycloak attribute either: if a voter
  who already voted online is disabled and later re-enabled, a new vote is
  accepted without re-sending `SetVoted`.

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
- VoterView requests time out after 30 seconds; a timed-out request counts as
  failed (a timed-out `SetVoted` leaves the vote `in-progress` and is
  retried).
- Workers must consume the `process_cast_vote_queue`; otherwise votes stay
  `in-progress` and tallies are blocked.
- The beat interval is configurable via the `--review-cast-votes-interval`
  flag (default 90 seconds).
- Cast votes can only be inserted through the `insert_cast_vote` action
  (harvest); there is no direct GraphQL insert, so no vote can skip the
  status pipeline.
