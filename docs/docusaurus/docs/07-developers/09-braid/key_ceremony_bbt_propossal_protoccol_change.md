---
id: key_ceremony_bbt_propossal
title: BBT Key Ceremony — Redesigned Protocol Proposal
sidebar_label: BBT Protocol Proposal
---

# BBT Key Ceremony — Redesigned Protocol Proposal

This document proposes a full redesign of the key ceremony protocol to support
Browser-Based Trustees (BBT).  The redesign removes `public_key` from the trustee DB table
entirely and manages trustee identity exclusively through new B4 board messages, making the
protocol self-contained and auditable.

See [BBT Signing Keypair — No Protocol Change](./key_ceremony_bbt_propossal_no_protoccol_change.md)
for the lower-risk interim design.  The `crypto-migration` branch supersedes and extends that
design: the per-ceremony DB row it introduces is the direct stand-in for the on-board
trustee registration message defined here, making the migration mechanical once the interim
path is stable.

See [Key Ceremony Design](./key_ceremony_design.md) for the current architecture baseline.

---

## Context: The B4 Board Lifetime

**The tally ceremony uses the same B4 board as the key ceremony.**  Both `execute_tally_session`
and `insert_ballots` resolve the board name by calling `get_keys_ceremony_board()` — the
identical function used by `create_keys` and `set_public_key`.

The full message sequence on a single board is:

```
Key ceremony:   Configuration → Channel → ChannelsAllSigned → Shares → PublicKey → PublicKeySigned
                                           (same board, appended later)
Tally ceremony: Ballots → Mix → DecryptionFactors → Plaintexts
```

---

## Context: Why a Redesign

Two findings motivate the change:

1. **`trustee.public_key` has four consumers** (see table below), all in Windmill.  Every one of
   them can be replaced by reading from the B4 board, which is the authoritative record anyway.

2. **The tally ceremony uses the same B4 board** as the key ceremony.  The `Configuration`
   message — containing all trustee public keys — is already on the board before the tally runs.
   `insert_ballots` currently does a redundant DB lookup to get keys that are already on the
   board it is about to write to.

| Current consumer of `trustee.public_key` | File | Replacement in new design |
|---|---|---|
| `create_keys` | `windmill/src/tasks/create_keys.rs:54` | Read from new `TrusteeKeyChecked` messages on the board |
| `set_public_key` | `windmill/src/tasks/set_public_key.rs:43` | Read from `Configuration` (already on board) |
| `get-private-key` / `check-private-key` | `windmill/src/services/ceremonies/keys_ceremony.rs:117` | Read from `Configuration` (already on board) |
| `insert_ballots` (tally) | `windmill/src/services/ceremonies/insert_ballots.rs:67` | Read from `Configuration` (already on board) |

---

## New Protocol: Full Message Sequence

The board for a ceremony now has a **pre-DKG phase** in which trustees establish and verify
their identities before the cryptographic protocol begins.

```
PHASE 0 — Board Initialisation (Windmill / Admin)

  Admin creates ceremony in Admin Portal
    ↓
  Windmill: create board (empty)
  Windmill: POST TrusteesList message
    └─ contains: list of trustee names (no public keys — table no longer stores them)
    └─ ceremony status: STARTED
    ↓
  Admin UI blocks, showing each trustee's connection status
  Waiting until all trustees are connected to the board

─────────────────────────────────────────────────────────────────────────

PHASE 1 — Trustee Key Generation (one per trustee, explicit UI steps)

  Trustee user logs into Admin Portal → opens the key ceremony
    ↓
  [Wizard Step: Generate Keys]
    Trustee clicks "Generate Keys" / "Next"
    WASM generates (equivalent of gen_trustee_config.rs for server-based trustees):
      - Ed25519 signing keypair  (StrandSignatureSk::generate() + StrandSignaturePk::from_sk())
      - AES-256 symmetric key    (symm::gen_key())
    These are the same three values stored in trustee1.toml for server-based trustees,
    now generated in the browser and held only in sessionStorage for the session lifetime.
    Keys stored in sessionStorage keyed by election_event_id
    Trustee: POST TrusteeKeyGenerated message to B4
      └─ signed with signing_key_sk
      └─ artifact: signing_key_pk (public key announcement)
    ↓
  [Wizard Step: Download Backup]
    Trustee downloads a backup file containing the trustee identity keys:
      { signing_key_sk, signing_key_pk, encryption_key }
    Trustee: POST TrusteeKeyDownloaded message to B4
    ↓
  [Wizard Step: Upload and Verify]
    Trustee re-uploads the downloaded backup file
    Integrity check: compare against values in sessionStorage
    Trustee: POST TrusteeKeyChecked message to B4
      └─ signed with signing_key_sk
      └─ artifact: signing_key_pk  ← this is the authoritative public key commitment

─────────────────────────────────────────────────────────────────────────

PHASE 2 — Configuration (Windmill / Admin)

  Windmill polls board
    └─ Checks all trustees in TrusteesList have a TrusteeKeyChecked message
    └─ If incomplete: Celery retry, admin UI shows remaining trustees
    └─ If complete:
         Windmill: POST AllTrusteesPublishedKey message
         Windmill: POST Configuration message
           └─ trustee public keys read from TrusteeKeyChecked messages on board
           └─ (not from sequent_backend_trustee table)
         ceremony status: IN_PROGRESS

─────────────────────────────────────────────────────────────────────────

PHASE 3 — DKG (unchanged)

  Trustees run existing braid protocol:
    Channel → ChannelsAllSigned → Shares → PublicKey → PublicKeySigned

─────────────────────────────────────────────────────────────────────────

PHASE 4 — Tally (unchanged, reads from board)

  insert_ballots reads trustee public keys from Configuration (already on board)
  Ballots → Mix → DecryptionFactors → Plaintexts
```

