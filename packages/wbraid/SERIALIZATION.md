<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# VSer serialization audit

Working document of the `exp/vsc-serialization/main` investigation. **Goal: canonical
serialization functionality with strong assurance.** Three outcomes are open: (1) the
existing VSer implementation passes the audit, possibly after targeted fixes; (2) the
findings motivate a rewrite; (3) the functionality is outsourced to a dependency (a
prior search found none satisfactory). This document records what the audit finds and
the evidence for choosing between those outcomes.

## 1. The required property

VSer is the wire format of the trustee protocol: every posted artifact, every signed
statement, and every hash identity is computed over VSer bytes. Three properties are
required, and the braid spec (§3.3, §3.5, §3.6) already *assumes* the first two:

- **Canonical**: `ser` is injective — every value has exactly one byte encoding.
- **Strict**: `deser` accepts exactly the image of `ser` — every accepted byte string
  is one `ser` produced. Together these make `ser`/`deser` a bijection between values
  and accepted byte strings, so `H(bytes)` is an identity for the value, and
  `signed_bytes = ser(Statement)` is malleability-free.
- **Safe on adversarial bytes**: no panics, no unbounded allocation, cost linear in
  input length. Deserialization runs on bytes relayed by the untrusted board (before
  and after signature checks).

## 2. Architecture (as found)

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
buffer is exhausted — `ser(v) ‖ junk` deserializes to `v`. The `[T; N]` impl has
exactly the missing check (`if !bytes.is_empty()`), so this is an inconsistency, not a
design decision. `Vec` appears in nearly every artifact (`Shares.commitments`,
`Ballots.ciphertexts`, `Mix.ciphertexts`, every head's `trustees`, `ProtocolMessage`'s
`head`/`body` fields), so the slack is reachable everywhere: any length-prefixed field
slice can carry junk between the vector's true end and the slice boundary.

### S2 — `String::deser` accepts trailing bytes (strictness violation)

Reads its length prefix, decodes that many bytes, ignores the rest. Same class as S1.
(The `(A,)` single-field impl, by contrast, checks exactness.) Production wire usage
of `String` is limited — to be confirmed in the per-artifact sweep — but the impl is
in the core set.

### S3 — `PhantomData::deser` accepts arbitrary bytes (strictness violation, reaches `Configuration`)

`PhantomData<T>::deser` ignores its input entirely and succeeds. Under tuple framing
this means: in a non-final position, a phantom field's length prefix may point at any
bytes whatsoever; in the final position, the struct accepts arbitrary trailing bytes.
`braid::messages::artifact::Configuration` **ends with `phantom: PhantomData<C>`**, so
`Configuration::deser(ser(cfg) ‖ anything)` succeeds — every byte string extending a
valid configuration is accepted, each with a distinct `cfg_hash`, all denoting the
same logical configuration. The fix is to require the empty slice.

### S4 — `BTreeMap` impl: accepts unsorted and duplicate keys, plus frame slack (latent)

`ser` emits sorted-unique entries; `deser` inserts whatever it reads (later duplicates
silently overwrite) and ignores bytes after its frame. Distinct byte strings decode to
the same map. **No production wire type uses `BTreeMap`** — candidate for deletion
under the unused-API rule rather than repair.

### S5 — `LargeVector` impl: floor-division slack and empty-vector failure (latent)

`each = bytes.len() / count` truncates, so up to `count − 1` trailing bytes pass the
`each == T::size_bytes()` check and are then dropped by `par_chunks_exact`. Separately
`count = 0` divides by zero and errors, so an empty `LargeVector` does not round-trip.
The type is documented as **unused in production** — fix or delete.

### S6 — F-tier tuple impls skip the total-size check (known, mitigated, should be pinned)

Flagged by an existing `#[crate::warning]`. Sound today because every F-leaf rejects
wrong-length input, making the check transitive — but that is an invariant no test
pins. Either add the total-size check in the macro or pin the leaf-strictness
invariant with tests.

### S7 — Ed25519 leaf strictness is unverified (open question)

`VerifyingKey::from_bytes` / `Signature::from_bytes` (ed25519-dalek v3): whether
non-canonical point encodings are rejected at parse time, and the fact that signature
`s`-range checking happens at verify time rather than parse time, need verification
against the dalek version in use, plus reject tests. Likely low impact (both types
round-trip their bytes verbatim, so no aliasing at the encoding level), but the audit
should not assume.

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

## 6. Remaining audit work (phase 1 completion)

- Per-artifact sweep: every braid wire struct (heads, artifacts, `Sender`, predicates
  as persisted) and every vsc proof struct, walked against the framing model for
  strictness end to end.
- Resolve S7 against ed25519-dalek v3 source.
- `vser_derive` details: unit-struct derive (tuple `()` has no impl — currently
  uncompilable, fine), `Hash`-via-ser note.
- The existing test suite has round-trips only — no reject tests. Phases 2–4 (pinning
  tests, proptest bijection properties, fuzzing) as planned.

## 7. Early read on the three outcomes

The core defects are *localized and repairable*: S1 is one missing `is_empty` check,
S2/S3 are exact-length checks, S4/S5 are deletable as unused. Fixes tighten the
accepted set only — the produced encodings do not change, so **no transcript or wire
compatibility is affected**. That favors outcome (1): audit + targeted fixes +
pinning/property/fuzz tests. The standing argument for outcome (2) is S8/S9
(efficiency and redundant-prefix design), which is a format change with full
transcript breakage — worth deciding *after* the fixes land and the property tests
exist, since a rewrite would inherit them. No new evidence bears on outcome (3).
