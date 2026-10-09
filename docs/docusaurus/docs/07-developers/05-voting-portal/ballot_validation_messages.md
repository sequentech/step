---
id: ballot_validation_messages
title: Ballot validation messages
description: How the warnings a voter sees while marking a ballot get their numbers, and when a new message needs a count derivation.
---

These are the warnings shown **while a voter is marking a ballot** — "select one more candidate", "shorten your
write-in by two characters". They are a different family from the cast-vote failures in
[Cast Vote Errors](./cast_vote_errors.md), which happen after the voter submits.

## Where they come from

The Rust checker in `sequent-core` reports a problem as a **message key** plus a **`message_map`**. Every value
in that map is a string, and the map describes the *state* of the selection — `numSelected`, `min`, `max` —
rather than the number the sentence is actually about.

That distinction is the whole reason this layer exists. "Select 2 more candidates" needs `min - numSelected`;
the checker never sends that number, it sends the two operands.

## Rendering them

`getBallotErrorOptions` bridges the two. Always render these messages through it:

```tsx
import {getBallotErrorOptions} from "@sequentech/ui-core"

t(error.message || "", getBallotErrorOptions(error.message, error.message_map))
```

It does two things:

1. **Copies every `message_map` entry into the options**, coercing numeric strings to numbers. So if the
   checker already sends a `count`, the message pluralises with no further work.
2. **Derives `count`** for the keys listed in `COUNT_DERIVATIONS`, overriding whatever came in the map.

The coercion in step 1 matters: i18next skips pluralisation entirely when `count` is a string and silently
falls back to the unsuffixed key. Passing the raw `message_map` straight to `t()` is a bug for exactly that
reason.

## Adding a new validation message

Adding the key in Rust and translating it is enough **when the sentence's number is already in
`message_map`**.

Add an entry to `COUNT_DERIVATIONS` in `packages/ui-core/src/services/ballotErrorMessages.ts` only when the
number has to be computed from the state fields:

| message | the sentence is about | derivation |
|---|---|---|
| `errors.implicit.selectedMin` | how many more to pick | `min - numSelected` |
| `errors.implicit.underVote` | how many more are allowed | `max - numSelected` |
| `errors.implicit.selectedMax` | how many to unpick | `numSelected - max` |
| `errors.implicit.maxSelectionsPerType` | how many to unpick from one candidate type | `numSelected - max` |
| `errors.implicit.overVoteDisabled` | the maximum already reached | `max`, falling back to `numSelected` when the map carries no `max` |

That is the whole of `COUNT_DERIVATIONS`. Every other validation message is rendered from the `message_map`
as it arrives.

A derivation always wins over a `count` that arrived in the map, so do not add one when the checker already
sends the right number.

## Pluralising them

A derived `count` is a number like any other, so these messages follow the ordinary plural rules — with one
wrinkle worth knowing: the derivations subtract, so they can produce values the checker never sends.

* Give the message the plural forms **every target language reports**, not just `_one` and `_other`. Spanish,
  Catalan and French also need `_many`; a missing form falls through to `fallbackLng` and renders the English
  sentence rather than the right one. `_many` is a verbatim copy of `_other`.
* Adding a suffix to `en.ts` makes it a required key in every other language file, because they are typed
  `TranslationType = typeof englishTranslation`.
* `overVoteDisabled` is the one message whose count is a bound rather than a difference, so it is the one that
  can carry a large operator-configured number straight into the sentence.

See [Add a New Language](../10-tutorials/01-add_new_language.md) for the per-language rules.