---

## New B4 Message Types

Five new message types are introduced, all in Phase 0 and Phase 1.  The existing
`Configuration` message and all DKG messages are unchanged.

| Message | Posted by | When | Contains |
|---|---|---|---|
| `TrusteesList` | Windmill | Ceremony starts | Ordered list of trustee names |
| `TrusteeKeyGenerated` | Each trustee | After clicking "Generate Keys" | Trustee's signing public key (announcement) |
| `TrusteeKeyDownloaded` | Each trustee | After downloading backup | Trustee name (no key material) |
| `TrusteeKeyChecked` | Each trustee | After successful upload/verify | Trustee's signing public key (authoritative commitment, signed) |
| `AllTrusteesPublishedKey` | Windmill | All trustees have `TrusteeKeyChecked` | List of (trustee name, public key) pairs |

`TrusteeKeyGenerated` and `TrusteeKeyChecked` are both signed with `signing_key_sk`.  Because
the message is signed with the private key, the signature proves the signer owns the matching
`signing_key_pk` — no separate proof of knowledge is needed.  `TrusteeKeyChecked` is the
authoritative commitment; `TrusteeKeyGenerated` is informational (allows the admin to track
per-trustee progress before download/verify).

---

## New UI Wizard Steps

### Admin view

The existing ceremony step gains:
- **Waiting for trustees** state after posting `TrusteesList` — shows each trustee's connection
  status (already partially implemented with the connection-status dots in `CeremonyStep`)
- **All trustees ready** indicator when `AllTrusteesPublishedKey` is on the board, before
  Windmill posts `Configuration`

### Trustee view (three new explicit steps before the existing DKG steps)

| Step | Action | B4 message posted |
|---|---|---|
| **Generate Keys** | Trustee clicks "Generate Keys"; WASM generates all three keys; stored in `sessionStorage` | `TrusteeKeyGenerated` |
| **Download Backup** | Trustee downloads encrypted key bundle | `TrusteeKeyDownloaded` |
| **Verify Backup** | Trustee re-uploads file; integrity check | `TrusteeKeyChecked` |

After `TrusteeKeyChecked` is confirmed, the trustee waits for `AllTrusteesPublishedKey` before
the DKG begins automatically.

---

## Component Changes

### Braid-wasm: new `generate_trustee_keys()` export

Add a WASM-exported function in `braid/src/wasm/mod.rs` that mirrors `gen_trustee_config.rs`:

```
pub fn generate_trustee_keys() -> JsValue
  returns {
    signing_key_sk:  string,   // StrandSignatureSk::generate() → base64-DER PKCS#8 Ed25519 private key
    signing_key_pk:  string,   // StrandSignaturePk::from_sk()  → base64-DER Ed25519 public key
    encryption_key:  string,   // symm::gen_key()               → base64 AES-256 symmetric key
  }
```

Produces the same three values found in `trustee1.toml`.  All underlying functions are already
present in braid — no new crypto dependency.

### B4: five new `Statement` variants

In `b4/src/messages/statement.rs`:

```rust
Statement::TrusteesList(timestamp, trustee_names_hash)
Statement::TrusteeKeyGenerated(timestamp, trustee_name, signing_pk_hash)
Statement::TrusteeKeyDownloaded(timestamp, trustee_name)
Statement::TrusteeKeyChecked(timestamp, trustee_name, signing_pk_hash)
Statement::AllTrusteesPublishedKey(timestamp, trustees_pks_hash)
```

Corresponding message constructors in `b4/src/messages/message.rs` and handler registration
in `b4/src/handlers.rs`.

### `HeadlessTrusteeProvider` / Trustee Wizard

The provider no longer generates keys silently in the background.  Instead:

1. On mount it connects to the board and reads current state (to detect which step the trustee
   is on if they reconnect mid-ceremony).
