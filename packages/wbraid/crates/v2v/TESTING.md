<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Testing v2v

For the tool itself rather than its tests, see [README.md](README.md).

```text
cargo test --release -p v2v -- --include-ignored
```

69 tests, about five minutes. `--release` matters: the DKG, shuffle and
decryption are compute-intensive and a debug build turns seconds into minutes.

On Windows this runs from PowerShell as written; only the environment
variables below differ, since PowerShell has no `VAR=value cmd` prefix.

Without `--include-ignored` you get the 47 tests that need no external tooling
(under a second). The other 22 run Verificatum, are `#[ignore]`d, and need the
prerequisites below — without them they skip.

## Prerequisites

A JDK on `PATH`, WSL if you are on Windows, and a directory holding
Verificatum. The tests look for that last one at `crates/braid/verificatum`,
which is **not in this repository** — you assemble it, or point `VMN_HOME`
somewhere else.

There is no single Verificatum repository. It is distributed as separate
projects, two of which are needed here, cloned side by side:

```sh
mkdir verificatum && cd verificatum
git clone https://github.com/verificatum/verificatum-vcr.git
git clone https://github.com/verificatum/verificatum-vmn.git
```

Neither ships its jar — both are build products, produced by each project's
own build and left at the root of its clone. Follow their instructions; nothing
here builds them for you. What the tests need afterwards is:

```text
$VMN_HOME/
  verificatum-vcr/verificatum-vcr-3.1.0.jar
  verificatum-vmn/verificatum-vmn-3.1.0.jar
  verificatum-vmn/demo/mixnet/conf        <- what the path is validated against
```

The demo is checked rather than a jar because it is the harder requirement:
`vmnv` needs only the two jars, but generating a corpus needs `vmn`'s demo
scripts, and a tree without them would fail later and less clearly.

## Nothing is checked in

Every test that compares against Verificatum runs **whatever Verificatum is
installed** and compares against what *that* produced. There are no fixed
corpora and no pinned expected values.

This is deliberate. A captured corpus and a golden value from 3.1.0 have a
failure mode this crate exists to catch: if a future VMN changes a derivation,
a pinned test keeps passing, because it cannot fail. It would succeed exactly
when it should have failed. Generating the inputs on every run costs about
thirty seconds per shape and removes that possibility.

## The two directions

The interop claim is symmetric, so the tests are split by which side produces
and which verifies.

| | produced by | verified by |
| --- | --- | --- |
| [`we_verify_theirs.rs`](tests/we_verify_theirs.rs) | `vmn` | `v2v::session::verify_session` |
| [`they_verify_ours.rs`](tests/they_verify_ours.rs) | `v2v` | `vmnv` |

**`we_verify_theirs`** runs Verificatum's demo to produce a complete session,
then verifies every proof in it with our code. Four shapes: `(k=1, λ=1)` where
α is 1 and the combination is trivial; `(3, 2)` where α and the Lagrange
coefficients bite; `(2, 2, width 3)`; and `(3, 2)` with party 2 taking no part,
so VMN writes its placeholder decryption material *and* leaves a gap in the
mixer slots.

**`they_verify_ours`** goes the other way: our shuffle and decryption proofs,
written in VMN's format, handed to `vmnv`. Two sweeps — ten shuffling shapes
(`k` from 1 to 4, every threshold) and seven mixing shapes with the active set
spread across them. The protocol info file is synthesized per session, so its
parameters and the ρ the verifier recomputes cannot drift apart.

Both directions have negative controls: tampered proofs must be *rejected*, not
merely "not accepted".

## What each of the other files covers

| file | |
| --- | --- |
| [`corpus_roundtrip.rs`](tests/corpus_roundtrip.rs) | Byte trees parse and re-encode byte-identically; the shuffle seed, challenge and decryption transcript match what `vmnv -t` reports for the same session. |
| [`random_oracle.rs`](tests/random_oracle.rs) | ρ, `RO_seed`, `RO_challenge`, PRG expansion. Structural properties locally, then ρ and the proof transcripts against the installed VMN. |
| [`spec_examples.rs`](tests/spec_examples.rs) | Wire format against the VMNV specification's own worked examples. |
| [`vmn_encode.rs`](tests/vmn_encode.rs) | Group elements and ciphertext arrays across the vsc ↔ VMN boundary. |
| [`vmn_decrypt.rs`](tests/vmn_decrypt.rs) | Γ₀ against the DKG's joint public key (VMNV Algorithm 24), and the factor conversion exponent. |
| [`lagrange.rs`](tests/lagrange.rs) | α = lcm(1..k)², Δ as the first λ true flags, and that the modified coefficients reconstruct. |

