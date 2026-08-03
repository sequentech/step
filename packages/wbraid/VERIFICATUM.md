<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Reusing Verificatum's verifier as an independent verifier for braid

Feasibility investigation for the **standalone independent verifier** item in
`crates/braid/v0.6_spec.md` §11. Question asked: can we reuse binaries from the
[Verificatum](https://www.verificatum.org) mix-net project — preferably unmodified — to verify
data produced by braid?

Source material reviewed: `crates/braid/verificatum/` (the VMN + VCR source trees, prebuilt JARs,
and the `papers/` set — principally `vmnv-3.1.0.md`, the stand-alone verifier specification, and
`vmnum-3.1.0.md`, the user manual). Section references of the form "VMNV §N" point into
`vmnv-3.1.0.md`.

> **Where the code lives now.** This document records the investigation stage by stage, so it names
> the crates as they were at the time. Since then everything Verificatum-related has been gathered
> into a single crate, **`crates/vsvmn`**, and the two braid improvements the investigation produced
> — the P-256 backend completion and the batched decryption proof — have been separated onto their
> own branch, since they stand on their own merits. Read the older sections with this mapping:
>
> | then | now |
> |---|---|
> | `crates/vcompat` | `vsvmn::wire` (a module; the "must not depend on vsc" rule is unchanged but no longer enforced by cargo) |
> | `crates/braid/src/vmn` | the rest of `vsvmn` |
> | `crates/braid/tests/vmn_*.rs` | `crates/vsvmn/tests/` |
>
> braid itself no longer has any dependency on Verificatum interop code.

---

## Verdict

**No cryptographic showstopper.** The two systems implement the *same* mix-net: El Gamal with
re-encryption, the Terelius–Wikström proof of shuffle, and threshold decryption from a Shamir
sharing. braid's shuffle proof is not merely similar to Verificatum's — it is term-for-term the same
proof, with the same commitments in the same order and the same five verification equations.

**But the goal as literally stated — convert existing braid output so unmodified `vmnv` accepts it —
is not achievable.** The blocker is not algebra, it is Fiat–Shamir. Verificatum's verifier
*recomputes* every challenge from its own byte-tree serialization, hash function, and PRG. braid
bakes different challenges into its proofs at proving time. No post-hoc converter can fix that,
because the challenge is a function of a serialization braid never produced.

What *is* achievable is a **Verificatum-compatible emitter mode in braid**: braid generates proofs
to VMN's spec, and unmodified `vmnv` verifies them. That preserves the entire point of the exercise
— the verifier is a separate Java codebase by different authors, sharing no code with braid — while
moving the compatibility burden to braid's prover, where it is tractable.

**That emitter now exists and works.** Unmodified `vmnv -mix` accepts a complete braid session — a
three-party DKG, a three-mixer chain, and threshold decryption — over P-256 at width 2 (Stage 4).
The two largest work items were the byte-tree and VMN-random-oracle layer (now the `vcompat` crate)
and replacing braid's per-ciphertext decryption proofs with VMN's batched form (now
`vmn::decrypt`). VMN's own test vectors and `vmnv -t` mode made each layer checkable in isolation,
which is most of why the estimate held.

---

## Stage 0 results — EXECUTED, all green

Stage 0 of the plan below has been carried out. Everything in this section is observed behaviour,
not analysis.

**The prebuilt JARs run unmodified on a current JDK.** `vmnv -version` prints `3.1.0` and exits 0
under OpenJDK 25. No recompilation, no dependency wrangling, no native libraries — the pure-Java
path suffices. The JARs are ~2.1 MB total and already in the tree.

**A full reference proof was generated and verified.** Using only VMN's own tools we produced a
single-mix-server session over **P-256** with **width 2** (matching braid's default `W`), 10
ciphertexts, and ran the verifier on it:

```
============ Verify shuffle of Party 1. ========================
Read permutation commitment... done.   Verify proof of shuffle... done.
============ Verify decryption. ================================
Combine indicated decryption factors... done.  Verify combined proof of decryption... done.
Compute plaintexts... done.  Match computed plaintexts with plaintexts... done.
VMNV_EXIT_CODE=0
```

The proof directory contains exactly the files §9.1 predicts, no more and no fewer:
`version`, `type`, `auxsid`, `width`, `FullPublicKey.bt`, `Ciphertexts.bt`, `Plaintexts.bt`, and
`proofs/{Ciphertexts01,CorrectIndices,DecrFactCommitment01,DecrFactReply01,DecryptionFactors01,
PermutationCommitment01,PoSCommitment01,PoSReply01,PolynomialInExponent}.bt` plus `activethreshold`.

**Golden test vectors captured.** `vmnv -t par,der,bas,PoS,Dec,u` dumps the intermediate values a
Rust reimplementation must reproduce, including `der.rho` (the random-oracle prefix), `bas.h`
(independent generators), `PoS.s`/`PoS.v` and every `PoS.k_*`, and `Dec.s`/`Dec.v`. Observed defaults
for this session: `n_e = 256`, `n_r = 100`, `n_v = 256`, `s_H = s_PRG = SHA-256`, `κ = 1`, `ω = 2`.

**The byte-tree model is validated to the byte.** Predicting each file's size purely from the
documented structure and comparing against the real corpus:

| File | Predicted | Actual |
|---|---|---|
| `FullPublicKey.bt` | 167 | 167 |
| `Ciphertexts.bt` | 3275 | 3275 |
| `Plaintexts.bt` | 1635 | 1635 |
| `PermutationCommitment01.bt` | 815 | 815 |
| `PoSCommitment01.bt` | 2217 | 2217 |
| `PoSReply01.bt` | 970 | 970 |
| `DecryptionFactors01.bt` | 1635 | 1635 |

All seven match exactly. This is strong confirmation of three things at once: the byte-tree encoding,
the product-group array *transposition* (§6.6), and — because `PoSCommitment01.bt` and
`PoSReply01.bt` come out right only if `τ^pos` and `σ^pos` have precisely the documented component
lists — that **braid's `ShuffleCommitments` and `Responses` are structurally identical to
Verificatum's proof objects, field for field**.

One subtlety the hexdump exposed that the prose does not emphasise, and that would silently break a
naive implementation: **P-256 coordinates occupy 33 bytes, not 32.** The integer encoding is signed
(§6.1 Example 6 encodes −263 as `FEF9`), and `p` for P-256 has its top bit set, so every coordinate
carries a leading `0x00`. `FullPublicKey.bt` is 167 bytes rather than the 163 a 32-byte assumption
predicts.

**Platform note.** `vmnv` — the verifier, the only part that matters for the real design — runs fine
on Windows. VMN's *prover-side* tooling (`vmni`, `vmn`) does not: `ProtocolDefaults.RandomDevice()`
hardcodes `/dev/urandom`, and `HOST()` derives a default URL from the machine hostname which must
match a lowercase-only regex. Both are evaluated eagerly, before argument parsing, so no CLI flag
avoids them. The corpus above was therefore generated under WSL, inside an unprivileged UTS namespace
(`unshare -U -u --map-root-user`) with the hostname set to `localhost` — which changes nothing
outside that process tree. This is a Stage-0 inconvenience only: in the real interop design braid is
the prover and VMN is never anything but the verifier.

This corpus is now checked in at **`testdata/verificatum/`** — the proof directory, `protInfo.xml`
and `testvectors.txt` — because the interop tests depend on it. Its `README.md` documents how it was
generated and, importantly, **which constants in the tests are pinned to it** and would need
updating if it were regenerated. No secret material is included (`privInfo.xml` is deliberately
absent). The tests find it automatically, so a plain `cargo test` exercises them rather than
silently skipping.

---

## 1. How Verificatum's own verifier runs

`vmnv` is a thin `/bin/sh` wrapper around a Java entry point
(`crates/braid/verificatum/verificatum-vmn/bin/vmnv`):

```
java -server -Djava.security.egd=file:/dev/./urandom \
  com.verificatum.protocol.mixnet.MixNetElGamalVerifyFiatShamirTool \
  "$COMMAND_NAME" "$VERIFICATUM_RANDOM_SOURCE" "$VERIFICATUM_RANDOM_SEED" "$@"
```

Prebuilt JARs are already in the tree, so **no Java build is required**:

| JAR | Size | Contains |
|---|---|---|
| `verificatum-vmn/verificatum-vmn-3.1.0.jar` | 268 KB | `MixNetElGamalVerifyFiatShamir{,Session,Tool}` |
| `verificatum-vcr/verificatum-vcr-3.1.0.jar` | 1.8 MB | `ECqPGroup`, `ModPGroup`, `ByteTree*`, `RandomOracle`, `PRG*` |

VCR also ships a `MANIFEST_NM.MF` (no-native manifest), i.e. a pure-Java path that does not need
the GMP/VMGJ native libraries the launcher script references.

Invocation (VMNV §10.1, `vmnum` §6.1):

```
vmnv -mix      [-auxsid <id>] [-width <w>] [-nopos] [-nodec] <protInfo.xml> <nizkp-dir>
vmnv -shuffle  [-auxsid <id>] [-width <w>]                   <protInfo.xml> <nizkp-dir>
vmnv -decrypt  [-auxsid <id>] [-width <w>]                   <protInfo.xml> <nizkp-dir>
```

Silent on success with exit 0, non-zero on rejection; `-v` turns on progress output. Two inputs: a
**protocol info XML file** and a **proof directory**.

**Practical prerequisite:** there is no JRE on this machine (`java` is absent from PATH and from the
usual Windows install locations). Installing one is step zero for any hands-on work.

Highly relevant tooling discovered in VMNV §10: `vmnv -th` lists available **test vectors** and
`vmnv -t <names>` prints intermediate values, named to match the notation in the specification. The
tool exists explicitly to "check the compatibility of independent verifiers". We would be using it
in the mirror-image direction, but it serves our purpose equally well — it lets each layer (byte
trees, PRG, random oracle, independent generators) be validated against VMN before any proof is
attempted. VMNV Appendix A additionally contains PRG test vectors.

---

## 2. Cryptographic compatibility

### 2.1 Groups — the first potential showstopper: PASS, with a caveat

Verificatum implements exactly two group families (VMNV §6.7):

| Implementation | Description |
|---|---|
| `com.verificatum.arithm.ModPGroup` | Multiplicative subgroups of `Z_p*` |
| `com.verificatum.arithm.ECqPGroup` | Standard elliptic curves over prime-order fields |

The `ECqPGroup` curve list (VMNV §6.5) is fixed: P-192/224/256/384/521, the brainpool family, and
the `secp*`/`prime*` families.

**Ristretto255 is not supported and cannot be added without modifying Verificatum** — which would
defeat the purpose. Since `RistrettoCtx` is braid's default context, braid-as-configured-today is
incompatible.

**P-256 is supported by both**, and braid has a P-256 backend (`crates/vsc/src/groups/p256/`). I
verified empirically that the Terelius–Wikström shuffle proof — the artifact `vmnv` actually checks
— generates and verifies over P-256 in braid today (temporary probe test, since removed;
`crates/vsc/benches/shuffle.rs` also already benchmarks P-256).

One concrete gap found by running braid's full protocol over `P256Ctx`: it panics in the DKG at
`crates/vsc/src/groups/p256/group.rs:115`, because four functions are `todo!()` for P-256:

| Function | Line | Used by |
|---|---|---|
| `encode_bytes` | 76 | scalar/byte encoding |
| `decode_bytes` | 85 | scalar/byte encoding |
| `encrypt_scalar` | 115 | DKG share encryption |
| `decrypt_scalar` | 124 | DKG share decryption |

These matter only for **braid-internal DKG share transport**, which Verificatum never sees or
verifies. They are a prerequisite for running braid on P-256 at all, but they are not an interop
problem, and the Ristretto implementations (`groups/ristretto255/group.rs:136`) are a direct
template.

### 2.2 Proof of shuffle — the second potential showstopper: PASS

Both sides implement Terelius–Wikström. braid's `crates/vsc/src/zkp/shuffle.rs` says so in its
module docs and cites `EVS` Protocol 12.3; Verificatum cites Terelius–Wikström [11] in VMNV §8.3.
The correspondence is exact:

| Verificatum (VMNV §8.3) | braid (`zkp/shuffle.rs`) |
|---|---|
| `μ` — permutation commitment, array `u` | `ShuffleCommitments.u_n` |
| `τ^pos = node(B, A', B', C', D', F')` | `ShuffleCommitments { big_b_n, big_a_prime, big_b_prime_n, big_c_prime, big_d_prime, big_f_prime }` — same order |
| `σ^pos = node(k_A, k_B, k_C, k_D, k_E, k_F)` | `Responses { k_a, k_b_n, k_c, k_d, k_e_n, k_f }` — same order |
| `k_F ∈ R_{κ,ω}` | `k_f: [C::Scalar; W]` |
| `C = ∏u_i / ∏h_i` | `big_c = u_n_fold · h_n_fold⁻¹` |
| `D = B_{N-1}·h_0^{−∏e_i}` | `big_d = big_b_last · (h[0]^{∏e_i})⁻¹` |
| `B_{−1} = h_0` | `big_b_0 = h_generators[0]` |
| `A^v A' = g^{k_A} ∏h_i^{k_E,i}` | Verification 1 |
| `B_i^v B_i' = g^{k_B,i} B_{i−1}^{k_E,i}` | Verification 2 |
| `C^v C' = g^{k_C}` | Verification 3 |
| `D^v D' = g^{k_D}` | Verification 4 |
| `F^v F' = Enc_pk(1,−k_F)·∏(w'_i)^{k_E,i}` | Verification 5 |

This is the single most encouraging finding in the investigation: the hard cryptographic content is
already aligned, and no new proof system has to be designed or implemented.

### 2.3 El Gamal and ciphertext width: PASS

Verificatum parameterises by **key width κ** and **ciphertext width ω** (VMNV §2.1). braid's
ciphertexts are `[[C::Element; W]; 2]` over a single public-key element, which is exactly
**κ = 1, ω = W**. braid's `W` maps onto Verificatum's `width` file and `<width>` info-file field
directly.

### 2.4 DKG and key representation: compatible, requires a derived artifact

Verificatum models the joint key as a Shamir polynomial in the exponent
`Γ = (Γ_0, …, Γ_{λ−1})`, `Γ_s = g^{γ_s}`, with joint public key `y = Γ_0` and per-party key
`y_l = ∏_s Γ_s^{l^s}` (VMNV §2.1). It reads `Γ` from `proofs/PolynomialInExponent.bt`.

braid runs a Joint-Feldman/Pedersen DKG: each dealer `d` publishes commitments to its own polynomial
coefficients (`Shares.commitments` in `crates/braid/src/messages/artifact.rs`), and each trustee's
secret is the sum over dealers of the share addressed to it
(`crates/braid/src/trustee/decrypt.rs`: `secret = Σ_d decrypt(shares_d[self_slot])`).

These are the same object viewed differently. The joint polynomial is the sum of the per-dealer
polynomials, so **Γ is derivable from braid's data** as the component-wise product of the dealers'
commitment vectors, `Γ_s = ∏_d C_{d,s}`. braid's `DkgPublicKey.verification_keys` should then equal
the `y_l` Verificatum derives from Γ — a cheap and valuable cross-check to assert early.

### 2.5 Proof of correct decryption: was a structural mismatch, now largely closed

This was where the two designs genuinely diverged, and it is the one place the investigation changed
braid's own cryptography rather than only adding an emitter.

**Originally:** braid emitted, per party, *N* independent DLEQ proofs — one per ciphertext
(`DecryptionFactor { value, proof: DlogEqProof }`) — while Verificatum expects *one batched proof
over all N*: `τ_l^dec = node(y'_l, B'_l)` and `σ_l^dec = k_{x,l}` (VMNV §8.6), verified against
`A = (∏u_i^{e_i}, 1)` and `B = ∏f_i^{e_i}`.

**Now:** braid batches too. `Recipient::partial_decrypt` publishes `N` factors and a single
`DlogEqProof` over the same random linear combination — the statement is unchanged, so no new proof
type was needed, only different bases. This was adopted on its own merits (smaller messages, one
proof to verify per party instead of `N`) with the interop as a secondary benefit; see the
`feat/braid-0.6-batched-decryption` work.

Three conventions still differ, and the emitter still bridges them:

- **Sign.** Verificatum's decryption factor is `f_l = u^{−x_l}` and combines as `TDec = v·f`;
  braid computes `u^{+x_l}` and divides during combination.
- **The α trick.** Verificatum computes `f_l = PDec_{x_l/α}` with `α = lcm(1,…,k)²`, cancelling it
  in the Lagrange combination so the coefficients stay small integers (VMNV §2.2). braid keeps plain
  Lagrange over the unscaled share — **deferred, not rejected on principle**; see below.
- **The batching transcript.** braid derives its exponents from its own hash of the verification key,
  ciphertexts and factors; Verificatum derives them from `RO_seed` over a byte-tree of `Γ` and *all
  k* parties' factors. These are necessarily different, so a braid proof is not a VMN proof.

The consequence for the emitter is unchanged: `vmn::decrypt::prove_decryption` still produces
Verificatum's proof from the secret rather than converting braid's, and the factors are still
re-exponentiated by `−1/α` at the boundary.

#### Why α is deferred for braid's own path

Not because it conflicts with anything. The design space collapsed to a single question, and α is the
only variable left in it:

- **Combining the parties' proofs into one is ruled out**, because it needs a shared `A` and a shared
  challenge, hence batching exponents seeded over *every* party's factors, hence factors published
  before anyone can commit. That is Verificatum's three-round decryption. braid publishes factors and
  proof together in one round and keeps proofs per party so a failure names its author, which the
  board needs in order to make progress. Ruling out the extra rounds rules out combining.
- **Given per-party proofs, α is a pure performance choice** confined to `combine`. Two shapes work:
  keep factors at `u^{x_l}`, combine by `α·c_l` and take one α-th root per ciphertext (proof
  untouched); or scale the witness as VMN does (no root, proof carries α). The first is less invasive
  here.

What blocks it is neither of those: `C::Scalar` is a fixed-width field element, so `α·c_l = 108` at
`k = 3` costs exactly what a 256-bit scalar costs — constant-time scalar multiplication walks all 256
bits by design. Realising the ~34× on that step needs a small-exponent entry point in the group API,
variable-time in an exponent that is public here but not in general. Verificatum has the equivalent:
`combineDecryptionFactors` passes the coefficients' maximum bit length into `expProd` precisely so
the ladder runs 7 iterations instead of 256.

And the step is `λN` of `combine`'s `3λN`, so even fully realised it is worth roughly a fifth. There
is no lock-in — α would live entirely inside `combine` — so this is a decision to take against a
profile, not now.

### 2.6 Fiat–Shamir transcript: THE dominant incompatibility

Every challenge in both systems is derived by hashing a transcript. The verifier recomputes it. So
the transcripts must agree **byte for byte**, and currently they disagree in every particular:

| Aspect | Verificatum | braid |
|---|---|---|
| Serialization | byte trees (VMNV §4) | VSer |
| Hash | SHA-256/384/512 only; VMNV §5.1 says SHA-3 is "future" | SHA3-512 (`Hasher512`) |
| PRG | `r_i = H(s ‖ bytes₄(i))` (VMNV §5.2) | hash + counter, different framing |
| Random oracle | output length prefixed: `s = H(bytes₄(n_out) ‖ d)`, expand, mask (VMNV §5.3) | `hash_to_scalar` with domain-separation tags |
| Batching exponents `e_i` | `n_e`-bit integers, `t_i mod 2^{n_e}` | full-width scalars (`challenge_e_n`) |
| Challenge `v` | `n_v`-bit integer | full-width scalar |
| Global prefix | `ρ = H(node(version, sid‖"."‖auxsid, n_r, n_v, n_e, s_PRG, s_Gq, s_H))` (VMNV §9.3 step 4) | none — no protocol-info binding |
| Independent generators | `randomArray` with a quadratic-residue walk over derived x-coordinates (VMNV §6.8) | `ind_generators` via hash-to-curve |

Note that braid's `hash_to_scalar` reduces into the scalar field, whereas Verificatum deliberately
works with fixed-bit-length non-negative integers and *does not* reduce mod q. Even with an
identical hash these produce different values.

**This is why a converter cannot work.** A proof's challenges are fixed when the proof is created.
Translating braid's serialization afterwards changes the bytes `vmnv` will hash, so the challenges
`vmnv` recomputes will not be the ones braid used, and every verification equation fails. The
transcript must be produced VMN's way *at proving time*.

---

## 3. What the verifier's input actually looks like

Two inputs.

**Protocol info file** (`protInfo.xml`, VMNV §7): UTF-8 XML, single `<protocol>` block. The verifier
uses only the preamble; `<party>` blocks can be ignored entirely. Values consumed (VMNV §7.2):
`version`, `sid`, `nopart` (k), `thres` (λ), `ebitlenro` (n_e), `statdist` (n_r), `vbitlenro` (n_v),
`rohash` (s_H), `prg` (s_PRG), `pgroup` (marshalled group), `keywidth` (κ), `width` (ω default).

**Proof directory.** Root holds `version`, `type` (`mixing`/`shuffling`/`decryption`), `auxsid`,
`width` as ASCII, plus `FullPublicKey.bt`, `Ciphertexts.bt`, and either `Plaintexts.bt` or
`ShuffledCiphertexts.bt`. A `proofs/` subdirectory holds the intermediate values and proofs
(VMNV §9.1):

| File | Contents |
|---|---|
| `activethreshold` | λ_a, ASCII decimal |
| `PolynomialInExponent.bt` | Γ = (Γ_0,…,Γ_{λ−1}) |
| `Ciphertexts<l>.bt` | l-th intermediate ciphertext list L_l |
| `PermutationCommitment<l>.bt` | μ_l |
| `PoSCommitment<l>.bt` / `PoSReply<l>.bt` | τ_l^pos / σ_l^pos |
| `CorrectIndices.bt` | boolean array of length k+1 selecting Δ, \|Δ\| = λ |
| `DecryptionFactors<l>.bt` | f_l |
| `DecrFactCommitment<l>.bt` / `DecrFactReply<l>.bt` | τ_l^dec / σ_l^dec |

(`maxciph`, `PoSC*`, `CCPoS*`, `KeepList<l>.bt` appear only when pre-computation was used — braid
has no analogue and would simply not produce them.)

Byte trees themselves are trivial: a leaf is `01 ‖ len₄ ‖ bytes`, a node is `00 ‖ count₄ ‖ children`
(VMNV §4). Group elements on a prime-field curve are `node(leaf(x), leaf(y))` with fixed-width
coordinates, and the point at infinity is `node(leaf(−1), leaf(−1))` (VMNV §6.5). Arrays of
product-group elements transpose: an array of ω-wide ciphertexts is stored as ω arrays, not N tuples
(VMNV §6.6) — an easy detail to get wrong.

---

## 4. Mapping braid's execution onto the expected shape

braid's protocol maps onto Verificatum's model cleanly at the structural level:

- braid's mix chain (`Ballots` → `Mix` → … driven by `mixing_position` in
  `crates/braid/src/datalog/mix.rs`) is exactly Verificatum's `L_0 … L_{λ_a}`.
- braid's `Ballots.trustees` (the ordered mixing set) determines which indices `<l>` appear.
- braid's final `Plaintexts` is Verificatum's `Plaintexts.bt`.
- braid's `Configuration` supplies most of `protInfo.xml`'s preamble.

Two semantic details need care. Verificatum's rule that a deactivated or rejected mix-server yields
`L_l = L_{l−1}` has no braid counterpart (braid halts instead), and `activethreshold`/`CorrectIndices`
encode a notion of partial participation that braid's all-or-halt datalog does not currently express.
For a first PoC with all trustees participating, both are constants.

---

## Stage 1 results — EXECUTED, acceptance criterion met

Implemented as **`crates/vcompat`**, a standalone, dependency-free crate (`hex` is dev-only). It is
deliberately small and auditable, because this is the layer whose bytes must match VMN exactly.

| Module | Role |
|---|---|
| `bytetree` | leaf/node encoding, **strict** parsing (trailing bytes rejected) |
| `arithm` | signed integers, fixed-width field elements, curve points, product-array transposition, boolean arrays |
| `marshal` | `comment::hex` group descriptors, P-256 parameters |

**Acceptance: all 12 byte trees in the Stage 0 corpus parse and re-emit byte-identically**, and
`serialized_len()` agrees with every file size. Structural checks pass too — the public key's `g`
decodes to the standard P-256 base point, `Ciphertexts.bt` transposes cleanly into 10 width-2
ciphertexts, `τ^pos` has its 6 components with `|B| = |B'| = 10`, and `σ^pos` has its 6 with
`|k_B| = |k_E| = 10` and `|k_F| = ω = 2`.

Fourteen tests in total: twelve conformance tests drawn from the VMNV worked examples (§4, §6.1,
§6.2, §6.3, §6.7) plus the size predictions, and two corpus tests. The corpus is throwaway demo key
material and is **not** checked in; the corpus tests are opt-in via `VCOMPAT_CORPUS`:

```
VCOMPAT_CORPUS=/path/to/nizkp/default cargo test -p vcompat
```

Without it they report a skip and pass, so the suite stays green anywhere.

Parsing strictness is deliberate rather than incidental: these bytes are hashed into Fiat–Shamir
transcripts, so a parser that accepted two encodings of one value would be a malleability surface.

Both encoding traps from Stage 0 are now pinned by tests — the 33-byte signed coordinate width
(`p256_width_is_33_not_32`) and the product-array transposition
(`example_14_product_arrays_are_transposed`).

---

## Stage 2 results — EXECUTED, THE GATE IS PASSED

Stage 2 was the go/no-go gate: braid's proof *algebra* already matches Verificatum's, so the
approach lives or dies on whether the **Fiat–Shamir transcript** can be reproduced bit for bit.

**It can.** The whole transcript path for a proof of a shuffle now reproduces exactly, checked
against values a real `vmnv` printed for the Stage 0 reference proof:

| Value | Derivation | Result |
|---|---|---|
| `der.rho` | `H(node(version, sid.auxsid, n_r, n_v, n_e, prg, pgroup, rohash))` | exact match |
| `PoS.s` | `RO_seed(ρ ‖ node(g, h, u, pk_ω, w, w'))` | exact match |
| `PoS.v` | `RO_challenge(ρ ‖ node(leaf(s), τ^pos))` | exact match |

The `PoS.v` check is the strongest single result: it combines ρ computed from the protocol info
parameters, the batching seed, and the **real `PoSCommitment01.bt` bytes parsed off disk** by our own
byte-tree parser. If the encoding, the oracle construction, or the query framing were wrong anywhere,
it would not match.

Implemented in `vcompat::crypto`: the three SHA-2 variants, VMN's PRG (`r_i = H(s ‖ bytes₄(i))`), the
random oracle (output-length-prefixed seed, then PRG expansion, then leading-bit masking), the global
prefix ρ, and the shuffle proof's seed and challenge queries. 24 tests pass in total.

### Two details the specification does not make obvious

Both were found by reading VMN's Java rather than the prose, and both silently produce a wrong
challenge with no useful diagnostic:

1. **The public key is widened before it enters the seed query.** VMNV §8.3 lists the input as
   `pk ∈ C_κ`, which reads like the stored `FullPublicKey.bt`. The verifier actually uses
   `getWidePublicKey(pk, ω)` — for ω > 1 that is `((g,…,g)_ω, (y,…,y)_ω)`, not `(g, y)`. This cost a
   debugging cycle; it is now [`crypto::wide_public_key`] with the reasoning recorded.
2. **ρ's session identifier is a dot-join, and `pgroup` keeps its comment.** ρ commits to
   `sid ‖ "." ‖ auxsid` as one string, and to the entire `<pgroup>` value *including* the
   `ECqPGroup(P-256)::` comment prefix, not just the hex payload.

### Independent generators — also DONE

Subsequently closed: `vcompat::generators` implements VMNV §6.8's quadratic-residue walk (PRG →
45-byte candidates → mask to `n_p + n_r` bits → reduce mod p → keep those where `f(z)` is a quadratic
residue), and **the derived generators match `vmnv -t bas.h` exactly**. With `h` derived rather than
borrowed, the batching seed now reproduces from the protocol parameters and proof files alone — the
position a real emitter is in.

