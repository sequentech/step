---
id: key_ceremony_bbt_propossal_no_protoccol_change
title: BBT Key Ceremony — Signing Keypair Proposal (No Protocol Change)
sidebar_label: BBT Signing Keypair (No Protocol Change)
---

# BBT Signing Keypair — No Protocol Change

This document describes the lower-risk approach to introducing Browser-Based Trustee (BBT)
signing keys — a new capability where each trustee generates their own keypair directly in the
browser via WASM, holds the private material **in memory only** for the duration of the
ceremony page session, and registers the public key in the existing `sequent_backend_trustee`
DB table via a new Harvest endpoint. The trustee's downloaded TOML backup is the only
durable copy of the secret material; nothing is ever written to `localStorage` or
`sessionStorage`. Previously, only server-based trustees existed; this is the first design
to support in-browser key generation.
The B4 protocol and all DKG (Distributed Keys Generation) message types remain unchanged.

See [BBT Protocol Change Proposal](./key_ceremony_bbt_propossal_protoccol_change.md) for the
alternative design that removes the DB dependency entirely.

See [Key Ceremony Design](./key_ceremony_design.md) for the current architecture baseline.

> **Scope note — tally ceremonies:** this document covers only the key ceremony.  Tally
> ceremonies are out of scope here but must be accounted for: BBT trustees will participate in
> tally ceremonies too, using the same identity keys (`signing_key_sk`, `encryption_key`) they
> generated during the key ceremony.  Any design decision made here that affects how identity
> keys are stored, looked up, or rotated has direct implications for the tally flow.

---

## 1. Service Communication Paths

Braid-wasm is considered part of admin-portal.  Rows marked **BBT** are new or changed by
this proposal; all others are unchanged from the current architecture.

| # | Origin | Destination | Protocol | What |
|---|--------|-------------|----------|------|
| 1 | Admin-portal | Harvest (via Hasura) | HTTP POST | `create-keys-ceremony` (now accepts optional `duplicate_from`), `list-keys-ceremonies`, `get-private-key`, `check-private-key`, all admin operations |
| 2 | Admin-portal | Harvest | HTTP POST | **`register-trustee-key` — BBT new:** registers the BBT signing public key for `(trustee, event, ceremony)` (no secret material) |
| 2a | Admin-portal (browser) | Harvest (Hasura action) | HTTP POST | **BBT new:** `confirm_key_backup` — writes `encryption_key_commitment` at Download Step time and advances `KEY_RETRIEVED` |
| 2b | Admin-portal (browser) | Harvest (Hasura action) | HTTP POST | **BBT new:** `issue_key_check_nonce` — issues a short-lived nonce for the signed-attestation Check Step |
| 2c | Admin-portal (browser) | Harvest (Hasura action) | HTTP POST | **BBT new:** `submit_key_check_attestation` — accepts signed attestation `{ payload, signature, nonce, commitment }`; advances `KEY_CHECKED` on success; receives no secret material |
| 2d | Admin-portal | Harvest (Hasura action) | HTTP POST | **BBT new:** `cancel_keys_ceremony` — atomic `CANCELLED` transition + clears `election_event.keys_ceremony_id` |
| 3 | Admin-portal | Hasura | GraphQL/HTTP | Read trustee config (`GET_TRUSTEE_CONFIG`), election events, ceremony status (`execution_status`), all entity queries |
| 4 | Admin-portal (braid-wasm) | B4 | HTTP | `GET /boards`, `GET /messages`, `POST /messages` — full DKG protocol (Channel, Shares, PublicKey…), signed locally in the browser |
| 5 | Harvest / Windmill | PostgreSQL (Hasura DB) | SQL direct | Read/write `keys_ceremony`, `trustee`, `election_event`, `sequent_backend_trustee_verification_nonce` tables.  **BBT change:** `trustee` now has `election_event_id`, `keys_ceremony_id`, `encryption_key_commitment`; reads filter by all three for BBT, by `NULL` for server-based |
| 6 | Windmill (Celery) | B4 PostgreSQL | SQL direct (`PgsqlB3Client`) | INSERT `Configuration` message (`add_config_to_board`), SELECT `PublicKey` message (`get_board_public_key`) |
| 7 | Braid-native | B4 | HTTP | `GET /boards`, `GET /messages`, `POST /messages` — full DKG protocol, same as wasm |
| 8 | Braid-native | Keycloak | HTTP | Fetch JWT access token using `TRUSTEE_NAME` / `TRUSTEE_PSW` env vars |

**Notes:**
- Windmill bypasses B4's HTTP API entirely — it writes directly into B4's PostgreSQL via
  `PgsqlB3Client` using `B4_PG_*` env vars.
- Braid (both native and wasm) uses B4's HTTP API exclusively.
- Admin-portal never talks to B4 directly except through the embedded braid-wasm trustee.
- Admin-portal reads ceremony status by polling Hasura (GraphQL), not by calling Harvest.
- For BBT trustees, the `signing_key_pk` is registered in the Hasura DB (path 2) so that
  Windmill can include it in the `Configuration` message posted to B4 (path 6).

> **B4 signing invariant.** BBT trustees sign every B4 message locally in the browser using
> their in-memory `signing_key_sk`.  Harvest never signs on behalf of a BBT trustee and never
> holds BBT signing material.  The browser posts signed B4 messages directly to B4 (path 4),
> and separately notifies Harvest/Hasura for ceremony state transitions and for recording the
> `encryption_key_commitment`.  If a future audit requirement ever justifies routing B4
> traffic through Harvest, the relay must forward the already-signed envelope verbatim —
> never open, modify, or re-sign it.

---

## 2. The Per-Event Key Problem

A BBT trustee may participate in multiple election events simultaneously.  A single global
`public_key` per trustee row would be overwritten by whichever event registered last, breaking
DKG verification on any previously started ceremony.

This approach solves it by adding an `election_event_id` column so each
`(trustee, election_event)` pair has its own independent public key.

---

## 3. Ceremony Creation Gate

### Current status lifecycle

The `KeysCeremonyExecutionStatus` enum (in `sequent-core`) has five variants:

| Status | Meaning |
|---|---|
| `USER_CONFIGURATION` | Defined but currently **unused** — the default is `STARTED` |
| `STARTED` | **Default on creation.** Config message not yet posted to the board. |
| `IN_PROGRESS` | Config posted; trustees are performing DKG. |
| `SUCCESS` | All trustees have reached `KEY_CHECKED`. |
| `CANCELLED` | Ceremony was cancelled. |

The beat service runs `review_boards` repeatedly, which fans out one `process_board` task per
election event.  `process_board` inspects each ceremony's `execution_status`:

