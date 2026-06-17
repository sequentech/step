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

Braid-wasm is embedded in the admin-portal browser bundle and handles only the DKG protocol
with B4 (path 4).  All Hasura and Harvest calls are made by admin-portal TypeScript/React
code — never by braid-wasm.  Rows marked **BBT** are new or changed by this proposal; all
others are unchanged from the current architecture.

| # | Origin | Destination | Protocol | What |
|---|--------|-------------|----------|------|
| 1–6 | Admin-portal (TypeScript) | Harvest (Hasura action) | HTTP POST | `create-keys-ceremony`, `list-keys-ceremonies`, `get-private-key`, `check-private-key` (existing)<hr/>`register_trustee_key` **BBT new** — registers a trustee's signing public key (any trustee type) into `trustee_ceremony_key` for `(trustee, election_event_id, keys_ceremony_id)` (no secret material)<hr/>`confirm_key_backup` **BBT new** — trustee self-reports backup downloaded; advances to `KEY_RETRIEVED` (JWT-authenticated, trustee-confirmed)<hr/>`confirm_key_check` **BBT new** — trustee self-reports local check passed; advances to `KEY_CHECKED` (JWT-authenticated, trustee-confirmed)<hr/>`cancel_keys_ceremony` **BBT new** — atomic `CANCELLED` transition + clears `election.keys_ceremony_id` |
| 7 | Admin-portal (TypeScript) | Hasura | GraphQL/HTTP | Read-only queries: trustee config (`GET_TRUSTEE_CONFIG`), election events, ceremony `execution_status`, all entity reads |
| 8 | Admin-portal (braid-wasm) | B4 | HTTP | `GET /boards`, `GET /messages`, `POST /messages` — full DKG protocol (Channel, Shares, PublicKey…), every message signed locally in the browser |
| 9 | Harvest / Windmill | PostgreSQL (Hasura DB) | SQL direct | Read/write `keys_ceremony`, `trustee`, `election_event` tables.  **BBT change:** new `trustee_ceremony_key` table holds one `public_key` row per `(trustee, event, ceremony)`; `get_trustees_by_id` / `get_trustees_by_name` resolve `public_key` by joining it on the requested scope |
| 10 | Windmill (Celery) | B4 PostgreSQL | SQL direct (`PgsqlB3Client`) | INSERT `Configuration` message (`add_config_to_board`), SELECT `PublicKey` message (`get_board_public_key`) |
| 11 | Braid-native | B4 | HTTP | `GET /boards`, `GET /messages`, `POST /messages` — full DKG protocol, same as wasm |
| 12 | Braid-native | Keycloak | HTTP | Fetch JWT access token using `TRUSTEE_NAME` / `TRUSTEE_PSW` env vars |

**Notes:**
- Paths 1–6 all route through Hasura actions: admin-portal calls Hasura GraphQL, Hasura
  proxies to the Harvest HTTP handler, Harvest writes back to PostgreSQL.
- Braid-wasm (path 8) has no Hasura or Harvest client — it only calls B4 directly.
- Windmill bypasses B4's HTTP API entirely — it writes directly into B4's PostgreSQL via
  `PgsqlB3Client` (path 10).
- Admin-portal reads ceremony status by polling Hasura (path 7, GraphQL), not by calling
  Harvest.
- For BBT trustees, the `signing_key_pk` registered via `register_trustee_key` (path 1–6) is
  what Windmill reads in path 9 to include in the `Configuration` message posted to B4 in path 10.

> **B4 signing invariant.** BBT trustees sign every B4 message locally in the browser using
> their in-memory `signing_key_sk`.  Harvest never signs on behalf of a BBT trustee and never
> holds BBT signing material.  The browser posts signed B4 messages directly to B4 (path 8),
> and separately notifies Harvest/Hasura for ceremony state transitions (path 1–6).  If a
> future audit requirement ever justifies
> routing B4 traffic through Harvest, the relay must forward the already-signed envelope
> verbatim — never open, modify, or re-sign it.

---

## 2. Per-Ceremony Key Registration

**Per-ceremony registration is the single mechanism for all trustee types.**

