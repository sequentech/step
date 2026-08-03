<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Verificatum three-party corpus, with a party that took no part

**3 mix-servers, threshold 2, P-256, width 2, N = 10** — and mixing was run with only parties 1 and
3 active, so **party 2 contributed nothing**. `CorrectIndices.bt` is `01 01 00 01`.

This is the only corpus that pins the convention for an absent party. Everything else about it
duplicates the sibling `verificatum-3party/`.

## Why it exists

A party that takes no part still occupies a slot, and the values in that slot are **not inert**: the
batching seed commits to *every* party's factors and the challenge to *every* party's commitment,
including the excluded one's. Get the placeholder wrong and the challenge moves, so the two
*participating* parties' proofs fail.

We derived the convention by reading `DistrElGamalSessionBasic`'s fallbacks, and every test of it
was against our own construction of the same values — consistency, not correctness. This corpus is
VMN's own output, and it confirms them byte for byte:

| file | what VMN wrote |
|---|---|
| `DecrFactReply02.bt` | `01 00000021` + 33 zero bytes — the zero scalar |
| `DecrFactCommitment02.bt` | `node(leaf(ff…), leaf(ff…))` — `−1` at the 33-byte signed width is the point at infinity, i.e. `node(1, 1^ω)` |
| `DecryptionFactors02.bt` | the same point-at-infinity pattern throughout, and **1635 bytes, the same as an active party's** — the array cannot be omitted or shortened |

Confirmed load-bearing by mutation: dropping the inactive party's factors from the seed fails this
test and no other.

## What it does *not* establish

The "Δ is the first λ true flags" rule. Here exactly λ flags are true, so there is nothing to
choose. In the sibling corpus three are true against a threshold of two, but all three published
valid factors, and any λ-subset of valid contributions interpolates to the same secret — selecting
the last λ instead of the first leaves those tests passing.

That rule is therefore unobservable in any *valid* proof. It only matters for how a verifier behaves
on malformed or adversarial input, and is covered by unit tests on `correct_set` rather than by a
corpus.

## Regenerating

As in `../verificatum-3party/README.md`, then restrict the active set and mix again:

```sh
./demo                  # full run first: info_files, keygen, mix
./sact '{1,3}'          # party 2 becomes inactive
./delete                # the 'default' session is spent; ./mix hangs otherwise
./mix
```

The `./delete` step is not optional and not obvious — without it `./mix` blocks indefinitely on the
already-used session rather than reporting anything.
