---
id: key_ceremony_design
title: Key Ceremony Design
sidebar_label: Key Ceremony Design
---

# Key Ceremony Design — Plain-English Walkthrough

A keys ceremony is how the system generates a **threshold election public key** collaboratively among
multiple trustees, so that no single party ever holds a complete decryption key.  This document
describes the process step by step from a cryptographic design perspective, explains what every
component is responsible for, and maps the current design to the Browser-Based Trustee (BBT) model.

---

## Background: What a Keys Ceremony Produces

At the end of a keys ceremony every trustee holds a **private key fragment** — an encrypted bundle
that, together with fragments from a minimum quorum of other trustees (the *threshold*), can later
decrypt the election results.  The ceremony also produces one **election public key**, shared with
everyone, that voters use to encrypt their ballots.

The cryptographic primitive underneath is **Pedersen Distributed Key Generation (DKG)** over the
Ristretto255 elliptic-curve group, with EdDSA (Ed25519) signatures for authenticating every
message posted to the bulletin board.

---

## The Bulletin Board: B4

B4 is a tamper-evident append-only message board.  Every party in a ceremony posts signed messages
to it and reads the messages posted by others.  Nobody can delete or reorder messages.  B4 does
**not** perform any cryptographic computation; it is a shared notarial ledger.

Key message types used during a keys ceremony (in order of appearance):

| Message | Who posts it | What it contains |
|---|---|---|
| `Configuration` | Windmill (protocol manager) | List of trustees, their public signing keys, threshold value |
| `Channel` | Each trustee | An ElGamal public key for receiving encrypted shares, plus a zero-knowledge proof of knowledge of the matching private key |
| `ChannelsAllSigned` | Each trustee | Trustee's signature confirming all other trustees' Channel messages are valid |
| `Shares` | Each trustee | A set of polynomial commitments plus one ElGamal-encrypted secret share for every other trustee |
| `PublicKey` | Trustee 0 | The aggregated election public key derived from everyone's polynomial commitments |
| `PublicKeySigned` | Every other trustee | Each trustee's independent verification and signature of the election public key |

---

## Step-by-Step Ceremony Flow

### 1 — Admin Creates the Ceremony (UI + Windmill)

**Who acts:** Admin user in the Admin Portal.

The admin selects which trustees will participate and sets the threshold (minimum number of trustees
required to decrypt later).  The Admin Portal sends this configuration to **Harvest** (the REST
backend), which:

1. Stores the new `keys_ceremony` record in the database with status `STARTED`.
2. Grants each trustee access to the relevant B4 board in Keycloak.
3. Enqueues the `create_keys` Celery task in Windmill.

**Windmill `create_keys` task** then:

1. Generates a **protocol-manager signing keypair** (EdDSA/Ed25519) and stores it securely in
   Vault.  This key represents the administrative authority that "opens" the board.
2. Assembles a `Configuration` message listing every trustee's public signing key and the
   threshold.
3. Signs the `Configuration` with the protocol-manager key and **writes it directly into B4's
   PostgreSQL** (bypassing the HTTP API) as the first board message.
4. Updates the ceremony status to `IN_PROGRESS`.

After this point Windmill steps back.  The Configuration on the board is the starting gun; trustees
now run the DKG protocol themselves.

---

### 2 — Key Fragment Generated (Trustees run DKG on B4)

**Who acts:** Each trustee, independently and concurrently.

Each trustee runs a copy of the **braid** library — either as a native server process
(*server-based trustee*) or as a WebAssembly module inside the browser (*browser-based trustee /
BBT*).  Both modes execute identical cryptographic logic; only the runtime differs.

The DKG protocol has five rounds.  Braid implements them as a Datalog state machine: it reads the
current board state, determines which action is now valid, executes it, and posts the resulting
message to B4.

#### Round 1 — Channel Generation

Each trustee generates a fresh **ElGamal keypair** (the *channel keypair*).  This key is used
exclusively to receive encrypted shares in the next round.

- The trustee **posts** a `Channel` message to B4 containing:
  - The channel public key.
  - A Schnorr zero-knowledge proof that the trustee knows the matching channel private key
    (prevents substitution attacks).
  - The channel private key symmetrically encrypted with the trustee's own long-term encryption
    key (so the trustee can recover it later without storing it in plaintext).

#### Round 2 — Channel Verification

Each trustee reads every other trustee's `Channel` message and verifies the Schnorr proofs.  If
all proofs are valid the trustee posts a `ChannelsAllSigned` message to B4.

#### Round 3 — Share Distribution

