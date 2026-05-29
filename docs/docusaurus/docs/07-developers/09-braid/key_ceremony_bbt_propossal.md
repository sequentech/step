---
id: key_ceremony_bbt_propossal
title: BBT Key Ceremony — Signing Keypair Proposal
sidebar_label: BBT Signing Keypair Proposal
---

# BBT Task: Generate Ed25519 Signing Keypair in WASM

This document details the implementation plan for the first design gap identified in
[Key Ceremony Design](./key_ceremony_design.md): replacing the hardcoded signing keypair in
`HeadlessTrusteeProvider` with one generated inside WASM and made available to Windmill so it
can embed the correct public key in the `Configuration` message.

---

## Current State

The WASM session lifecycle has already been extracted from `useHeadlessTrustee` into a new
`HeadlessTrusteeProvider` mounted at the election event level.  Key properties of this provider:

- Initialises WASM and connects to the board as soon as the trustee views the election event
  (not on wizard entry), so the session is ready before the ceremony wizard opens.
- Runs a background heartbeat; pauses it when `useHeadlessTrustee` acquires exclusive control
  during the wizard, preventing concurrent access to the single `WasmSession` object.
- Frees the session when the trustee navigates away from the election event.
- Still carries the hardcoded `signing_key_sk` / `encryption_key` (marked WIP) — replacing these
  is the subject of this task.

The provider is the right place to generate and consume the keypair since it is scoped to one
election event and initialises early.

---

## The B4 Board Lifetime

A critical piece of context for both options: **the tally ceremony uses the same B4 board as the
key ceremony.**  Both `execute_tally_session` and `insert_ballots` resolve the board name by
calling `get_keys_ceremony_board()` — the identical function used by `create_keys` and
`set_public_key`.

The full message sequence on a single board is:

```
Key ceremony:   Configuration → Channel → ChannelsAllSigned → Shares → PublicKey → PublicKeySigned
                                           (same board, appended later)
Tally ceremony: Ballots → Mix → DecryptionFactors → Plaintexts
```

This means the `Configuration` message — which already contains all trustee public keys — is
present on the board before the tally begins.  Any code that needs trustee public keys during
the tally can read them directly from the `Configuration` on the board rather than from the DB.

---

## Where `trustee.public_key` Is Used Today

Before choosing an approach it is important to know every consumer of `trustee.public_key` so we
can decide whether the DB column can be dropped in the new design.

There are exactly **four consumers**:

| Consumer | File | What it does |
|---|---|---|
| `create_keys` Celery task | `windmill/src/tasks/create_keys.rs:54` | Reads each trustee's public key and passes it to `add_config_to_board`, which embeds it in the `Configuration` message |
| `set_public_key` Celery task | `windmill/src/tasks/set_public_key.rs:43` | Reads each trustee's public key and matches it against board messages to determine `KEY_GENERATED` trustee status |
| `get-private-key` / `check-private-key` (Harvest → Windmill) | `windmill/src/services/ceremonies/keys_ceremony.rs:117` | Reads the trustee's public key to locate their `Channel` message on the board (the encrypted private key fragment is identified by the sender's public key) |
| `insert_ballots` (tally) | `windmill/src/services/ceremonies/insert_ballots.rs:67` | Reads trustee public keys to call `generate_trustee_set()`, which identifies the participating trustees in the tally `Ballots` message — same pattern as `create_keys` |

The last consumer (`insert_ballots`) deserves special attention: it reads `trustee.public_key`
from the DB and then immediately uses those keys to match against the board's `Configuration`
message (which already contains them).  This is a **redundant DB lookup** — the information is
already on the board it is about to write to.  Both `create_keys` and `insert_ballots` could
read trustee public keys from the board's `Configuration` instead of the DB, which is exactly
what Option B exploits.

---

## The Per-Event Key Problem

A BBT trustee may participate in multiple election events simultaneously.  If the signing key is
stored as a single global value per trustee in the DB, registering a fresh key for event B
overwrites the key already embedded in event A's `Configuration` on the board, and event A's DKG
message verification breaks permanently.

Both options below solve this by scoping the key to a `(trustee, election_event)` pair.

---

## Option A — Register via DB (lower risk, more moving parts)

The public key is generated in the browser and stored in the DB via a new Harvest endpoint.
Windmill continues to read it from the DB, exactly as it does today.

### Flow

```
BBT trustee opens election event in admin portal
  └─ HeadlessTrusteeProvider mounts
  └─ WASM generates keypair for this election_event_id
  └─ private key → sessionStorage['bbt_signing_key_sk_{election_event_id}']
  └─ public key  → POST Harvest /register-trustee-key
                    UPSERT (trustee_id, election_event_id) → public_key in DB
                    ↓
Admin creates ceremony (selects trustees)
                    ↓
Windmill create_keys reads trustee.public_key WHERE election_event_id = $1
  → builds Configuration with correct per-event public key
                    ↓
HeadlessTrusteeProvider reads signing_key_sk from sessionStorage
  → WasmSession signs DKG messages with matching private key
                    ↓
Windmill set_public_key reads same trustee.public_key from DB
  → matches against board messages to set KEY_GENERATED status
```

