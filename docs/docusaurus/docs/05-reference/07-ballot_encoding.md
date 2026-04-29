---
id: ballot_encoding
title: Ballot Encoding Reference
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This reference describes the single-contest ballot codec implemented in
`packages/sequent-core/src/ballot_codec/`. It focuses on how a decoded contest
is transformed into the mixed-radix representation that is later serialized
into the encrypted 30-byte ballot payload.

## Scope

The single-contest codec is used by the `RawBallotCodec`, `BasesCodec`, and
`BigintCodec` traits in `sequent-core`. The flow is:

1. `DecodedVoteContest`
2. `RawBallotContest { bases, choices }`
3. `BigUint`
4. fixed-size byte array for encryption

The same codec is also used in reverse when a ballot is decoded for review,
verification, and tallying.

## Raw ballot structure

`RawBallotContest` is made of two parallel vectors:

- `bases`: the radix used for each position
- `choices`: the value encoded at each position

For plurality contests without write-ins, the raw ballot layout is:

| Position | Meaning | Base | Values |
| --- | --- | --- | --- |
| `0` | explicit invalid flag | `2` | `0 = normal`, `1 = explicit invalid` |
| `1` | explicit blank flag, only if the contest has an explicit blank candidate | `2` | `0 = not selected`, `1 = selected` |
| remaining positions | normal candidate selections in candidate-id order | `2` | `0 = not selected`, `1 = selected` |

For preferential contests, the candidate slots use `contest.max_votes + 1` as
their base:

- `0` means “not selected”
- `1..n` means rank/position plus one

Write-in contests append character-map based slots after the candidate section.

## Explicit versus implicit blank votes

Blank votes are represented in two different ways:

- **Implicit blank vote**: no candidate is selected and the explicit blank flag
  is `0`
- **Explicit blank vote**: the contest includes one explicit blank candidate and
  the explicit blank flag is set to `1`

Explicit blank candidates are **not** encoded as ordinary candidate selections.
They are stored in the dedicated explicit blank flag so that:

- explicit and implicit blank votes produce different encoded ballots
- decoding can reconstruct the explicit blank candidate as selected
- blank-vote validation still treats the contest as a blank vote instead of a
  normal candidate selection

## Candidate ordering

Normal candidate slots are encoded using the contest candidates sorted by
candidate id. Two candidate types are excluded from the ordinary candidate
section:

- explicit invalid candidates
- explicit blank candidates

This keeps both flags orthogonal to the regular candidate marks.

## Examples

Assume a plurality contest with:

- one explicit blank candidate
- two normal candidates
- no write-ins

The bases are:

```text
[2, 2, 2, 2]
```

### Implicit blank vote

No candidate selected:

```text
choices = [0, 0, 0, 0]
```

### Explicit blank vote

Explicit blank selected:

```text
choices = [0, 1, 0, 0]
```

### Explicit invalid vote

Explicit invalid selected:

```text
choices = [1, 0, 0, 0]
```

## Decoding rules

When decoding a raw ballot:

1. the explicit invalid flag is read first
2. if the contest has an explicit blank candidate, the explicit blank flag is
   read second
3. the remaining candidate slots are decoded in candidate-id order
4. the decoded explicit blank candidate is reintroduced as a selected choice
   when the explicit blank flag is set

The decoded contest therefore preserves the semantic difference between:

- an empty contest
- an explicit blank selection
- an explicit invalid selection

## Validation notes

Blank-vote policy checks continue to operate on the number of **non-explicit
blank** candidate selections. This means an explicit blank selection is still
validated as a blank vote, not as a regular candidate mark.

## Related code

- `packages/sequent-core/src/ballot_codec/bases.rs`
- `packages/sequent-core/src/ballot_codec/raw_ballot.rs`
- `packages/sequent-core/src/ballot_codec/bigint.rs`
- `packages/sequent-core/src/fixtures/ballot_codec.rs`