A third trap, caught by that golden test: **the root choice is normalised one level above `sqrt`.**
`ECqPGroup.sqrt` returns `v^((p+1)/4)`, which is the larger root about half the time; it is
`randomElementArray` that then applies `y' = p - y; if (y' < y) y = y'`. Implementing from `sqrt`
alone yields every x-coordinate right and half the y-coordinates inverted — which is precisely the
signature the failing test showed, and why it was diagnosable in one step.

### What Stage 2 did not cover

- **The decryption transcript** (`Dec.s`, `Dec.v`) is not yet reproduced. The shuffle path is the
  representative and harder case, so this is expected to be mechanical — but it is unproven.

**Verdict: the go/no-go gate is passed.** Nothing found so far blocks a Verificatum-compatible
emitter in braid, and the layer that was most likely to be intractable is now demonstrated working.

---

## Stage 3 results — EXECUTED, interop achieved in both directions

**Unmodified `vmnv` verifies a proof of a shuffle produced entirely by braid's cryptography, and
braid verifies one produced by Verificatum.** The central question of this investigation is answered
affirmatively.

How it fits together:

- `vsc`'s `Shuffler` takes its two Fiat–Shamir challenges through a `ShuffleChallenges` trait.
  `NativeChallenges` is the default and preserves braid's existing behaviour exactly.