## How `vmn` and `vmnv` are actually run

They are run differently, and the difference explains most of the machinery in
[`tests/common/mod.rs`](tests/common/mod.rs).

**`vmnv`** — the verifier — runs natively via `java -cp <jars>`. Nothing about
it needs a Unix host.

**`vmn`** — the prover — is only ever reached through its bundled demo, which
is a set of shell scripts. On Windows those run inside WSL. Corpus generation
therefore shells out to `bash`, and the script it runs has to undo several
things first:

- the launcher scripts hardcode `/usr/local/share/java`, so their `CLASSPATH`
  line is rewritten to the jars under `VMN_HOME` — and only that line, so
  everything else about how an operator invokes them is preserved;
- every demo file is stripped of CRLF, which git checkout introduces and
  `bash` chokes on;
- the demo is run under `unshare -U -u --map-root-user` with the hostname set
  to `localhost`, because it insists on resolving its own hostname;
- `TERM=./vterm` and `SILENT=-s` in `conf`, or it tries to open terminals;
- `./delete` between runs — a spent session makes `./mix` hang indefinitely
  rather than fail.

The demo binds fixed ports (4040, 8040) and uses a fixed working directory, so
generation is serialised behind a mutex. Each generated corpus is built **once
per test binary** and shared, since thirty seconds each would otherwise be paid
by every test in the file.

`vmnv -t <names>` prints the intermediate values Verificatum computed for a
session — `der.rho`, `bas.h`, `PoS.s`, `Dec.v` and so on. That is where the
expected values come from. It runs once per test binary too, and every `vmnv`
invocation gets its own `-wd`: concurrent runs sharing a working directory
delete each other's scratch space.

### Never trust `vmnv`'s exit code alone

`vmnv` exits 0 on shuffling proofs it has itself rejected. Acceptance is
decided by `vmnv_accepts()`, which requires a zero exit **and** the expected
line in the output, and every test asks through it. Two tests pin this
behaviour deliberately and will fail if it is ever fixed upstream. `-v` also
matters: some failures are reported only when verbose.

See [`VERIFICATUM.md`](../../VERIFICATUM.md) for this and the two other
divergences found, all three sharing one pattern — a condition evaluated but
its conclusion not enforced.

## The random source

`vmnv` refuses to start without an initialised random source, even though
verification consumes none. The tests provision one automatically. The
documented recipe, `vog -rndinit RandomDevice /dev/urandom`, has no Windows
equivalent, so the native path takes the portable route instead: 512 bytes from
the OS into a file, handed to `PRGHeuristic`. Corpus generation, already inside
a Unix shell, uses `/dev/urandom` directly.

The seed is *rewritten* on every run — that is what a seeded PRG source does —
so each invocation copies it first. The source file is read-only and stays
shared.

## Skipping, and how to stop it

A machine without a JDK or WSL skips the external tests and the suite passes,
having verified nothing against Verificatum. That is the right default for a
contributor who only touched the wire format, and the wrong one for anyone
checking interop.

```powershell
$env:V2V_REQUIRE_VMN = "1"
cargo test --release -p v2v -- --include-ignored
```

```sh
V2V_REQUIRE_VMN=1 cargo test --release -p v2v -- --include-ignored
```

turns every skip into a failure naming what was missing. **Use it before
believing a green run.**

`$env:` assignments persist for the rest of the session, so unset it with
`$env:V2V_REQUIRE_VMN = $null` when you want skipping back.

## Overrides

Only `VMN_HOME` is likely to matter, and only if you keep Verificatum
somewhere other than `crates/braid/verificatum`. Each is set the same way as
above — `$env:NAME = "value"` in PowerShell, `NAME=value` inline in a POSIX
shell.

| | |
| --- | --- |
| `VMN_HOME` | the directory holding both clones (see [Prerequisites](#prerequisites)) |
| `VMN_JAVA` | a different `java` |
| `VMN_RANDOM_SOURCE`, `VMN_RANDOM_SEED` | a source initialised by hand |
| `VMN_PROTINFO` | a protocol info file from elsewhere, instead of the synthesized one |

Windows paths go in as-is; the WSL side converts them (`C:\x` → `/mnt/c/x`)
when it hands them to the demo.

## If tests pass alone and fail together

That has happened three times, always the same cause: two things sharing
external state that VMN gives a fixed name. The working directory, the demo's
ports, the seed file. If you add a test that runs Verificatum, give it its own
`-wd` and its own seed copy, or take the generation mutex.