2. Key generation is triggered explicitly from the wizard UI (user clicks "Generate Keys").
3. The WASM `generate_trustee_keys()` call, `sessionStorage` write, and `TrusteeKeyGenerated`
   B4 post all happen as part of the wizard step transition, not on provider mount.
4. Download and verify steps post `TrusteeKeyDownloaded` and `TrusteeKeyChecked` respectively.
5. The existing `WasmSession` construction (with `signing_key_sk` from `sessionStorage`) happens
   after `TrusteeKeyChecked` is confirmed, immediately before the DKG rounds begin.

### Windmill `create_keys` task

Replace the current unconditional `add_config_to_board` call:

```
1. Create board (if not yet created)
2. POST TrusteesList message (trustee names from ceremony record)
3. Return — ceremony stays in STARTED

On retry (Celery):
4. Read all TrusteeKeyChecked messages from board
5. If not all trustees have checked: return early (retry)
6. If complete:
   POST AllTrusteesPublishedKey
   Build Configuration using public keys from TrusteeKeyChecked messages
   POST Configuration
   Update ceremony status → IN_PROGRESS
```

The `sequent_backend_trustee.public_key` column is not read at any point.

### Windmill `set_public_key` task

Replace `get_trustees_by_name` + `trustee.public_key` lookup:

```
Read Configuration artifact from board
  → extract Vec<StrandSignaturePk> (trustee public keys)
Match PublicKey / PublicKeySigned messages against those keys
  → determine TrusteeStatus::KEY_GENERATED
```

### Windmill `get-private-key` / `check-private-key`

Replace `get_trustee_by_name` + `trustee.public_key` lookup:

```
Read Configuration from board
  → find this trustee's public key by trustee name
Use that key to locate the trustee's Channel message
```

### Windmill `insert_ballots` (tally)

Replace `get_trustees_by_name` + `trustee.public_key` lookup:

```
Read Configuration from board (already present — same board as key ceremony)
  → extract Vec<StrandSignaturePk>
Pass to generate_trustee_set() directly
```

No new board reads required; the `Configuration` is fetched later in the same function anyway.

---

## `trustee.public_key` Column

With this design, `sequent_backend_trustee.public_key` is **not written or read** for any DKG
or tally operation.  It can be:

- Set to `null` for all trustees immediately (migration)
- Removed from the `Trustee` struct and DB schema in a follow-up

The rest of the `sequent_backend_trustee` table (`id`, `name`, `tenant_id`, `annotations`,
`labels`) remains in use for ceremony configuration, tally UI, user dropdowns, and
`trustee_mode_policy`.

---

## Where `trustee.public_key` Is Used Today

There are exactly **four consumers**:

| Consumer | File | What it does |
|---|---|---|
| `create_keys` Celery task | `windmill/src/tasks/create_keys.rs:54` | Reads each trustee's public key and passes it to `add_config_to_board`, which embeds it in the `Configuration` message |
| `set_public_key` Celery task | `windmill/src/tasks/set_public_key.rs:43` | Reads each trustee's public key and matches it against board messages to determine `KEY_GENERATED` trustee status |
| `get-private-key` / `check-private-key` (Harvest → Windmill) | `windmill/src/services/ceremonies/keys_ceremony.rs:117` | Reads the trustee's public key to locate their `Channel` message on the board |
| `insert_ballots` (tally) | `windmill/src/services/ceremonies/insert_ballots.rs:67` | Reads trustee public keys to call `generate_trustee_set()`, which identifies participating trustees in the tally `Ballots` message |

`insert_ballots` is a notable case: it reads `trustee.public_key` from the DB only to match
against the board's `Configuration` (which already contains those keys) — a redundant lookup
that this approach does not fix.  The protocol-change proposal eliminates it.

---

## Feasibility Assessment

| Concern | Assessment |
|---|---|
| New B4 message types (×5) | Low–medium — adds variants to `Statement`; no change to existing DKG message validation |
| Board pre-creation | Low — one extra SQL call before posting `TrusteesList` |
| Windmill retry loop | Low — Celery retries already exist; add early-return guard |
| UI wizard steps (×3 new steps) | Medium — new React components for Generate / Download / Verify; reuses existing download/upload patterns from current `DownloadStep` / `CheckStep` |
| `HeadlessTrusteeProvider` refactor | Medium — shifts from silent background generation to explicit wizard-driven steps |
| braid-native (server-based trustees) | Must also go through the same three steps (Generate / Download / Verify) — either via a CLI flow or automated on daemon startup |
| Stale key on reconnect | First `TrusteeKeyChecked` per trustee name wins; the Datalog rule fires once and is idempotent on reconnect |
| Tally `insert_ballots` | Simplification — removes a redundant DB lookup |
| `trustee.public_key` removal | Clean — no consumers remain after migration |