- `braid::vmn::challenges` implements Verificatum's convention against that seam;
  `braid::vmn::generators` supplies VMNV §6.8 generators as vsc elements (required, since `h` feeds
  both the Pedersen commitments and the batching seed); `braid::vmn::encode` is the sole adapter
  between vsc types and byte trees; `braid::vmn::proof_dir` writes the directory.
- `vcompat` remains a pure format-and-transcript library with no dependency on vsc.

### Direction 1 — braid verifies Verificatum

Reads a real VMN shuffle proof off disk, rebuilds it as vsc types, verifies with vsc's own code.
Passes, exercising encoding, transcript and algebra at once. Three negative controls (generators from
a different prefix, a mismatched prefix, swapped output ciphertexts) are all rejected.

### Direction 2 — Verificatum verifies braid

braid shuffles 8 width-2 ciphertexts over P-256, writes a `type=shuffling` proof directory, and:

```
============ Verify shuffle of Party 1. ========================
Read permutation commitment... done.
Read output of Party 1... done.
Verify proof of shuffle... done.
VMNV_EXIT_CODE=0
```

Four independent tampering controls are correctly **rejected** (exit 1): a flipped bit in
`PoSReply01.bt`, in `PoSCommitment01.bt`, in `PermutationCommitment01.bt`, and in the input
ciphertexts. So the acceptance is discriminating, not vacuous.

