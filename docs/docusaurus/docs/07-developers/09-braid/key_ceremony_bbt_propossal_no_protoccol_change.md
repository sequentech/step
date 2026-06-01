---
id: key_ceremony_bbt_propossal_no_protoccol_change
title: BBT Key Ceremony — Signing Keypair Proposal (No Protocol Change)
sidebar_label: BBT Signing Keypair (No Protocol Change)
---

# BBT Signing Keypair — No Protocol Change

This document describes the lower-risk approach to replacing the hardcoded signing keypair in
`HeadlessTrusteeProvider`: generate the key in WASM, store it in `localStorage`, and register
the public key in the existing `sequent_backend_trustee` DB table via a new Harvest endpoint.
The B4 protocol and all DKG message types remain unchanged.

See [BBT Protocol Change Proposal](./key_ceremony_bbt_propossal_protoccol_change.md) for the
alternative design that removes the DB dependency entirely.

See [Key Ceremony Design](./key_ceremony_design.md) for the current architecture baseline.

---

## The Per-Event Key Problem

A BBT trustee may participate in multiple election events simultaneously.  A single global
`public_key` per trustee row would be overwritten by whichever event registered last, breaking
DKG verification on any previously started ceremony.

This approach solves it by adding an `election_event_id` column so each
`(trustee, election_event)` pair has its own independent public key.

---

## Ceremony Creation Gate

If a BBT trustee has not yet opened the election event page, `localStorage` is empty and no
entry exists in the DB for `(trustee_id, election_event_id)`.

The admin/Windmill does not allow creating the ceremony if any BBT trustee has no key registered
for this event yet or is offline.  The admin confirms all trustees are online before creating
the ceremony.  This is sufficient for attended ceremonies where all trustees are expected to be
logged in before the admin proceeds.

---

## Trustee Online Presence

For the admin to confirm that all BBT trustees are online before creating the ceremony, each
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

## Cancellation Window

A ceremony can be cancelled at any point up until the Election Event starts.  Once the election
is open for voting, cancellation is no longer permitted.

---

## Flow

```
BBT trustee opens election event in admin portal
  └─ HeadlessTrusteeProvider mounts
  └─ WASM silently generates trustee keys (once, if not already in localStorage):
         - Ed25519 signing keypair  (StrandSignatureSk::generate())
         - AES-256 symmetric key    (symm::gen_key())
       All three values stored in localStorage keyed by election_event_id
       (localStorage survives page refresh; keys are only lost when explicitly cleared
        or when the trustee deliberately rotates them)
  └─ signing_key_pk → POST Harvest /register-trustee-key
                       UPSERT (trustee_id, election_event_id) → public_key in DB
                    ↓
Admin portal confirms all BBT trustees are online and have registered keys
Admin creates ceremony (selects trustees) — blocked if any BBT trustee is missing or offline
                    ↓
allTrusteesRegistered: all selected trustees have a public_key for this election_event_id in DB
  → Windmill create_keys reads trustee.public_key WHERE election_event_id = $1
  → posts Configuration to B4
  → ceremony status: IN_PROGRESS
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

## Open Question: Browser Closed Mid-Ceremony

If the trustee closes the browser tab during the key ceremony, `localStorage` is cleared and
the generated keys are lost.  When they reopen the admin portal, the keys are gone.

Two sub-cases:

**Before `TrusteeKeyChecked` is posted** (key generation step not yet completed):
The public key has not yet been committed to the board, so the trustee can simply regenerate
a fresh keypair, re-register the new public key in the DB, and resume.  The ceremony has not
yet used the old key for anything.

**After `TrusteeKeyChecked` is posted** (key generation completed, DKG in progress or waiting):
The trustee's public key is already embedded in the `Configuration` message on the board (or
committed in `TrusteeKeyChecked` for the protocol-change variant).  Regenerating would produce
a different keypair that no longer matches what is on the board — the trustee would be unable
to post valid DKG messages.

**Recovery path:** the trustee can re-upload their downloaded identity key backup file.  The
browser reads `signing_key_sk`, `signing_key_pk`, and `encryption_key` from the file, writes
them back into `localStorage`, and the `HeadlessTrusteeProvider` resumes the session with
the original keys.

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