Each trustee posts **one `Shares` message** that contains **N encrypted shares — one addressed to
each trustee** (including itself).

Concretely, trustee A:

1. Chooses a **random polynomial of degree `threshold − 1`**.  The constant term of this
   polynomial is trustee A's secret contribution to the final private key.
2. Computes **commitments** to every polynomial coefficient (points on the curve — a public
   fingerprint of each coefficient that allows verification without revealing the value).
3. For **each trustee i** (iterating over all N trustees):
   - Evaluates the polynomial at point `i + 1` to get a share value meant only for trustee `i`.
   - Looks up **trustee i's channel public key** from trustee i's `Channel` message on the board.
   - **ElGamal-encrypts the share with trustee i's channel public key.**
4. **Posts** a single `Shares` message to B4 containing the commitments and the array of N
   individually encrypted shares.

Because each share is encrypted with the **recipient's** channel public key, only the intended
trustee — who holds the matching channel private key — can decrypt their own share.  No other
party, including the sender, can read anyone else's share from the board.

Think of it as: trustee A deposits N sealed envelopes on the board, each locked with a different
trustee's padlock.  Only the padlock's owner can open theirs.

At this point the board holds one `Shares` message per trustee.  The encrypted shares are opaque
to everyone except the intended recipients, but the commitments are public and allow any observer
to verify the shares are consistent without decrypting them.

#### Round 4 — Public Key Assembly (Trustee 0)

Trustee 0 (the designated aggregator) reads all `Shares` messages from the board and:

1. Sums the **constant-term commitments** of all trustees.  This sum is the election public key
   (because the constant term of each trustee's polynomial represents their secret contribution,
   and the public key is the combined group element for all contributions).
2. Decrypts its own shares using its channel private key and verifies them against the published
   commitments (sanity check).
3. **Posts** a `PublicKey` message to B4.

#### Round 5 — Public Key Verification and Sign-off

Every other trustee independently:

1. Reads all `Shares` messages.
2. Decrypts its own encrypted share from each `Shares` message using its channel private key.
3. Verifies each decrypted share against the corresponding published commitment.
4. Independently reconstructs the election public key from the constant-term commitments.
5. Confirms it matches Trustee 0's `PublicKey` message.
6. If everything checks out, **posts** a `PublicKeySigned` message to B4.

At this point the **Key Fragment Generated** milestone is complete:
- Every trustee has verified the protocol ran honestly.
- The board contains a full auditable transcript.
- Each trustee can now reconstruct its **private key fragment** from the decrypted shares.

**What keys exist and where:**

| Key | Where it lives | Who can access it |
|---|---|---|
| Election public key | B4 board (public) | Everyone |
| Each trustee's channel private key | Encrypted inside that trustee's `Channel` message on B4 | That trustee only (symmetric decryption key never leaves the trustee) |
| Each trustee's secret polynomial shares | Encrypted inside every `Shares` message on B4 | Recipient trustee only (encrypted with channel public key) |
| Protocol-manager signing key | Vault | Windmill only |

---

### 3 — Windmill Reads Back the Public Key (`set_public_key` task)

After all `PublicKeySigned` messages are on the board, Windmill's `set_public_key` task:

1. Reads all messages from the B4 board via SQL.
2. Extracts and deserializes the `PublicKey` message.
3. Writes the base64-encoded election public key into the `keys_ceremony.status.public_key`
   database column.
4. Marks each trustee as `KEY_GENERATED` in the ceremony status.

From this moment the election public key is available to the rest of the system.

---

### 4 — Private Key Fragment Downloaded

**Who acts:** Each trustee, via the Admin Portal (or programmatically).

After the DKG rounds complete each trustee must **download and store their private key fragment**
(the encrypted bundle that lets them participate in decryption later).

The Admin Portal calls **Harvest `POST /get-private-key`**, which:

1. Verifies the calling trustee is part of the ceremony and the ceremony is `IN_PROGRESS`.
2. Fetches the trustee's own `Channel` message (contains the encrypted channel private key) and all
   `Shares` messages (contain the ElGamal-encrypted shares addressed to this trustee) from B4.
3. Packages them together into a `TrusteeShareData` bundle (a structured binary blob, Borsh-encoded
   and base64-wrapped).
4. Returns the bundle to the Admin Portal for the trustee to **download to their local storage**.
5. Updates the trustee's status to `KEY_RETRIEVED`.

The `TrusteeShareData` bundle contains everything the trustee needs to later recover their private
key fragment: the channel private key (symmetrically encrypted) plus the incoming encrypted shares
from every other trustee.  It is safe to store because the sensitive values are encrypted; only the
trustee with the right long-term encryption key can use it.

