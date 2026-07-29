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

Each candidate belongs to exactly one of the following logical classes for
encoding purposes:

1. **Normal candidate**
2. **Explicit invalid marker**
3. **Explicit blank marker**
4. **Write-in candidate**

### 2.3 Ballot states relevant to encoding

For each contest, the codec must distinguish the following states:

- **Normal vote**: one or more normal candidates are selected
- **Explicit invalid**: the contest is deliberately marked invalid
- **Implicit blank**: no selectable option is chosen
- **Explicit blank**: the dedicated blank option is chosen

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

The encoded vector is composed in this order:

1. **explicit invalid flag**
2. **explicit blank flag**, if the contest defines an explicit blank marker
3. **ordinary candidate section**
4. **write-in text section**, if write-ins are enabled

The fixed-size part of the vector therefore contains:

```text
1
+ (1 if the contest defines an explicit blank marker else 0)
+ number_of_ordinary_candidates
```

positions, followed by the variable-length write-in section.

### 4.2 Explicit invalid flag

The first position is always reserved for the explicit invalid flag.

- base: `2`
- values:
  - `0` = contest is not explicitly invalid
  - `1` = contest is explicitly invalid

This flag is orthogonal to all candidate slots.

### 4.3 Explicit blank flag

If the contest defines an explicit blank marker, the second position is
reserved for the explicit blank flag.

- base: `2`
- values:
  - `0` = explicit blank not selected
  - `1` = explicit blank selected

The explicit blank marker is **not** encoded as an ordinary candidate choice.
It has its own dedicated slot so that an explicit blank vote remains distinct
from an implicit blank vote.

### 4.4 Ordinary candidate section

The ordinary candidate section contains all candidates that are **not**:

- explicit invalid markers, and
- explicit blank markers.

These positions encode the actual voted options.

#### 4.4.1 Plurality-style contests

For plurality-style contests, each ordinary candidate receives one slot.

- base: `2`
- values:
  - `0` = not selected
  - `1` = selected

#### 4.4.2 Ranked or preferential contests

For preferential contests, each ordinary candidate receives one slot whose base
is:

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

#### 4.4.3 Cumulative contests

For cumulative contests, each ordinary candidate slot uses:

```text
cumulative_number_of_checkboxes + 1
```

The zero value means no selection, and positive values represent the configured
selection strength according to the contest’s cumulative policy.

### 4.5 Write-in section

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
- the explicit blank flag is either absent or `0`
- all ordinary candidate slots are unset
- all write-in values are empty

### 5.2 Explicit blank

A contest is an **explicit blank vote** if:

- the explicit blank flag is `1`
- no ordinary candidate is selected

The explicit blank state must survive round-trip encode/decode unchanged.

### 5.3 Tallying interpretation

For tallying purposes:

- implicit blank and explicit blank both contribute to the overall blank total
- explicit blank must remain distinguishable from implicit blank in decoded and
  reported data
- explicit blank must **not** be counted as a normal candidate vote
- a contest that selects explicit blank together with an ordinary candidate is an
  implicit invalid vote, not a blank vote

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
2. if present, read the explicit blank flag
3. decode the ordinary candidate section
4. decode the write-in section
5. reconstruct the semantic contest state

### 7.1 Reconstruction requirements

After decoding:

- an explicit invalid flag must reconstruct the explicit invalid state
- an explicit blank flag must reconstruct the dedicated blank candidate as
  selected
- ordinary candidate slots must reconstruct normal candidate selections
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

An explicit blank vote must still be treated as a blank vote when applying blank
vote policy. It must not be treated as a normal candidate selection.

For the minimum-selections rule, a set explicit invalid flag or a set explicit
blank flag counts as one selection: a contest that is deliberately marked
invalid or blank must not additionally be reported as under the minimum.

## 9. Error conditions

Compatible implementations must detect and report at least the following error
classes:

### 9.1 Configuration errors

- more than one explicit invalid marker
- more than one explicit blank marker
- invalid voting bounds
- a generated multi-contest ballot style whose maximum encoded size exceeds
  the platform's fixed-size payload (Section 11.2)

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

Assume a plurality contest with:

- one explicit invalid marker
- one explicit blank marker
- two normal candidates
- no write-ins

The base vector is:

```text
[2, 2, 2, 2]
```

The positions mean:

1. explicit invalid flag
2. explicit blank flag
3. first ordinary candidate
4. second ordinary candidate

### 10.1 Implicit blank

```text
choices = [0, 0, 0, 0]
```

