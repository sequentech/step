---
id: key_ceremony_bbt_propossal_no_protoccol_change
title: BBT Key Ceremony — Signing Keypair Proposal (No Protocol Change)
sidebar_label: BBT Signing Keypair (No Protocol Change)
---

# BBT Signing Keypair — No Protocol Change

This document describes the lower-risk approach to introducing Browser-Based Trustee (BBT)
signing keys — a new capability where each trustee generates their own keypair directly in the
browser via WASM, stores it in `localStorage`, and registers the public key in the existing
`sequent_backend_trustee` DB table via a new Harvest endpoint. Previously, only server-based
trustees existed; this is the first design to support in-browser key generation.
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
| 1 | Admin-portal | Harvest (via Hasura) | HTTP POST | `create-keys-ceremony`, `list-keys-ceremonies`, `get-private-key`, `check-private-key`, all admin operations |
| 2 | Admin-portal | Harvest | HTTP POST | **`register-trustee-key` — BBT new:** registers the BBT signing public key for an election event |
| 3 | Admin-portal | Hasura | GraphQL/HTTP | Read trustee config (`GET_TRUSTEE_CONFIG`), election events, ceremony status (`execution_status`), all entity queries |
| 4 | Admin-portal (braid-wasm) | B4 | HTTP | `GET /boards`, `GET /messages`, `POST /messages` — full DKG protocol (Channel, Shares, PublicKey…) |
| 5 | Harvest / Windmill | PostgreSQL (Hasura DB) | SQL direct | Read/write `keys_ceremony`, `trustee`, `election_event` tables.  **BBT change:** `trustee` now has `election_event_id`; `get_trustees_by_id` / `get_trustees_by_name` filter `public_key` by event |
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

