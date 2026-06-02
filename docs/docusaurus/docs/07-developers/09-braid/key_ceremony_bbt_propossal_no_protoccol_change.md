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
The B4 protocol and all DKG message types remain unchanged.

See [BBT Protocol Change Proposal](./key_ceremony_bbt_propossal_protoccol_change.md) for the
alternative design that removes the DB dependency entirely.

See [Key Ceremony Design](./key_ceremony_design.md) for the current architecture baseline.

> **Scope note — tally ceremonies:** this document covers only the key ceremony.  Tally
> ceremonies are out of scope here but must be accounted for: BBT trustees will participate in
> tally ceremonies too, using the same identity keys (`signing_key_sk`, `encryption_key`) they
> generated during the key ceremony.  Any design decision made here that affects how identity
> keys are stored, looked up, or rotated has direct implications for the tally flow.

---

## The Per-Event Key Problem

A BBT trustee may participate in multiple election events simultaneously.  A single global
`public_key` per trustee row would be overwritten by whichever event registered last, breaking
DKG verification on any previously started ceremony.

This approach solves it by adding an `election_event_id` column so each
`(trustee, election_event)` pair has its own independent public key.

---

## Ceremony Creation Gate

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
| **Browser-based** | Filter by both `tenant_id` **and** `election_event_id` to get the per-event key. |

`get_trustee_mode_policy()` in `windmill/src/postgres/trustee.rs` already reads the
`TrusteeModePolicy` from the trustee's `annotations.trustee_mode_policy` JSON field, so the
mode is already detectable.  The query in the document's Component Changes section
(`ORDER BY election_event_id NULLS LAST`) covers both cases in one SQL statement, but the
calling code must know whether to pass an `election_event_id` parameter at all.

### Options

> **Recommended: Option C — see below.**

**Option A — Block at `create_keys_ceremony` (admin-gate)**
Check at Harvest time that every selected BBT trustee has a registered `public_key` for the
event.  Return a descriptive error (e.g. `TRUSTEE_KEY_NOT_REGISTERED`) if any key is missing
so the admin portal can display:

> *"All parties must be present — some trustee users have not signed in yet."*

Pro: immediate, visible feedback to the admin.
Con: `create_keys_ceremony` currently has no key-reading logic; this mixes concerns.

**Option B — Early-exit from `create_keys_impl` (silent retry)**
`create_keys_ceremony` creates the record normally (`STARTED`).  `create_keys_impl` checks
whether every BBT trustee in the ceremony has a `public_key`; if not, it returns `Ok(())`
without advancing the status.  On the next beat cycle `process_board` dispatches `create_keys`
again and the check is retried automatically.

Pro: no new status needed; ceremony advances on its own once trustees are ready.
Con: ceremony appears stuck in `STARTED` with no UI feedback; admin cannot distinguish
"waiting for trustees" from a genuine failure.

**Option C — Rename `USER_CONFIGURATION` to `AWAITING_TRUSTEE_KEYS` ✓ Recommended**
`USER_CONFIGURATION` exists in the enum today but is unused and carries a generic name.
Rename it to `AWAITING_TRUSTEE_KEYS` to give it a clear, purpose-built meaning.
`create_keys_ceremony` inserts the record with `AWAITING_TRUSTEE_KEYS`.  `process_board` gains
a new arm: for ceremonies in `AWAITING_TRUSTEE_KEYS`, check whether all BBT trustees have a
registered `public_key`; if so, advance to `STARTED`.  Once in `STARTED` the existing
`create_keys` path runs unchanged.  The enum becomes:

```rust
pub enum KeysCeremonyExecutionStatus {
    AWAITING_TRUSTEE_KEYS,   // renamed from USER_CONFIGURATION: waiting for BBT keys
    STARTED,                 // ready for create_keys
    IN_PROGRESS,             // DKG running
    SUCCESS,
    CANCELLED,
}
```

Pro: repurposes the existing unused variant with no net change to the enum size; intent is
explicit; the UI can show a distinct "waiting for keys" state; transition table is enforced at
runtime via a `try_transition` guard (see below).
Con: any code that already serialises/stores `USER_CONFIGURATION` as a string must be
migrated (currently there is none — the variant is unused).

### Runtime state machine

The transition table is enforced at **runtime** via a `try_transition` method added to
`KeysCeremonyExecutionStatus` in `sequent-core`.  This is the correct tool here: the status
is a string in the DB, reconstructed on every service call — there is no long-lived typed
value for a compile-time typestate pattern to protect.

See [Keys Ceremony State Machine](./key_ceremony_state_machine.md) for the full design,
implementation code, tests, and call-site table.

---

## Trustee Online Presence

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

## TrusteeStatus Lifecycle

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

## Cancellation Window

### Keys ceremony — not currently implemented

`KeysCeremonyExecutionStatus::CANCELLED` is defined in the enum in `sequent-core` but is
**dead code**: no Harvest endpoint, no Windmill task, and no service function ever writes it.
Keys ceremony cancellation does not exist today.