**Private Key Fragment Downloaded** milestone is complete once the trustee saves the file.

---

### 5 — Private Key Fragment Checked

**Who acts:** Each trustee, via the Admin Portal.

To confirm the downloaded file is correct and uncorrupted the trustee uploads it back through the
Admin Portal, which calls **Harvest `POST /check-private-key`**.  Harvest:

1. Re-fetches the same `Channel` and `Shares` messages from B4.
2. Re-assembles the canonical `TrusteeShareData` bundle.
3. Compares it byte-for-byte with the uploaded file.
4. If they match: updates the trustee's status to `KEY_CHECKED`.
5. When **all** trustees reach `KEY_CHECKED`, the ceremony's `execution_status` is set to
   `SUCCESS`.

**Private Key Fragment Checked** milestone is complete when the ceremony reaches `SUCCESS`.

---

## Component Responsibility Summary

| Component | Responsibility in a Keys Ceremony |
|---|---|
| **Admin Portal (UI)** | Lets admin configure and launch the ceremony; lets trustees download and verify their key fragments; polls Hasura for status updates |
| **Harvest (REST API)** | Validates calls, manages the DB record lifecycle, orchestrates Windmill tasks, serves the `/get-private-key` and `/check-private-key` endpoints |
| **Windmill (Celery)** | Posts the `Configuration` to B4 (via direct SQL); later reads back the election public key and writes it to the DB |
| **B4 (Bulletin Board)** | Stores and serves all DKG messages; enforces append-only ordering; never computes anything |
| **Braid (DKG library)** | Implements the five DKG rounds; runs as a native server process (server-based trustee) or as WebAssembly in the browser (BBT) |
| **Vault** | Stores the protocol-manager signing key |
| **Hasura / PostgreSQL** | Stores ceremony metadata, trustee status, and the final election public key |

---

## Where Each Key Is Produced

| Key | Produced by | Lives in |
|---|---|---|
| Protocol-manager signing keypair | Windmill | Vault |
| Trustee channel keypair (ephemeral, per ceremony) | Braid (trustee) | Trustee's local state; channel private key encrypted on B4 |
| Polynomial coefficients (trustee secrets) | Braid (trustee) | Never persisted — used only during round 3 to compute shares |
| Encrypted shares | Braid (trustee) | B4 board |
| Election public key | Derived by Trustee 0 from all commitments | B4 board; Windmill copies it to the DB |
| `TrusteeShareData` bundle (private key fragment) | Harvest, assembling from B4 | Trustee's local file system |

---

## Current Architecture vs. BBT (Browser-Based Trustee)

### How Server-Based Trustees Work Today

In the server-based model trustees are headless daemons (`braid-native`) running on dedicated
servers.  They authenticate against Keycloak using a machine account (`TRUSTEE_NAME` /
`TRUSTEE_PSW`), connect to B4 via HTTP, and run the DKG rounds automatically without human
interaction.  The "download" and "check" steps are manual actions performed by a human operator
who logs into the Admin Portal.

### How BBT Works

In the Browser-Based Trustee model the braid DKG logic runs as a **WebAssembly module inside the
trustee's browser tab** while they are logged into the Admin Portal.  The `useHeadlessTrustee`
React hook manages the lifecycle:

1. When the Admin Portal detects an `IN_PROGRESS` ceremony where the logged-in trustee has
   `trustee_mode_policy = BROWSER_BASED`, it loads the braid-wasm module.
2. It initialises a `WasmSession` with the trustee's signing key (Ed25519), their encryption key,
   and their B4 access token (JWT from their existing Keycloak session — no separate machine
   account needed).
3. The hook polls every second: `session.step()` runs one iteration of the Datalog DKG state
   machine, reads new messages from B4, and posts any new messages the trustee must contribute.
4. The DKG rounds (Channel → ChannelsAllSigned → Shares → PublicKey/PublicKeySigned) proceed
   automatically inside the browser.
5. Once the protocol completes the election public key is on the board.

The "download" step is the same in both models: Harvest assembles and returns the
`TrusteeShareData` bundle from B4.  The difference is that in BBT the DKG runs in the browser
automatically rather than requiring a separately deployed server daemon.

### Design Gaps to Address for Full BBT Migration

