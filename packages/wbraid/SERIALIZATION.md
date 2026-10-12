<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Canonical serialization

The wire format of the trustee protocol: every posted artifact, every signed
statement, and every hash identity is computed over the bytes this module
produces. The implementation lives in `vsc::utils::serialization` (one module),
with the derive macro in the `canonical_derive` crate.

How the format is *verified* — the bijection property tests and the fuzzing
campaign — is documented in `ASSURANCE.md` §§2–3. This document describes the
format itself and the design decisions it embodies. (The audit of the previous
format, which motivated this design, is preserved in this file's git history.)

## 1. The required property

- **Canonical**: `ser` is injective — every value has exactly one byte
  encoding.
- **Strict**: `deser` accepts exactly the image of `ser` — every accepted byte
  string is one `ser` produced. Together these make `ser`/`deser` a
  **bijection** between values and accepted byte strings, so `H(bytes)` is an
  identity for the value and `signed_bytes = ser(Statement)` is
  malleability-free — properties the braid spec (§3.3, §3.5, §3.6) depends on.
- **Safe on adversarial bytes**: no panics, no allocation sized by
  attacker-controlled counts, cost linear in input length. Deserialization
  runs on bytes relayed by the untrusted board, before and after signature
  checks.

## 2. The model

```rust
trait Serializable   { fn write(&self, out: &mut Vec<u8>);
                       fn ser(&self) -> Vec<u8> { /* provided: write into a Vec */ } }
trait Deserializable { fn read(input: &mut &[u8]) -> Result<Self, Error>;
                       fn deser(b: &[u8]) -> Result<Self, Error>
                       { /* provided: read, then error unless input is exhausted */ } }
```

Parsing is **cursor-based**: every type's `read` consumes exactly the bytes its
`write` produced, from the front of a shared slice. Composition is therefore
plain concatenation and needs no framing; explicit lengths appear only where
the types genuinely lack the information — collection counts, opaque byte
lengths, and tag bytes. `deser` adds the format's **single** strictness check:
input exhausted. Nothing else needs validating, because the encoding states
nothing twice.

## 3. The encoding rules — the entire format

1. **Fixed-width leaves** — integers (`u8`–`u128` big-endian; `usize` as
   `u64`), group elements, scalars, digests, keys, signatures: their natural
   fixed encodings (§4). No framing.
2. **`bool`**: one byte, `0` or `1`; anything else rejected.
3. **Structs, tuples, arrays `[T; N]`**: the concatenation of the members'
   encodings in declaration order. No tags, prefixes, or counts (`N` is in the
   type).
4. **`Vec<T>`**: the element count as `u64` big-endian, then the elements'
   encodings concatenated. No per-element framing — each element delimits
   itself by consuming its own bytes. A collection element whose `read`
   consumes zero bytes is an error (a count must be bound by content).
5. **`String`**: the byte length as `u64` big-endian, then the UTF-8 bytes
   (invalid UTF-8 rejected).
6. **`Option<T>`**: one byte `0` (`None`) or `1` (`Some`), then `Some`'s
   payload.
7. **Enums** (hand-written, e.g. `MessageType`, `Predicate`): one `u8`
   discriminant in declaration order, then the variant's payload. Unknown
   discriminants rejected.
8. **`PhantomData`**: zero bytes (and only zero bytes accepted).

All integers entering the format are big-endian, matching the convention used
by every integer entering a hash transcript (PROTOCOL.md §2.4).

**Deriving.** `#[derive(Canonical)]` implements both traits plus
`std::hash::Hash` (via the serialized bytes) for a struct, emitting rule 3
directly — field-by-field `write`/`read` calls in declaration order, no
intermediate representation, no arity limit; `cargo expand` shows exactly the
function a person would write by hand. Unit structs derive fine; enums are
written by hand per rule 7.

**Why the properties hold by construction.** Every rule is a deterministic
function of the value (canonical), and `read` never skips or ignores bytes, so
there is no slack to cross-validate (strict): the redundant length prefixes
that plagued the previous format cannot be expressed in this one. Safety:
truncation is a checked-slice error, collection loops are bounded by input
length (each element consumes ≥ 1 byte), and results grow per parsed element
rather than by claimed counts.

## 4. Leaf encodings

| Type | Encoding | Strictness notes |
|---|---|---|
| `u8`–`u128` | big-endian, natural width | exact width |
| `usize` | as `u64` big-endian | value must fit `usize` on read |
| ristretto255 element | 32-byte compressed | strict decompress (ristretto encodings are canonical by construction) |
| ristretto255 scalar | 32 bytes | `from_canonical_bytes` — values ≥ ℓ rejected |
| P-256 element | 33-byte SEC1 compressed | field/curve-validated; the identity, which has no 33-byte SEC1 form, uses the custom unique encoding `[0u8; 33]` |
| P-256 scalar | 32 bytes | `from_repr` — values ≥ order rejected |
| SHA3-512 digest | 64 raw bytes | exact width |
| Ed25519 signing/verifying key | 32 raw bytes | see below |
| Ed25519 signature | 64 raw bytes | `s < ℓ` is enforced at *verify* time (dalek ≥ 2.0), not parse time |

**The Ed25519 nuance and its standing rule.** dalek's
`VerifyingKey::from_bytes` validates points under ZIP-215 rules — RFC 8032
canonicality is explicitly unsupported — so a handful of curve points admit two
accepted 32-byte encodings. The *encoding* layer stays value-injective (both
key and signature types round-trip their input bytes verbatim), so the
bijection holds; the aliasing exists only at the curve-*point* level. It is
neutralized structurally in braid: dalek's key equality is byte-wise, sender
resolution uses that equality, and signatures verify under the *configured*
key, never a sender-supplied one. The standing rule this imposes: **identity
comparisons on keys must remain byte-wise — never compare or index keys by
decompressed point.**

## 5. What deliberately does not exist

- **No map types on the wire.** A map deserializer must reject out-of-order
  and duplicate keys or distinct byte strings decode to the same value; until
  a wire map is actually needed, none is implemented (a note in the module
  records the requirement).
- **No threshold-branded key or ciphertext types.** A type-level brand like
  "encrypted under a T-threshold key" cannot survive the wire — a
  deserializing peer would apply it unilaterally, asserting nothing. DKG
  outputs are plain `elgamal::PublicKey`/`elgamal::Ciphertext`; the threshold
  lives where it is enforced, in `Recipient`/`combine`'s const parameters.
- **No dedicated large-collection type.** `Vec<T>` with fixed-size `T` already
  has the compact encoding (a count, then raw elements). Parallel
  serialization for very large collections remains possible *behind this same
  encoding* — tracked as `PERFORMANCE.md` work item 3.
- **Non-goals**: versioning, schema evolution, derive support for enums,
  zero-copy, type-level size computation (a prior exploration of
  `generic_const_exprs` for compile-time sizes was abandoned as
  disproportionate to its value; clarity outranks compile-time cleverness).

## 6. Scope: storage serialization vs. transcript serialization

There are two distinct byte domains that both get called "serialization":

- **This format** serializes artifacts for communication, storage, signing,
  and hashing.
- **Fiat-Shamir transcript inputs** are serialized transiently, from in-memory
  values, at proving time, under whatever convention the transcript demands.
  The native transcripts use this format's `ser` for their components;
  **Verificatum-compatible proofs** use VMN's ByteTree encoding *inside the
  challenge derivation* (`v2v::encode` through the `VmnChallenges` seam) —
  which is why VMN interop neither constrains nor is affected by this format.

Adopting ByteTree as the native format was considered and declined: it would
save only the boundary re-serialization of `VERIFICATUM.md`'s **convert** row
(the transcript machinery is required regardless), while re-importing per-leaf
tag+length framing and a second, schemaless validation layer. Should producing
Verificatum-verifiable *production* proofs ever become a requirement, the
decision it forces is about transcripts — run `VmnChallenges` in production or
prove twice at mix time — and the storage format remains a bystander.

## 7. Assurance

The bijection and safety properties are pinned by unit reject-tests beside the
implementation, property-based tests over every composition rule and the braid
wire boundaries, and coverage-guided fuzzing with bijection oracles — see
`ASSURANCE.md` §§2–3 for the inventory, platform notes, and how to run
everything.