A future implementation would require at minimum:

- A new Harvest endpoint (e.g. `POST /cancel-keys-ceremony`) that validates the current
  `execution_status` is not already `CANCELLED` and that the election has not been started yet, then writes `CANCELLED`.
  It should be allowed to cancel on `SUCCESS` but only if the voting period has not been started.
- A decision on what to do with the B4 board if DKG was already `IN_PROGRESS`: the board
  already holds `Channel`, `Shares`, and possibly `PublicKey` messages.  These cannot be
  deleted from an append-only board, so a cancelled ceremony would leave orphaned board
  messages.  The admin would need to create a fresh ceremony on a new board.
- For BBTs: a decision on whether to delete the `(trustee_id, election_event_id)` rows from
  `sequent_backend_trustee` on cancellation, or leave them so the trustee's keys are already
  registered if a replacement ceremony is created immediately after.

Valid transitions to add (mirroring tally ceremony):
```
AWAITING_TRUSTEE_KEYS → CANCELLED
STARTED               → CANCELLED
IN_PROGRESS           → CANCELLED
SUCCESS               → (terminal, no cancellation)
```

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

## Flow

```
Admin creates ceremony (selects trustees)
  → create_keys_ceremony inserts record with execution_status: AWAITING_TRUSTEE_KEYS
                    ↓
BBT trustee opens election event in admin portal  ← can happen before or after ceremony creation
  └─ HeadlessTrusteeProvider mounts
  └─ WASM silently generates trustee keys (once, if not already in localStorage):
         - Ed25519 signing keypair  (StrandSignatureSk::generate())
         - AES-256 symmetric key    (symm::gen_key())
       All three values stored in localStorage keyed by election_event_id
       (localStorage persists across tab closures and browser restarts; keys are only
        lost if the trustee explicitly clears site data or the device/profile is lost)
  └─ signing_key_pk → POST Harvest /register-trustee-key
                       UPSERT (trustee_id, election_event_id) → public_key in DB
                    ↓
beat service (review_boards → process_board) polls each ceremony in AWAITING_TRUSTEE_KEYS:
  checks whether every selected BBT trustee has a public_key for this election_event_id in DB
  if any key is still missing → stays in AWAITING_TRUSTEE_KEYS, retried on next beat cycle
  once all keys are present  → execution_status: STARTED
                    ↓
Windmill create_keys (dispatched by process_board when STARTED):
  reads trustee.public_key WHERE election_event_id = $1 (BBT) or no filter (server-based)
  → posts Configuration to B4
  → execution_status: IN_PROGRESS
                    ↓
HeadlessTrusteeProvider reads signing_key_sk + encryption_key from localStorage
  → WasmSession runs DKG automatically in background:
       Channel → ChannelsAllSigned → Shares → PublicKey → PublicKeySigned
                    ↓
allTrusteesGenerated: all trustees have a PublicKey or PublicKeySigned message on the board
  → Windmill set_public_key matches board messages against trustee.public_key from DB
  → each trustee status: KEY_GENERATED
                    ↓
[Existing Download Step — repurposed for identity key]
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

> **Note on the Download step:** the trustee downloads a backup of their **identity keys** —
> `{ signing_key_sk, signing_key_pk, encryption_key }` — the browser equivalent of
> `trustee1.toml`.  This is **not** the DKG private key fragment (`TrusteeShareData`).
> There is no `/get-private-key` download in the BBT flow.  The identity key backup is the
> only file the trustee needs to save, and it must also be uploaded during the tally ceremony
> to identify the trustee on the board.

---

## Security: localStorage Key Storage

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

## Key Loss: Recovery from Cleared localStorage

`localStorage` is **persistent storage** — it survives tab closures, page refreshes, and
browser restarts.  Closing the tab mid-ceremony does not lose the keys; the trustee simply
reopens the admin portal and the `HeadlessTrusteeProvider` finds the keys already in
`localStorage` and continues normally.

Keys are only lost in two exceptional situations:
- The trustee explicitly clears site data (browser settings, developer tools, or a
  deliberately triggered key rotation).
- The device itself is lost or the browser profile is deleted.

Two sub-cases for explicit loss:

**Before `Configuration` is posted** (ceremony still in `AWAITING_TRUSTEE_KEYS` or `STARTED`):
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

---

## Component Changes

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

### 2 — DB schema: add `election_event_id` to `sequent_backend_trustee`

Add a nullable `election_event_id` column:

| Column | Type | Meaning |
|---|---|---|
| `election_event_id` | `uuid \| null` | `null` for server-based trustees; event UUID for BBT |
| `public_key` | `text \| null` | Ed25519 public key scoped to this (trustee, event) pair |

Server-based trustees keep `election_event_id = null` and `public_key` set once at provisioning
time — no behaviour change.  BBT trustees upsert `(trustee_id, election_event_id) → public_key`
on every session start.

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

## `trustee.public_key` Column

The column is **retained and extended** with the companion `election_event_id` column.  It
remains the single source of truth for the public key in this approach.