| Current assumption | Problem for BBT | Suggested change |
|---|---|---|
| **Trustee Ed25519 signing key lives in `trustee.toml` on a server** | In BBT there is no server file; the key must be generated and held in the browser for the duration of the ceremony | Generate the keypair inside WASM at session start; store the private key in `sessionStorage` (cleared on tab close) and register the public key in the trustee DB record so Windmill can embed it in the `Configuration` |
| **Trustee symmetric encryption key is server-provisioned** | The key that seals the channel private key inside the `Channel` message must be available in the browser | Generate it inside WASM at session start and store it in `sessionStorage` alongside the signing key |
| **Machine account JWT for B4 auth** | The user's Keycloak session token may expire mid-ceremony, breaking B4 calls | Detect 401 responses inside `WasmSession` and refresh the access token from the existing Keycloak session without restarting the DKG state machine |
| **`get-private-key` requires a Harvest round-trip** | In BBT the browser already ran the protocol and has all the data in IndexedDB; fetching from Harvest is redundant | Assemble the `TrusteeShareData` bundle client-side from the braid-wasm IndexedDB local state, eliminating the Harvest call |
| **`check-private-key` expects a Harvest-assembled bundle** | If the bundle is assembled locally its byte layout must still match what Harvest would produce | Harvest's check endpoint compares against B4 directly, so it remains valid regardless of assembly path — no change needed as long as the format is identical |
| **Windmill posts `Configuration` via direct SQL into B4's PostgreSQL** | This bypasses B4's authentication layer and tightly couples Windmill to B4's internal DB schema | Expose an authenticated HTTP endpoint on B4 for posting the `Configuration` message so Windmill uses the same API as all other parties |

---

## End-to-End Sequence Summary

```
Admin Portal (admin)
  │  Configure ceremony (trustees, threshold)
  ▼
Harvest /create-keys-ceremony
  │  Insert keys_ceremony (status: STARTED)
  │  Grant board access in Keycloak
  ▼
Windmill: create_keys task
  │  Generate protocol-manager keypair → Vault
  │  Build Configuration message
  │  INSERT Configuration → B4 PostgreSQL directly
  │  Set status: IN_PROGRESS
  ▼
B4 board: Configuration message posted
  ▼
Each trustee (braid-native OR braid-wasm in browser)
  │  Round 1: POST Channel to B4
  │  Round 2: POST ChannelsAllSigned to B4
  │  Round 3: POST Shares to B4
  │  Round 4 (trustee 0): POST PublicKey to B4
  │  Round 5 (all others): POST PublicKeySigned to B4
  ▼
★ KEY FRAGMENT GENERATED
  ▼
Windmill: set_public_key task
  │  SELECT PublicKey from B4 → write to DB
  │  Set each trustee status: KEY_GENERATED
  ▼
Admin Portal (each trustee)
  │  Download step → Harvest /get-private-key
  │    Harvest fetches Channel + Shares from B4
  │    Returns TrusteeShareData bundle (base64 file)
  │  Trustee saves file locally
  │  Set trustee status: KEY_RETRIEVED
  ▼
★ PRIVATE KEY FRAGMENT DOWNLOADED
  ▼
Admin Portal (each trustee)
  │  Check step → Harvest /check-private-key
  │    Harvest re-fetches Channel + Shares from B4
  │    Compares with uploaded file byte-for-byte
  │    Set trustee status: KEY_CHECKED
  │  When all trustees checked: set status SUCCESS
  ▼
★ PRIVATE KEY FRAGMENT CHECKED
  ▼
Keys ceremony complete.
Election public key is in the DB.
Each trustee holds their encrypted private key fragment.
```

---

## BBT Task: Generate Ed25519 Signing Keypair in WASM

This section details the implementation plan for the first design gap: replacing the hardcoded
signing keypair in `HeadlessTrusteeProvider` with one generated inside WASM, stored in
`sessionStorage`, and registered in the DB per election event.

### Current State

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

### The Per-Event Key Problem

A BBT trustee may participate in multiple election events simultaneously.  If the signing key is
stored globally (one entry per trustee in the DB), registering a fresh key for event B overwrites
the key already embedded in event A's `Configuration` on the board, breaking event A's DKG
verification.

The solution is to store the public key **per (trustee, election event)** in the DB and to
namespace the `sessionStorage` entries by `election_event_id`.

### DB Schema Change: add `election_event_id` to `sequent_backend_trustee`

Add a nullable `election_event_id` column to the `sequent_backend_trustee` table:

| Column | Type | Meaning |
|---|---|---|
| `election_event_id` | `uuid \| null` | `null` for server-based trustees; set to the event UUID for BBT trustees |
| `public_key` | `text \| null` | Ed25519 public key (base64-DER) for this trustee + event combination |

