<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# vsvmn

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
cargo build --release -p vsvmn
```

The binary lands in the **cargo workspace's** `target/release`, not this
crate's directory and not the git root's — `packages/wbraid/target/release/vsvmn`,
`vsvmn.exe` on Windows. It has no runtime dependency on Verificatum; that is
the point of `verify`.

Every example below writes `vsvmn` as if it were on `PATH`. If it is not, call
it by path, or put it there for the session:

```sh
export PATH="$(dirname "$(cargo locate-project --workspace --message-format plain)")/target/release:$PATH"
```

## verify

```text
vsvmn verify <protInfo.xml> <proof-directory>
```

The argument order mirrors `vmnv`'s own. Everything else — session type,
width, auxiliary session id, active threshold — is read from the directory, so
there is nothing to get wrong and no flag that could silently check the wrong
thing.

```text
$ vsvmn verify election/protInfo.xml election/nizkp/default
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

In production the input comes from a real Verificatum run. To get a session to
try `verify` against, use VMN's own demo — the point being that the data is
produced by Verificatum, not by us.

It needs a Unix host — on Windows, run it under WSL — and Verificatum's
launcher scripts on `PATH`. Those are shell scripts, spread across both
projects: `vog` comes from VCR, everything named `vmn*` from VMN.

```text
$VMN_HOME/verificatum-vcr/bin/    vog, vbt, ...
$VMN_HOME/verificatum-vmn/bin/    vmn, vmni, vmnv, vdemo, ...
```

**Putting those two directories on `PATH` is not enough by itself.** Each
launcher hardcodes where the jars were installed:

```sh
export CLASSPATH=/usr/local/share/java/verificatum-vcr-3.1.0.jar:::${CLASSPATH}
```

so they work as shipped only after installing Verificatum the way each project
documents. To run from a built tree without installing, copy the launchers and
repoint that one line — leaving the rest alone, so how the tools are invoked
does not change:

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
cp -r "$VMN_HOME/verificatum-vmn/demo/mixnet" demo
find demo -type f -exec sed -i 's|\r$||' {} +
cd demo

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

Six lines, none of them commented. Anything missing or still carrying a
shipped value means an edit did not apply, and the demo will not say so — it
runs a long way before failing for a reason that looks unrelated.

Then run it:

```sh
export VERIFICATUM_RANDOM_SOURCE="$PWD/random_source"
export VERIFICATUM_RANDOM_SEED="$PWD/random_seed"
vog -rndinit RandomDevice /dev/urandom

./demo
```

That leaves a session at:

```text
demo/mydemodir/Party01/dir/nizkp/default    # the proof directory
demo/mydemodir/Party01/protInfo.xml         # the protocol info file
```

so, from the directory you copied the demo into:

```text
vsvmn verify demo/mydemodir/Party01/protInfo.xml \
             demo/mydemodir/Party01/dir/nizkp/default
```

On Windows this crosses a boundary the paths do not show. The demo has to run
under WSL; `vsvmn` runs wherever you built it. Pick one and stay there:

- **Built under WSL** — the command above works as written.
- **Built on Windows** — run it from PowerShell against the same files by
  their Windows path. A demo run under `/mnt/c/work/...` *is* `C:\work\...`;
  the two are one directory seen from two sides.

Do not hand a `/mnt/c/...` path to the Windows executable. It cannot resolve
one, and the error will not say so — from Git Bash the path is rewritten
before the binary ever sees it, so `/mnt/c/tmp/x.xml` is reported missing as
`C:/Program Files/Git/mnt/c/tmp/x.xml`.

### If your hostname has capitals in it

`vmni` builds `http://<hostname>:<port>` and validates it against a pattern
that does not admit uppercase, before anything cryptographic happens:

```text
InfoException: Value does not match expression! (http://NewKid:8040 is not urlport)
```

WSL takes its hostname from the Windows machine name, so this is the common
case on Windows rather than an unusual one. Run the demo in a private UTS
namespace:

```sh
unshare -U -u --map-root-user bash -c 'hostname localhost; ./delete; ./demo'
```

`-U` maps you to root inside a user namespace, so no privileges are needed, and
`-u` scopes the hostname to that process tree — nothing outside it sees a
change. Wrap the whole sequence, not just `./demo`: `./sact`, `./delete` and
`./mix` re-read the info files and need the same hostname.

The alternative is to set it for the distro in `/etc/wsl.conf` under
`[network]` and `wsl --shutdown`, which is a bigger hammer than the job needs.

### Two more that cost time

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

```text
vsvmn generate [OPTIONS] <DIR>
```

Writes `<DIR>/protInfo.xml` and `<DIR>/nizkp`. Both come from one
specification, so the parameters and the ρ a verifier recomputes cannot
disagree — handing over an info file describing a different session is the one
way to make a correct proof look wrong.

```text
$ vsvmn generate -k 3 -t 2 -w 2 -n 20 --active 1,3 /tmp/session
wrote a mixing session: k=3, lambda=2, omega=2, N=20, active=[1, 3]
  /tmp/session/protInfo.xml
  /tmp/session/nizkp

Verify it with Verificatum:
  vmnv -mix -auxsid default -width 2 /tmp/session/protInfo.xml /tmp/session/nizkp
or with this tool:
  vsvmn verify /tmp/session/protInfo.xml /tmp/session/nizkp
```

It generates and stops — running the verifier is yours to do — but prints the
command, since getting `vmnv`'s arguments right is the fiddly part.

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

### This generates, it does not export

The session is synthetic: the ciphertexts are encryptions of random group
elements and every party is played by one process. It is not a real braid
session converted into Verificatum's format.

The shuffle half of a real export would be possible. The decryption half is
not, and the obstacle is structural: Verificatum's decryption transcript is
joint over *all k* parties' factors — the batching seed commits to every
party's factor array before any commitment is formed — so producing one needs
three rounds among the trustees, which braid's decryption protocol does not
have.

### Do not trust `vmnv`'s exit code on a shuffling proof

`vmnv` exits 0 on shuffling proofs it has itself rejected, reporting the
rejection only in its output and only under `-v`. Read its output for
`Verify proof of shuffle... done.`, once per mixer. `generate` prints this
warning too when it writes a shuffling session.

This is one of three divergences found in `vmnv`, all sharing one pattern — a
condition evaluated but its conclusion not enforced. They are written up in
[VERIFICATUM.md](../../VERIFICATUM.md).