### 10.2 Explicit blank

```text
choices = [0, 1, 0, 0]
```

### 10.3 Normal vote for the first ordinary candidate

```text
choices = [0, 0, 1, 0]
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
- one explicit blank flag per contest when the contest defines an explicit
  blank marker

### 11.2 Encoding mode

Each ballot style persists an encoding mode that determines how many ordinary
candidate slots (`vote_slot_count`) each contest reserves:

- **`legacy`**: `vote_slot_count` equals `contest.max_votes`. A selection with
  more ordinary candidates than `max_votes` cannot be encoded.
- **`expanded-capacity`**: `vote_slot_count` equals the contest's number of
  ordinary candidates, so a selection can include every ordinary candidate.

The mode is resolved once per election, when its ballot styles are generated:
`expanded-capacity` applies to every contest in the election if *any* contest
in that election has an over-vote policy of `allowed`, `allowed-with-msg`, or
`allowed-with-msg-and-alert` — the policies that let a voter select more than
`max_votes` in the first place. Otherwise the election uses `legacy`.

The mode must be resolved once and persisted on the ballot style, not
re-derived from a contest's current over-vote policy at encode or decode
time. Encoding and decoding of the same ballot style can happen far apart (a
ballot may be decoded for tallying long after it was cast), so re-deriving the
mode from live configuration risks disagreement between the encode-time and
decode-time layouts if that configuration changes in between.

Maximum-selections and overvote **validation** (Section 8.2) always uses
`contest.max_votes`, independently of the encoding mode: `expanded-capacity`
only widens what the codec can *represent*; it does not change what counts as
a valid vote. A selection wider than `max_votes` is still reported as an
overvote error — `expanded-capacity` only keeps that overvote from also being
an encoding failure.

Because a ballot style's total encoded size must still fit the platform's
fixed-size encrypted payload, election publication must validate the maximum
encoded size of every generated ballot style and reject publication if any
contest's expanded slot count would exceed that limit.

### 11.3 Layout

The multi-contest representation is:

1. one ballot-level decline-to-vote flag, when decline-to-vote is enabled
2. a contiguous block of fixed-size contest segments, sorted by contest
   identifier

Each contest segment is encoded as:

1. explicit invalid flag
2. explicit blank flag, if the contest defines an explicit blank marker
3. `vote_slot_count` sparse ordinary-candidate positions (Section 11.2)

The number of positions is therefore:

```text
(1 if decline-to-vote is enabled else 0)
+ number_of_contests
+ number_of_contests_with_explicit_blank_markers
+ sum(contest.vote_slot_count)
```

### 11.4 Slot meaning

The explicit invalid and explicit blank flags use base `2`.

Each sparse ordinary-candidate position uses:

```text
number_of_ordinary_candidates + 1
```

Values are interpreted as:

- `0` = unused slot
- `1..n` = selected candidate position plus one

Unlike the single-contest codec, the multi-contest codec is **sparse**: it
stores up to `vote_slot_count` selected candidate references rather than one
boolean slot per ordinary candidate.

Explicit invalid and explicit blank marker candidates are not part of the sparse
ordinary-candidate positions. Selecting one of those marker candidates sets the
corresponding bit instead, and decoding that bit reconstructs the marker
candidate as selected.

### 11.5 Decoding

To decode a multi-contest ballot:

1. read the ballot-level decline-to-vote flag, when present
2. for each contest in contest identifier order, read the explicit invalid flag
3. read the explicit blank flag when the contest defines an explicit blank
   marker
4. decode each non-zero sparse slot back to its ordinary candidate reference
5. reconstruct explicit invalid and explicit blank marker candidates from their
   bits
6. apply the same semantic validation rules described earlier

A decoder must use the same encoding mode (Section 11.2) that was used to
encode the ballot style, read from the persisted ballot style rather than
re-derived, or it will misread the sparse ordinary-candidate positions.

## 12. Interoperability requirements

An implementation is compatible with this specification only if it:

1. preserves the distinction between explicit blank and implicit blank
2. preserves explicit invalid markers
3. uses the same stable candidate ordering
4. applies the same positional offsets and zero-value conventions
5. rejects invalid configurations with duplicated explicit markers
6. decodes write-ins using the same delimiter semantics
7. applies policy validation after decoding

## 13. Backward-compatibility expectation

Consumers that only understand the aggregate blank count may continue to use the
consolidated blank total. Consumers that need full semantic fidelity must retain
the explicit-versus-implicit blank distinction exposed by decoding and tally
results.
