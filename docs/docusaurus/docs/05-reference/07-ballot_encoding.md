---
id: ballot_encoding
title: Ballot Encoding Specification
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This document specifies the ballot encoding used to transform human-readable
ballot selections into the compact numeric representation that is later
serialized, encrypted, audited, decoded, and tallied.

The goal of this specification is to be implementation-neutral. Any compatible
encoder or decoder should be able to interoperate as long as it follows the
rules below.

## 1. Design goals

The codec is designed to satisfy the following requirements:

1. **Determinism** — the same ballot configuration and the same voter
   selections must always produce the same encoded values.
2. **Compactness** — the encoded ballot must fit in a fixed-size encrypted
   payload.
3. **Reversibility** — decoding must reconstruct the original ballot semantics,
   including explicit invalid and explicit blank markers.
4. **Auditability** — the representation must be stable enough for independent
   verifiers and external implementations.
5. **Policy preservation** — the codec must preserve the distinctions required
   by ballot validation and tallying, especially the difference between:
   - a normal vote,
   - an explicit invalid vote,
   - an implicit blank vote,
   - an explicit blank vote.

## 2. Terms

### 2.1 Contest

A contest is a single voting unit within a ballot. Each contest has its own:

- candidate list,
- counting algorithm,
- minimum and maximum selections,
- validation policies,
- optional write-in support.

### 2.2 Candidate classes

The following candidate roles affect encoding:

- **Regular candidate**: encoded in a candidate slot.
- **Explicit invalid marker**: represented by the contest's explicit invalid
  flag and omitted from the candidate slots.
- **Explicit blank candidate**: encoded in its normal, identifier-sorted
  candidate slot. It does not have a separate flag.
- **Write-in candidate**: encoded in a candidate slot and, when write-ins are
  enabled, has an associated text subsequence.

The candidate slots therefore contain every candidate except the explicit
invalid marker. In particular, an explicit blank candidate remains part of the
same positional layout as a regular candidate.

### 2.3 Ballot states relevant to encoding

For each contest, the codec must distinguish the following states:

- **Normal vote**: one or more regular candidates are selected
- **Explicit invalid**: the contest is deliberately marked invalid
- **Implicit blank**: no selectable option is chosen
- **Explicit blank**: the configured explicit blank candidate is chosen

## 3. Global rules

### 3.1 Stable ordering

Whenever a sequence of candidates must be encoded positionally, candidates are
ordered by **candidate identifier in ascending order** unless a more specific
rule is defined for a different codec mode.

### 3.2 Uniqueness constraints

A valid contest configuration may contain:

- **at most one explicit invalid marker**
- **at most one explicit blank marker**

Configurations that violate either rule are invalid and must be rejected before
the voter is allowed to proceed. An implementation must not silently choose one
marker and ignore the others.

### 3.3 Mixed-radix representation

The codec produces two aligned vectors:

- `bases`: the radix of each position
- `choices`: the encoded value stored at that position

The encoded integer is obtained by interpreting `choices` under the positional
radices defined by `bases`.

## 4. Single-contest codec

The single-contest codec is the reference format for one contest encoded into a
fixed ballot payload.

### 4.1 Layout overview

The dense encoded vector is composed in this order:

1. **explicit invalid flag**
2. **candidate section**, containing every non-explicit-invalid candidate in
   candidate identifier order
3. **write-in text section**, if write-ins are enabled

The fixed-size part of the vector therefore contains:

```text
1 + number_of_non_explicit_invalid_candidates
```

positions, followed by the variable-length write-in section.

An explicit blank candidate uses its existing position in the candidate
section. Adding the explicit-blank meaning to candidate metadata does not add or
move an encoded position.

### 4.2 Explicit invalid flag

The first position is always reserved for the explicit invalid flag.

- base: `2`
- values:
  - `0` = contest is not explicitly invalid
  - `1` = contest is explicitly invalid

This flag is orthogonal to all candidate slots.

### 4.3 Candidate section

The candidate section contains every candidate that is not an explicit invalid
marker. Candidates are sorted by identifier before their slots are encoded.

An explicit blank candidate is encoded exactly like another selected candidate.
Its `is_explicit_blank` metadata gives that selection its blank-vote
meaning after decoding.

#### 4.3.1 Plurality-style contests

