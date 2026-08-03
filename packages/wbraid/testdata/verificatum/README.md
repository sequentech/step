<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Verificatum reference corpus

A complete, genuine mix-net proof produced by [Verificatum](https://www.verificatum.org) 3.1.0,
used as the ground truth for braid's Verificatum interoperability tests. The design rationale is in
the repo-root `VERIFICATUM.md`; this file covers only the corpus itself — what it is, how to
regenerate it, and what in the code is pinned to it.

**This is throwaway demo key material from a local run.** It never protected anything. No private
key material is included: `privInfo.xml` is deliberately absent, `protInfo.xml` carries only public
party data, and `testvectors.txt` contains no `bas.x_l` entries because a verifier holds no secret
keys.

## Contents

| Path | What it is |
|---|---|
| `nizkp/` | the proof directory, exactly as VMN wrote it (VMNV §9.1) |
| `protInfo.xml` | the protocol info file; the source of the parameters ρ is derived from (§7.2) |
| `testvectors.txt` | `vmnv -t` output: intermediate values including `der.rho`, `bas.h`, `PoS.s`, `PoS.v` |

The session: **one mix-server, threshold 1, P-256, ciphertext width 2, N = 10 ciphertexts**,
`sid = braidpoc`, `auxsid = default`, `n_r = 100`, `n_e = n_v = 256`, SHA-256 for both the random
oracle and the PRG. It is a `mixing` proof, so it carries the decryption artifacts too, even though
only the shuffle half is currently exercised.

## Which tests use it

They locate it automatically relative to the crate, so a plain `cargo test` exercises them.
`VCOMPAT_CORPUS` overrides the location if you want to point at a different corpus.

| Test | Uses |
|---|---|
| `vsvmn/tests/corpus_roundtrip.rs` | byte-tree round-trip, structure, `PoS.s`/`PoS.v`, `bas.h` |
| `vsvmn/tests/vmn_encode.rs` | vsc ↔ byte-tree conversion against real elements |
| `vsvmn/tests/vmn_interop.rs` | verifying this Verificatum proof with vsc's cryptography |

`vsvmn/tests/vmn_verifier.rs` uses `protInfo.xml` but not the proof — it emits a fresh one and hands
it to `vmnv`.

## Regenerating

Requires a JVM and a Unix-ish environment. VMN's **prover-side** tooling (`vmni`, `vmn`) hardcodes
`/dev/urandom` and derives a default URL from the machine hostname against a lowercase-only regex,
both evaluated before argument parsing, so it cannot run natively on Windows. (`vmnv`, the verifier,
runs fine anywhere.) Under WSL, an unprivileged UTS namespace supplies an acceptable hostname without
changing anything outside the process tree.

With the jars from `crates/braid/verificatum/`:

```sh
JARS=/path/to/crates/braid/verificatum
CP="$JARS/verificatum-vmn/verificatum-vmn-3.1.0.jar:$JARS/verificatum-vcr/verificatum-vcr-3.1.0.jar"
RS=$PWD/random_source; SEED=$PWD/random_seed

vog () { java -cp "$CP" com.verificatum.ui.gen.GeneratorTool vog ":VERIFICATUM_VOG_BUILTIN" "$RS" "$SEED" "$@"; }
vmni () { java -cp "$CP" com.verificatum.ui.info.InfoTool vmni "$RS" "$SEED" com.verificatum.protocol.mixnet.MixNetElGamal "$@"; }
vmn  () { java -cp "$CP" com.verificatum.protocol.mixnet.MixNetElGamalTool $$ vmn "$@"; }
vmnd () { java -cp "$CP" com.verificatum.protocol.elgamal.ProtocolElGamalDemo vmnd "$@"; }
vmnv () { java -cp "$CP" com.verificatum.protocol.mixnet.MixNetElGamalVerifyFiatShamirTool vmnv "$RS" "$SEED" "$@"; }

# Everything below must run with a lowercase, resolvable hostname:
#   unshare -U -u --map-root-user bash -c 'hostname localhost; ...'

vog -rndinit RandomDevice /dev/urandom
PGROUP=$(vog -gen ECqPGroup -name P-256)

vmni -prot -sid braidpoc -name BraidPoC -nopart 1 -thres 1 -pgroup "$PGROUP" -width 2 stub.xml
vmni -party -name Party1 stub.xml privInfo.xml protInfo1.xml
vmni -merge protInfo1.xml protInfo.xml

vmn -keygen privInfo.xml protInfo.xml publicKey
vmnd -ciphs -width 2 publicKey 10 ciphertexts
vmn -mix privInfo.xml protInfo.xml ciphertexts plaintexts

vmnv -mix -v protInfo.xml dir/nizkp/default            # must exit 0
vmnv -mix -t par,der,bas,PoS,Dec,u protInfo.xml dir/nizkp/default > testvectors.txt
```

Then copy `dir/nizkp/default` here as `nizkp/`, along with `protInfo.xml` and `testvectors.txt`.
**Do not copy `privInfo.xml`** — it holds the secret key.

## What is pinned to this corpus

Regenerating produces a *different* proof, because key generation and the shuffle are randomized.
Some constants in the tests will therefore need updating. They fall into three groups.

### 1. Always change on regeneration

These are functions of the randomized proof, so any new corpus invalidates them.

| Constant | Where | Recover from |
|---|---|---|
| `GOLDEN_POS_S` | `vsvmn/tests/corpus_roundtrip.rs` | `PoS.s` in `testvectors.txt` |
| `GOLDEN_POS_V` | `vsvmn/tests/corpus_roundtrip.rs` | `PoS.v` in `testvectors.txt` |

### 2. Change only if the session parameters change

ρ is a hash of the protocol info parameters alone (VMNV §9.3 step 4), so it is stable across
regenerations **as long as the `vmni -prot` invocation above is unchanged**. Change `sid`, the group,
the widths or the hash functions and it moves.

| Constant | Where | Recover from |
|---|---|---|
| `GOLDEN_RHO` | `vsvmn/tests/golden_vectors.rs` | `der.rho` in `testvectors.txt` |
| `PGROUP` | `vsvmn/tests/golden_vectors.rs`, `vsvmn/tests/vmn_interop.rs`, `vmn_emit.rs`, `vmn_verifier.rs` | the `<pgroup>` element of `protInfo.xml`, verbatim including the `ECqPGroup(P-256)::` comment |
| `SID`, `AUXSID`, `N_R`, `N_E`, `N_V` | the same four files | `<sid>`, `<statdist>`, `<ebitlenro>`, `<vbitlenro>` in `protInfo.xml`; `auxsid` in `nizkp/` |

### 3. Change only if the shape changes

| Assumption | Where | Notes |
|---|---|---|
| `W = 2` (ciphertext width ω) | all the interop tests | must equal `<width>`; several tests assert it structurally |
| `N = 10`, `W = 2` size literals | `vsvmn/tests/spec_examples.rs`, `predicted_sizes_match_the_real_corpus` | the seven expected file sizes are computed for this shape. That test is self-contained — it does not read the corpus — so it will keep passing while silently no longer describing it. **Recompute it if N or ω change.** |
| party count and threshold | `protInfo.xml` vs `protInfo-3party.xml` | the emitter derives `activethreshold` from the number of mixers, so this is a property of the info file the test picks, not of the emitter |

The trap in group 3 is worth restating: everything else fails loudly against a mismatched corpus, but
the size-prediction test would keep passing while describing a corpus that no longer exists.

## The three-party protocol info file

`protInfo-3party.xml` declares `nopart = 3`, `thres = 3`, with every other parameter identical to
`protInfo.xml`. It supports the two tests that need a session declaring more than one party:
`vmnv_accepts_a_three_party_chain` and `vmnv_accepts_a_braid_mixing_proof`.

It has **no matching proof directory and needs none.** Both tests emit their own — the first a
three-mixer chain with no DKG, the second a full `type = mixing` session including a real
three-party DKG, so `PolynomialInExponent.bt` there is genuine rather than the placeholder a
shuffling proof carries.

Because `thres` equals `nopart`, it cannot exercise a party that fails to decrypt. That is what
`protInfo-3party-t2.xml` is for: `nopart = 3`, `thres = 2`, everything else identical, supporting
`vmnv_accepts_a_mixing_proof_with_an_inactive_party`. Regenerate it the same way with
`-nopart 3 -thres 2`.

Regenerate either exactly as above but with `-nopart 3` and the matching `-thres`, running the
`vmni -party` step once per party and merging all three. Note ρ is unaffected by the party count and
the threshold — it commits to the version, session identifier, bit lengths, group and hash
functions, but not to `nopart` or `thres` — so all three info files share a prefix and the same
session constants in the tests.

## Running the `vmnv` tests

**The short way**, from the repo root — it does everything in this section for
you, including creating the random source:

```powershell
.\vmnv.ps1                # the three-party shuffle chain
.\vmnv.ps1 -All           # every interop test, including -mix
.\vmnv.ps1 -Java C:\path\to\java.exe
```

The rest of this section is what that script automates, for anyone reproducing
it by hand or on a platform without PowerShell.

`crates/vsvmn/tests/vmn_verifier.rs` shells out to a JVM and is `#[ignore]`d, so
it needs four environment variables. Only `VMNV_RANDOM_SOURCE`/`_SEED` need
creating; the rest point at things already in the repo.

```
VMNV_JAVA           path to java (omit to use `java` from PATH)
VMNV_JAR_DIR        crates/braid/verificatum  (contains the two jars)
VMNV_PROTINFO       defaults to this directory's protInfo.xml
VMNV_RANDOM_SOURCE  \
VMNV_RANDOM_SEED    /  written once by `vog -rndinit`, see below
```

The verifier refuses to start without an initialised random source, even though
verification consumes no randomness. On Unix:

```sh
vog -rndinit RandomDevice /dev/urandom
```

`/dev/urandom` does not exist on Windows, so use the seeded PRG instead — write
512 random bytes to a file, then:

```
vog -gen HashfunctionHeuristic SHA-256          # prints the descriptor
vog -seed <seedfile> -rndinit PRGHeuristic "<that descriptor>"
```

with `vog` invoked as

```
java -cp <vmn.jar>;<vcr.jar> com.verificatum.ui.gen.GeneratorTool \
     vog :VERIFICATUM_VOG_BUILTIN <random_source> <random_seed> ...
```

The two path arguments are where the source and seed are written; point
`VMNV_RANDOM_SOURCE` and `VMNV_RANDOM_SEED` at the same files afterwards. They
are throwaway — regenerate them freely.

Then:

```
cargo test -p vsvmn --test vmn_verifier -- --ignored --nocapture
```

### `vmnv` is not safe to run concurrently by default

Verificatum spools large integer arrays into a working directory under
`/tmp/com.verificatum` and deletes it on exit, so two `vmnv` runs sharing one
destroy each other's scratch space part-way through. That surfaces as `File not
found!` or `Unable to delete storage directory!` in whichever run lost — under
`cargo test`'s default parallelism, it looks like unrelated tests failing at
random.

Without `-wd`, `TempFile.init` names that directory from
`randomSource.getBytes(10)`, so it is unique only if the random source is. **Ours
is not**: the seeded PRG below is deterministic, and concurrent runs all read the
same seed file before any of them rewrites it. On Unix with
`vog -rndinit RandomDevice /dev/urandom` the source is genuinely random and this
does not arise, so it is an artifact of the Windows setup rather than an upstream
bug.

`run_vmnv_mode` passes a unique `-wd` per invocation, so the tests do not need
`--test-threads=1`. Anything else driving `vmnv` in parallel must do the same.
`-wd` has to be a *relative* name: `TempFile.init` treats a path as absolute only
if it begins with `/`, so a Windows path would be appended to the default root
rather than replacing it.
