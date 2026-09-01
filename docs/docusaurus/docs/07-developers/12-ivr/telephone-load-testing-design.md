---
id: telephone-load-testing-design
title: Telephone Load Testing — Design
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Telephone Load Testing — Design

This documents how the telephone (IVR/DTMF) load-testing tooling works. For
step-by-step instructions to actually run it, see
[Telephone Load Testing Guide](telephone-load-testing-guide.md).

> This covers the **TELEPHONE** channel only, driven by `step-cli` +
> `ivr-cli`. It is unrelated to the **ONLINE** channel load-testing tooling
> documented under [CLI tutorials](../02-cli/02-tutorials/load-testing/load_testing.md).

## Two stages

1. **Stage 1 — Provisioning** (`step-cli`, network calls to Hasura/Keycloak
   only): imports an election event for a tenant, bulk-creates voters with
   telephone-friendly credentials, runs the keys ceremony, publishes, and
   opens telephone voting. Ends with an election event ready to accept IVR
   calls, plus a voters CSV. Implemented as
   `packages/step-cli/scripts/setup-telephone-load-test.sh`.
2. **Stage 2 — Calling** (`ivr-cli --bundle dev`): builds a local
   `phone_config.json` pointing at the event from Stage 1, turns the voters
   CSV into one DTMF input script per voter, and fans out N parallel
   `ivr-cli` processes — each one an independent simulated phone call —
   against the dev container's real Keycloak/Hasura, no telephony
   infrastructure involved. Implemented as
   `packages/step-cli/scripts/run-telephone-load-test.sh`.

## Design constraints

### Telephone voting needs numeric, ≤8-digit credentials

A caller only has a phone keypad. The IVR authentication flow
(`beyond/packages/ivr-core/src/execution/phases/auth.rs`) maps DTMF fields
straight onto Keycloak ROPC form parameters, each capped at a small number of
digits — the default flow uses `voter_id`/`password`; this realm's flow (see
below) uses `dateOfBirth`/`password` instead. Either way, every load-test
voter's identifying fields must be plain numeric strings. `step generate-voters`'s
bare-counter username already satisfies this, and setting
`voter_password_policy` to `RandomNumeric { digits: <=8 }` in the run's
`external_config.json` produces a matching numeric PIN.

The identifying fields an IVR auth flow actually asks for are **not fixed** —
they are fetched live per-realm from a Keycloak extension endpoint
(`{realm}/ivr-config`). Don't assume the generic voter-id+PIN default; check
it for the realm you're testing against. See
[DOB + PIN Authentication](dob-pin-direct-grant-authenticator.md) for one
concrete flow this system supports.

### The TELEPHONE voting channel must be explicitly opened

`update-event-voting-status` has a `--voting-channel` flag distinct from
`ONLINE`; the IVR eligibility check gates specifically on
`VotingStatusChannel::TELEPHONE`. Stage 1 always opens `TELEPHONE`
voting — opening only `ONLINE` (as the general CLI tutorials do) makes every
simulated call fail at the eligibility phase after a successful login.

### Every voter is pinned to a single election area

Contests are assigned per voter area, and different areas can have different
contest counts. Since the DTMF script hard-codes one keystroke sequence per
contest, a script captured for a 4-contest area would diverge for a voter
whose area only has 3. Stage 1 avoids this entirely: it restricts voter
generation to a single area (`--voter-area-name`, defaulting to the election
event's first area) before handing the file to `generate-voters`, so every
generated voter shares the same contest count and one captured DTMF template
is valid for every call in the run. This only affects which area voters are
generated into — the imported election event itself keeps every area.

### Every voter casts exactly one vote, with distinct credentials

Each Stage 1 run generates a fresh voters CSV — unique numeric
username/password/date-of-birth per voter — so every simulated call logs in
as a genuinely new voter rather than reusing one across calls. The election
does not need to allow revoting for a normal run; re-running Stage 2 against
the *same* Stage 1 output re-uses the same (already-voted) voters and is
correctly rejected as a duplicate vote, not a bug. To place a fresh batch of
calls, re-run Stage 1 to provision a new election event and voter set.

### The DTMF ballot script is captured empirically

Login prompts are fixed and simple (see above), but the rest of the call —
language selection, per-contest candidate numbering, confirm/clear, submit —
depends on the specific election's contest/candidate counts and the runtime
numbering the IVR prompt store assigns them. Rather than reimplementing that
numbering logic, the script is captured once by driving a real call
interactively with `ivr-cli --show-internal-state` against the freshly
provisioned Stage 1 event, noting every keystroke. That sequence, with the
identifier/PIN lines replaced by placeholders, becomes the template Stage 2
substitutes per voter (`packages/step-cli/scripts/dtmf-template.example.txt`
is a working example, captured against the tracked example election event —
see the guide for the exact procedure).

### `phone_config.json` is a template fill from Stage 1's outputs

`ivr-cli --bundle dev` reads `PHONE_CONFIG_PATH` as a local JSON file mapping
a dialed-in system number to `{tenant_id, election_event_id, keycloak_realm,
keycloak_url, hasura_url, ...}`. `keycloak_realm` always follows
`tenant-{tenant_id}-event-{election_event_id}`, and both ids are already
known from Stage 1's `summary.json` — so Stage 2 generates this file directly
from that summary, no new data to invent.