See the [Flow](#flow) section below for the full sequence diagram.

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
**identity keys** (`signing_key_sk`, `signing_key_pk`, `encryption_key`) from `localStorage`
instead.  The reason is that these identity keys are the root credential: without
`signing_key_sk` the trustee cannot authenticate on the board at all, and without
`encryption_key` the `encrypted_channel_sk` inside the on-board `Channel` message cannot be
decrypted — making the `TrusteeShareData` inaccessible regardless.  Backing up the identity
keys is therefore sufficient and more portable than downloading the raw board data.

A Harvest call is still needed to advance the status to `KEY_RETRIEVED` after the identity
key download, but the endpoint logic must be a new or mode-aware path — not a call to the
existing `/get-private-key` which performs a board fetch keyed on the server-trustee
`public_key`.

### KEY_CHECKED

Set by the Harvest `/check-private-key` endpoint when the trustee re-uploads and the uploaded
key matches the one stored on the board.  When **all** trustees in the ceremony reach
`KEY_CHECKED`, the ceremony `execution_status` transitions to `SUCCESS`.

**BBT impact**: the check for BBTs is entirely local — the UI compares the re-uploaded file
values against `localStorage`.  The board is not involved.  A Harvest call is still needed to
advance the status to `KEY_CHECKED` (and trigger the `SUCCESS` transition), but the comparison
logic in `/check-private-key` cannot be reused as-is; a new BBT-specific endpoint or a
mode-aware branch is required.

### Summary table

| Status | Who sets it | Trigger | BBT difference |
|---|---|---|---|
| `WAITING` | `create_keys_ceremony` / `set_public_key_impl` | Ceremony creation or no board message yet | None for initial set; board-message match needs per-event key |
| `KEY_GENERATED` | `set_public_key_impl` | `PublicKey`/`PublicKeySigned` message found on board for this trustee | Must match per-event `public_key`, not global one |
| `KEY_RETRIEVED` | `/get-private-key` (Harvest) | Trustee downloads `TrusteeShareData` (Channel + Shares) from board | Same data exists on board for BBTs (protocol unchanged); Download step repurposed to back up identity keys from `localStorage` — new or mode-aware endpoint needed |
| `KEY_CHECKED` | `/check-private-key` (Harvest) | Re-uploaded key matches board copy | New endpoint / branch needed: local `localStorage` vs file comparison, no board read |

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
will also participate in tally ceremonies, and the same identity keys must be present in
`localStorage` (or restored from backup) at that point.

---

## 7. Flow

```
Admin creates ceremony (selects trustees)
  → create_keys_ceremony inserts record with execution_status: AWAITING_TRUSTEE_KEYS
                    ↓
BBT trustee opens the election event's keys ceremony in admin portal
  └─ HeadlessTrusteeProvider mounts
  └─ WASM silently generates trustee identity keys (once, if not already in localStorage):
         - Ed25519 signing keypair  (StrandSignatureSk::generate())
         - AES-256 symmetric key    (symm::gen_key())
       All three values stored in localStorage keyed by keys_ceremony_id
       (localStorage persists across tab closures and browser restarts; keys are only
        lost if the trustee explicitly clears site data or the device/profile is lost)
  └─ signing_key_pk → POST Harvest /register-trustee-key  ← can happen onnly once the keys ceremony is created
                       UPSERT (trustee_id, election_event_id, keys_ceremony_id) → public_key in DB  ← depends on keys_ceremony_id
                    ↓
beat service (review_boards → process_board) polls each ceremony in AWAITING_TRUSTEE_KEYS:
  checks whether every selected BBT trustee has a public_key for this election_event_id and keys_ceremony_id in DB
  if any key is still missing → stays in AWAITING_TRUSTEE_KEYS, retried on next beat cycle
  once all keys are present it launches Windmill create_keys
  reads trustee.public_key WHERE (trustee_id, election_event_id, keys_ceremony_id) = $1, $2, $3; (BBT) 
  or WHERE (trustee_id, election_event_id, keys_ceremony_id) = $1, NULL, NULL; (server-based)
  → posts Configuration protoccol message to B4
  → execution_status: IN_PROGRESS
                    ↓
HeadlessTrusteeProvider reads signing_key_sk + encryption_key from localStorage
  → WasmSession runs DKG protoccol automatically in background:
       Channel → ChannelsAllSigned → Shares → PublicKey → PublicKeySigned
                    ↓
allTrusteesGenerated: all trustees have a PublicKey or PublicKeySigned message on the board
  → Windmill set_public_key matches board messages against trustee.public_key from DB
  → each trustee status: KEY_GENERATED
                    ↓
[Existing Download Step — repurposed for identity key]
  Download is enabled only after all selected trustees have reached KEY_GENERATED
  Trustee downloads identity key backup from localStorage:
    { signing_key_sk, signing_key_pk, encryption_key }
  → trustee status: KEY_RETRIEVED

[Existing Check Step — repurposed for identity key]
  Trustee re-uploads backup file
  UI compares values from file against localStorage
  If match → POST Harvest to update trustee status: KEY_CHECKED
  All trustees KEY_CHECKED → ceremony status: SUCCESS
```

### What the verify step triggers

When the trustee uploads the backup file in the Check Step, the UI reads `signing_key_sk`,
`signing_key_pk`, and `encryption_key` from the file and compares them against the values in
`localStorage`.  If they match, a Harvest call updates the trustee status to `KEY_CHECKED`.
No B4 read is needed — the check is purely local.  When all trustees reach `KEY_CHECKED` the
ceremony status advances to `SUCCESS`.

### Backup validation is reusable, not a one-shot step

The Download/Check pair is drawn above as a linear progression, but the underlying validation
— **read backup file, compare against `localStorage`** — is available any time after
`KEY_GENERATED`.  The admin portal should expose a "check my backup" affordance the trustee
can use during `KEY_RETRIEVED` (to confirm the file is valid before signing off), after
`KEY_CHECKED` (re-verify days/weeks later), and during tally-ceremony preparation (to
re-hydrate `localStorage` on a fresh device, or just to confirm the file still works before
tally begins).

The logic is identical in every case.  Only the side-effect differs: the formal Check Step
at `KEY_RETRIEVED → KEY_CHECKED` is the one place where a successful match also advances the
trustee status.  Outside that window the check is read-only — same UI, same WASM helpers,
reused across the key ceremony and tally preparation with no protocol or DB changes.

> **Note on the Download step:** the trustee downloads a backup of their **identity keys** —
> `{ signing_key_sk, signing_key_pk, encryption_key }` — the browser equivalent of
> `trustee1.toml`.  This is **not** the DKG private key fragment (`TrusteeShareData`).
> There is no `/get-private-key` download in the BBT flow.  The identity key backup is the
> only file the trustee needs to save, and it must also be uploaded during the tally ceremony
> to identify the trustee on the board.

---

## 8. Security: localStorage Key Storage

### Is the risk real?

Yes.  `localStorage` is readable by any JavaScript executing on the same origin.  A
successful XSS attack against the admin portal would allow an attacker to exfiltrate
`signing_key_sk` and `encryption_key` in plaintext.  With those two values an attacker can:

- Impersonate the trustee on the B4 board (sign messages with `signing_key_sk`).
- Decrypt `encrypted_channel_sk` from the on-board `Channel` message (using `encryption_key`),
  obtaining the channel private key and therefore the trustee's DKG shares.

The admin portal currently has **no Content-Security-Policy header** configured and includes
TinyMCE (a rich-text editor), which is a classic XSS surface if any user-supplied content
reaches it.  Introducing cleartext cryptographic private keys into `localStorage` is therefore
a meaningful new risk, not a theoretical one.

### Why sessionStorage does not solve it

The review comment suggests `sessionStorage` (cleared on tab close) as an alternative.
`sessionStorage` is still fully readable by XSS — it addresses only a subset of
*physical-access* scenarios (e.g. an attacker who can inspect a browser profile on disk after
the user has closed their tab).  It does not reduce the XSS surface at all.

It also conflicts with the design: `localStorage` persistence is intentional so that
long-running WASM sessions and the heartbeat daemon survive navigation and tab restore.
Switching to `sessionStorage` would require the trustee to re-import their identity keys on
every new tab, defeating the purpose of the Download/Check backup flow.

### Why the explicit export/import flow alone is not sufficient

The Download/Check steps already provide an export/import path, but they are a *recovery
mechanism*, not the primary storage.  Keys still live in `localStorage` between generation
and the point where the trustee downloads the backup — and they stay there for the entire
ceremony and tally lifecycle.

### Recommended mitigation: WebCrypto encryption at rest

The correct fix is to encrypt the private key material before writing it to `localStorage`,
using a key derived from a user-provided passphrase via the WebCrypto API:

1. On first key generation, prompt the trustee for a passphrase.
2. Derive a wrapping key: `PBKDF2(passphrase, random_salt, iterations) → AES-256-GCM key`.
3. Encrypt `signing_key_sk` and `encryption_key` with the wrapping key.
4. Store only the ciphertext, the salt, and the IV in `localStorage` — no plaintext key
   material ever touches storage.
5. On subsequent page loads, prompt for the passphrase again, re-derive the wrapping key,
   and decrypt in memory.

This means an XSS attacker who can only read `localStorage` obtains useless ciphertext.  An
active XSS attacker who can execute code *while the user is logged in and has entered their
passphrase* can still steal keys from memory — but this is a much harder attack to land and
the window is limited to the active session.

**Comparison to server-based trustees:** server trustees store their keys in `trusteeN.toml`
files on a server filesystem with file-system-level access controls.  WebCrypto-encrypted
`localStorage` provides a comparable security posture: the key material is protected at rest
and exposed only in memory during active use.

### Short-term vs long-term

This can be implemented incrementally:

- **Phase 1 (initial implementation):** store cleartext in `localStorage`, document the
  risk, and rely on the controlled ceremony environment (trusted admins, no untrusted content
  in the portal, attended ceremony).
- **Phase 2 (before production use):** add WebCrypto passphrase wrapping.  The
  `generate_trustee_keys()` WASM export and the `HeadlessTrusteeProvider` logic remain the
  same; only the storage layer changes.

---

## 9. Key Loss: Recovery from Cleared localStorage

`localStorage` is **persistent storage** — it survives tab closures, page refreshes, and
browser restarts.  Closing the tab mid-ceremony does not lose the keys; the trustee simply
reopens the admin portal and the `HeadlessTrusteeProvider` finds the keys already in
`localStorage` and continues normally.

Keys are only lost in two exceptional situations:
- The trustee explicitly clears site data (browser settings, developer tools, or a
  deliberately triggered key rotation).
- The device itself is lost or the browser profile is deleted.

Two sub-cases for explicit loss:

**Before `Configuration` is posted** (ceremony still in `AWAITING_TRUSTEE_KEYS`):
The public key has not yet been embedded in a board message, so the trustee can regenerate a
fresh keypair, re-register the new `public_key` in the DB, and the beat service will proceed
normally.  The ceremony has not used the old key for anything.

**After `Configuration` is posted** (ceremony in `IN_PROGRESS`):
The trustee's public key is already embedded in the `Configuration` message on the board.
Regenerating produces a different keypair that no longer matches — the trustee would be unable
to post valid DKG messages.  The recovery path is to restore from the identity key backup file
downloaded in the Download step.  The browser reads `signing_key_sk`, `signing_key_pk`, and
`encryption_key` from the file, writes them back into `localStorage`, and
`HeadlessTrusteeProvider` resumes the session with the original keys.

**Are the changes straightforward?**  Mostly yes — `HeadlessTrusteeProvider` already checks
`localStorage` on mount and skips generation if keys are present.  Adding an upload/restore
path requires:
- A UI affordance (file picker) on the election event page when `localStorage` is empty but
  the trustee's public key is already registered in the DB for this event (detectable via the
  existing `GET_TRUSTEE_CONFIG` query)
