<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# The serialization audit and rewrite

Record of the `exp/vsc-serialization/main` investigation. **Goal: canonical
serialization functionality with strong assurance.** Three outcomes were open: (1) the
then-current implementation ("VSer") passes the audit, possibly after targeted fixes;
(2) the findings motivate a rewrite; (3) the functionality is outsourced to a
dependency (a prior search found none satisfactory). **Outcome (2) was chosen and is
realized** — the current format is defined in §9 (the mini-spec) and the transition is
recorded in §10; §§2–8 are the audit record of the replaced v1 format and of the
decisions that led here. Findings S1–S6 were fixed in v1 first (and backported to
`feat/braid-0.6.3/main`) before the rewrite superseded them structurally.

## 1. The required property

The serialization module (`vsc::utils::serialization`) is the wire format of the
trustee protocol: every posted artifact, every signed statement, and every hash
identity is computed over its bytes. Three properties are required, and the braid
spec (§3.3, §3.5, §3.6) already *assumes* the first two:

- **Canonical**: `ser` is injective — every value has exactly one byte encoding.
- **Strict**: `deser` accepts exactly the image of `ser` — every accepted byte string
  is one `ser` produced. Together these make `ser`/`deser` a bijection between values
  and accepted byte strings, so `H(bytes)` is an identity for the value, and
  `signed_bytes = ser(Statement)` is malleability-free.
- **Safe on adversarial bytes**: no panics, no unbounded allocation, cost linear in
  input length. Deserialization runs on bytes relayed by the untrusted board (before
  and after signature checks).

## 2. Architecture (as found — the v1 format, replaced in §10)

Two tiers in `vsc::utils::serialization`:

