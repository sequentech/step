---
id: voting_portal_datetime_format
title: Voting Portal Date & Time Format
sidebar_position: 8
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Voting Portal date & time format** controls how dates and times are displayed to
voters across the Voting Portal for a given Election Event. It is configured in two
complementary ways:

- An **event-wide format** chosen in the Admin Portal — either a preset from a
  controlled list, or a **custom token pattern**.
- An optional **per-language override** typed as a translation string in the
  Localization tab.

Both are stored on the Election Event and resolved by a single shared helper, so every
voter-visible date renders consistently.

---

## Presets

The preset is set in **Election Event → Data → Advanced Configurations**, in the
**Voting Portal date & time format** select. It applies **event-wide** (to every voter,
regardless of language, unless overridden — see below).

The default is **Legacy GB 24h**, which preserves the behavior in place before this
feature existed, so existing events do not change appearance.

The following presets are available (examples shown for the instant
**9 March 2026, 07:05** in the voter's local time):

| Preset | Wire value | Example output |
|---|---|---|
| **Legacy GB 24h** (default) | `legacy-gb-24h` | `09/03/2026, 07:05` |
| **ISO Local** | `iso-local` | `2026-03-09 07:05` |
| **US 12h** | `us-12h` | `03/09/2026, 7:05 AM` |
| **Locale Medium** | `locale-medium` | `9 Mar 2026, 07:05` (in the voter's locale) |
| **Date Only** | `date-only` | date in the voter's locale, no time component |

`LEGACY_GB_24H` is the default and reproduces the prior ballot-list formatting exactly.
`LOCALE_MEDIUM` and `DATE_ONLY` are rendered in the voter's active language; the other
presets are locale-pinned for stable output.

---

## Custom event-wide format

The same select also offers **Custom format**. Selecting it reveals a **Custom date &
time format** field where a token pattern (see the token reference below) applies
**event-wide**, exactly like a preset.

The pattern is validated when saving the Data tab: an invalid pattern blocks the save
and shows the error below the field, so it is never persisted. It is stored inline in
the same `voting_portal_datetime_format` field as the presets, as
`{"custom": "<pattern>"}`.

---

## Per-language override

The preset can be overridden **per language** using the existing Localization workflow.
This is useful when a specific language needs a different field layout than the
event-wide preset provides.

Set it in **Election Event → Localization**: pick the language, then add the key
`votingPortalDateTimeFormat` with a token pattern as its value. The override applies
**only to that language**; languages without an override fall through to the event
preset.

The value is interpreted as a **token pattern**. Tokens render in the voter's local
time; any other characters (separators, spaces, literals) pass through unchanged.

| Token | Meaning | Example (for 2026-03-09 07:05) |
|---|---|---|
| `yyyy` | 4-digit year | `2026` |
| `MM` | 2-digit month | `03` |
| `dd` | 2-digit day | `09` |
| `HH` | 2-digit hour (24h) | `07` |
| `mm` | 2-digit minute | `05` |
| `ss` | 2-digit second | `00` |

**Worked examples:**

| Pattern | Output |
|---|---|
| `dd/MM/yyyy HH:mm` | `09/03/2026 07:05` |
| `yyyy-MM-dd` | `2026-03-09` |
| `dd.MM.yyyy HH:mm:ss` | `09.03.2026 07:05:00` |

Tokens follow the Unicode LDML (CLDR) date field symbols and are **case-sensitive**.
Patterns containing the lookalike tokens `YYYY`, `DD`, or `hh` (common in Moment-style
or 12-hour conventions) are rejected outright: in LDML they mean something else
entirely (`YYYY` is the week-numbering year, `DD` the day of year), so accepting them
would silently corrupt voter-facing dates. Use `yyyy`, `dd`, and `HH` instead.

An invalid pattern (empty, containing no recognized token, or containing one of the
rejected tokens above) is **rejected at save time** — in the Localization tab with an
error notification, and in the Data tab's custom format field with an inline error —
so it is never persisted. The same validation applies to both.

---

## Behavior and resolution order

For each voter-facing date, the format is resolved in this order:

1. **Per-language override** — `votingPortalDateTimeFormat` in the voter's active
   language, if present and valid.
2. **Event format** — the preset or custom pattern selected in the Data tab.
3. **Legacy GB 24h** — the hard fallback.

The override is per-language: a language without an override falls through to the event
preset.

**Graceful fallback:** if an override that nonetheless reaches a voter is empty,
unrecognized, or fails to format (for example, legacy data predating save-time
validation), the Voting Portal silently uses the event format for that language.
Likewise, a stored custom event-wide pattern that is no longer valid falls back to
**Legacy GB 24h**. There is **no voter-visible error** — only a `console.warn` is
logged, and other languages are unaffected.

---

## Affected surfaces

The configured format applies to the voter-visible dates rendered by the Voting Portal:

- **Election cards** — the open / close period dates shown in the election list.
- **Ballot Locator** — the timestamps in the Logs tab.

> **Note:** Ballot Locator timestamps previously rendered as a raw UTC string. With this
> feature they render in the configured format (in local time), consistent with the rest
> of the Voting Portal.

This setting only affects **display**; the underlying stored instants are unchanged.