For plurality-style contests, each candidate receives one slot.

- base: `2`
- values:
  - `0` = not selected
  - `1` = selected

#### 4.3.2 Ranked or preferential contests

For preferential contests, each candidate receives one slot whose base is:

```text
max_votes + 1
```

Values are interpreted as:

- `0` = candidate not ranked
- `1` = first usable rank
- `2` = second usable rank
- `...`
- `n` = rank `n`

The zero value is therefore reserved for absence of selection, and positive
values encode rank plus an offset of one.

#### 4.3.3 Cumulative contests

For cumulative contests, each candidate slot uses:

```text
cumulative_number_of_checkboxes + 1
```

The zero value means no selection, and positive values represent the configured
selection strength according to the contest’s cumulative policy.

### 4.4 Write-in section

If write-ins are enabled, a text section is appended after the candidate
section.

Each write-in string is encoded as:

1. zero or more character values using the configured character map
2. a terminating zero value

Therefore each write-in occupies a variable-length subsequence ending in a
sentinel zero.

An empty write-in is encoded as only the terminating zero.

## 5. Meaning of blank votes

### 5.1 Implicit blank

A contest is an **implicit blank vote** if:

- the explicit invalid flag is `0`
- every candidate slot, including the explicit blank candidate's slot, is unset
- all write-in values are empty

### 5.2 Explicit blank

A contest is an **explicit blank vote** if:

- the explicit invalid flag is `0`
- the explicit blank candidate's own slot is selected
- no regular candidate is selected

The explicit blank candidate identifier and selection survive round-trip
encode/decode. This selected candidate distinguishes the ballot from an
implicit blank without changing the encoded layout.

### 5.3 Tallying interpretation

For tallying purposes:

- implicit blank and explicit blank both contribute to the overall blank total
- both kinds of blank ballot are included in total valid votes
- explicit blank must remain distinguishable from implicit blank in decoded and
  reported data
- explicit blank must **not** be counted as a regular candidate vote
- a contest that selects explicit blank together with a regular candidate is an
  implicit invalid vote, not a valid or blank vote

## 6. Meaning of explicit invalid votes

An explicit invalid vote is represented only by the explicit invalid flag.

It is not encoded as a candidate selection and must remain distinct from:

- overvotes,
- undervotes,
- implicit blank votes,
- explicit blank votes.

## 7. Decoding rules for the single-contest codec

A decoder must process the encoded vector in the following order:

1. read the explicit invalid flag
2. decode the identifier-sorted, non-explicit-invalid candidate section
3. decode the write-in section
4. reconstruct the semantic contest state from the decoded selections and
   candidate metadata

### 7.1 Reconstruction requirements

After decoding:

- an explicit invalid flag must reconstruct the explicit invalid state
- each candidate slot must reconstruct its corresponding candidate selection
- a selected explicit blank candidate must remain selected
- write-in text must reconstruct the original text sequence

## 8. Validation rules

Validation happens at two distinct levels and both are required.

### 8.1 Configuration validation

Before any ballot is cast, the contest definition must be checked for structural
consistency, including:

- no more than one explicit invalid marker
- no more than one explicit blank marker
- valid numeric constraints for minimum and maximum votes
- any other constraints required by the contest type

If configuration validation fails, the voting flow must stop before casting.

### 8.2 Ballot-state validation

After decoding, the resulting selections are evaluated against the contest
policies, including:

- minimum selections
- maximum selections
- invalid vote policy
- blank vote policy
- undervote and overvote policies
- duplicated rank policy
- preference-gap policy

The codec reconstructs an explicit blank as a selected candidate. Validation
and tallying then use the candidate's `is_explicit_blank` metadata to
distinguish that state from both an implicit blank and a regular candidate
selection. Selecting explicit blank together with a regular candidate produces
an implicit invalid ballot at tally time.

## 9. Error conditions

Compatible implementations must detect and report at least the following error
classes:

### 9.1 Configuration errors

- more than one explicit invalid marker
- more than one explicit blank marker
- invalid voting bounds

### 9.2 Structural decoding errors

- too few encoded positions
- values outside the allowed base range
- incomplete write-in terminators
- trailing unexpected data
- invalid character-map decoding

### 9.3 Semantic ballot errors