### A defect found in `vmnv`

One control was *accepted*: replacing the output list with the input, so the proof no longer holds.
`vmnv` **detects** this and, under `-v`, says so —

```
Verify proof of shuffle... failed.
--> Replacing output of Party 1 by its input.
--> Too few proofs are valid! (0)
```

— then exits **0**. Verificatum's own proofs behave identically under the same tampering, so this is
`vmnv`'s behaviour, not braid's emitter.

**Root cause.** `MixNetElGamalVerifyFiatShamirSession` reaches the right conclusion and routes it to
the wrong handler:

```java
if (validProofs < v.threshold) {
    v.failInfo("Too few proofs are valid! (" + validProofs + ")");
}
```

`failInfo` only prints, and only when verbose:

```java
void failInfo(final String message) {
    if (verbose) { println("--> " + message); }
}
```

Its sibling `failStop` prints a `FAIL!` banner and `throw new ProtocolError(...)`, halting.
`validProofs < threshold` is exactly VMNV §2.3's reject condition — *"If less than λ proofs are
valid, then reject"* — so the condition is evaluated correctly and then not enforced. This reads as a
straightforward bug rather than an intentional semantic: the guard exists, and one identifier is
wrong.

**Scope**, established by experiment:

| Invocation | Result on a proof `vmnv` internally rejected |
|---|---|
| `-shuffle` with `-v` | exits 0, prints the failure |
| `-shuffle` without `-v` | **exits 0 with no output at all** |
| `-mix -nodec` | exits 0 |
| `-mix` (full) | exits 1 — rejected |