- `STARTED` → dispatches a `create_keys` Celery task
- `IN_PROGRESS` (no `public_key` yet) → dispatches a `set_public_key` Celery task

`create_keys_impl` (not `create_keys_ceremony`) is the code that actually reads trustee public
keys from the DB and posts the Configuration message to the B4 board.
`create_keys_ceremony` (Harvest) only inserts the ceremony record with `execution_status =
STARTED`; it never touches `public_key`.

### Where the BBT problem surfaces

When `create_keys_impl` runs for a ceremony that includes BBT trustees it calls
`get_trustees_by_id` and collects their `public_key` fields.  If a BBT trustee has not yet
opened the election event page their row has no `public_key`; the current code uses
`.filter_map(|t| t.public_key)` which silently drops missing keys.

The resulting truncated list is passed to `public_keys::create_keys` →
`protocol_manager::add_config_to_board` → `Configuration::new` (in
`b4/src/messages/artifact.rs`), which immediately calls `assert!(c.is_valid())`.
`is_valid()` requires `trustees.len() > 1` and `threshold <= trustees.len()`, so if any
keys are silently dropped the assert fires and the Windmill worker **panics**.

### Key-reading must be mode-aware

Once the `election_event_id` column is added to `sequent_backend_trustee`, the logic that
reads public keys in `create_keys_impl` must differentiate by trustee mode:

| Trustee type | How to read `public_key` |
|---|---|
| **Server-based** | Read the global `public_key` directly — no `election_event_id` filter. |
| **Browser-based** | Filter by both `tenant_id`, `election_event_id` **and** `keys_ceremony_id` to get the per-event key. |

`get_trustee_mode_policy()` in `windmill/src/postgres/trustee.rs` already reads the
`TrusteeModePolicy` from the trustee's `annotations.trustee_mode_policy` JSON field, so the
mode is already detectable.  The query in the document's Component Changes section
(`ORDER BY election_event_id NULLS LAST`) covers both cases in one SQL statement, but the
calling code must know whether to pass an `election_event_id` parameter at all.

### Repurpose `USER_CONFIGURATION` as `AWAITING_TRUSTEE_KEYS`

`USER_CONFIGURATION` exists in the enum today but is unused and carries a generic name.
Rename it to `AWAITING_TRUSTEE_KEYS` to give it a clear, purpose-built meaning.

#### How it works today

Posting the `Configuration` message to the bulletin board happens in the same beat-task step
that advances the status:

1. Harvest's `create_keys_ceremony`, triggered by the admin portal, inserts the
   `keys_ceremony` row and sets `execution_status = STARTED` synchronously.  It never reads
   trustee keys or touches the board.
2. On the next beat cycle, a Windmill task picks up the `STARTED` ceremony, posts the
   `Configuration` message to B4, and advances the status to `IN_PROGRESS` — both in one
   step.

#### How it works with this change

- The call that creates the keys ceremony writes `AWAITING_TRUSTEE_KEYS` instead of
  `STARTED`.
- The beat-triggered Windmill task that previously took `STARTED → IN_PROGRESS` now operates
  from `AWAITING_TRUSTEE_KEYS`.  Each beat it checks every trustee in the ceremony:
  - **BBT trustees**: have they registered a `public_key` for this `(trustee_id,
    election_event_id, keys_ceremony_id)`?
  - **Server-based trustees**: is their `public_key` configured?

  If any key is still missing the ceremony stays in `AWAITING_TRUSTEE_KEYS` and the check
  retries on the next beat.  Once every key is present, the task posts the `Configuration`
  message to B4 **and** advances the status to `IN_PROGRESS` in the same step — exactly as
  today, only now gated on the key-availability check.

#### Gate-firing edge case: registered-then-lost

