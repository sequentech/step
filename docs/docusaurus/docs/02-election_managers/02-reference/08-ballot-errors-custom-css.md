---
id: ballot_errors_custom_css
title: Styling Ballot Errors and Warnings with Custom CSS
sidebar_position: 8
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

While a voter fills in their ballot, the Voting Portal validates the selections and displays error and warning boxes — for example when the voter selects fewer choices than the minimum, exceeds the maximum, or leaves a preferential order with gaps. Each of these boxes carries stable CSS hooks so that individual messages can be styled (or hidden) using the **Custom CSS** setting, without affecting the rest of the messages.

Custom CSS can be configured in the Admin Portal under **Data → Ballot Design → Custom CSS**, at the Election Event level or per Election.

---

## CSS hooks

Every error or warning box rendered by the Voting Portal exposes:

- **A message-specific CSS class**, derived from the message identifier by prefixing `warn--` and replacing every character that is not a letter, digit, underscore or hyphen with a hyphen. For example, the message `errors.implicit.underVote` gets the class `warn--errors-implicit-underVote`.
- **`data-warn-id`**: the raw message identifier, e.g. `data-warn-id="errors.implicit.underVote"`.
- **`data-warn-type`**: the category of the validation, one of `Implicit`, `Explicit` or `EncodingError` (see [Message types](#message-types) below).

Messages are shown in two severities:

- **Errors** are shown in **warning** boxes (yellow by default). They indicate the selection is invalid or could not be decoded.
- **Alerts** are shown in **info** boxes (blue by default). They inform the voter about something noteworthy — such as an undervote — that does not necessarily invalidate the ballot.

Errors also appear on the review screen next to each contest before the ballot is cast, with the same CSS hooks.

## Message types

The `data-warn-type` attribute classifies each message:

- **`Implicit`**: the combination of selections makes the vote invalid or noteworthy (undervote, overvote, blank vote, duplicated preference position, …). Whether each condition produces an error, an alert, or nothing depends on the corresponding contest policy (invalid vote policy, over/under vote policy, etc.).
- **`Explicit`**: related to the voter explicitly marking the ballot as invalid, where the contest configuration allows or disallows it.
- **`EncodingError`**: the ballot could not be correctly encoded or decoded (for example, a write-in text that is too long).

## Message reference

| Message identifier | CSS class | Type | Severity |
|---|---|---|---|
| `errors.implicit.selectedMin` | `warn--errors-implicit-selectedMin` | Implicit | Error. Fewer choices selected than the minimum. |
| `errors.implicit.selectedMax` | `warn--errors-implicit-selectedMax` | Implicit | Error (overvote); depending on the over vote policy it is also raised as an alert. |
| `errors.implicit.underVote` | `warn--errors-implicit-underVote` | Implicit | Alert. Fewer choices selected than the maximum (shown when the under vote policy is not `ALLOWED`). |
| `errors.implicit.overVoteDisabled` | `warn--errors-implicit-overVoteDisabled` | Implicit | Alert. The maximum number of selections was reached and further options are disabled. |
| `errors.implicit.blankVote` | `warn--errors-implicit-blankVote` | Implicit | Error when the blank vote policy is `NOT_ALLOWED`, alert otherwise. |
| `errors.implicit.maxSelectionsPerType` | `warn--errors-implicit-maxSelectionsPerType` | Implicit | Error. More choices selected for a candidate list than that list's maximum. |
| `errors.implicit.duplicatedPosition` | `warn--errors-implicit-duplicatedPosition` | Implicit | Error. The same preference position was selected for two or more candidates. |
| `errors.implicit.preferenceOrderWithGaps` | `warn--errors-implicit-preferenceOrderWithGaps` | Implicit | Error. The order of preference has one or more gaps. |
| `errors.explicit.notAllowed` | `warn--errors-explicit-notAllowed` | Explicit | Error. The ballot was explicitly marked invalid but the contest does not allow it. |
| `errors.explicit.alert` | `warn--errors-explicit-alert` | Explicit | Alert. The selection marked will be considered an invalid vote. |
| `errors.encoding.writeInCharsExceeded` | `warn--errors-encoding-writeInCharsExceeded` | EncodingError | Error. Write-in text exceeds the maximum number of characters. |
| `errors.encoding.notEnoughChoices` | `warn--errors-encoding-notEnoughChoices` | EncodingError | Error. Not enough choices to decode the ballot. |
| `errors.encoding.writeInChoiceOutOfRange` | `warn--errors-encoding-writeInChoiceOutOfRange` | EncodingError | Error. A write-in choice is out of range. |
| `errors.encoding.writeInNotEndInZero` | `warn--errors-encoding-writeInNotEndInZero` | EncodingError | Error. A write-in encoding doesn't end in 0. |
| `errors.encoding.bytesToUtf8Conversion` | `warn--errors-encoding-bytesToUtf8Conversion` | EncodingError | Error. A write-in could not be converted from bytes to a UTF-8 string. |
| `errors.encoding.ballotTooLarge` | `warn--errors-encoding-ballotTooLarge` | EncodingError | Error. The encoded ballot is larger than expected. |
| `errors.encoding.invalidMaxVotes` | `warn--errors-encoding-invalidMaxVotes` | EncodingError | Error. The contest's maximum votes configuration is invalid. |
| `errors.encoding.invalidMinVotes` | `warn--errors-encoding-invalidMinVotes` | EncodingError | Error. The contest's minimum votes configuration is invalid. |

## Examples

Highlight the undervote alert with a red border and bold text:

```css
.warn--errors-implicit-underVote {
    border: 2px solid #d32f2f;
    font-weight: bold;
}
```

Hide the "maximum reached" alert entirely:

```css
.warn--errors-implicit-overVoteDisabled {
    display: none;
}
```

Style a whole category of messages using the `data-warn-type` attribute:

```css
[data-warn-type="EncodingError"] {
    background-color: #fdecea;
}
```

Target a message by its raw identifier instead of the derived class:

```css
[data-warn-id="errors.implicit.selectedMax"] {
    text-decoration: underline;
}
```

Since the Custom CSS setting exists both on the Election Event and on each Election, styles can be applied globally to the whole event or only to a specific election.