The full mixing path is safe, but only *incidentally*: the downstream plaintext comparison catches
it, not the shuffle check. Disabling decryption verification — a documented option — re-exposes it.

**Why it matters.** A shuffling session is VMN's documented mode for re-randomising without
decrypting (§2.4). Accepting one in which no mixing occurred means accepting a mix-net that provided
no privacy, silently, with a zero exit status. §10.1 designates the exit code as *the* accept/reject
signal, so a conforming integration would be misled.

**Status.** Understood, reproduced, and pinned by two tests in `braid/tests/vmn_verifier.rs`
(`vmnv_exit_code_alone_is_not_sufficient` and `vmnv_is_silent_about_a_failed_shuffle`) that will fail
if it is ever fixed upstream. **Not yet reported to the Verificatum maintainers** — that is the
outstanding action.

**How this project is protected.** The danger for us is not the bug itself but what it hides: if
braid's transcript layer ever drifts, the emitted proofs stop verifying and `vmnv` reports that by
exiting 0, so an exit-code-only check would stay green while the interop was silently broken. So:

- `vmnv_accepts()` in `vmn_verifier.rs` is the single place that decides acceptance, and it requires
  a zero exit **and** `Verify proof of shuffle... done.` in the output. Every test asks through it.
- `vmnv_would_catch_emitter_drift` perturbs the prefix to simulate exactly that regression, asserts
  `vmnv` exits 0 on the result — confirming the trap is real — and asserts our check rejects anyway.
  The guard is demonstrated, not just claimed.
- If this interop later grows a CI job or tooling, it must use that predicate, never `$?`.
- When `-mix` becomes reachable, note that full `-mix` rejects these cases only via its downstream
  plaintext comparison; **`-mix -nodec` is affected exactly like `-shuffle`** and must not be treated
  as a safe way to check the mixing phase alone.

(Separately, `vmnv` exits 1 on rejection, not the `-1`/255 that §10.1 specifies.)

### A second silent skip: a mixer is verified only if its proof file exists