- overvote
- undervote
- disallowed explicit invalid vote
- duplicated preference positions
- gaps in ranked preferences

## 10. Canonical examples

Assume a plurality contest with no write-ins and these candidates after
identifier sorting:

1. `a-normal`, a regular candidate
2. `m-blank`, the explicit blank candidate
3. `z-normal`, a regular candidate

An explicit invalid marker is also configured, but it is represented by the
leading flag rather than by a candidate slot.

The base vector is:

```text
[2, 2, 2, 2]
```

The positions mean:

1. explicit invalid flag
2. `a-normal`
3. `m-blank`
4. `z-normal`

### 10.1 Implicit blank

```text
choices = [0, 0, 0, 0]
```

No candidate, including `m-blank`, is selected.

### 10.2 Explicit blank

```text
choices = [0, 0, 1, 0]
```

The existing candidate slot for `m-blank` carries the selection.

### 10.3 Normal vote for `a-normal`

```text
choices = [0, 1, 0, 0]
```

### 10.4 Explicit invalid

```text
choices = [1, 0, 0, 0]
```

## 11. Multi-contest codec

The platform also defines a compact multi-contest codec for bundling several
plurality contests into one encoded payload.

### 11.1 Scope

This codec is intended for:

- plurality-at-large contests only
- no write-ins
- an optional ballot-level decline-to-vote flag
- one explicit invalid flag per contest
- up to `max_votes` sparse candidate references per contest; an explicit
  blank is one of those candidate references

### 11.2 Layout

The multi-contest representation is:

1. one ballot-level decline-to-vote flag, when decline-to-vote is enabled
2. a contiguous block of fixed-size contest segments, sorted by contest
   identifier

Each contest segment is encoded as:

1. explicit invalid flag
2. `max_votes` sparse candidate positions

The number of positions is therefore:

```text
(1 if decline-to-vote is enabled else 0)
+ number_of_contests
+ sum(contest.max_votes)
```

### 11.3 Slot meaning

Each contest's explicit invalid flag uses base `2`.

Each sparse candidate position uses:

```text
number_of_non_explicit_invalid_candidates + 1
```

Values are interpreted as:

- `0` = unused slot
- `1..n` = selected candidate's identifier-sorted position plus one

Unlike the single-contest codec, the multi-contest codec is **sparse**: it
stores up to `max_votes` selected candidate references rather than one
boolean slot per candidate.

The explicit invalid marker is excluded from candidate indexing. The explicit
blank candidate is not excluded: it keeps its identifier-sorted candidate index
and is encoded in one of the same sparse positions as a regular candidate.

### 11.4 Decoding

To decode a multi-contest ballot:

1. read the ballot-level decline-to-vote flag, when present
2. for each contest in contest identifier order, read the explicit invalid flag
3. read the contest's `max_votes` sparse positions
4. decode each non-zero position against the identifier-sorted candidates,
   excluding only the explicit invalid marker
5. apply the same semantic validation and tally classification described
   earlier

### 11.5 Explicit blank example

For one contest with `max_votes = 1`, decline-to-vote disabled, and the
three non-explicit-invalid candidates from the dense example, the bases are:

```text
[2, 4]
```

The explicit blank candidate sorts second, so these states are:

```text
implicit blank: choices = [0, 0]
explicit blank: choices = [0, 2]
```

No extra base or choice is introduced for explicit blank.

## 12. Interoperability requirements

An implementation is compatible with this specification only if it:

1. preserves the distinction between explicit blank and implicit blank
2. preserves the explicit invalid flags
3. uses the same stable candidate ordering
4. applies the same positional offsets and zero-value conventions
5. rejects invalid configurations with duplicated explicit markers
6. decodes write-ins using the same delimiter semantics
7. applies policy validation after decoding
8. retains explicit blank candidates in both dense slots and sparse candidate
   indexing

## 13. Backward-compatibility expectation

The explicit blank behavior does not change the dense or multi-contest ballot
layout. Existing positions, bases, and candidate-index offsets remain valid,
and previously encoded ballots remain decodable.

Consumers that only understand the aggregate blank count may continue to use the
consolidated blank total. Consumers that need full semantic fidelity must retain
whether the explicit blank candidate was selected so that decoded and tally
results preserve the explicit-versus-implicit distinction.