Every key is registered through `/register-trustee-key`, which upserts a row in the dedicated
`trustee_ceremony_key` table keyed on `(tenant_id, trustee_id, election_event_id,
keys_ceremony_id)` — **one independent row per ceremony**, so registrations for different
ceremonies never overwrite each other.

- **BBT trustees** register interactively: the browser calls `/register-trustee-key` after
  generating keys in WASM.
- **Server-based (braid-native) trustees** register themselves on startup: they discover their
  active ceremonies via `/active-ceremonies` and call the same `/register-trustee-key` endpoint
  with the public key from their config.

The registration path is therefore identical for all trustee types — there is no separate
server-side population step.

The result is **one gate** ("every trustee selected for this ceremony has a registered key
row"), one read query (a scoped join, no NULL-fallback branch), and **uniform cancellation**:
rows are scoped by `keys_ceremony_id` for all trustee types and do not interfere with any
replacement ceremony.

The per-ceremony row also protects the tally ceremony from future key rotation: because no
later ceremony touches this `(trustee, event, ceremony)` row, the key that went into the
Configuration is still readable, unchanged, at tally time — no separate snapshot is needed
(see [§3](#3-ceremony-creation-gate)).

### Why `keys_ceremony_id` scope (not just `election_event_id`)

Ceremonies can be cancelled and recreated for the same election event.  Scoping to
`keys_ceremony_id` ensures that keys registered for a cancelled ceremony — by any trustee
type — are never reused in the replacement.

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
`get_trustees_by_id` and collects their `public_key` fields.  If a BBT trustee user has not yet
opened the keys ceremony tab their row has no `public_key`; the current code uses
`.filter_map(|t| t.public_key)` which silently drops missing keys.

The resulting truncated list is passed to `public_keys::create_keys` →
`protocol_manager::add_config_to_board` → `Configuration::new` (in
`b4/src/messages/artifact.rs`), which immediately calls `assert!(c.is_valid())`.
`is_valid()` requires `trustees.len() > 1` and `threshold <= trustees.len()`, so if any
keys are silently dropped the assert fires and the Windmill worker **panics**.

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
  from `AWAITING_TRUSTEE_KEYS` and follows a **single uniform flow for all trustee types**:

  1. **Uniform gate check**: query whether every trustee selected for this ceremony has a
     `public_key` row in `trustee_ceremony_key` scoped to `(election_event_id,
     keys_ceremony_id)`.  There is no mode-switch; the same query covers all trustee types.
     All trustees — BBT browsers and braid-native daemons alike — register their own row
     independently via `/register-trustee-key`; Windmill does not populate any rows itself.
  2. If any key is still missing the ceremony stays in `AWAITING_TRUSTEE_KEYS` and the check
     retries on the next beat.  Once every key is present, the task posts the `Configuration`
     message to B4 **and** advances the status to `IN_PROGRESS` in the same step.

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

**Impact**: `get_trustee_status` currently reads the global `public_key` column via
`get_trustees_by_name`.  With the unified per-ceremony registration it must filter by
`keys_ceremony_id` so the board-message match is against the correct key — the same query
works for all trustee types because all rows are now scoped to `(election_event_id,
keys_ceremony_id)`.

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
`confirm_key_backup` (see [§10](#10-component-changes)).  The browser exports the identity
keys to a local TOML file for the trustee to save, then makes a plain JWT-authenticated
call to Harvest to self-report the backup as done and advance the status.  No secret
material reaches the server.  The legacy `/get-private-key` endpoint is not used by BBTs.

`KEY_RETRIEVED` is **trustee-confirmed**, not cryptographically proven.  Harvest accepts the
self-report; it cannot verify that the trustee actually saved the file.

### KEY_CHECKED

Set by the Harvest `/check-private-key` endpoint when the trustee re-uploads and the uploaded
key matches the one stored on the board.  When **all** trustees in the ceremony reach
`KEY_CHECKED`, the ceremony `execution_status` transitions to `SUCCESS`.

**BBT impact**: the board is never involved.
(a) **Browser-local sanity check** (always run, also reused outside the formal Check Step):
parse the uploaded TOML and verify that `signing_key_sk` derives the expected
`signing_key_pk`.  This gives the trustee immediate feedback.
(b) **Status advance** (formal Check Step only): the browser makes a plain
JWT-authenticated call to a new Harvest action `confirm_key_check` to self-report the
check as passed and advance the status.  No secret material is sent.  The legacy
`/check-private-key` endpoint is not used by BBTs.

`KEY_CHECKED` is **trustee-confirmed**, not cryptographically proven.  Harvest verifies
only the JWT — it cannot verify that the local TOML check actually passed.

### Summary table

| Status | Who sets it | Trigger | BBT difference |
|---|---|---|---|
| `WAITING` | `create_keys_ceremony` / `set_public_key_impl` | Ceremony creation or no board message yet | None for initial set; board-message match uses per-ceremony `public_key` (uniform for all trustee types) |
| `KEY_GENERATED` | `set_public_key_impl` | `PublicKey`/`PublicKeySigned` message found on board for this trustee | Must match the per-ceremony `public_key` row (same query for all trustee types) |
| `KEY_RETRIEVED` | `/get-private-key` (Harvest) — server-based only | Trustee downloads `TrusteeShareData` (Channel + Shares) from board | New Hasura action `confirm_key_backup`: trustee self-reports backup downloaded; no board read; **trustee-confirmed, not proven** |
| `KEY_CHECKED` | `/check-private-key` (Harvest) — server-based only | Re-uploaded key matches board copy | New Hasura action `confirm_key_check`: browser runs local `sk→pk` verification, then plain JWT-authenticated call advances status; no board read; **trustee-confirmed, not proven** |

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

Each election row points to its current keys ceremony via `election.keys_ceremony_id`.
On `/cancel-keys-ceremony`, in the same transaction as the status transition:

1. `try_transition(current → CANCELLED)` on the `keys_ceremony` row.
2. For every election whose `keys_ceremony_id` points to the cancelled ceremony, set
   `election.keys_ceremony_id = NULL`.

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
  keys against that new id; no `public_key` is copied over.

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
  → sets election.keys_ceremony_id (on each election in the event) to point at the new ceremony
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
                       UPSERT trustee_ceremony_key
                         (trustee_id, election_event_id, keys_ceremony_id) SET public_key
                    ↓
beat service (review_boards → process_board) polls each ceremony in AWAITING_TRUSTEE_KEYS:
  checks whether every selected trustee has a trustee_ceremony_key row for this
    (trustee_id, election_event_id, keys_ceremony_id)
  if any key is still missing → stays in AWAITING_TRUSTEE_KEYS, retried on next beat
  once all keys are present it launches Windmill create_keys
  reads public_key from trustee_ceremony_key
    WHERE (trustee_id, election_event_id, keys_ceremony_id)   (all trustee types)
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
  → Windmill set_public_key matches board messages against the per-ceremony trustee_ceremony_key from DB
  → each trustee status: KEY_GENERATED
                    ↓
[Download Step — backup file]
  Download is enabled only after all selected trustees have reached KEY_GENERATED
  Browser exports identity keys to a downloadable TOML (file leaves the browser
    only when the trustee saves it locally):
    { signing_key_sk, signing_key_pk, encryption_key }
  Browser POSTs → Harvest confirm_key_backup (Hasura action, JWT-authenticated)
    → trustee self-reports backup as downloaded (trustee-confirmed, not proven)
    → trustee status advances to KEY_RETRIEVED
  Navigate-away guard relaxes once the Download Step is complete (recovery is now possible)

[Check Step — browser-local validation + plain status advance]
  Trustee re-uploads backup TOML
  Browser parses TOML locally:
    - verify signing_key_pk derives from signing_key_sk
  Browser POSTs → Harvest confirm_key_check (Hasura action, JWT-authenticated)
    → trustee self-reports local check passed (trustee-confirmed, not proven)
    → trustee status advances to KEY_CHECKED
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
why the guard stays active until the Download and Check steps are both complete.

### What the Check Step triggers

When the trustee uploads the backup file in the Check Step, the browser runs local
validation (parse TOML, verify `signing_key_sk` derives `signing_key_pk`) and gives the
trustee immediate pass/fail feedback.

If the local check passes, the browser calls `confirm_key_check` (a plain
JWT-authenticated Harvest action) to advance the trustee to `KEY_CHECKED`.  Harvest
verifies only the JWT — it cannot verify that the local TOML check actually passed.
`KEY_CHECKED` is trustee-confirmed.  When all trustees reach `KEY_CHECKED` the ceremony
status advances to `SUCCESS`.

### Backup validation is reusable, not a one-shot step

The Download/Check pair is drawn above as a linear progression, but the underlying *local*
validation — parse TOML, verify `sk` derives `pk` — is available any time after
`KEY_GENERATED`.  The admin portal should expose a "check my backup" affordance the trustee
can use during `KEY_RETRIEVED` (to confirm the file is valid before signing off), after
`KEY_CHECKED` (re-verify days/weeks later), and during tally-ceremony preparation (to
re-load keys into memory on a fresh device, or just to confirm the file still works before
tally begins).

The local logic is identical in every case.  Only the formal Check Step at
`KEY_RETRIEVED → KEY_CHECKED` calls `confirm_key_check` and advances trustee status.
Outside that window the check is purely local feedback to the trustee; no Harvest call is
made.  Same UI, same WASM helpers, reused across the key ceremony and tally preparation.

> **Note on the Download step:** the trustee downloads a backup of their **identity keys** —
> `{ signing_key_sk, signing_key_pk, encryption_key }` — the browser equivalent of
> `trustee1.toml`.  This is **not** the DKG private key fragment (`TrusteeShareData`).
> There is no `/get-private-key` download in the BBT flow.  The identity key backup is the
> only file the trustee needs to save, and it must also be uploaded during the tally
> ceremony to re-load the trustee's identity keys into the browser's in-memory session.

---

## 8. Discovery: the `/active-ceremonies` Endpoint

### The endpoint already exists

The Harvest endpoint `POST /active-ceremonies` is **implemented and in use today** — it is the
discovery mechanism braid-native trustees call each loop to find which ceremonies to join
(see [§11](#11-component-changes)). It returns the same data for any trustee type:

```
POST /active-ceremonies
Auth:   existing Keycloak JWT — identifies which trustee is calling
Body:   { election_event_id?: string }   // optional filter
Response: {
  ceremonies: [
    {
      keys_ceremony_id: string,
      election_event_id: string,
      tenant_id: string,
      board_name: string,
      execution_status: string
    },
    ...
  ]
}
```

The endpoint returns **every** ceremony where:
- The caller's trustee is a registered participant (`trustee_ids` array)
- The ceremony status is `AWAITING_TRUSTEE_KEYS` or `IN_PROGRESS`
- Optionally restricted to one event via `election_event_id` in the body

Results are ordered most-recent-first. A trustee can be enrolled in several events at once, so
the response is a **list, one entry per event** — there is no single "current" ceremony to pick.
This matches braid-native, which already runs the DKG protocol across many boards concurrently:
it simply runs over the discovered boards (and only those) instead of every board in the B3
index. It also removes the ambiguity of a single-ceremony endpoint when two events each have an
open ceremony.

`board_name` is resolved server-side, per ceremony, from the election event's
`bulletin_board_reference` (`database_name`) — the authoritative board name — so callers never
reconstruct it.

Because braid-native already drives the full discover → register → run-DKG flow through this
endpoint, it is proven against a real caller. **No backend work is needed to extend discovery to
browser trustees** — only the admin-portal frontend integration described below remains.

### braid-native loop

Each iteration, braid-native:

1. Refreshes its access token.
2. Calls `POST /active-ceremonies` to get the current set of ceremonies (across all its events).
3. Registers its public key via `/register-trustee-key` for any ceremony it has not registered
   yet (idempotent server-side; tracked locally to avoid redundant calls).
4. Creates a DKG session for each discovered `board_name` not already running, and steps the
   protocol on those boards only.

New ceremonies created later are picked up on a subsequent loop; completed ceremonies drop out of
the response and their sessions are released on the periodic session reset.

### Future enhancement: browser-trustee discovery in the admin portal (frontend only)

Browser trustees currently access the key ceremony by manually navigating to
`/election/{eventId}/keys-ceremony` in the admin portal after ceremony creation, which requires
the trustee to already know which ceremony to join. The existing `/active-ceremonies` endpoint
can remove that manual step.

When a browser trustee logs into the admin portal:

1. **Check for active ceremonies** — on login or at the tenant-selection screen, call
   `/active-ceremonies` to fetch any pending ceremonies for this trustee.
2. **Show a discovery banner / list** — if one or more ceremonies are returned, display them
   with, for each:
   - Ceremony name and election event
   - Status and progress indicator
   - **Join ceremony** button linking to that ceremony's screen
3. **Navigate** — use each entry's `election_event_id` to navigate to
   `/election/{eventId}/keys-ceremony`. The `board_name` is already provided by the response, so
   no client-side derivation is needed.

### What remains (frontend only)

The only outstanding work for browser-trustee discovery is in the admin-portal package:
- Frontend call to `/active-ceremonies` on login (JS/TS)
- Optional UI for the discovery banner/list (Figma-gated)

There are **no backend or database changes** — the `/active-ceremonies` endpoint is done. This is
a pure usability improvement and is not blocking the unified registration path or any other core
BBT functionality; it can be deferred to a later phase.

---

## 9. Security: In-Memory Key Handling

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

- Harvest never receives `signing_key_sk`, `encryption_key`, or the TOML.  It holds only
  `signing_key_pk` (already public).
- Harvest never signs B4 messages on behalf of a BBT trustee.  Browser sessions sign every
  B4 message locally with their in-memory `signing_key_sk` and POST directly to B4 (path 8
  in [§1](#1-service-communication-paths)); see the B4 signing invariant in §1.
- A compromise of Harvest must not yield BBT impersonation.  Any future feature that
  appears to need a signature for a BBT trustee must round-trip through that trustee's
  browser, not synthesize one server-side.

### Limits of the trustee-confirmation model

`KEY_RETRIEVED` and `KEY_CHECKED` are self-reported status advances authenticated only by
the trustee's JWT.  Harvest accepts these calls without being able to verify that the
trustee actually saved the backup file or that the local TOML check actually passed.  This
is a deliberate simplification: we cannot prove possession of a symmetric key over the
wire, and a commitment only ever verified a self-report anyway.  The statuses reflect
trustee intent, not cryptographic assurance.  The navigate-away guard and the local
browser-side validation exist to make the trustee's self-report accurate in practice.

---

## 10. Key Loss and Recovery

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
in the DB (overwriting the previous entry), and the
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
- `election.keys_ceremony_id` is cleared (on every election in the event) atomically with
  the status transition (see [§6](#6-cancellation-window)) so the elections can be re-assigned.
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

## 11. Component Changes

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

### 2 — DB schema: new `sequent_backend_trustee_ceremony_key` table

A trustee can participate in several ceremonies at once (one per election event), so the key
must be stored **per ceremony**, not on the single identity row in `sequent_backend_trustee`
(a flat column there can only hold one ceremony's scope and would be overwritten by the next
registration). Per-ceremony keys live in a dedicated table:

| Column | Type | Meaning |
|---|---|---|
| `id` | `uuid` (PK) | Row id |
| `tenant_id` | `uuid` (FK → `tenant.id`) | Tenant |
| `trustee_id` | `uuid` (FK → `trustee.id`) | Trustee identity |
| `election_event_id` | `uuid` (FK → `election_event.id`) | Event |
| `keys_ceremony_id` | `uuid` (FK → `keys_ceremony.id`) | Ceremony |
| `public_key` | `text` | Ed25519 public key for this `(trustee, event, ceremony)` |

with `UNIQUE (tenant_id, trustee_id, election_event_id, keys_ceremony_id)`. The identity table
`sequent_backend_trustee` keeps the trustee's name, identity, and stable/global `public_key`
(provisioned out-of-band for server-based trustees); it no longer carries per-ceremony scope
columns.

**All trustee types** upsert a `(trustee_id, election_event_id, keys_ceremony_id) →
public_key` row via Harvest `/register-trustee-key` when their key is registered:

- **BBT trustees**: the browser calls it after WASM key generation.
- **Server-based (braid-native) trustees**: the daemon calls it on startup with the public key
  from its config (discovered ceremonies via `/active-ceremonies`).

Registrations for different ceremonies write different rows and never overwrite each other; a
re-registration for the same ceremony updates only that ceremony's row.

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
Action: INSERT INTO sequent_backend_trustee_ceremony_key
          (tenant_id, trustee_id, election_event_id, keys_ceremony_id, public_key)
        VALUES (..., <trustee_id from JWT>, ..., $public_key)
        ON CONFLICT (tenant_id, trustee_id, election_event_id, keys_ceremony_id)
          DO UPDATE SET public_key = EXCLUDED.public_key
```

The private key and `encryption_key` never reach the server.  All trustee types call this
endpoint (BBT browsers and braid-native daemons alike). Re-registering the same ceremony
updates only that ceremony's row; it never affects another ceremony's key.

### 5 — Harvest: new Hasura action `confirm_key_backup`

```
Input:  { keys_ceremony_id: string }
Auth:   existing Keycloak JWT  — identifies which trustee is calling
Action (one transaction):
  1. Load the row for (trustee_id from JWT, keys_ceremony_id).
     Reject NO_PUBLIC_KEY_REGISTERED if public_key is NULL.
  2. Advance TrusteeStatus → KEY_RETRIEVED via try_transition (idempotent on KEY_RETRIEVED).
Returns: { status: "ok" } | error code.
```

Trustee self-reports that the backup was downloaded.  Harvest cannot verify the claim; it
accepts any call from a JWT holder whose `public_key` is already registered.  Server-based
trustees never call this endpoint; they use the existing `/get-private-key` path.

### 6 — Harvest: new Hasura action `confirm_key_check`

```
Input:  { keys_ceremony_id: string }
Auth:   existing Keycloak JWT  — identifies which trustee is calling
Action (one transaction):
  1. Load the row for (trustee_id from JWT, keys_ceremony_id).
     Reject NO_PUBLIC_KEY_REGISTERED if public_key is NULL.
     Reject INVALID_STATE if TrusteeStatus ≠ KEY_RETRIEVED.
  2. Advance TrusteeStatus → KEY_CHECKED via try_transition.
Returns: { status: "ok" } | error code.
```

Trustee self-reports that the local TOML validation passed.  Harvest verifies only the JWT;
it cannot verify that the browser-side check actually ran.  `KEY_CHECKED` is
trustee-confirmed.  Server-based trustees never call this endpoint; they use the existing
`/check-private-key` path.

### 7 — Harvest: new Hasura action `cancel_keys_ceremony`

```
Input:  { keys_ceremony_id: string }
Auth:   admin role
Action (one transaction):
  1. Load the keys_ceremony row.  Reject CEREMONY_NOT_FOUND if missing.
  2. For each election in the event:
       - reject VOTING_PERIOD_STARTED if its voting period has begun.
  3. try_transition(current → CANCELLED) on the keys_ceremony row.
  4. For every entry in the election's table whose keys_ceremony_id points at this ceremony,
       SET keys_ceremony_id = NULL.
Returns: { status: "ok" } | error code.
```

See [§6 Cancellation Window](#6-cancellation-window) for the rationale and the resulting
"unassigned" election state.

### 8 — Harvest: update `create-keys-ceremony` with `duplicate_from`

Add an optional input field `duplicate_from: keys_ceremony_id?`.

- If present: load the source ceremony; copy trustee set, threshold, and naming convention
  onto the new ceremony.  Reject DUPLICATE_SOURCE_TRUSTEE_MISSING if any source trustee no
  longer exists.  Reject ELECTION_ALREADY_ASSIGNED if the target election already has
  `keys_ceremony_id` set (admin must cancel first).  Do **not** copy `public_key` —
  BBT trustees must regenerate against the new `keys_ceremony_id`.
- If absent: behaves as today (fresh create).

In both cases, on success set `election.keys_ceremony_id` (on each election in the event)
to the new ceremony id.

### 9 — Admin portal: ceremony-assignment states on the election page

The election-event ceremony page must render different controls depending on
`election.keys_ceremony_id` (read from the elections within the event — there is no
`keys_ceremony_id` on the `election_event` row itself):

- **10.1 — `NULL` (no ceremony assigned)**: show "Create ceremony" and, if any prior ceremony
  exists for this election event, "Duplicate from previous".
- **10.2 — Assigned, voting period not started**: show ceremony status + "Cancel ceremony"
  button (calls `cancel_keys_ceremony`).
- **10.3 — Assigned, voting period started or election in progress**: show ceremony status
  only; no cancel/recreate controls (the state machine + voting-period gate would reject
  anyway).

### 10 — Windmill `get_trustees_by_id` / `get_trustees_by_name`: join `trustee_ceremony_key`

Both DB queries resolve `public_key` by `LEFT JOIN`ing `trustee_ceremony_key` on the requested
`(election_event_id, keys_ceremony_id)`.  Every trustee in the ceremony (server-based and BBT
alike) has a row scoped to that tuple, so no NULL-fallback branch is needed:

```sql
SELECT t.id, t.name, ..., tck.public_key
FROM sequent_backend.trustee AS t
LEFT JOIN sequent_backend.trustee_ceremony_key AS tck
  ON tck.trustee_id = t.id
  AND tck.tenant_id = t.tenant_id
  AND tck.election_event_id = $event_id
  AND tck.keys_ceremony_id = $ceremony_id
WHERE t.tenant_id = $tenant_id AND t.id = ANY($trustee_ids)
```

When no scope is requested (both NULL), or there is no matching per-ceremony row, the join
yields `NULL` — there is no fallback to the trustee's stable/global key. A missing row means a
trustee has not yet registered; the gate check upstream keeps the ceremony in
`AWAITING_TRUSTEE_KEYS` until all rows are present.

No change to `create_keys`, `set_public_key`, or `insert_ballots` logic — they already use
whatever `public_key` the scoped query returns. Tally (`insert_ballots`) reads the same
per-ceremony row, which is never overwritten by a later ceremony, so it always matches the
on-board Configuration.

---

## 12. `trustee.public_key` Column

The `sequent_backend_trustee.public_key` column will be removed as the legacy trustee's stable/global
key (provisioned out-of-band for server-based trustees, generated in-browser for BBT). The
per-ceremony key that actually goes into a Configuration lives in the separate
`trustee_ceremony_key` table (see [§11.2](#11-component-changes)).

---

## 13. Trustee Public Key Visibility in the Ceremony UI

Each trustee row in the ceremony trustee table must show an info (ℹ) icon.  Clicking it
opens a popup displaying that trustee's full public key as a text block — for every
trustee in the ceremony, server-based and BBT alike.

The purpose is **out-of-band verification**: each trustee independently confirms, through a
side channel (call, Signal message, in-person comparison), that the set of keys they see
matches what every other trustee sees.  One-line justification: the software is open source,
and a trustee who trusts the software they run can trust that the displayed keys are exactly
the ones that went into the Configuration.

### When comparison is meaningful

Key display is only meaningful from **`IN_PROGRESS`** onward — after the Configuration
message has been posted to the board and the full trustee key set is frozen.  Before that,
not all BBT trustees may have registered their keys yet.  The comparison therefore
naturally happens at the Download/Check stage, when every trustee is already on the
ceremony page with an active board session.

### Board as the authoritative source

Once the ceremony is `IN_PROGRESS`, the public key shown in the popup must be read from the
**Configuration message on the board**, not from the `trustee.public_key` DB row.  The
board is the frozen, auditable artifact; the browser session reads it anyway.  Displaying
the board copy guarantees that what the trustee sees is exactly what was committed to the
DKG — ruling out a late DB update or a race condition between registration and the
Configuration post.

### Figma dependency

A Figma design is required before frontend implementation: icon placement in the trustee
table row, and the popup layout (monospaced key block, selectable text, copy-to-clipboard
button).  **This design is a blocking dependency for the frontend task.**

---

## 14. Honest Trade-Offs

The unified registration path is a significant improvement but it does not reach full
cryptographic integrity without protocol-level changes.  The remaining gaps are worth
naming plainly:

### The key registry is a mutable DB, not the append-only board

Keys are registered into `sequent_backend_trustee_ceremony_key`, a regular Postgres table that
can be updated or deleted with the right DB credentials.  The board is append-only and
tamper-evident; the DB is neither.  An attacker with DB write access could swap a key
before Windmill reads it and builds the Configuration.

Mitigation within scope: the per-ceremony row is scoped to `(trustee, event, ceremony)`, so no
later ceremony overwrites it — once the Configuration is posted, the key that went into it
stays readable and unchanged for tally.
The row remains mutable in principle, so the window between registration and Configuration
posting is still an implicit trust boundary on the DB; the only tamper-evident copy is the
Configuration message on the board itself.

### Windmill builds the Configuration from the DB, not from a trustee-signed artifact

There is no trustee-signed enrollment message.  Windmill reads the per-ceremony `public_key`
row and constructs the Configuration unilaterally.  A trustee whose key is read from the DB
has no ceremony-specific act of consent recorded anywhere.

Eliminating this gap would require the Configuration to be constructed from trustee-signed
enrollment messages posted to the board — a protocol-level change.

### The trustee↔key binding rests entirely on the Keycloak session

`/register-trustee-key` authenticates the caller by their Keycloak JWT.  There is no
proof-of-possession of `signing_key_sk`: the endpoint accepts any Ed25519 public key the
JWT bearer supplies.  A compromised Keycloak session could register an attacker-controlled
key.

Mitigation within scope: Keycloak is the existing identity root for all operations in the
system; this endpoint's trust requirement is no stronger than any other authenticated
endpoint.  A proof-of-possession mechanism (e.g., a challenge signed by `signing_key_sk`)
would require a two-round protocol change.

---

## 15. Convergence with the Protocol-Change Design

This design is intentionally shaped so that migration to the full protocol-change design
(see [BBT Protocol Change Proposal](./key_ceremony_bbt_propossal_protoccol_change.md),
implemented on the `crypto-migration` branch) is mechanical once the unified registration
path is in place.

### The per-ceremony DB row is the stand-in for an on-board registration message

In the protocol-change design, each trustee posts a signed enrollment message to the board
before the Configuration is built.  Windmill (or an equivalent orchestrator) reads those
board messages to construct the Configuration — never the DB.

In this design, the per-ceremony `(trustee_id, election_event_id, keys_ceremony_id,
public_key)` row in `sequent_backend_trustee_ceremony_key` holds the same data as that
enrollment message, but stored in a mutable DB table rather than the append-only board.  The
row is the stand-in: same scope, same content, different storage medium.

### DB-row registration → braid-native posting a registration message

In the unified path, each trustee (BBT or braid-native) writes its per-ceremony key row via
`/register-trustee-key`.  In the protocol-change design, this is replaced by the trustee
posting a signed registration message to the board.  The content of that message is identical
to the DB-written row (the trustee's public key); only the storage medium and the act of
signing change.  Because braid-native trustees always use the same long-lived key, the message
content is constant across ceremonies — it is "braid-native posting its constant registration."

### The migration is mechanical

Once the unified DB-row path is stable:

1. Add a new board message type for trustee key registration (the protocol-change spec).
2. Change `/register-trustee-key` to post that message to the board **in addition to** writing
   the DB row — or instead of, if the read path is also switched.
3. Change `create_keys_impl` to build the `Configuration` from board registration messages
   rather than DB rows.
4. The DB rows become redundant and can be deprecated.

No new ceremony-level state machine states are needed.  The `AWAITING_TRUSTEE_KEYS` gate
logic is unchanged — it just reads a different source.  The `crypto-migration` branch
supersedes and extends this design at the point where step 3 is implemented.

---

## 16. Future Work: Certificate Chain

Per-ceremony trustee certificates will eventually chain upward:

```
trustee cert → tenant cert → environment cert → Sequent root cert
```

This is what gives a fresh per-ceremony key **verifiable meaning**: a verifier who trusts
the Sequent root can confirm that a given `signing_key_pk` was legitimately generated for
this trustee, in this tenant, in this environment, for this ceremony — without relying on
an out-of-band comparison or a Keycloak session as the sole trust anchor.

Until the certificate chain exists, the **on-screen public key comparison** described in
[§12](#12-trustee-public-key-visibility-in-the-ceremony-ui) is the only available
verification: trustees manually confirm, out of band, that they all see the same key set.
It is a usable stopgap but it provides no cryptographic binding to any identity hierarchy.