The gate fires on `public_key` presence only.  A BBT trustee may register a key, close the
tab before downloading the backup, and lose `signing_key_sk` (in-memory only — see
[§7 Flow](#7-flow) and [§9 Key Loss and Recovery](#9-key-loss-and-recovery)).  On the next
beat the gate still sees a `public_key` for that trustee and posts the `Configuration`
message with it.  When the trustee returns, they regenerate fresh keys whose `public_key`
no longer matches what is now on the board — they cannot post valid DKG messages.

The navigate-away guard (see [§7](#7-flow)) is the only practical mitigation; the gate
itself cannot distinguish "registered and still held" from "registered and lost".  **The
only real recovery is to cancel the ceremony and recreate it** (see
[§6 Cancellation Window](#6-cancellation-window)), provided no election in the event has
opened its voting period.

See the [Flow](#7-flow) section below for the full sequence diagram.

The enum becomes:

```rust
pub enum KeysCeremonyExecutionStatus {
    AWAITING_TRUSTEE_KEYS,   // renamed from USER_CONFIGURATION: waiting for trustee keys
    IN_PROGRESS,             // Configuration posted to board; DKG running
    SUCCESS,
    CANCELLED,
}
```

`STARTED` is **removed**: `AWAITING_TRUSTEE_KEYS` absorbs the pre-config gating role and the
transition straight to `IN_PROGRESS` happens once all keys are present.

Pro: repurposes the existing unused variant; intent is explicit; the UI can show a distinct
"waiting for keys" state; transition table is enforced at runtime via a `try_transition` guard
(see below).
Con: any code that already serialises/stores `USER_CONFIGURATION` or `STARTED` as a string
must be migrated.  `USER_CONFIGURATION` is currently unused so that side is a no-op.
Existing references to `STARTED` in `process_board`, `create_keys_ceremony`, and admin-portal
status filters must be updated to the new `AWAITING_TRUSTEE_KEYS → IN_PROGRESS` transition.

### Runtime state machine

The transition table is enforced at **runtime** via a `try_transition` method added to
`KeysCeremonyExecutionStatus` in `sequent-core`.  This is the correct tool here: the status
is a string in the DB, reconstructed on every service call — there is no long-lived typed
value for a compile-time typestate pattern to protect.

See [Keys Ceremony State Machine](./key_ceremony_state_machine.md) for the full design,
implementation code, tests, and call-site table.

---

## 4. Trustee Online Presence

For the admin to observe that all BBT trustees are online after creating the ceremony, each
trustee's browser must continuously report its presence to B4 via a heartbeat POST to
`/boards/{board_name}/sessions/heartbeat`.

### How the heartbeat works

The heartbeat is driven by a background daemon spawned inside the WASM session itself, not by
any JavaScript timer. When a `WasmSession` successfully connects to a board, the admin portal
calls `start_heartbeat_daemon` on it, which launches an internal async loop (using Rust's
`spawn_local`). That loop wakes at the configured `BRAID_B4_HEARTBEAT` interval, sends the
heartbeat POST, and goes back to sleep — independently of whatever the JavaScript side is doing.

Because the daemon lives inside WASM rather than in a JavaScript `setInterval`, it is not
subject to the browser's background-tab timer throttling. The trustee remains visible as online
even when they switch to a different browser tab or leave the page idle for an extended period.
The daemon stops only when the WASM session is freed, which happens either on an explicit
identity/key-rotation flush or when the browser tab is closed.

### Session lifetime across navigation

Sessions are stored in a module-level registry (one entry per board name) that persists for the
entire browser tab session, independently of React component mount/unmount cycles. When a trustee
navigates from one election event page to another and then back, the existing session is reused
from the registry rather than recreated — the heartbeat daemon has been running the whole time.
If the trustee has boards open for multiple election events simultaneously, each has its own
independent session and daemon running concurrently.

---

## 5. TrusteeStatus Lifecycle

Each trustee inside a ceremony carries a `TrusteeStatus` (in `sequent-core`):

```rust
pub enum TrusteeStatus {
    WAITING,
    KEY_GENERATED,
    KEY_RETRIEVED,
    KEY_CHECKED,
}
```

### WAITING

Set for every selected trustee when `create_keys_ceremony` inserts the ceremony record.
Also written back to a trustee by `set_public_key_impl` when either:
- the trustee's name is not found in the DB, or
- the trustee has no `public_key` in the DB, or
- the trustee has a `public_key` but no matching `PublicKey` / `PublicKeySigned` message yet
  exists on the board.

This means `WAITING` is not a one-way initial state — `set_public_key_impl` re-evaluates and
re-writes every trustee's status each time the beat cycle dispatches the `set_public_key` task.

### KEY_GENERATED

Set by `set_public_key_impl` (the `set_public_key` Celery task, dispatched by `process_board`
when `execution_status == IN_PROGRESS` and `status.public_key` is not yet set).  The function
`get_trustee_status()` looks up the trustee's `public_key` from the DB, then scans board
messages for a `PublicKey` or `PublicKeySigned` message whose sender matches that key.  If
found, the trustee advances to `KEY_GENERATED`.

**BBT impact**: `get_trustee_status` currently reads the global `public_key` column via
`get_trustees_by_name`.  For BBTs this must resolve the per-event key (filtered by
`election_event_id`) so the board-message match is against the correct key.

### KEY_RETRIEVED

Set by the Harvest `/get-private-key` endpoint when the trustee downloads their
`TrusteeShareData` from the board.  `TrusteeShareData` bundles:
- the `Channel` message — which contains `encrypted_channel_sk`, the channel private key
  encrypted with the trustee's AES-256 `encryption_key`
- all `Shares` messages posted to the board during DKG

The endpoint looks up the trustee's `public_key` from the DB, uses it to fetch the `Channel`
message on the board addressed to that key, collects all `Shares`, serialises the result, and
advances the trustee to `KEY_RETRIEVED`.

**BBT impact**: because the B4 protocol is unchanged, BBTs post the same `Channel` and
`Shares` messages during DKG, so `TrusteeShareData` is present on the board for BBT trustees
too.  `/get-private-key` is therefore technically applicable.

The Download/Check steps are nonetheless being **repurposed** for BBTs to back up the
**identity keys** (`signing_key_sk`, `signing_key_pk`, `encryption_key`) held in the
browser's in-memory ceremony session.  The reason is that these identity keys are the root
credential: without `signing_key_sk` the trustee cannot authenticate on the board at all,
and without `encryption_key` the `encrypted_channel_sk` inside the on-board `Channel`
message cannot be decrypted — making the `TrusteeShareData` inaccessible regardless.
Backing up the identity keys is therefore sufficient and more portable than downloading the
raw board data.

For BBTs, advancing to `KEY_RETRIEVED` is the responsibility of a **new** Hasura action,
`confirm_key_backup` (see [§10](#10-component-changes)).  The browser computes an
`encryption_key_commitment` over the canonical ceremony data using the in-memory
`encryption_key`, exports the TOML for the trustee to save locally, and POSTs **only** the
commitment to Harvest.  Harvest writes the commitment onto the trustee's row and advances
the status.  The TOML and `encryption_key` itself never leave the browser.  The legacy
`/get-private-key` endpoint is not used by BBTs.

### KEY_CHECKED

Set by the Harvest `/check-private-key` endpoint when the trustee re-uploads and the uploaded
key matches the one stored on the board.  When **all** trustees in the ceremony reach
`KEY_CHECKED`, the ceremony `execution_status` transitions to `SUCCESS`.

**BBT impact**: the check for BBTs happens in two layers and the board is never involved.
(a) **Browser-local sanity check** (always run, also reused outside the formal Check Step):
parse the uploaded TOML, verify that `signing_key_sk` derives the expected `signing_key_pk`,
recompute the `encryption_key_commitment`, and compare locally against the in-memory
ceremony session.  This gives the trustee immediate feedback.
(b) **Signed attestation to Harvest** (run only at the formal Check Step, to advance the
status): the browser requests a short-lived nonce from Harvest, builds a canonical
verification payload binding `(tenant_id, election_event_id, keys_ceremony_id, trustee_id,
signing_key_pk, encryption_key_commitment, nonce)`, signs it with `signing_key_sk`, and
POSTs only `{ payload, signature, nonce, commitment }`.  Harvest verifies the JWT trustee
binding, the nonce, the signature under the registered `signing_key_pk`, and the commitment
match against the value previously recorded by `confirm_key_backup`.  No TOML, no
`signing_key_sk`, and no `encryption_key` ever reach the server.  The legacy
`/check-private-key` endpoint and its board-key comparison logic are not used by BBTs.

### Summary table

| Status | Who sets it | Trigger | BBT difference |
|---|---|---|---|
| `WAITING` | `create_keys_ceremony` / `set_public_key_impl` | Ceremony creation or no board message yet | None for initial set; board-message match needs per-event key |
| `KEY_GENERATED` | `set_public_key_impl` | `PublicKey`/`PublicKeySigned` message found on board for this trustee | Must match per-event `public_key`, not global one |
| `KEY_RETRIEVED` | `/get-private-key` (Harvest) — server-based only | Trustee downloads `TrusteeShareData` (Channel + Shares) from board | New Hasura action `confirm_key_backup` writes `encryption_key_commitment` and advances status; TOML stays in the browser, no board read |
| `KEY_CHECKED` | `/check-private-key` (Harvest) — server-based only | Re-uploaded key matches board copy | New Hasura actions `issue_key_check_nonce` + `submit_key_check_attestation`: local parse + sk→pk verification + commitment recompute, then signed attestation (nonce-bound) to Harvest; no TOML / secret material sent; no board read |

---

## 6. Cancellation Window

### Keys ceremony — not currently implemented

`KeysCeremonyExecutionStatus::CANCELLED` is defined in the enum in `sequent-core` but is
**dead code**: no Harvest endpoint, no Windmill task, and no service function ever writes it.
Keys ceremony cancellation does not exist today.

A future implementation would require at minimum:

- A new Harvest endpoint (e.g. `POST /cancel-keys-ceremony`) that validates that:
  - The current`execution_status` is not already `CANCELLED`.
  - The election event has not been started yet.
  - Any of event's elections haven´t been started either.
  Then writes `CANCELLED`.
  So it should be allowed to cancel on `SUCCESS` but only if the voting period has not been started.
- Handling of the B4 board if DKG was already `IN_PROGRESS`: the board already holds
  `Channel`, `Shares`, and possibly `PublicKey` messages, which cannot be deleted from an
  append-only board.  **Decision:** leave the messages orphaned.  The ceremony only advances
  when the browser trustees act on it, and to do so they must input their keys via the admin
  portal — which they cannot do once the ceremony is cancelled.  The admin creates a fresh
  ceremony on a new board.
- Handling of BBT key rows in `sequent_backend_trustee` on cancellation.  **Decision:** leave
  the rows in place.  A new keys ceremony will produce different/regenerated trustee public
  keys regardless, so stale rows do not interfere.  This implies the trustee key identity is
  not just `(trustee_id, election_event_id)` but `(trustee_id, election_event_id,
  keys_ceremony_id)`.

See [Keys Ceremony State Machine](./key_ceremony_state_machine.md) for the full transition
table, including the cancellation arms.

### Election ↔ ceremony reference handling

The election row points to its current keys ceremony via `election_event.keys_ceremony_id`.
On `/cancel-keys-ceremony`, in the same transaction as the status transition:

1. `try_transition(current → CANCELLED)` on the `keys_ceremony` row.
2. For every election event whose `keys_ceremony_id` points to the cancelled ceremony, set
   `election_event.keys_ceremony_id = NULL`.

After cancellation the election is in an **unassigned** state: no current ceremony, but the
cancelled-ceremony row remains in `keys_ceremony` for audit (and its BBT trustee rows remain
in `sequent_backend_trustee` under that `keys_ceremony_id`, per the decision above).  The
admin can now create a fresh ceremony — or duplicate from a previous one — to re-assign the
election.

### Duplicate / recreate option

An election with no current `keys_ceremony_id` can be given a new ceremony in one of two
ways:

- **Fresh create** — admin picks trustees and threshold from scratch (today's
  `create-keys-ceremony` path).
- **Duplicate from previous** — admin picks any prior ceremony for the same election event
  (typically the just-cancelled one); the new ceremony inherits its trustee set, threshold,
  and naming.  The new ceremony gets a fresh `keys_ceremony_id`; BBT trustees regenerate
  keys against that new id; no `public_key` or `encryption_key_commitment` is copied over.

UX rule: the duplicate option is offered only when the source election has
`keys_ceremony_id = NULL`.  If a ceremony is already assigned (active or successful), the
admin must cancel first.

The in-memory-only key model (see [§7 Flow](#7-flow), [§9 Key Loss and Recovery](#9-key-loss-and-recovery))
makes this cancel+recreate path a first-class workflow rather than an edge case: any time a
trustee loses their in-memory keys before downloading a backup, the gate-firing edge case
documented in [§3](#3-ceremony-creation-gate) becomes unrecoverable and the only fix is to
cancel and recreate.

### Tally ceremony — supported

Tally ceremony cancellation is fully implemented.  `POST /update-tally-ceremony` (Harvest)
accepts any `TallyExecutionStatus` value including `CANCELLED` and enforces the valid
transition table via `update_tally_ceremony` in `windmill/src/services/ceremonies/tally_ceremony.rs`:

| From | To |
|---|---|
| `STARTED` | `CANCELLED` |
| `CONNECTED` | `IN_PROGRESS` or `CANCELLED` |
| `IN_PROGRESS` | `CANCELLED` |
| `SUCCESS` | — (terminal) |
| `CANCELLED` | — (terminal) |

This is out of scope for this document but relevant background for BBT design: BBT trustees
will also participate in tally ceremonies, and the same identity keys must be re-loaded into
the browser's in-memory ceremony session from the downloaded backup at the start of the
tally session.

---

## 7. Flow

```
Admin creates ceremony (selects trustees)
  → create_keys_ceremony inserts record with execution_status: AWAITING_TRUSTEE_KEYS
  → sets election_event.keys_ceremony_id to point at the new ceremony
                    ↓
BBT trustee opens the election event's keys ceremony in admin portal
  └─ HeadlessTrusteeProvider mounts; checks the in-memory session registry
  └─ If no in-memory keys for this ceremony AND no public_key registered yet:
       WASM generates trustee identity keys (in-memory only — never written to
         localStorage or sessionStorage):
         - Ed25519 signing keypair  (StrandSignatureSk::generate())
         - AES-256 symmetric key    (symm::gen_key())
       Browser installs a beforeunload guard: warn the trustee if they try to
         close/navigate before the Check Step completes.
  └─ If no in-memory keys but public_key is already registered (cold start
       after tab close): prompt the trustee to upload their TOML backup to
       restore the in-memory keys.  If they cannot restore, the only recovery
       is admin cancel + recreate (see §3 gate edge case, §6 cancellation).
  └─ signing_key_pk → POST Harvest /register-trustee-key  (only after ceremony exists)
                       UPSERT (trustee_id, election_event_id, keys_ceremony_id)
                         SET public_key, encryption_key_commitment = NULL
                    ↓
beat service (review_boards → process_board) polls each ceremony in AWAITING_TRUSTEE_KEYS:
  checks whether every selected BBT trustee has a public_key for this
    (trustee_id, election_event_id, keys_ceremony_id) in DB
  if any key is still missing → stays in AWAITING_TRUSTEE_KEYS, retried on next beat
  once all keys are present it launches Windmill create_keys
  reads trustee.public_key WHERE (trustee_id, election_event_id, keys_ceremony_id) (BBT)
                            or WHERE election_event_id IS NULL AND
                                     keys_ceremony_id IS NULL    (server-based)
  → posts Configuration protocol message to B4 (browser is not involved in this post)
  → execution_status: IN_PROGRESS
                    ↓
HeadlessTrusteeProvider passes signing_key_sk + encryption_key from the in-memory
    ceremony session into WasmSession
  → WasmSession runs DKG protocol automatically in background, signing every B4
      message locally with signing_key_sk (Harvest is not in the B4 path):
       Channel → ChannelsAllSigned → Shares → PublicKey → PublicKeySigned
                    ↓
allTrusteesGenerated: all trustees have a PublicKey or PublicKeySigned message on the board
  → Windmill set_public_key matches board messages against trustee.public_key from DB
  → each trustee status: KEY_GENERATED
                    ↓
[Download Step — backup file + commitment]
  Download is enabled only after all selected trustees have reached KEY_GENERATED
  Browser exports identity keys to a downloadable TOML (file leaves the browser
    only when the trustee saves it locally):
    { signing_key_sk, signing_key_pk, encryption_key }
  Browser computes encryption_key_commitment over canonical ceremony data:
    HMAC(encryption_key,
         "BBT/key-check/v1" ‖ tenant_id ‖ election_event_id ‖
                              keys_ceremony_id ‖ trustee_id)
  Browser POSTs only the commitment → Harvest confirm_key_backup (Hasura action)
    → server writes encryption_key_commitment on the trustee row
    → if a commitment was already recorded:
        - matching value → accept silently (no-op)
        - differing value → reject (COMMITMENT_MISMATCH)
    → trustee status advances to KEY_RETRIEVED
  Navigate-away guard relaxes once a backup exists (recovery is now possible)

[Check Step — signed attestation, no secret material sent]
  Trustee re-uploads backup TOML
  Browser parses TOML locally:
    - verify signing_key_pk derives from signing_key_sk
    - recompute encryption_key_commitment over the same canonical ceremony data
  Browser requests a short-lived nonce from Harvest (issue_key_check_nonce)
  Browser builds canonical verification payload:
    { tenant_id, election_event_id, keys_ceremony_id, trustee_id,
      signing_key_pk, encryption_key_commitment, nonce }
  Browser signs the payload with signing_key_sk
  Browser POSTs { payload, signature, nonce, commitment } → Harvest
    (submit_key_check_attestation).  Never sends signing_key_sk, encryption_key,
    or the TOML.
  Harvest verifies:
    - JWT trustee_id matches payload.trustee_id
    - nonce was issued for this trustee, is unexpired, and unused
    - signature verifies under the registered signing_key_pk
    - commitment matches the value recorded by confirm_key_backup
  On success: mark nonce consumed; advance trustee status → KEY_CHECKED
  All trustees KEY_CHECKED → ceremony status: SUCCESS
```

### Navigate-away guard

Because identity keys live only in the browser's in-memory ceremony session, the page must
make it visible to the trustee that closing the tab or navigating away before completing
the Download + Check steps will destroy those keys.

While in-memory keys exist but the Check Step has not advanced the trustee to
`KEY_CHECKED`, the page installs a `beforeunload` handler that prompts the trustee with a
warning along the lines of:

> *"You haven't finished saving your identity keys.  If you leave now, you may not be able
> to recover your keys and the ceremony may need to be cancelled and recreated."*

The guard is uninstalled once the trustee reaches `KEY_CHECKED`.  The warning is
best-effort: browsers can suppress the custom message, and tab crashes / OS kills cannot be
guarded against at all.  The only durable recovery is the downloaded backup file, which is
why the guard stays active until the backup-plus-attestation round-trip is complete.

### What the verify step triggers

When the trustee uploads the backup file in the Check Step, the browser runs the local
checks (parse TOML, verify `signing_key_sk` derives `signing_key_pk`, recompute
`encryption_key_commitment`) and gives the trustee immediate match/mismatch feedback.

For the status advance, the browser then runs the **signed attestation** flow described in
the diagram above: request a Harvest-issued nonce, sign a canonical payload with
`signing_key_sk`, POST `{ payload, signature, nonce, commitment }` to Harvest.  Harvest
**never receives the TOML, `signing_key_sk`, or `encryption_key`** — only proof of
possession.  If Harvest validates the JWT binding, the nonce, the signature under the
registered `signing_key_pk`, and the commitment match, the trustee advances to
`KEY_CHECKED`.  When all trustees reach `KEY_CHECKED` the ceremony status advances to
`SUCCESS`.

### Backup validation is reusable, not a one-shot step

The Download/Check pair is drawn above as a linear progression, but the underlying *local*
validation — parse TOML, verify `sk` derives `pk`, recompute the commitment, compare
against the in-memory keypair — is available any time after `KEY_GENERATED`.  The admin
portal should expose a "check my backup" affordance the trustee can use during
`KEY_RETRIEVED` (to confirm the file is valid before signing off), after `KEY_CHECKED`
(re-verify days/weeks later), and during tally-ceremony preparation (to re-load keys into
memory on a fresh device, or just to confirm the file still works before tally begins).

The local logic is identical in every case.  Only the formal Check Step at
`KEY_RETRIEVED → KEY_CHECKED` invokes the **server-side attestation flow** (nonce + signed
payload) and advances trustee status.  Outside that window the check is purely local
feedback to the trustee; no nonce is issued and no Harvest call is made.  Same UI, same
WASM helpers, reused across the key ceremony and tally preparation with no protocol or DB
changes.

> **Note on the Download step:** the trustee downloads a backup of their **identity keys** —
> `{ signing_key_sk, signing_key_pk, encryption_key }` — the browser equivalent of
> `trustee1.toml`.  This is **not** the DKG private key fragment (`TrusteeShareData`).
> There is no `/get-private-key` download in the BBT flow.  The identity key backup is the
> only file the trustee needs to save, and it must also be uploaded during the tally
> ceremony to re-load the trustee's identity keys into the browser's in-memory session.

---

## 8. Security: In-Memory Key Handling

### Storage rule

BBT identity keys (`signing_key_sk`, `signing_key_pk`, `encryption_key`) **must never be
written to `localStorage` or `sessionStorage`**.  They live only in the browser process's
in-memory ceremony session (the existing JS module-level session registry described in
[§4](#4-trustee-online-presence)) and inside the WASM linear memory.  The trustee's
downloaded TOML backup is the only durable copy of the secret material and exists outside
the browser.

This design choice eliminates the at-rest XSS surface by construction: there is nothing for
a passive read of web storage to find.

### Residual risk: active XSS during a live session

An attacker who can execute JavaScript on the admin-portal origin **while the trustee is on
the ceremony page** can still read the in-memory keys.  This is fundamentally harder than
reading `localStorage` (the attack must land in the right window) but it is not zero.
Residual mitigations to keep in place:

- A strict Content-Security-Policy header on admin-portal — the portal currently has none,
  and adding one is a precondition for treating the in-memory boundary as load-bearing.
- Scrutiny of any rich-text or user-content surface (TinyMCE is the existing one) for
  reflected/stored XSS; ban any path that lets user-controlled HTML reach the ceremony
  route.
- No third-party scripts on the ceremony route — every script source must be in the CSP
  allowlist.
- Treat the ceremony page as a high-sensitivity context: no embedded iframes that aren't
  same-origin, no `dangerouslySetInnerHTML` from untrusted strings, etc.

### Harvest holds no BBT secret material

Harvest is the server-side custodian of ceremony state, but it is **never** a custodian of
BBT signing material.  In particular:

- Harvest never receives `signing_key_sk`, `encryption_key`, or the TOML.  The verify flow
  is a signed attestation (see [§7 Flow](#7-flow)); the server sees only `signing_key_pk`
  (already public) and the one-way `encryption_key_commitment`.
- Harvest never signs B4 messages on behalf of a BBT trustee.  Browser sessions sign every
  B4 message locally with their in-memory `signing_key_sk` and POST directly to B4 (path 4
  in [§1](#1-service-communication-paths)); see the B4 signing invariant in §1.
- A compromise of Harvest must not yield BBT impersonation.  Any future feature that
  appears to need a signature for a BBT trustee must round-trip through that trustee's
  browser, not synthesize one server-side.

### Threat-model summary for the verify flow

Authorization for the Check Step comes from three independent checks; **none of them
involve the nonce**:

- Possession of `signing_key_sk` alone is insufficient to forge a Check Step attestation:
  the attacker would also need `encryption_key` to reproduce the commitment recorded by
  `confirm_key_backup`.
- Possession of `encryption_key` alone is insufficient: the attacker would also need
  `signing_key_sk` to sign the canonical verification payload under the registered
  `signing_key_pk`.
- Harvest's per-trustee JWT binding (`payload.trustee_id == JWT trustee`) prevents one
  trustee from attesting on behalf of another.

The nonce serves a different purpose: it is **not** an authorization secret and is freely
obtainable by anyone with a valid JWT.  Its only job is to make a signed attestation
**non-replayable**.  The signed payload `{ tenant_id, election_event_id, keys_ceremony_id,
trustee_id, signing_key_pk, encryption_key_commitment }` is otherwise constant for the
life of the trustee's row in the ceremony — without a freshness value, a single legitimate
`{ payload, signature }` artifact would be a permanently valid Check Step token, replayable
forever by anyone who observed the request (logs, transient TLS-terminator buffers, memory
snapshots).  Including a server-issued, single-use, time-bounded nonce in the signed
payload makes each signature a one-shot proof:

- Captured attestations are useless after the legitimate submission consumes their nonce
  (or after 5 min, whichever comes first).
- Attestations cannot be pre-signed and stashed: the payload cannot be constructed until
  Harvest has issued the challenge.
- Cross-binding is blocked: a nonce issued for trustee A / ceremony X cannot be lifted into
  an attestation for trustee B or ceremony Y (the row lookup keys on
  `(trustee_id, keys_ceremony_id, nonce)`).
- Even idempotency confusion is blocked: `consumed_at` is set in the same transaction as
  the `KEY_CHECKED` advance, so one signed payload cannot be processed twice.

Canonicalization (for both the commitment input and the signed payload) must be exact
across browser and server.  Reuse a single canonicalization implementation, version it
(`canonicalization_version` field on the attestation), and treat drift as the most likely
correctness foot-gun.

---

## 9. Key Loss and Recovery

BBT identity keys live **only in the browser's in-memory ceremony session** — never in
`localStorage` or `sessionStorage`.  They survive React mount/unmount cycles and in-app
navigation (the JS module-level session registry described in
[§4](#4-trustee-online-presence) holds them), but they are **destroyed on tab close, hard
refresh, browser restart, or tab crash**.  The trustee's downloaded TOML backup is the only
durable copy.

Loss is therefore the *default* expectation any time a trustee leaves the ceremony page
without completing the Download Step.  The navigate-away guard (see [§7](#7-flow)) is a
best-effort warning, not a guarantee.

### Recovery depends on what has already been posted to the board

**Before `Configuration` is posted** (ceremony still in `AWAITING_TRUSTEE_KEYS`):
the trustee can regenerate a fresh keypair on next visit, re-register the new `public_key`
in the DB (overwriting the previous entry; the commitment column stays `NULL`), and the
beat service will proceed normally.  The ceremony has not used the old key for anything.

**After `Configuration` is posted** (ceremony in `IN_PROGRESS` or later):
the old `public_key` is embedded in the on-board `Configuration` message and cannot be
substituted.  Two sub-cases:

- *Trustee has the TOML backup* (Download Step completed before loss): the trustee uploads
  the backup on the ceremony page; the browser parses it, restores the three values into
  the in-memory ceremony session, and `HeadlessTrusteeProvider` resumes with the original
  keys.  No DB write is required — the row already matches what the backup contains.
- *Trustee does not have the TOML backup*: the ceremony is unrecoverable for that trustee.
  See "Unrecoverable loss" below.

### Cold-start UX on the ceremony page

`HeadlessTrusteeProvider` already checks the in-memory session registry on mount.  When the
registry is empty (cold start — fresh tab or after navigation back), it must additionally:

- If no `public_key` is registered for `(trustee_id, election_event_id, keys_ceremony_id)`:
  treat this as first visit; generate fresh keys in memory and run the registration flow.
- If a `public_key` is already registered: this is a return-after-loss.  Show a file picker
  affordance to upload the TOML backup.  On upload, parse the file, load the three values
  into the in-memory ceremony session, and only then construct the `WasmSession`.  If the
  trustee cannot produce the backup, surface the cancel-and-recreate path described below.

No changes to Windmill, Harvest, or B4 are needed for the *successful* upload-restore path.

### Unrecoverable loss: cancel and recreate

If a trustee **cannot download** their identity backup (e.g. the Download Step never
completed) or **loses** their in-memory keys *and* their TOML backup with no way to restore
the latter, the trustee can no longer participate in the ceremony — and because their
`public_key` is embedded in the on-board `Configuration` message, no fresh keypair can be
substituted.  This is the same failure mode as the gate-firing edge case documented in
[§3 Ceremony Creation Gate](#3-ceremony-creation-gate); the in-memory-only key model makes
it a real-world risk rather than an edge case.

The recovery path is to **cancel the ceremony and create a new one**, provided no election
in the event has opened its voting period yet.  The state machine permits
`AWAITING_TRUSTEE_KEYS`, `IN_PROGRESS`, and `SUCCESS` all to transition to `CANCELLED`
(see [Keys Ceremony State Machine](./key_ceremony_state_machine.md); the `SUCCESS` arm is
additionally gated by the voting-period check at the cancel endpoint, see
[§6 Cancellation Window](#6-cancellation-window)).

After cancellation:
- `election_event.keys_ceremony_id` is cleared atomically with the status transition (see
  [§6](#6-cancellation-window)) so the election can be re-assigned.
- The orphan `Channel`, `Shares`, and `PublicKey` messages on the old B4 board remain in
  place (append-only) but are simply ignored — the admin creates the replacement ceremony
  on a fresh board.
- The old BBT rows in `sequent_backend_trustee` keyed by the cancelled `keys_ceremony_id`
  remain in place but no longer match the new ceremony's `keys_ceremony_id`, so they do
  not interfere with the replacement.
- The admin uses either fresh create or duplicate-from-previous (see [§6 Duplicate /
  recreate option](#6-cancellation-window)).  The trustees regenerate keys in their
  browsers under the new `keys_ceremony_id` and the ceremony proceeds normally from
  `AWAITING_TRUSTEE_KEYS`.

Once any election in the event has opened its voting period, this path is no longer
available — the ceremony's public key is already in production use and cancellation is
blocked by the endpoint.

---

## 10. Component Changes

### 1 — Braid-wasm: new `generate_trustee_keys()` export

Add a WASM-exported function in `braid/src/wasm/mod.rs` that mirrors what
`gen_trustee_config.rs` does for server-based trustees:

```
pub fn generate_trustee_keys() -> JsValue
  returns {
    signing_key_sk:  string,   // StrandSignatureSk::generate() → base64-DER PKCS#8 Ed25519 private key
    signing_key_pk:  string,   // StrandSignaturePk::from_sk()  → base64-DER Ed25519 public key
    encryption_key:  string,   // symm::gen_key()               → base64 AES-256 symmetric key
  }
```

This produces the same three values found in `trustee1.toml`.  All underlying functions
(`StrandSignatureSk::generate`, `symm::gen_key`) are already present in braid and used in
`gen_trustee_config.rs` — no new crypto dependency.

### 2 — DB schema: extend `sequent_backend_trustee` and add a verification-nonce table

Extend `sequent_backend_trustee` with `election_event_id`, `keys_ceremony_id`, and
`encryption_key_commitment`.  All three are nullable but, when set, the first two must be
foreign keys to their respective tables:

| Column | Type | Meaning |
|---|---|---|
| `election_event_id` | `uuid \| null` (FK → `election_event.id`) | `null` for server-based trustees; event UUID for BBT |
| `keys_ceremony_id` | `uuid \| null` (FK → `keys_ceremony.id`) | `null` for server-based trustees; ceremony UUID for BBT |
| `public_key` | `text \| null` | Ed25519 public key scoped to this `(trustee, event, ceremony)` tuple |
| `encryption_key_commitment` | `text \| null` | HMAC commitment to the trustee's `encryption_key`; written by `confirm_key_backup` at Download Step.  NULL until the trustee has downloaded their TOML backup. |

Server-based trustees keep all four BBT-only columns NULL — no behaviour change.  BBT
trustees upsert `(trustee_id, election_event_id, keys_ceremony_id) → public_key` on every
session start; `encryption_key_commitment` is written separately at Download Step time.

**New table — `sequent_backend_trustee_verification_nonce`** (challenge state, not a durable
record):

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` | primary key |
| `trustee_id` | `uuid` | FK |
| `keys_ceremony_id` | `uuid` | FK |
| `nonce` | `text` | random, ≥256 bits, base64 |
| `issued_at` | `timestamptz` | |
| `expires_at` | `timestamptz` | e.g. `issued_at + 5 minutes` |
| `consumed_at` | `timestamptz \| null` | non-null once an attestation has spent it |

Unique index on `(trustee_id, keys_ceremony_id, nonce)`.  Rows are reaped past
`expires_at + grace` by a Windmill task (`reap_expired_check_nonces`); no audit trail is
kept here — successful Check Step is recorded via `TrusteeStatus = KEY_CHECKED`.

### 3 — `HeadlessTrusteeProvider`: cold-start, generate-or-restore, beforeunload guard

Replace the localStorage-backed initialize effect with this in-memory flow.  Before
constructing `WasmSession`:

1. Look up the in-memory ceremony session in the JS module-level session registry (the same
   registry described in [§4 Session lifetime across navigation](#4-trustee-online-presence)).
2. If present (warm start — same tab, mid-ceremony navigation): reuse and pass the existing
   `signing_key_sk` / `signing_key_pk` / `encryption_key` into `WasmSession`.
3. If absent, query Hasura for an existing `public_key` registered for
   `(trustee_id, election_event_id, keys_ceremony_id)`:
   - **No `public_key`** (first visit): call `generate_trustee_keys()` via WASM, hold the
     three values **in memory only** (never write to `localStorage` or `sessionStorage`),
     POST `signing_key_pk` to Harvest `/register-trustee-key` with `election_event_id` and
     `keys_ceremony_id`.
   - **`public_key` already registered** (cold start after tab close): render an
     upload-backup affordance; parse the uploaded TOML, load `signing_key_sk` /
     `signing_key_pk` / `encryption_key` into the in-memory ceremony session.  Do **not**
     re-POST `/register-trustee-key` — the DB row already matches.  If the trustee cannot
     produce a backup, surface the cancel-and-recreate path (see [§9](#9-key-loss-and-recovery)).
4. Install a `beforeunload` handler that warns the trustee if they try to close or navigate
   away.  Uninstall it once the trustee reaches `KEY_CHECKED`.
5. Pass `signing_key_sk`, `signing_key_pk`, and `encryption_key` from the in-memory ceremony
   session into the `WasmSession` config.

### 4 — Harvest: `POST /register-trustee-key`

```
Input:  { public_key: string, election_event_id: string, keys_ceremony_id: string }
Auth:   existing Keycloak JWT  — identifies which trustee is calling
Action: UPSERT sequent_backend_trustee
          SET public_key = $public_key,
              encryption_key_commitment = NULL   -- reset on fresh key generation
          WHERE trustee_id = <from JWT>
            AND election_event_id = $election_event_id
            AND keys_ceremony_id  = $keys_ceremony_id
```

The private key and `encryption_key` never reach the server.  Server-based trustees never
call this endpoint.  The endpoint is idempotent on `public_key`; resetting
`encryption_key_commitment` on re-registration is intentional so that a regenerated
keypair starts from a clean slate.

### 5 — Harvest: new Hasura action `confirm_key_backup`

```
Input:  { keys_ceremony_id: string, encryption_key_commitment: string }
Auth:   existing Keycloak JWT  — identifies which trustee is calling
Action (one transaction):
  1. Load the row for (trustee_id from JWT, election_event_id implied by ceremony,
     keys_ceremony_id).  Reject NO_PUBLIC_KEY_REGISTERED if public_key is NULL.
  2. If the row's encryption_key_commitment is already set:
       - matches input → accept silently (no-op, return ok)
       - differs from input → reject COMMITMENT_MISMATCH
  3. Otherwise: write encryption_key_commitment.
  4. Advance TrusteeStatus → KEY_RETRIEVED via try_transition.
Errors: NO_PUBLIC_KEY_REGISTERED, COMMITMENT_MISMATCH, INVALID_STATE.
```

Server-based trustees never call this endpoint; they use the existing `/get-private-key`
path.

### 6 — Harvest: new Hasura action `issue_key_check_nonce`

```
Input:  { keys_ceremony_id: string }
Auth:   existing Keycloak JWT  — identifies which trustee is calling
Action:
  1. Generate a random nonce (≥256 bits, base64).
  2. Insert a row into sequent_backend_trustee_verification_nonce with
       expires_at = now() + 5 minutes.
  3. Optionally enforce a per-(trustee, ceremony) cap on un-consumed nonces; evict the
       oldest or reject past the cap.
Returns: { nonce, expires_at }
```

### 7 — Harvest: new Hasura action `submit_key_check_attestation`

```
Input:
  {
    payload: {
      tenant_id, election_event_id, keys_ceremony_id, trustee_id,
      signing_key_pk, encryption_key_commitment, nonce
    },
    signature: string (base64),
    canonicalization_version: int
  }
Auth: existing Keycloak JWT — identifies the calling trustee.
Action (one transaction):
  1. Look up the nonce row by (payload.trustee_id, payload.keys_ceremony_id, payload.nonce).
     Reject NONCE_NOT_FOUND / NONCE_EXPIRED / NONCE_ALREADY_USED accordingly.
  2. Reject TRUSTEE_MISMATCH if payload.trustee_id ≠ JWT trustee.
  3. Reject CEREMONY_MISMATCH if any of payload.tenant_id / .election_event_id /
     .keys_ceremony_id disagrees with the ceremony record.
  4. Load the registered row for (trustee_id, election_event_id, keys_ceremony_id).
     Reject KEY_MISMATCH if payload.signing_key_pk ≠ row.public_key.
     Reject COMMITMENT_MISMATCH if payload.encryption_key_commitment ≠
        row.encryption_key_commitment.
  5. Re-serialize payload using canonicalization_version; verify signature under
     payload.signing_key_pk.  Reject SIGNATURE_INVALID on failure.
  6. Mark the nonce consumed (set consumed_at).
  7. Advance TrusteeStatus → KEY_CHECKED via try_transition.
Returns: { status: "ok" } | descriptive error code.
```

Harvest never sees `signing_key_sk`, `encryption_key`, or the TOML file.  Server-based
trustees never call this endpoint.

### 8 — Harvest: new Hasura action `cancel_keys_ceremony`

```
Input:  { keys_ceremony_id: string }
Auth:   admin role
Action (one transaction):
  1. Load the keys_ceremony row.  Reject CEREMONY_NOT_FOUND if missing.
  2. For each election in the event:
       - reject VOTING_PERIOD_STARTED if its voting period has begun.
  3. try_transition(current → CANCELLED) on the keys_ceremony row.
  4. For every election_event whose keys_ceremony_id points at this ceremony,
       SET keys_ceremony_id = NULL.
Returns: { status: "ok" } | error code.
```

See [§6 Cancellation Window](#6-cancellation-window) for the rationale and the resulting
"unassigned" election state.

### 9 — Harvest: update `create-keys-ceremony` with `duplicate_from`

Add an optional input field `duplicate_from: keys_ceremony_id?`.

- If present: load the source ceremony; copy trustee set, threshold, and naming convention
  onto the new ceremony.  Reject DUPLICATE_SOURCE_TRUSTEE_MISSING if any source trustee no
  longer exists.  Reject ELECTION_ALREADY_ASSIGNED if the target election already has
  `keys_ceremony_id` set (admin must cancel first).  Do **not** copy `public_key` or
  `encryption_key_commitment` — BBT trustees must regenerate against the new
  `keys_ceremony_id`.
- If absent: behaves as today (fresh create).

In both cases, on success set `election_event.keys_ceremony_id` to the new ceremony id.

### 10 — Admin portal: ceremony-assignment states on the election page

The election-event ceremony page must render different controls depending on
`election_event.keys_ceremony_id`:

- **`NULL` (no ceremony assigned)**: show "Create ceremony" and, if any prior ceremony
  exists for this election event, "Duplicate from previous".
- **Assigned, voting period not started**: show ceremony status + "Cancel ceremony" button
  (calls `cancel_keys_ceremony`).
- **Assigned, voting period started or election in progress**: show ceremony status only;
  no cancel/recreate controls (the state machine + voting-period gate would reject
  anyway).

### 11 — Windmill `get_trustees_by_id` / `get_trustees_by_name`: filter by `(election_event_id, keys_ceremony_id)`

Update both DB queries to prefer the row matching the ceremony's `election_event_id` **and**
`keys_ceremony_id`, falling back to `(NULL, NULL)` for server-based trustees:

```sql
SELECT public_key FROM sequent_backend_trustee
WHERE id = $trustee_id
  AND ( (election_event_id = $event_id AND keys_ceremony_id = $ceremony_id)
        OR (election_event_id IS NULL AND keys_ceremony_id IS NULL) )
ORDER BY election_event_id NULLS LAST
LIMIT 1
```

No change to `create_keys`, `set_public_key`, or `insert_ballots` logic — they already use
whatever `public_key` the query returns.

### 12 — Windmill: nonce-reap task

Add a periodic Windmill task `reap_expired_check_nonces` that deletes rows from
`sequent_backend_trustee_verification_nonce` where
`expires_at < now() - <grace>` OR `consumed_at IS NOT NULL`.  Frequency: every 5–15
minutes; the table holds only ephemeral challenge state.

---

## 11. `trustee.public_key` Column

The column is **retained and extended** with the companion `election_event_id`,
`keys_ceremony_id`, and `encryption_key_commitment` columns (see [§10.2](#10-component-changes)).
It remains the single source of truth for the public key in this approach.  The new
companion `encryption_key_commitment` column is the single source of truth for the trustee's
encryption-key commitment, written at Download Step time by `confirm_key_backup` and read
at Check Step time by `submit_key_check_attestation`.