Found later, while generating corpora at arbitrary shapes, and the same shape of problem.

Mixer slots are numbered by **party index**, not sequentially. A party that takes no part in
shuffling leaves a gap: with the active set `{1,3}`, the directory holds `PermutationCommitment01`
and `03` but no `02`, and `activethreshold` is `3` — the highest active index, *not* the count. So a
verifier cannot iterate `1..=activethreshold` and assume a proof for each; ours did, and broke on the
first such corpus.

How `vmnv` knows which to skip is the interesting part:

```java
public boolean getPoSCActive(final int l) {
    final File file = PermutationCommitment.PCfile(proofs, l);
    return file.exists();
}
```

**Presence of a file decides whether a mixer is verified.** The whole body of the loop — reading the
permutation commitment, reading the output list, checking the proof — sits inside that condition, so
a slot with no proof file contributes nothing and is passed over without comment. Remove a mixer's
`PermutationCommitment<l>.bt` and `PoSCommitment<l>.bt` and `vmnv` does not reject: it verifies a
shorter chain and reports success.

That is not a soundness break — the remaining shuffles are still checked, and the output is still
tied to them — but the *privacy* claim quietly weakens, since a session presented as an `n`-mixer mix
may have had its output produced by fewer. Nothing in the transcript binds the number of mixers,
because `activethreshold` is read from the same directory an attacker would be editing.

**What we do about it.** Skipping absent slots is necessary — otherwise valid VMN proofs are rejected
— so we match the behaviour but count what was actually verified and require it to meet the
threshold. The rule for our verifier stays the one stated in `verify.rs`: match VMN where a valid
proof depends on it, but never inherit a silence.

### Caveats

**Multiple parties — DONE.** `vmnv` accepts a chain of three braid mixers, each shuffling the
previous output, verifying all three proofs. The emitter writes the per-party files
(`Ciphertexts<l>.bt`, `PermutationCommitment<l>.bt`, `PoSCommitment<l>.bt`, `PoSReply<l>.bt`) and a
real `activethreshold`. Corrupting any single mixer's reply breaks the chain, so acceptance is not
carried by the other members.

**A note on step 5, where an early reading of ours was wrong.** VMNV §9.3 step 5 reads the keys, and
Algorithm 24 splits that into two parts: read the joint public key `pk`, then read the polynomial in
the exponent `Γ` and *reject if `Γ_0 ≠ y`*. `vmnv` does the first unconditionally and the second only
when verifying decryption — deleting `FullPublicKey.bt` fails a `-shuffle` run outright, while
deleting `PolynomialInExponent.bt` leaves it at exit 0.

Both halves matter, for different reasons. `pk` is genuinely required by a shuffling proof: Algorithm
25 takes it as an input and it appears in the fifth verification equation, which is why step 5 sits
ahead of the type branch. `Γ` is *not* among Algorithm 25's inputs, so skipping it is sound for
`vmnv` — but a verifier written to the specification still reads it and cross-checks it against `pk`.

braid's emitter therefore writes it, rather than relying on `vmnv`'s leniency: a proof that passes
only because one implementation skips half a step is not the property this exercise is buying. Since
Algorithm 24 *checks* the file rather than merely requiring it, supplying a wrong `Γ` would be worse
than omitting it — so the emitter validates `Γ_0 = y` and refuses otherwise.

Deriving `Γ` from a real DKG — `Γ_s = ∏_d C_{d,s}` over braid's per-dealer commitments (§2.4) —
belongs to the decryption work, where the shares are actually used.

**Decryption — remaining.** Shuffle only so far. `vmnv -mix` needs the batched decryption proof braid
does not yet have (§2.5), plus the `Dec.s`/`Dec.v` transcript, the decryption artifacts in the proof
directory, and the real `Γ` above. This is new cryptography, not a translation.

### Non-participating parties, and why braid's model differs

The two systems disagree about who publishes decryption factors, and the emitter has to bridge it.

- **VMN:** *all* `k` mix-servers publish factors. `CorrectIndices.bt` then names Δ, the λ whose
  proofs are correct, and only those are Lagrange-combined. The batching seed commits to **all `k`**
  factor arrays, not only Δ's.
- **braid:** only the λ trustees selected for the mix produce factors at all. The datalog's
  `ComputePartialDecryptions` requires a `mixing_position`, and `partial_decryptions_all` fires only
  at `threshold`, so either every participant succeeds or the protocol stalls. There is no
  recover-with-a-different-subset path. Lagrange is still used, and its coefficients are non-trivial,
  because the participant set is an arbitrary λ-subset of the trustees rather than `{1..λ}`.

So braid produces λ factor arrays where VMN's verifier wants `k`, and the gap cannot be closed by
relabelling participants to `1..λ`: shares are evaluations `p(l)` at braid's own trustee indices, and
`y_l = ∏_s Γ_s^{l^s}` only holds if VMN's party index equals braid's trustee index.

**What to write for the missing parties** (from `DistrElGamalSession.exchangeDecryptionFactors`): an
array of the group identity, one per ciphertext, in the width-ω plaintext group.

```java
tempLog.info("Not active, setting to all-one array.");
correct[l] = false;
...
if (!correct[l]) {
    decryptionFactors[l] = leftPGroup.toElementArray(size, leftPGroup.getONE());
}
```

The file cannot simply be omitted: the verifier's `readArray` calls `failStop` on a missing file, so
every `l ∈ [1, k]` needs one. For P-256 the identity is the point at infinity,
`node(leaf(−1), leaf(−1))`, which `vcompat::arithm::point_at_infinity` already emits.

**The same applies to the proof files, and there the values are load-bearing.**
`DecrFactCommitment<l>.bt` and `DecrFactReply<l>.bt` are also read for every party, and
`DistrElGamalSessionBasic` falls back to `yp[l] = getONE(); Bp[l] = getONE()` and
`k_x[l] = getZERO()`, which `DistrElGamalSession` then writes out. Those are not free to choose even
though the party is excluded from Δ: `getCommitment()` builds its container over **all `k`** parties
and that container is hashed into the decryption challenge, so a different placeholder moves `v` and
breaks the participants' proofs. `vmnv_accepts_a_mixing_proof_with_an_inactive_party` asserts exactly
this, by substituting a well-formed commitment over the generator and requiring rejection — the
substitute has to parse, because `setCommitment` falls back to the identity on a malformed file and
a corrupted one would leave the challenge unchanged.

So the emitter's rule is: **`DecryptionFactors<l>.bt` for a non-participant is an all-identity array
of the same shape as a real one, and `CorrectIndices.bt` marks that party false.** Δ is then exactly
braid's participant set, whose size is already λ — which matches VMN's requirement that `|Δ| = λ`
exactly.

Also confirmed from the same source, resolving the sign and scaling questions of §2.5:
`f_j = u^{−x_j·(1/α)}` with `α = lcm(1,…,k)²`, i.e.
`firstComponents.exp(secretKey.neg().mul(inverseFactor))`.

### The batched decryption proof, from source