- **`fixed` (F-tier)**: types with a compile-time byte size. Composition is raw
  concatenation — no tags, no prefixes. Unique split points make the parse injective
  *provided every leaf rejects wrong-length input* (the tuple impls deliberately do
  not check total size; the last leaf's exact-width check catches it transitively).
- **`variable` (V-tier)**: the wire tier. A struct derives `VSerializable`
  (`vser_derive`), which converts it to a tuple; the tuple encoding gives every field
  except the last an 8-byte big-endian byte-length prefix (`LengthU = u64`), and the
  last field the remaining bytes. `Vec<T>` adds an item-count prefix plus a per-item
  length prefix. Leaves (group elements, scalars, digests, integers) have manual
  impls.

Signatures are verified against the **re-serialization of the parsed statement**
(`statement_bytes(&head, Some(&body_hash))`), not against raw received bytes — an
important mitigating design (see §4).

## 3. Findings

Severity reflects distance from the required property, not a demonstrated exploit.

### S1 — `Vec<T>::deser` accepts trailing bytes (strictness violation, core)

`variable.rs` `Vec<T>::deser` reads the item count, loops, and never checks that the
buffer is exhausted — `ser(v) ‖ junk` deserializes to `v`. `Vec` appears in nearly
every artifact (`Shares.commitments`, `Ballots.ciphertexts`, `Mix.ciphertexts`, every
head's `trustees`, `ProtocolMessage`'s `head`/`body` fields), so the slack is
reachable everywhere: any length-prefixed field slice can carry junk between the
vector's true end and the slice boundary.

Notably, the acceptance was **ratified by an existing test** — `test_vector_vser`
asserted that "a vector with padded bytes works" — even though the `[T; N]` impl has
exactly the opposite check. Whatever motivated that assertion, no production code
depends on padded acceptance (all suites pass with the strict behavior).

**Fixed**: `deser` rejects trailing bytes; the ratifying test now asserts rejection.

### S2 — `String::deser` accepts trailing bytes (strictness violation)

Reads its length prefix, decodes that many bytes, ignores the rest. Same class as S1.
(The `(A,)` single-field impl, by contrast, checks exactness.) Production wire usage
of `String` is limited — to be confirmed in the per-artifact sweep — but the impl is
in the core set.

**Fixed**: exact-length check added.

### S3 — `PhantomData::deser` accepts arbitrary bytes (strictness violation, reaches `Configuration`)

`PhantomData<T>::deser` ignores its input entirely and succeeds. Under tuple framing
this means: in a non-final position, a phantom field's length prefix may point at any
bytes whatsoever; in the final position, the struct accepts arbitrary trailing bytes.
`braid::messages::artifact::Configuration` **ends with `phantom: PhantomData<C>`**, so
`Configuration::deser(ser(cfg) ‖ anything)` succeeds — every byte string extending a
valid configuration is accepted, each with a distinct `cfg_hash`, all denoting the
same logical configuration.

**Fixed**: `PhantomData::deser` requires the empty slice.

### S4 — `BTreeMap` impl: accepts unsorted and duplicate keys, plus frame slack (latent)

`ser` emitted sorted-unique entries; `deser` inserted whatever it read (later
duplicates silently overwriting) and ignored bytes after its frame. Distinct byte
strings decoded to the same map. No production wire type used `BTreeMap`.

**Deleted** under the unused-API rule, with an in-code note recording the sortedness
requirement should a map ever be needed on the wire.

### S5 — `LargeVector` impl: floor-division slack and empty-vector failure (latent)

`each = bytes.len() / count` truncated, so up to `count − 1` trailing bytes passed the
`each == T::size_bytes()` check and were then dropped by `par_chunks_exact`. Separately
`count = 0` divided by zero and errored, so an empty `LargeVector` did not round-trip.
The type is documented as unused in production but deliberately retained (a
performance vehicle with an honest status note).

**Fixed** (exact `count × size` check; empty vector round-trips; zero-sized elements
rejected). Whether the type should instead be deleted under the unused-API rule is an
open call.

### S6 — F-tier tuple impls skip the total-size check (known, mitigated, should be pinned)

Flagged by an existing `#[crate::warning]`. Sound today because every F-leaf rejects
wrong-length input, making the check transitive — but that was an invariant no test
pinned.

**Fixed**: the tuple base case and the macro now validate the total size up front;
the warning is retired.

### S7 — Ed25519 leaf strictness (resolved against ed25519-dalek 3.0.0 source)

`VerifyingKey::from_bytes` validates points under **ZIP-215 rules — RFC 8032
canonicality is explicitly unsupported** (its own doc says so, citing
curve25519-dalek#626): a y-coordinate ≥ p is silently reduced, so a handful of curve
points admit two accepted encodings. `Signature::from_bytes` accepts any 64 bytes
(the `s < ℓ` check happens at verify time, where dalek ≥ 2.0 enforces it).

**Why this does not break the canonicality property or braid**:

- Both types store and return their input bytes verbatim (`VerifyingKey` keeps the
  compressed bytes; `as_bytes` returns them), so `ser∘deser = id` on the accepted
  set and the *encoding* stays value-injective — the aliasing exists only at the
  curve-*point* level: two distinct `VerifyingKey` values can denote one point.
- dalek's `PartialEq for VerifyingKey` compares **compressed bytes**, and braid's
  sender resolution (`Configuration::get_trustee_position`) uses that equality — a
  non-canonical alias of a configured key simply fails to match ("sender not part of
  the configuration"). And even when a sender matches, the signature is verified
  under the **configured** key, never the sender-supplied one (`board/verify.rs`).
  Point aliasing is therefore neutralized twice over.

Standing rule this implies: **identity comparisons on keys must remain byte-wise**;
never compare or index by decompressed points.

### S8 — Encoding overhead (efficiency; input to the rewrite decision)

- `Vec<u8>` pays the generic path: an 8-byte length prefix **per byte** — 9× blowup.
  Paid by every `ProtocolMessage.head`/`.body` (`Vec<u8>` fields) and
  `Shares.encrypted_shares` (`Vec<Vec<u8>>`).
- Element arrays pay a per-element prefix: a width-2 ciphertext serializes to ~1.44×
  its raw size; across N-thousand ciphertext lists this is material.
- The V-tuple macro copies each field slice (`head_bytes.to_vec()`) during
  deserialization.

None of this affects soundness. It is the main evidence that would motivate outcome
(2): a fixed-size-aware encoding (elements, digests, and integers need no prefixes at
all — the F-tier proves it) would shrink most artifacts several-fold.

### S9 — Design observation: redundant prefixes are attacker degrees of freedom

Tuple framing gives every non-final field a length prefix even when the field type is
fixed-size, and `Vec` gives every item one even when `T` is fixed-size. Each redundant
prefix is a byte-string degree of freedom that strictness must neutralize: today it is
neutralized exactly where the pointed-to type is strict (integers, elements, arrays,
`(A,)` structs, `Option::None`, `bool`) and *not* where it is not (S1–S5). A canonical
format has two clean shapes: prefix nothing fixed-size (F-tier philosophy), or verify
every prefix against the value actually parsed.

## 4. Impact on braid as deployed

The protocol's semantics largely survive S1–S3 today, for reasons worth recording:

- **Authentication is unaffected**: signatures are checked against the
  re-serialization of the *parsed* statement, so framing slack cannot forge or alter
  what is signed.
- **Agreement hashing is unaffected among honest parties**: each trustee serializes
  its own `DkgPublicKey`/`Plaintexts` canonically, so identical logical values hash
  identically; a malicious trustee padding its encoding produces a hash *mismatch* and
  a halt — the failure direction is safe.
- What actually bites: (a) the spec's §3.5 assumption ("deserializers reject
  non-canonical input") is **currently false**, so the malleability argument rests on
  the mitigations above rather than on the stated property; (b) `H(body)` is not an
  identity for the logical artifact — padded variants of one artifact have distinct
  hashes; (c) a future independent verifier that re-serializes parsed artifacts to
  recompute hashes would reject boards braid accepted; (d) byte-distinct duplicates of
  one message are board-spammable (predicate idempotence absorbs them).

**Safety on adversarial bytes is good**: bounds-checked slicing (`get_slice`)
throughout, checked arithmetic in the deserializers, no `with_capacity(attacker
length)` allocation bombs, cost linear in input. No panic path was found in the core
read.

## 5. Already-hardened spots (evidence of a prior partial pass)

`bool` (rejects non-0/1), `Option::None` (rejects trailing), `[T; N]` (rejects
trailing, with a comment documenting a fixed cursor bug), checked add/div helpers, and
the leaf strictness of ristretto255 (canonical scalar via `from_canonical_bytes`,
strict point decompress) and P-256 (SEC1 compressed only, strict field/scalar decode,
a custom but unique `[0u8; 33]` identity encoding). The unfixed impls (S1–S5) are the
complement of that pass.

## 6. Per-artifact sweep (complete)

Every wire type walked against the framing model, with all field types now strict:

- **Heads** (`Configuration…PlaintextsHead`): derives over hash newtypes (1-field
  exact framing), `Timestamp = u64`, `Vec<TrusteeIndex>` (`TrusteeIndex = usize`,
  encoded via the strict u64 bridge), `tally_id: u128`. Strict.
- **Artifacts**: `Configuration` (trailing hole closed via the `PhantomData` fix),
  `Shares`, `DkgPublicKey`, `Ballots`, `Plaintexts` — derives over strict types.
  `Mix` has a hand-written impl that is exactly the two-field tuple encoding
  (`(Vec<Ciphertext>, Option<ShuffleProof>)`) — equivalent to a derive, strict.
  `PartialDecryption` is the vsc type re-exported.
- **`Predicate`** (persisted for anti-rewrite): hand-written enum as a
  `(u8 tag, Vec<u8> inner)` tuple; unknown tags rejected, inner exactly consumed,
  tags follow declaration order. Strict. (Note for S8: the inner `Vec<u8>` pays the
  per-byte prefix blowup on every persisted commitment.)
- **`ProtocolMessage`** (the pre-signature adversarial boundary): derive over
  `Sender` (String + VerifyingKey), `Signature`, `MessageType` (manual enum impl,
  strict), `Vec<u8>` head/body. Strict post-fixes.
- **vsc proof and cryptosystem structs**: `SchnorrProof`, `DlogEqProof`, `PlEqProof`,
  `ShuffleProof`/`ShuffleCommitments`/`Responses`, `elgamal::Ciphertext`/keys,
  `naoryung::Ciphertext`, `dkgd` types, `ParticipantPosition` — all derives over
  strict leaves (elements, scalars, digests, integers, `Vec`s, arrays).
- **b4** stores and relays opaque bytes only — it never deserializes VSer content.
- Out of wire scope: `wasm/persistence.rs` uses its own little-endian framing for
  local storage only.

## 7. Plan of record (agreed 2026-08-28): rewrite attempt first, under guardrails

With transcript compatibility explicitly a non-constraint (the sole external pin,
Verificatum interop, lives on `v2v`'s separate ByteTree path and is untouched), the
riskiest item goes first: a bounded attempt at the outcome-(2) rewrite, **before** the
remaining assurance campaign, so that campaign runs once, against whichever format
survives. The property harness below was pulled forward as the rewrite's acceptance
net (it pins the bijection, not byte layouts, so it transfers unchanged).

Two showstoppers govern the attempt, either one aborting it:

- **(a) Rabbit-holing**: iteration without end, losing the original goal.
- **(b) Complexity**: the result must be *simpler and clearer* than current VSer, or
  it is not viable regardless of its other merits — on this project clarity outranks
  everything, including compile-time cleverness. (Standing evidence: a prior
  exploration of compile-time size arithmetic via `generic_const_exprs` had to be
  abandoned for exactly this reason. Accordingly, the `SIZE` associated const is cut
  from the design — cursor parsing needs no size arithmetic for correctness.)

Guardrails:

1. **Mini-spec before code** — the format definition must fit in roughly a page of
   prose here; if it cannot be described that briefly, the design is wrong.
2. **Hard checkpoint** — the spike ends at: core module + derive + three
   representative types (a fixed struct, a `Vec`-bearing artifact, the `Predicate`
   enum) round-tripping under the property harness. Review happens there; no
   iteration past it without an explicit go.
3. **Objective simplicity criterion** — net-negative diff on serialization code;
   strictly fewer concepts (no GATs, no tuple indirection, one trait); a single
   strictness point.
4. **Written non-goals** — no versioning, no schema evolution, no enum-derive
   generalization, no zero-copy, no type-level size computation, no performance work
   beyond what the format gives for free.

## 7a. Remaining work

- **Done — phase-3 harness** (`utils/serialization/properties.rs`): proptest
  bijection properties P1 (`deser(ser(x)) == x`) and P2 (`deser(b) = Ok(v) ⟹
  ser(v) == b`) over a kitchen-sink type covering every composition rule, both
  contexts, both an all-fixed-width and a variable-size struct shape, with
  random-byte and mutation distributions. The mutation distribution reproduces the
  S1/S3 failure class, so the harness is a genuine regression net — it was built
  against v1, served as the rewrite's acceptance suite, and now runs against the
  final format (migrated in the flip, §10).
- **Done — the rewrite** (per §7): see §10.
- Phase 4, against the final format: fuzzing the deserializers and the `verify()`
  boundary (one pre-existing fuzz target under `vsc/fuzz` is already migrated), plus
  deeper property-campaign parameters (case counts).
- **Resolved — `LargeVector`**: deleted in the flip; its parallel-serialization
  intent is tracked as `PERFORMANCE.md` work item 3 (an implementation strategy
  behind the unchanged `Vec` encoding).
- Derive note: unit structs are supported (empty `write`, `Self` on `read`); enums
  are implemented by hand per rule 7.

## 8. Early read on the three outcomes

The core defects were *localized and repairable*, and the fixes have landed on this
branch: S1–S3 and S6 strictness checks, S4 deleted, S5 repaired — each pinned by a
reject test. The fixes tighten the accepted set only — the produced encodings do not
change, so **no transcript or wire compatibility is affected**, and every suite (vsc,
b4, v2v, braid end-to-end) passes under the strict behavior, demonstrating that no
production path relied on the slack. That favors outcome (1): audit + targeted fixes
+ pinning/property/fuzz tests. The standing argument for outcome (2) is S8/S9
(efficiency and redundant-prefix design), which is a format change with full
transcript breakage — worth deciding *after* the property tests exist, since a
rewrite would inherit them. No new evidence bears on outcome (3).

**Terminology settled 2026-08-28**: there is serialization *for challenge
derivation* (Fiat-Shamir transcript inputs, computed transiently from in-memory
values at proving time — VMN interop fixes ByteTree here, via the existing
`VmnChallenges`/`v2v::encode` seam) and serialization *for communication and
storage* (the wire format — what this investigation is about). Interop constrains
only the first.

**A fourth possibility, raised and declined (2026-08-28)**: adopt Verificatum's
ByteTree as the native format, on the theory that it might cheapen
Verificatum-compatible proof production. Evaluated against the precedence rule that
our own format's merits outrank interop: it does not qualify. The interop saving is
marginal — serialization is one of *seven* divergence rows in `VERIFICATUM.md`'s
table (hash family, PRG, random oracle, challenge widths, and the global prefix ρ
all differ), so native ByteTree proofs still would not verify under `vmnv`; it would
shave only the serializer clause of the **convert** row, which v2v's existing seam
already covers. Against the higher-priority criteria it *loses*: ByteTree frames
every node and leaf with a tag byte plus a 4-byte length — including fixed-size
leaves (a ristretto element becomes 37 bytes), re-importing S9's redundant-framing
problem — and being schemaless it needs a separate typed-conventions layer, putting
the strictness obligation on two layers instead of one. What ByteTree does
contribute: its two-constructor minimalism is the simplicity benchmark the mini-spec
(§7 guardrail 1) should be measured against.

The conclusion holds even under a stronger future requirement — *verifying real
production proofs with Verificatum* (`VERIFICATUM.md`'s **convert** row). Convert
constrains the **transcript**, not the wire format: it requires the production
proofs' challenges to be VMN's (`VmnChallenges` in production, with ByteTree used
*inside the prover at challenge time* — machinery v2v already has), while the
"serializer" clause is a cryptographically inert boundary conversion of stored
proofs, indifferent to the native format. The decision such a requirement would
force is about transcripts — adopt VMN's Fiat-Shamir convention for production
mixing (displacing the §6.3 native design now normative in PROTOCOL.md) or prove
twice at mix time — and serialization is a bystander to it. The native-format
choice remains free.

## 9. Mini-spec: the v2 encoding (guardrail 1 — the format, in one page)

**Model.** The existing trait pair keeps its names and entry points; only the
required methods change:

```rust
trait Serializable   { fn write(&self, out: &mut Vec<u8>);
                       fn ser(&self) -> Vec<u8> { /* provided: write into a Vec */ } }
trait Deserializable { fn read(input: &mut &[u8]) -> Result<Self, Error>;
                       fn deser(b: &[u8]) -> Result<Self, Error>
                       { /* provided: read, then error unless input is exhausted */ } }
```

(Names as settled at the flip, §10; structs opt in with `#[derive(Canonical)]`.)

`read` consumes exactly the bytes `write` produced, from the front of the slice
(advancing it). `deser` adds the format's **single** top-level strictness check:
input exhausted.

**Encoding rules — the entire format:**

1. **Fixed-width leaves** — integers (`u8`–`u128` big-endian; `usize` as `u64`),
   group elements, scalars, digests, keys, signatures: their existing v1
   encodings, byte-for-byte. No framing.
2. **`bool`**: one byte, `0` or `1`; anything else rejected.
3. **Structs, tuples, arrays `[T; N]`**: the concatenation of the members'
   encodings in declaration order. No tags, prefixes, or counts (`N` is in the
   type).
4. **`Vec<T>`**: the element count as `u64` big-endian, then the elements'
   encodings concatenated. No per-element framing. A collection element whose
   `read` consumes zero bytes is an error (guards zero-sized types: a count must
   be bound by content).
5. **`String`**: the byte length as `u64` big-endian, then the UTF-8 bytes
   (invalid UTF-8 rejected).
6. **`Option<T>`**: one byte `0` (`None`) or `1` (`Some`), then `Some`'s payload.
7. **Enums** (hand-written, e.g. `MessageType`, `Predicate`): one `u8`
   discriminant in declaration order, then the variant's payload. Unknown
   discriminants rejected.
8. **`PhantomData`**: zero bytes.

All integers entering the format are big-endian (the D2 uniformity rule).

**Properties, by construction.** *Canonical*: every rule is a deterministic
function of the value — there is no encoding choice anywhere, and composition of
injective, self-delimiting encodings is injective. *Strict*: `read` never skips
or ignores bytes, so there is no slack to validate; the one exhaustion check in
`deser` closes the top level. *Safe*: truncation is a slice-bounds error;
collection loops are bounded by input length (each element consumes ≥ 1 byte);
no allocation is sized by attacker-controlled counts.

**The derive** emits rule 3 directly — field-by-field `write` calls and
field-by-field `read` calls, plus the existing `Hash`-via-`ser` impl. No tuple
conversion, no GATs, no arity-limited macros.

**What this deletes**: `TFTuple` and its GATs; both tuple-impl macro towers; the
entire `fixed` tier (`FSerializable`/`FDeserializable`, absorbed by rule 1 + rule
3); `LargeVector` (rule 4 *is* its encoding, minus the type); the unused
`Marker`/`ConstMarker` infrastructure. Net-negative diff is expected, per §7
guardrail 3.

On `LargeVector`, the intent behind it — parallel serialization of large
collections — survives the type as a possible implementation strategy behind the
same `Vec` encoding; it is tracked as `PERFORMANCE.md` work item 3.

**What changes on the wire**: every composite encoding (prefixes disappear);
therefore every hash identity and transcript. Leaf encodings are unchanged. No
golden bytes pin the old format anywhere (§7); all suites regenerate
self-consistently, and the §7a property harness is the acceptance net.

**Not in this format** (non-goals, §7 guardrail 4): versioning, schema evolution,
derive support for enums, zero-copy, type-level size computation, runtime type
tags.

## 10. The flip (2026-08-28): v2 is the format

Following the checkpoint review, the naming was settled — traits
**`Serializable`/`Deserializable`** (the V/F prefixes died with the tiers; the
adjective forms are distinct from serde's `Serialize`/`Deserialize` verbs), the
derive **`#[derive(Canonical)]`** (it derives both traits plus `Hash`, and names
the contract a type opts into), the macro crate **`canonical_derive`** — and the
v2 encoding was flipped in as the only format:

- `serialization/mod.rs` is now the entire implementation: **309 lines**
  (including docs) replacing `variable.rs` + `fixed.rs` + the spike file
  (~1,900 lines). The derive is **101 lines** replacing 177. Deleted with the
  tiers: `TFTuple` and its GATs, both tuple macro towers, `LargeVector` (and
  its benchmark), `Marker`/`ConstMarker`, and `serde.rs` — the §11-banked
  retirement landed here of necessity (it sat on the v1 traits; its optional
  feature was enabled by nothing in the workspace).
- Leaf `write`/`read` impls moved beside their types (elements, scalars,
  digests, keys, signatures), encoding-identical to v1. The three hand-written
  impls became direct transcriptions of their rules: `Mix` (rule 3),
  `MessageType` and `Predicate` (rule 7 — `Predicate` thereby lost its former
  double-encoded `(u8, Vec<u8>)` envelope, so persisted predicate digests from
  before the flip do not carry over; only dev/demo state existed).
- Every suite passes under the new format: vsc (187 unit + doctests, including
  the migrated property harness), v2v (VMN interop untouched, as designed — the
  transcript path is separate), b4, and the braid release suite end-to-end on
  both groups with all 17 symbolic model checks — which run measurably faster
  (~33s vs ~47–55s), a first concrete return on S8.

Outcome (2) is therefore realized. What remains of the investigation: phase 4
(fuzzing — one pre-existing fuzz target under `vsc/fuzz` already updated) and
the deeper property-campaign parameters, both now running against the final
format.

## 11. Phase 4: fuzzing (against the final format)

**Design.** Every deserializer target is a **bijection oracle**: `if let Ok(v) =
T::deser(data) { assert_eq!(v.ser(), data) }` — so coverage-guided fuzzing hunts
panics *and* canonicality violations in one pass. Two additional targets exercise
the verify boundary on adversarial bytes under fixed, deterministically derived
keys (only panics are findings there).

**Targets.** In `crates/vsc/fuzz` (run with `cargo fuzz run <target>` from
`crates/vsc`): `deser_eg_ciphertext_ristretto`, `deser_ny_ciphertext_ristretto`,
`deser_shuffle_proof_ristretto`, `deser_verifiable_share_ristretto` (a DKG
dealing including proof-carrying checking values), `verify_ny_strip_ristretto`
(the PlEq well-formedness verifier), `verify_schnorr_ristretto`, plus the
pre-existing `encode_bytes`/`encode_scalar` targets. In `crates/braid/fuzz`
(new): `deser_protocol_message_ristretto` (the outermost adversarial boundary)
and `deser_predicate` (the anti-rewrite persistence boundary).

**Platform note.** The braid fuzz targets do not link on Windows/MSVC: braid's
wasm `cdylib` crate-type conflicts with libFuzzer's `/include:main` (and
`--no-include-main-msvc` removes the fuzz binary's own entry point — a genuine
catch-22). They build and run as-is on Linux. Host-side coverage for those two
boundaries is provided instead by `crates/braid/tests/serialization_properties.rs`
— the same oracle through proptest over valid, mutated, and random byte
distributions (braid gained a minimal `fuzzing` cargo feature exposing just the
message layer's cryptography dependency for the fuzz build).

**Smoke results (2026-08-28, this host, 40s/target).** All eight vsc targets
clean — ~7.9M executions total, zero crashes, zero bijection violations —
including 2.19M runs through the Schnorr verifier, 677k through the NY
verify-and-strip, and 1.4M/838k/431k/1.03M through the four deserializer
oracles. The braid property tests (6) pass. Deeper campaigns: raise
`-max_total_time`, and for the proptest side set `PROPTEST_CASES` (default 256).

The v1 modules' standing `#[crate::warning]`s ("arithmetic side effects …
pending fuzzing") were deleted with those modules; the new module uses checked
arithmetic throughout and needs no such allowance — the fuzzing debt those
warnings tracked is discharged.