For server-based trustees the existing row keeps `election_event_id = null` and `public_key` set
once at provisioning time — no behaviour change.

For BBT trustees a row is upserted on every session start:
`(trustee_id, election_event_id) → public_key`.  Two simultaneous events produce two independent
rows with independent keys.

Windmill's `create_keys` task already fetches trustees by ID
(`get_trustees_by_id`, `create_keys.rs:49`).  That query must be updated to filter by
`election_event_id` when selecting the `public_key` to embed in the `Configuration`.

### The Core Timing Constraint

The public key must be in the DB **before** the ceremony is created.  The required ordering is:

```
BBT trustee opens election event in admin portal
  └─ HeadlessTrusteeProvider mounts
  └─ WASM generates keypair for this election_event_id
  └─ private key → sessionStorage['bbt_signing_key_sk_{election_event_id}']
  └─ public key  → POST to Harvest /register-trustee-key
                    (upserts trustee row with election_event_id + public_key)
                    ↓
Admin creates ceremony (selects trustees)
                    ↓
Windmill create_keys reads trustee.public_key WHERE election_event_id = $1
  → builds Configuration with correct per-event public key
                    ↓
HeadlessTrusteeProvider reads signing_key_sk from sessionStorage
  → WasmSession signs DKG messages with matching private key
```

### Component Changes

#### 1 — Braid-wasm: new `generate_signing_keypair()` export

Add a WASM-exported function in `braid/src/wasm/mod.rs` that generates an Ed25519 keypair and
returns both halves as base64-DER strings — the same format already consumed by `WasmSession`:

```
pub fn generate_signing_keypair() -> JsValue
  returns { signing_key_sk: string, signing_key_pk: string }
```

Reuses `StrandSignatureSk` already present in braid — no new crypto dependency.

#### 2 — `HeadlessTrusteeProvider`: generate and register on mount

Inside the existing `initialize` effect (already runs when `boardName` + `trusteeRecord` are
available), before constructing `WasmSession`:

1. Derive the sessionStorage key: `bbt_signing_key_sk_{electionEventId}`.
2. If absent: call `generate_signing_keypair()` via WASM, store both halves in `sessionStorage`,
   POST the public key to Harvest `/register-trustee-key` with the `election_event_id`.
3. If present: re-POST on page refresh (tab has not closed, private key is still valid).
4. Pass `signing_key_sk` / `signing_key_pk` from `sessionStorage` into the `WasmSession` config
   instead of the hardcoded strings.

#### 3 — Harvest: new `POST /register-trustee-key` endpoint

```
Input:  { public_key: string, election_event_id: string }
Auth:   existing Keycloak JWT  — identifies which trustee is calling
Action: UPSERT sequent_backend_trustee
          SET public_key = $public_key
          WHERE trustee_id = <from JWT> AND election_event_id = $election_event_id
```

The private key never reaches the server.  Server-based trustees never call this endpoint;
their `public_key` continues to be set at provisioning time with `election_event_id = null`.

#### 4 — Windmill `get_trustees_by_id`: filter by `election_event_id`

Update the DB query to return the `public_key` that matches the ceremony's `election_event_id`,
falling back to the row with `election_event_id = null` (server-based):

```sql
SELECT public_key FROM sequent_backend_trustee
WHERE id = $trustee_id
  AND (election_event_id = $event_id OR election_event_id IS NULL)
ORDER BY election_event_id NULLS LAST
LIMIT 1
```

No change to the `create_keys` task logic itself — it already uses whatever `public_key` the
query returns.

### Open Question: Trustee Not Logged In at Ceremony Creation

If a BBT trustee has not yet opened the election event page when the admin creates the ceremony,
`sessionStorage` is empty and the DB has no entry for this `(trustee, election_event_id)` pair.
Windmill will find no public key and either skip the trustee (silently wrong) or fail.

Two mitigation options:

| Option | Approach | Trade-off |
|---|---|---|
| **A — Warn at ceremony creation (recommended for now)** | Admin portal shows a "not ready" badge next to any BBT trustee who has no `public_key` registered for this `election_event_id` yet | Simple, no Windmill change — relies on admin confirming all trustees are online before creating the ceremony |
| **B — Defer `create_keys` until all trustees ready** | Ceremony stays in `STARTED`; Windmill only runs `create_keys` once all BBT trustees have a key registered for this event | Safe by design but requires a readiness signal and a polling or webhook trigger in Windmill |

Option A is sufficient for the initial BBT rollout since attended ceremonies imply the trustee is
already logged in.  Option B is the correct long-term design for unattended or scheduled
ceremonies.