The specification's rendering of §8.6's equations is garbled enough by OCR not to be trustworthy
(it shows `Γ_0^{-v} y' = g^{k_x}` alongside a `PDec_{k_x}(A)` term whose signs do not reconcile), so
these come from `DistrElGamalSessionBasic` instead.

**Verification** (`verifyCombined`) is a plain combined Chaum–Pedersen, two equations:

```java
combinedy.inv().exp(v).mul(combinedyp).equals(g.exp(combinedk_x))   //  y^-v · y' = g^k_x
&& combinedB.exp(v).mul(combinedBp).equals(A.exp(combinedk_x))      //  B^v · B' = A^k_x
```

**The honest prover** therefore picks a random `r` and sets

```text
y' = g^r        B' = A^r        k_x = r − v·x
```

Both equations then hold, since `B = A^{−x}`: the first gives `g^{−xv + r} = g^{k_x}`, the second
`A^{−xv + r} = A^{k_x}`, and they agree on `k_x = r − vx`. `A` is the batched first components
`∏ u_i^{e_i}` and `B` the batched combined factors `∏ f_i^{e_i}`.

**The α trick**, which is what makes the combination cheap (`prodFactor`,
`modifiedLagrangeCoefficient`):

1. `α = lcm(1,…,k)²`, computed as `∏_{p ≤ k prime} p^{⌊log_p k⌋}` squared.
2. Each party scales its secret down: `f_l = u^{−x_l/α}`.
3. Combination uses the exponent `α·c_l`, which α makes an **integer** rather than a field element,
   so `∏_l f_l^{α c_l} = u^{−Σ c_l x_l} = u^{−x}` and the α cancels.
4. That integer is then reduced to the representative of **smallest absolute value**, choosing
   between `res` and `res − q` — so **it may be negative**. This is the point of the whole
   manoeuvre: exponents stay small integers instead of full-width scalars.

**Δ is the first λ true entries** of `CorrectIndices.bt`, not an arbitrary subset — the loops in
`modifiedLagrangeCoefficients` stop at `threshold`. braid always has exactly λ participants, so it
must mark exactly λ true; marking more would silently select a prefix.

P-256 is no longer a caveat: the encoding gap is closed and braid's full protocol — DKG, mix,
threshold decryption, board union — runs over it.

---

## Stage 4 results — EXECUTED, `vmnv -mix` accepts a complete braid session

**Unmodified `vmnv -mix` accepts a full braid mixing session**: a real three-party DKG, a chain of
three shuffles, and threshold decryption with the batched proof. This is the whole exercise, not
just the shuffle half.

`vmn_verifier.rs::vmnv_accepts_a_braid_mixing_proof` runs it at `k = λ = 3` against
`protInfo-3party.xml`:

```text
============ Verify decryption. ================================
Read indices of correct decryption factors... done.
Read decryption factors... done.
Combine indicated decryption factors... done.
Batch input... done.          Batch combined decryption factors... done.
Combined proofs... done.      Verify combined proof of decryption... done.
Compute plaintexts... done.   Read plaintexts... done.
Match computed plaintexts with plaintexts... done.
```

`k = λ = 3` is chosen deliberately. Every party decrypts, so the non-participant path is not
exercised — but nothing else is degenerate: `α = lcm(1,2,3)² = 36`, and the modified Lagrange
coefficients over `{1,2,3}` are `108`, `−108` and `36`, none of them the identity that the
single-party corpus collapses to. Unlike `-shuffle`, `-mix`'s exit code *is* a sound signal here,
because the downstream plaintext comparison uses `failStop` (see the `vmnv` defect above); the test
still asserts on the transcript as well.

Emitting requires, beyond the shuffle files: `Plaintexts.bt`, `CorrectIndices.bt`, and per party
`DecryptionFactors<l>.bt`, `DecrFactCommitment<l>.bt`, `DecrFactReply<l>.bt` — written by
`vmn::proof_dir::MixingProof` — plus a real `Γ` from braid's per-dealer commitments, which is now
derived by `vmn::decrypt::polynomial_in_exponent` and cross-checked against the DKG's own joint key.

### Verificatum's specification disagrees with Verificatum's implementation about where α goes

This is the substantive finding of the stage, and it cost the most to establish. Both sides of the
disagreement are Verificatum's own:

- **the specification** — `vmnv-3.1.0`, *How to Implement a Stand-Alone Verifier for the Verificatum
  Mix-Net*, cited throughout this document as "VMNV §N";
- **the implementation** — the Java in `verificatum-vmn`, specifically `DistrElGamalSessionBasic`.
  That one class serves *both* roles: VMN's prover calls it to write `DecrFactReply<l>.bt`, and
  `vmnv` calls it to check them.

Which is why the discrepancy is invisible from inside Verificatum. Real VMN proofs verify against
real `vmnv` because the same class writes and reads them; only a third implementation, written from
the document, would find them wrong.

**They agree on the verification equations.** Both check

```text
Γ_0^{−v} · y' = g^{k_x}          B^v · B' = A^{k_x}
```

with no α anywhere. These are stated over the **combined** values, and by the time `B` is formed the
α has already cancelled: `B` is the batching of `f_i = ∏_l f_{l,i}^{α c_l} = u_i^{−x}`. So the proof
never sees α at all, which is why VMNV §2.4 can speak of the unscaled `x_l` even though `x_l/α` is
what was physically exponentiated.

**They disagree one level down, on how the combined values are built** from the parties' pieces:

```text
specification    y' = ∏ (y'_l)^{c_l}       B' = ∏ (B'_l)^{c_l}       k_x = Σ c_l · k_{x,l}
implementation   y' = ∏ (y'_l)^{α c_l}     B' = ∏ (B'_l)^{α c_l}     k_x = Σ α c_l · k_{x,l}
```

(The *factors* are combined by `α c_l` in both — that part is not in dispute.) Since the combined
`k_x` has to come out as `R − v·x` either way, the two rules demand different replies from the
prover: `k_{x,l} = r_l − v·x_l` for the specification, `r_l − v·(x_l/α)` for the implementation
(`k_x[j] = x.neg().mul(inverseFactor).mul(v).add(r)`).

### Why α may be included or omitted here

The only constraint the verification equations place on the proof combination is

```text
Σ_l γ_l · w_l = x
```

for whatever `γ_l` combines the pieces and `w_l` each party replies over — because that sum is what
appears in `k_x`, and it has to match the `x` inside `Γ_0` and `B`. Two solutions:

| | `γ_l` | `w_l` |
|---|---|---|
| specification | `c_l` | `x_l` |
| implementation | `α c_l` | `x_l/α` |

Both satisfy it, and they differ in the emitted bytes. `y'` and `B'` cancel nothing — they are
`g^{r_l}` and `A^{r_l}`, with no α in them — but they must use the same `γ_l` as `k_x`, so that the
`R = Σ γ_l r_l` they define is the `R` sitting inside the reply.