- Parsing the backup file and writing the three values into `localStorage` before the
  provider initialises the `WasmSession`

No changes to Windmill, Harvest, or B4 are needed for this recovery path.

### Unrecoverable loss: cancel and recreate

If a trustee **cannot download** their identity backup (e.g. the Download step never
completed) or **loses both** their `localStorage` keys and the backup file with no way to
restore them, the trustee can no longer participate in the ceremony — and because their public
key is embedded in the on-board `Configuration` message, no fresh keypair can be substituted.

In this situation the ceremony itself is unrecoverable.  The recovery path is to **cancel the
ceremony and create a new one**, provided no election in the event has opened its voting
period yet.  This is permitted by the state machine: `AWAITING_TRUSTEE_KEYS`, `IN_PROGRESS`,
and `SUCCESS` all transition to `CANCELLED` (see [Keys Ceremony State
Machine](./key_ceremony_state_machine.md); the SUCCESS arm is additionally gated by the
voting-period check at the cancel endpoint, see [§6 Cancellation Window](#6-cancellation-window)).

After cancellation:
- The orphan `Channel`, `Shares`, and `PublicKey` messages on the old B4 board remain in
  place (append-only), but are simply ignored — the admin creates the replacement ceremony on
  a fresh board.
- The old BBT rows in `sequent_backend_trustee` keyed by the cancelled `keys_ceremony_id`
  remain in place but no longer match the new ceremony's `keys_ceremony_id`, so they do not
  interfere with the replacement (see [§6](#6-cancellation-window)).
- The remaining trustees regenerate their keys in their browsers under the new
  `keys_ceremony_id` and the ceremony proceeds normally from `AWAITING_TRUSTEE_KEYS`.

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

### 2 — DB schema: add `election_event_id` and `keys_ceremony_id` to `sequent_backend_trustee`

Add nullable `election_event_id` and `keys_ceremony_id` columns.  Both are nullable but, when
set, must be foreign keys to the `id` columns of their respective tables (`election_event` and
`keys_ceremony`):

| Column | Type | Meaning |
|---|---|---|
| `election_event_id` | `uuid \| null` (FK → `election_event.id`) | `null` for server-based trustees; event UUID for BBT |
| `keys_ceremony_id` | `uuid \| null` (FK → `keys_ceremony.id`) | `null` for server-based trustees; ceremony UUID for BBT |
| `public_key` | `text \| null` | Ed25519 public key scoped to this (trustee, event, ceremony) tuple |

Server-based trustees keep `election_event_id = null`, `keys_ceremony_id = null`, and
`public_key` set once at provisioning time — no behaviour change.  BBT trustees upsert
`(trustee_id, election_event_id, keys_ceremony_id) → public_key` on every session start.

### 3 — `HeadlessTrusteeProvider`: generate and register on mount

Inside the existing `initialize` effect, before constructing `WasmSession`:

1. Derive the localStorage key: `bbt_keys_{electionEventId}`.
2. If absent: call `generate_trustee_keys()` via WASM, store all three values in `localStorage`,
   POST `signing_key_pk` to Harvest `/register-trustee-key` with the `election_event_id`.
3. If present: re-POST `signing_key_pk` on page load to keep the DB record in sync.
4. Pass `signing_key_sk`, `signing_key_pk`, and `encryption_key` from `localStorage` into the
   `WasmSession` config instead of the hardcoded strings.

### 4 — Harvest: new `POST /register-trustee-key` endpoint

```
Input:  { public_key: string, election_event_id: string }
Auth:   existing Keycloak JWT  — identifies which trustee is calling
Action: UPSERT sequent_backend_trustee
          SET public_key = $public_key
          WHERE trustee_id = <from JWT> AND election_event_id = $election_event_id
```

The private key and encryption key never reach the server.  Server-based trustees never call
this endpoint.

### 5 — Windmill `get_trustees_by_id` / `get_trustees_by_name`: filter by `election_event_id`

Update both DB queries to prefer the row matching the ceremony's `election_event_id`, falling
back to `election_event_id = null` for server-based trustees:

```sql
SELECT public_key FROM sequent_backend_trustee
WHERE id = $trustee_id
  AND (election_event_id = $event_id OR election_event_id IS NULL)
ORDER BY election_event_id NULLS LAST
LIMIT 1
```

No change to `create_keys`, `set_public_key`, or `insert_ballots` logic — they already use
whatever `public_key` the query returns.

---

## 11. `trustee.public_key` Column

The column is **retained and extended** with the companion `election_event_id` column.  It
remains the single source of truth for the public key in this approach.
