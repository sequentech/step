<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Verificatum three-party reference corpus

A genuine multi-party mix-net proof produced by [Verificatum](https://www.verificatum.org) 3.1.0:
**3 mix-servers, threshold 2, P-256, ciphertext width 2, N = 10**, `sid = MyDemo`.

The sibling `testdata/verificatum/` corpus is single-party, which makes the decryption half
degenerate — `α = 1`, every Lagrange coefficient is `1`, and Δ is everything. This one has
`α = lcm(1,2,3)² = 36` and coefficients `72` and `−36`, so it exercises the combination for real.

**Throwaway demo key material.** `privInfo.xml` is deliberately absent.

## Contents

| Path | What it is |
|---|---|
| `nizkp/` | the proof directory, as VMN wrote it (its `default` session) |
| `protInfo.xml` | the protocol info file — **every** session parameter is read from here |

Note there is no `testvectors.txt`: the single-party corpus supplies the golden intermediate values,
and this one is used for end-to-end verification rather than layer-by-layer transcript checks.

## Nothing is pinned to it

Unlike `testdata/verificatum/`, no constant in the tests corresponds to this corpus. Session
parameters — `sid`, the group, the bit lengths, `k`, `λ`, `ω` — are read from its `protInfo.xml` by
`wire::protinfo`, and the number of mixers from `proofs/activethreshold`. So it can be regenerated
with different parameters and the tests still work.

That is the whole reason it exists: the constants that used to be hardcoded said `sid = braidpoc`,
and this corpus says `MyDemo`. ρ commits to `sid`, so before the reader existed this proof could not
have been checked at all.

## Regenerating, or generating others

VMN ships a local multi-party demo, which is what produced this. It needs a Unix environment (the
prover-side tools hardcode `/dev/urandom` and derive a URL from the hostname), so under Windows use
WSL with an unprivileged UTS namespace.

```sh
# The demo scripts come through a Windows checkout with CRLF; /bin/sh chokes on
# them ("./conf: : not found"). Strip it first.
cp -r crates/braid/verificatum/verificatum-vmn/demo/mixnet ~/vmndemo
find ~/vmndemo -type f -exec sed -i 's/\r$//' {} +

# Point the shipped launchers at the in-tree jars instead of /usr/local/share/java.
# Then in ~/vmndemo/conf:
#   TERM=./vterm     — no X server needed
#   SILENT=-s        — single terminal
#   WIDTH=2  NO_MIXSERVERS=3  THRESHOLD=2  NO_CIPHERTEXTS=10

unshare -U -u --map-root-user bash -c 'hostname localhost; ./demo'
```

`./demo` runs `clean`, `info_files`, `keygen`, `precomp`, `mix` and leaves the proof in
`mydemodir/Party01/dir/nizkp/default`. `./verify` runs `vmnv` over it.

Vary `NO_MIXSERVERS`, `THRESHOLD`, `WIDTH` and `NO_CIPHERTEXTS` in `conf` for other shapes.

## What this corpus still does not cover

**A genuinely inactive party.** All three parties published real factors here; `CorrectIndices.bt`
marks all three correct even though the threshold is two. That means Δ selection is *unobservable* —
any 2-subset of three valid contributions interpolates to the same secret, which was confirmed by
mutation: making `correct_set` take the last λ instead of the first leaves the tests passing.

So the all-identity factor array, the identity commitment and the zero reply that VMN writes for a
non-participant are still checked only against our own construction. Producing that needs `./sact`
to restrict the active set before mixing.