### Component Changes

#### 1 — Braid-wasm: new `generate_signing_keypair()` export

Add a WASM-exported function in `braid/src/wasm/mod.rs`:

```
pub fn generate_signing_keypair() -> JsValue
  returns { signing_key_sk: string, signing_key_pk: string }
```

Both values are base64-DER encoded Ed25519 — the same format consumed by `WasmSession`.
Reuses `StrandSignatureSk` already present in braid.

#### 2 — DB schema: add `election_event_id` to `sequent_backend_trustee`

Add a nullable `election_event_id` column:

| Column | Type | Meaning |
|---|---|---|
| `election_event_id` | `uuid \| null` | `null` for server-based trustees; event UUID for BBT |
| `public_key` | `text \| null` | Ed25519 public key scoped to this event |

Server-based trustees keep `election_event_id = null` — no behaviour change.
BBT trustees upsert `(trustee_id, election_event_id) → public_key` on every session start.

#### 3 — `HeadlessTrusteeProvider`: generate and register on mount

Inside the existing `initialize` effect, before constructing `WasmSession`:

1. Check `sessionStorage` for `bbt_signing_key_sk_{electionEventId}`.
2. If absent: call `generate_signing_keypair()`, store both halves in `sessionStorage`, POST
   public key to Harvest `/register-trustee-key`.
3. If present: re-POST on page refresh to keep DB in sync.
4. Pass `signing_key_sk` / `signing_key_pk` from `sessionStorage` into `WasmSession`.

#### 4 — Harvest: new `POST /register-trustee-key` endpoint

```
Input:  { public_key: string, election_event_id: string }
Auth:   existing Keycloak JWT
Action: UPSERT sequent_backend_trustee
          SET public_key = $public_key
          WHERE trustee_id = <from JWT> AND election_event_id = $election_event_id
```

#### 5 — Windmill `get_trustees_by_id`: filter by `election_event_id`

```sql
SELECT public_key FROM sequent_backend_trustee
WHERE id = $trustee_id
  AND (election_event_id = $event_id OR election_event_id IS NULL)
ORDER BY election_event_id NULLS LAST
LIMIT 1
```

`set_public_key` calls `get_trustees_by_name` with the same logic — same change applies there.

### Can `trustee.public_key` be dropped with Option A?

No.  The column is still required for both Windmill tasks and for the provider.  It gains a
companion `election_event_id` column but remains the single source of truth for the public key.

### Trustee not logged in at ceremony creation

If no entry exists for `(trustee_id, election_event_id)` when `create_keys` runs, Windmill
finds no public key and will either silently skip the trustee or fail.

Recommended mitigation: admin portal shows a "not ready" indicator next to any BBT trustee who
has no key registered for this event yet.  The admin confirms all trustees are online before
creating the ceremony.

---

## Option B — Register via B4 (cleaner design, more B4 changes required)

The public key is posted directly to the B4 board as a new protocol step, before the
`Configuration` message.  Windmill reads it from the board rather than the DB.  The
`trustee.public_key` DB column becomes unnecessary for BBT trustees.

### Core Idea

Each trustee — BBT or server-based — announces its signing public key to the board by posting a
signed `TrusteeAnnouncement` message.  Because the message is signed with the private key, the
signature already proves ownership.  Windmill waits until all expected trustees have posted their
announcement, then builds and posts the `Configuration` using the keys it found on the board.

### Flow

```
Windmill (ceremony created, status STARTED)
  └─ Create board (explicit board-init step, before any trustee connects)
  └─ Ceremony stays in STARTED, waiting for trustee announcements
                    ↓
Each BBT trustee opens election event in admin portal
  └─ HeadlessTrusteeProvider mounts
  └─ WASM generates keypair for this election_event_id
  └─ private key → sessionStorage['bbt_signing_key_sk_{election_event_id}']
  └─ public key  → POST TrusteeAnnouncement message to B4 (signed with private key)

Each server-based trustee daemon starts
  └─ braid-native connects to board
  └─ POST TrusteeAnnouncement message to B4 (signed with static key from trustee.toml)
                    ↓
Windmill create_keys polls board
  └─ Checks whether all expected trustees have posted TrusteeAnnouncement
  └─ If not: return early, Celery retries
  └─ If yes: read public keys from announcements → build Configuration → post to B4
                    ↓
Ceremony advances to IN_PROGRESS
DKG rounds proceed as normal
                    ↓
Windmill set_public_key
  └─ Reads Configuration from board (already has all trustee public keys embedded)
  └─ Matches PublicKey/PublicKeySigned messages against keys in Configuration
  └─ No DB lookup needed
```

