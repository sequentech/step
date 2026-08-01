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

Cost estimate: **substantial but bounded**, and unusually well-supported by tooling (VMN ships test
vectors and a `vmnv -t` mode that prints intermediate values, so each layer can be validated in
isolation). The single largest work item is a byte-tree + VMN-random-oracle layer in Rust; the
second is replacing braid's per-ciphertext decryption proofs with VMN's batched form.

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

### 2.5 Proof of correct decryption: STRUCTURAL MISMATCH — a real work item

This is where the two designs genuinely diverge.

- **braid** emits, per party, *N* independent DLEQ proofs — one per ciphertext
  (`crates/vsc/src/dkgd/recipient.rs:353-378`: `DecryptionFactor { value, proof: DlogEqProof }`,
  collected into `DecryptionFactors`).
- **Verificatum** expects, per party, *one batched proof over all N ciphertexts*:
  `τ_l^dec = node(y'_l, B'_l)` and `σ_l^dec = k_{x,l}` (VMNV §8.6), verified against batched
  elements `A = (∏u_i^{e_i}, 1)` and `B = ∏f_i^{e_i}`.

Two further conventions differ:

- **Sign.** Verificatum's decryption factor is `f_l = u^{−x_l}` and combines as `TDec = v·f`;
  braid computes `u^{+x_l}` and divides during combination.
- **The α trick.** Verificatum computes `f_l = PDec_{x_l/α}` with `α = lcm(1,…,k)²`, cancelling it
  in the Lagrange combination so the coefficients stay small integers (VMNV §2.2). braid has no
  analogue.

Closing this means implementing VMN's batched decryption proof in braid. It is standard
(Bellare et al. batching over a Chaum–Pedersen proof) and small compared with the shuffle, but it is
new code, not a translation.

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

### A `vmnv` behaviour worth knowing about

One control was *accepted*: replacing the output list with the input. Investigating with `-v` shows
`vmnv` **does** detect it —

```
Verify proof of shuffle... failed.
--> Replacing output of Party 1 by its input.
--> Too few proofs are valid! (0)
```

— and then exits **0** anyway. Verificatum's *own* proof behaves identically under the same
tampering, so this is a property of `vmnv`, not of braid's emitter.

It follows from the documented multi-server semantics (VMNV §2.3: a mix-server whose proof fails is
skipped, `L_l = L_{l-1}`), but the accompanying "less than λ proofs valid ⇒ reject" is reported and
then not reflected in the exit status. VMNV §10.1 specifies the exit code as the accept/reject
signal, so **an integration must not rely on `vmnv`'s exit code alone** for a single-server shuffle;
parse the output, or verify in a mixing context. Worth raising upstream.

(Also note `vmnv` exits 1 on rejection, not the `-1`/255 that §10.1 specifies.)

### Caveats

- Shuffle only. The decryption transcript (`Dec.s`, `Dec.v`) and the batched decryption proof braid
  would need (§2.5) are still not done, so `vmnv -mix` is unreached.
- Single mix-server, threshold 1. Multi-party proofs need per-party files and an `activethreshold`
  greater than 1.
- P-256 only, and braid's DKG still cannot run on P-256 (the four `todo!()`s in §2.1).

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

**Stage 4 — decryption, `vmnv -mix`.** Implement the batched decryption proof (§2.5), derive Γ from
braid's per-dealer commitments (§2.4), and emit the decryption files.

**Stage 5 — P-256 DKG.** Fill in the four `todo!()`s so braid runs end-to-end on P-256.

Stages 1–3 are the real experiment. If Stage 2 proves intractable, stop there — everything after it
depends on it.

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
- **Validation status.** The compatibility analysis (§2) is from specification and source reading,
  plus two executed checks: braid's TW shuffle proof verifies over P-256, and the Stage 0 corpus
  confirms VMN's side end to end. What remains unproven by execution is the *conjunction* — that a
  braid-produced proof can be made to satisfy `vmnv`. That is precisely what Stages 1–3 test, and
  Stage 2 (the random-oracle layer) is the go/no-go gate.
