<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# v2v

Interoperability between braid and [Verificatum](https://www.verificatum.org/),
in both directions:

- **`verify`** — check a session Verificatum produced, with our cryptography.
  No JVM, nothing to configure; it reads the directory it is given.
- **`generate`** — write a session in Verificatum's format, with our
  cryptography, for `vmnv` to check.

Together they are the interop claim: two independently written implementations,
in different languages, each accepting the other's proofs.

P-256 only, and Verificatum 3.1.0 only. A session declaring anything else is
refused rather than attempted.

## Build

```text
cargo build --release -p v2v
```

The binary lands in the **cargo workspace's** `target/release`, not this
crate's directory and not the git root's — `packages/wbraid/target/release/v2v`.
 It has no runtime dependency on Verificatum; that is the point of `verify`.

Every example below writes `v2v` as if it were on `PATH`. Put the target/release directory
on the `PATH`, or call the binary by its path.

## verify

The verify subcommand checks a session Verificatum produced, using vsc cryptography:

```text
v2v verify <protInfo.xml> <proof-directory>
```

The argument order mirrors `vmnv`'s own. Everything else — session type,
width, auxiliary session id, active threshold — is read from the directory, so
there is nothing to get wrong and no flag that could silently check the wrong
thing.

```text
$ v2v verify election/protInfo.xml election/nizkp/default
session: mixing, k=3, lambda=2, omega=2, auxsid=default, active threshold=2
  2 mixers verified
  10 ciphertexts in the output
  10 plaintexts recovered
ACCEPTED
```

Exit status is `0` only on `ACCEPTED`. Two different things can prevent that,
and they are reported differently on purpose:

| | |
| --- | --- |
| `REJECTED` | the directory is well formed and a proof does not verify |
| `error:` | it could not be checked — malformed, truncated, wrong version |

Neither is ever reported as success. A verifier that lets *could not check*
read as *checked and passed* is worse than no verifier, so this one fails
closed.

`REJECTED` also covers a chain with fewer mixers than the session threshold,
where every proof present verifies but the ones missing were never noticed.
`vmnv` accepts that case in silence.

### Sample data to try it on

In production the input comes from a real Verificatum run. To run the demo
from a raw clone of verificatum repositories without installing, copy the
launchers from vcr and vmn — `$VMN_HOME` below is the directory holding both
clones, each with the jar its own build produced:

```sh
JARS="$VMN_HOME/verificatum-vmn/verificatum-vmn-3.1.0.jar:$VMN_HOME/verificatum-vcr/verificatum-vcr-3.1.0.jar"
mkdir -p bin
for f in "$VMN_HOME"/verificatum-{vmn,vcr}/bin/*; do
    [ -f "$f" ] && sed "s|^export CLASSPATH=.*|export CLASSPATH=$JARS|" "$f" \
        | tr -d '\r' > "bin/$(basename "$f")"
done
chmod +x bin/*
export PATH="$PWD/bin:$PATH"
```

The `tr -d '\r'` matters if the clones are a Windows checkout: git leaves CRLF
in the scripts, and `/bin/sh` reports that as `./conf: : not found` followed by
a syntax error at end of file, which says nothing about line endings. On a Unix
checkout it is a no-op.

Then the demo itself, stripped the same way:

```sh
cp -r "$VMN_HOME/verificatum-vmn/demo/mixnet" vmndemo
find vmndemo -type f -exec sed -i 's|\r$||' {} +
cd vmndemo

# conf already says three parties, threshold two; NO_MIXSERVERS is k and
# THRESHOLD is lambda if you want another shape. WIDTH ships commented out, so
# omega is 1 until it is set. TERM and SILENT are what let the demo run without
# an X server -- supported, but documented only in a comment inside conf.
sed -i 's|^#*WIDTH=.*|WIDTH=2|
        s|^NO_CIPHERTEXTS=.*|NO_CIPHERTEXTS=10|
        s|^TERM=.*|TERM=./vterm|
        s|^#*SILENT=.*|SILENT=-s|' conf

# Confirm they took. Each matches on the key rather than the shipped value, so
# this is re-runnable over an already-edited conf -- but a typo or an upstream
# rename would leave a line unchanged, and the demo says nothing about it until
# it fails several steps later for an unrelated-looking reason.
grep -E '^(NO_MIXSERVERS|THRESHOLD|WIDTH|NO_CIPHERTEXTS|TERM|SILENT)=' conf
```

Stop here and read that `grep`, which should print

```text
NO_MIXSERVERS=3
THRESHOLD=2
NO_CIPHERTEXTS=10
WIDTH=2
TERM=./vterm
SILENT=-s
```

Anything missing or still carrying a shipped value means an edit did not apply, 
and the demo will not say so — it runs a long way before failing for a reason 
that looks unrelated.

Then run it:

```sh
export VERIFICATUM_RANDOM_SOURCE="$PWD/random_source"
export VERIFICATUM_RANDOM_SEED="$PWD/random_seed"
vog -rndinit RandomDevice /dev/urandom

./demo
```

That leaves a session at:

```text
vmndemo/mydemodir/Party01/dir/nizkp/default    # the proof directory
vmndemo/mydemodir/Party01/protInfo.xml         # the protocol info file
```

The recipe leaves you inside `vmndemo`, so step back out first:

```text
cd ..
v2v verify vmndemo/mydemodir/Party01/protInfo.xml \
           vmndemo/mydemodir/Party01/dir/nizkp/default
```

### Two things that cost time

- **Run `./delete` before running again.** A second `./mix` against a spent
  session blocks indefinitely rather than reporting anything.
- To make a party sit out — worth doing, since it exercises the placeholder
  decryption material and leaves a gap in the mixer slots — run the full demo
  first, then `./sact '{1,3}'`, `./delete`, `./mix`.

The test harness automates all of this, including the fixups a Windows
checkout needs (CRLF, hostname resolution, rewritten launchers). See
[TESTING.md](TESTING.md) and `tests/common/mod.rs` if you would rather not do
it by hand.

## generate

The generate subcommand generates synthetic session data, produced by `vsc`, in Verificatum's 
format, for `vmnv` to check:

```text
v2v generate [OPTIONS] <DIR>
```

Writes `<DIR>/protInfo.xml` and `<DIR>/nizkp`. Both come from one
specification, so the parameters and the ρ a verifier recomputes cannot
disagree — handing over an info file describing a different session is the one
way to make a correct proof look wrong.

```text
$ v2v generate -k 3 -t 2 -w 2 -n 20 --active 1,3 /tmp/session
wrote a mixing session: k=3, lambda=2, omega=2, N=20, active=[1, 3]
  /tmp/session/protInfo.xml
  /tmp/session/nizkp

Verify it with Verificatum:
  vmnv -v -mix -auxsid default -width 2 /tmp/session/protInfo.xml /tmp/session/nizkp
or with this tool:
  v2v verify /tmp/session/protInfo.xml /tmp/session/nizkp
```

It generates and stops — running the verifier is yours to do — but prints the
command, since getting `vmnv`'s arguments right is the fiddly part. The options
are:

| | |
| --- | --- |
| `--kind mixing\|shuffling` | whether to include the decryption phase (default `mixing`) |
| `-k, --parties` | party count *k* (default 3) |
| `-t, --threshold` | parties needed to decrypt, λ (default 2) |
| `-w, --width` | ciphertext width ω (default 2) |
| `-n, --ciphertexts` | how many to shuffle (default 100) |
| `--active 1,3` | which λ parties take part; default is the first λ |
| `--sid`, `--auxsid` | session identifiers |
| `--force` | replace `<DIR>` if it exists |

`--active` is where the interesting cases are: a party left out contributes
Verificatum's placeholder decryption material, which it must, since the
verifier reads a file for every party and the batching seed commits to all of
them.

Widths 1–3 and *k* up to 4 are instantiated. The const generics come from vsc's
ciphertext and DKG types, so each shape is a separate monomorphisation; asking
for one that is not compiled in is an error naming the function to extend.

### Do not trust `vmnv`'s exit code on a shuffling proof

`vmnv` exits 0 on shuffling proofs it has itself rejected, reporting the
rejection only in its output and only under `-v`. Read its output for
`Verify proof of shuffle... done.`, once per mixer. `generate` prints this
warning too when it writes a shuffling session.

This is one of three divergences found in `vmnv`, all sharing one pattern — a
condition evaluated but its conclusion not enforced. They are written up in
[VERIFICATUM.md](../../VERIFICATUM.md).