### New B4 Message Type: `TrusteeAnnouncement`

A minimal new statement type is needed:

```
Statement::TrusteeAnnouncement(timestamp, trustee_name)
```

The artifact is the trustee name (or empty — the sender public key is already in `Message.sender.pk`).
The message is self-authenticating: a valid signature over the statement proves the signer holds the
private key matching `sender.pk`.

Changes required:
- `b4/src/messages/statement.rs` — add `TrusteeAnnouncement` variant
- `b4/src/messages/message.rs` — add `announcement_msg()` constructor
- `b4/src/handlers.rs` — accept and store the new message type
- Braid Datalog rules — BBT: add `PostAnnouncement` action triggered by board initialisation;
  braid-native: same action triggered on startup

### Board Pre-creation

Currently the board is implicitly created when Windmill posts the first message via direct SQL.
Option B requires the board to exist before any trustee connects.

Proposed: Windmill creates the board (empty) as part of processing the `create_keys_ceremony`
request (in Harvest, after inserting the `keys_ceremony` record) or as the first action in the
`create_keys` task before returning.  The board can be created via the same `PgsqlB3Client`
without posting any message yet.

### Windmill `create_keys`: wait for all announcements

Replace the current unconditional `add_config_to_board` call with:

```
1. Read all TrusteeAnnouncement messages from the board
2. Collect the set of trustee names that have announced
3. Compare against the expected trustee list from the ceremony record
4. If incomplete: return Ok(()) — Celery retries the task
5. If complete: build Configuration from announced public keys, post it
```

The ceremony stays in `STARTED` during this waiting period.  No new Windmill infrastructure is
needed beyond Celery's existing retry mechanism.

### Windmill `set_public_key`: read keys from Configuration

Instead of calling `get_trustees_by_name` and reading `trustee.public_key`, read the
`Configuration` artifact from the board (already fetched in `get_board_public_key_messages`).
The Configuration contains all trustee public keys.  Match `PublicKey` / `PublicKeySigned`
messages against those keys directly.

### Can `trustee.public_key` be dropped with Option B?

**For all four consumers: yes**, once Option B is fully implemented:

- `create_keys` — reads public keys from `TrusteeAnnouncement` messages on the board instead of the DB.
- `set_public_key` — reads public keys from the `Configuration` message on the board (already embedded there by `create_keys`).
- `get-private-key` / `check-private-key` — the trustee's `Channel` message is identified by matching against the `Configuration` on the board, not the DB.
- `insert_ballots` — currently a redundant DB lookup: it reads keys from the DB only to match them against the board's `Configuration`, which already has them.  It can read directly from the `Configuration` instead.

Since the key ceremony and tally share the same board, the `Configuration` is guaranteed to be
present before `insert_ballots` runs.  No additional board reads are needed.

**For server-based trustees:** the column can remain `null` or be removed.  Braid-native would
post a `TrusteeAnnouncement` on startup (required for hybrid ceremonies anyway), making the
column fully redundant for all trustee types.  The column can be deprecated and eventually
dropped in a follow-up migration.

### Feasibility Assessment

| Concern | Assessment |
|---|---|
| New B4 message type | Low complexity — adds one variant to `Statement`; no change to existing message validation logic |
| Board pre-creation | Low complexity — one extra SQL call in `create_keys` or Harvest |
| Windmill polling | Low complexity — Celery retries already exist; just add an early-return guard |
| braid-native announcement | Medium — braid-native needs a new startup action in its Datalog rules |
| Hybrid ceremonies (mixed server/BBT) | Works naturally: both types post announcements; Windmill waits for all |
| Stale key on reconnect (append-only board) | The first announcement per trustee name wins; reconnections are rejected by the Datalog rule (action only fires once) |
| Tally ceremony (`insert_ballots`) | No extra work — tally uses the same board; `Configuration` is already present; `insert_ballots` can read from it directly, eliminating a redundant DB lookup |
| Security | Strictly better than Option A — the public key is on the tamper-evident log, not a mutable DB column |

---

## Comparison

| | Option A | Option B |
|---|---|---|
| DB schema change | Yes — add `election_event_id` column | No |
| New Harvest endpoint | Yes — `/register-trustee-key` | No |
| New B4 message type | No | Yes — `TrusteeAnnouncement` |
| braid-native change | No | Yes — post announcement on startup |
| `trustee.public_key` still needed | Yes (for DKG) | No (can be dropped for BBT) |
| Windmill task changes | Query filter only | Add announcement-wait loop |
| Self-contained audit trail | No — key in DB, not on board | Yes — key on append-only board |
| Risk | Lower — stays within existing patterns | Medium — touches B4 and braid protocol |

Option A is the lower-risk path and can ship first.  Option B is the architecturally cleaner
design and should be the target once BBT is stable.