Only the **factor** combination is genuinely forced to `α c_l`: that is the unique exponent set
undoing the `1/α` the factors were computed with, and it is the whole reason α exists, since
`combineDecryptionFactors` is then an `expProd` with small integer exponents over all `N`
ciphertexts. The proof pieces are under no such constraint. Verificatum reused the coefficient array
it already had — `combine` calls the same `modifiedLagrangeCoefficients` — and that reuse, not any
requirement of the scheme, is what forces its witness to be scaled.

They coincide only when `α = 1`, i.e. `k = 1`, which is exactly why the single-party reference corpus
cannot tell them apart, and why this had to wait for a multi-party run to settle. braid follows the
implementation (`vmn::decrypt::prove_decryption` takes `x_l/α`), and the `-mix` test above is the
adjudication.

**Why the defect is quiet.** VMNV is a *verifier* specification: it contains no prover algorithm for
decryption, and `k_{x,l}` appears in the whole document exactly twice — once as a type declaration
and once inside the combination formula. So the document holds no wrong formula to notice, only a
combination rule whose implied prover is not the one Verificatum ships. It surfaces only when
someone builds a prover from the document and watches real `vmnv` reject the result.

**Why it matters beyond us.** A third implementation written strictly from VMNV §8.6 would reject
every genuine Verificatum mixing proof with more than one party — and every braid proof too, since
braid now emits the same bytes VMN does. It is a documentation defect rather than a soundness one,
but it lands directly on the goal this investigation exists to serve: `vmnv-3.1.0` exists precisely
so that stand-alone verifiers can be written from it, and on this point it is not sufficient for
that.

### The non-participant path — also confirmed

`vmnv_accepts_a_mixing_proof_with_an_inactive_party` runs the same emitter at `k = 3`, `λ = 2`
against a 3-of-2 info file, with **party 2 taking no part** — Δ is `{1, 3}` rather than `{1, 2}` so
the gap sits in the middle, where an off-by-one in party indexing would show. `α` is still
`lcm(1,2,3)² = 36`, a function of `k` and not of the threshold, and the coefficients are `54` and
`−18`.

This is the case braid's own model does not produce: braid only computes decryption factors for the
trustees selected for the mix, whereas VMN expects a file from every party and names Δ separately.
The bridge is the all-identity factor array, identity commitment and zero reply described under
"Non-participating parties" below, and this test is what confirms it against `vmnv` rather than
against a reading of the source.

### What Stage 4 did not cover

- **Wiring to the live protocol.** The tests drive the DKG, shuffle and decryption directly rather
  than through `Trustee::step` and the board, so they prove the cryptography and the emitter, not the
  session plumbing.
- **Anything but P-256 at width 2.** Which is inherent — `vmnv` supports no group braid also
  supports other than the standard curves.

---

## 5. Recommended path

Staged, each stage independently checkable, ordered so the cheapest disproof comes first.

**Stage 0 — get a reference. ✅ DONE** (see "Stage 0 results" above). Reference corpus generated over
P-256/width-2, verified by unmodified `vmnv` with exit 0, golden test vectors captured, byte-tree
model validated against seven files.

**Stage 1 — byte trees and P-256 encoding in Rust. ✅ DONE** — implemented as the `vcompat` crate
(`crates/vcompat`), standalone and dependency-free. See "Stage 1 results" below.

**Stage 2 — VMN's hash / PRG / random oracle. ✅ DONE (gate passed)** — ρ, `PoS.s` and `PoS.v` all
reproduce exactly; see "Stage 2 results" above. Remaining within this layer: deriving the independent
generators (VMNV §6.8's quadratic-residue walk, needs bignum) and the decryption transcript.

**Stage 3 — shuffle-only proof, `vmnv -shuffle`. ✅ DONE** (see "Stage 3 results" above).

Original plan: The smallest end-to-end win, and the one that
needs no DKG work: emit `PermutationCommitment`, `PoSCommitment`, `PoSReply` from a braid shuffle
whose challenges were derived VMN-style, with `type = shuffling`. Getting exit code 0 here proves
the concept.

**Stage 5 — P-256 DKG. ✅ DONE (done early, out of order).** The four `todo!()`s are filled in and
braid's full protocol runs over P-256; it was needed before anything could be emitted at all.

**Multi-party shuffling. ✅ DONE** (not in the original plan, done after Stage 3): `vmnv` verifies a
three-mixer chain. See the Stage 3 results.

**Stage 4 — decryption, `vmnv -mix`. ✅ DONE** (see "Stage 4 results" above). The batched decryption
proof (§2.5), Γ from braid's per-dealer commitments (§2.4), and the decryption files. Unlike
everything before it this was new cryptography rather than a translation, so it was done on its own
branch. The non-participant path is covered too, at `k = 3`, `λ = 2`.

Stages 1–3 were the real experiment, and they passed. If Stage 2 had proved intractable everything
after it would have been moot; it did not.

## 6. Risks and honest caveats

- **Independence is preserved, but asymmetrically.** braid's prover would be written *against
  Verificatum's specification*, so the two codebases are no longer conceptually independent even
  though they share no source. The verifier remains genuinely independent — different language,
  different authors, different implementation of every primitive — which is the property that
  matters for catching braid bugs. It would not catch a flaw in the shared *specification*.
- **Version pinning.** `vmnv` rejects unless `version` is exactly `3.1.0` and matches the info file
  (VMNV §9.3 step 2b). Any VMN upgrade is a re-validation event.
- **A second emitter is a maintenance burden.** braid would carry a Verificatum-compatible proof
  path alongside its native one, and they must not drift.
- **braid's native Ristretto path stays unverifiable this way.** Only P-256 executions can be
  checked by `vmnv`, so this is an assurance tool for a specific configuration, not for production
  as currently deployed — unless deployment moves to P-256.
- **Prover-side tooling is Unix-bound.** Reproducing the Stage 0 corpus needs WSL (or Linux); only
  the verifier runs natively on Windows. Harmless for the design, mildly annoying for CI.
- **Validation status.** The central claim is no longer an analysis: unmodified `vmnv -mix` accepts
  a complete braid session (Stage 4), at `k = λ = 3` and again at `k = 3, λ = 2` with a party that
  did not decrypt. What that covers is the cryptography and the emitter over P-256 at width 2; what
  it does not cover is the live protocol plumbing, or any other group or width.
- **The specification alone is not enough to write a verifier.** Three places where a strict reading
  of VMNV would produce a verifier that disagrees with `vmnv`: the α placement in the decryption
  proof (Stage 4); `Γ`, which Algorithm 24 checks but `vmnv -shuffle` does not read; and mixer slots
  being indexed by party rather than sequentially, with absent ones skipped by file existence. Each
  was found only by reading Verificatum's Java or by running it at a shape the corpus did not cover.
  Anyone reusing this work should expect more of them.
- **Three silent skips, one pattern.** `validProofs < threshold` routed to a print-only handler; a
  mixer omitted because its file is missing; and — the same instinct in the specification rather than
  the code — Algorithm 24's `Γ` check that `vmnv` does not perform. A verifier built on this work
  should treat "condition evaluated, conclusion not enforced" as the failure mode to look for first.
