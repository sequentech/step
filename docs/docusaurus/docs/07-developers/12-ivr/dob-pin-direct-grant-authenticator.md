---
id: dob-pin-direct-grant-authenticator
title: DOB + PIN Authentication (Direct Grant)
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# DOB + PIN Authentication (Direct Grant)

## Overview

By default, IVR voters authenticate with a voter ID (`direct-grant-validate-username`) followed by
a PIN (`direct-grant-validate-password`). Some elections instead want voters to authenticate with
one or more attributes they already know - a date of birth, a national ID - plus a PIN, without a
separate voter ID step. `MultiAttributePasswordDirectGrantAuthenticator`
(provider ID `multi-attribute-password-direct-grant`) is the IVR/Direct Grant counterpart of the
web login's [Multi-Attribute + Password Form](../../02-election_managers/01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md) -
both share the same resolution logic (`MultiAttributeCredentialResolver`): every configured
identifying attribute must match the same user, and the PIN then disambiguates among candidates.

This is a single Direct Grant flow step - it replaces both `direct-grant-validate-username` and
`direct-grant-validate-password` at once.

---

## Current IVR Lambda compatibility

The Keycloak side (this authenticator and `ivr-config-provider`) supports any number of
`identifier` fields plus one `secret` field, each with an independently configurable `maps_to`.
**As validated against `beyond` at `feat/meta-10554/main`
(`packages/ivr-core/src/execution/phases/auth.rs`), the IVR Lambda does not yet exercise that full
range:**

- **Only one identifier field is collected.** `AuthState::IdentifierPrompt` picks the *first*
  `identifier`-kind step (`.find(...)`) and moves straight to the secret step after collecting it.
  A second `identifier` entry in the config is accepted by `/ivr-config` but never prompted for or
  collected by the Lambda. Until that loop is added, configure **exactly one** `identifier` field.
- **`maps_to` is not yet honored when submitting to Keycloak.** The token request is hardcoded to
  `("username", <identifier value>)` and `("password", <secret value>)`
  (`// TODO: Use maps_to` in `auth.rs`), regardless of what `maps_to` says. Until that's fixed,
  `maps_to` must literally be `username` for the identifier field and `password` for the secret
  field for a voter to actually authenticate through IVR - anything else (e.g. `maps_to: dob`)
  will pass `/ivr-config` validation but silently fail token requests, because the value never
  arrives under the parameter name this authenticator expects.
- **There's no automatic prompt for non-standard fields.** `map_auth_prompt()` only has a built-in
  spoken prompt for the literal field names `voter_id` and `pin`. Any other `field` value (e.g.
  `dob`) falls back to an "external" prompt, whose text an admin must configure separately via the
  IVR prompt-override admin interface - there is no automatic `auth_enter_dob`-style default.

None of this limits what this authenticator can be configured to *express* (it's designed for the
target end state), but until `beyond` closes the gaps above, the only IVR-usable configuration
today is one `identifier` field mapped to `username` plus the `secret` field mapped to `password` -
functionally equivalent to the default voter-ID + PIN flow, just collected through a single
combined step instead of two, and letting the "identifier" be any user attribute (not necessarily
the username) as long as it's submitted as `username`.

---

## How field configuration works

Unlike the web form, this authenticator has no separate "which attributes to match" setting.
Instead, it derives its resolution inputs directly from the same `field` / `max_digits` / `kind` /
`maps_to` / `prompt_key` properties the [`ivr-config-provider`](./keycloak-config.md) module reads
to describe DTMF collection steps to the IVR Lambda - one shared source of truth, so the two can
never drift out of sync:

- Every entry whose `kind` is `identifier` is a Keycloak user attribute to match. Its `maps_to`
  value is **both** the attribute name **and** the `grant_type=password` POST parameter name the
  Lambda submits it under.
- Exactly one entry's `kind` must be `secret` - the PIN. Its `maps_to` value should normally be
  `password` (the standard OAuth2 ROPC parameter name).
- All four required properties (`field`, `max_digits`, `kind`, `maps_to`) must have the same
  `##`-separated value count; `prompt_key` is optional but must also match that count if present.
- **Date-valued identifier fields** (e.g. a date of birth collected as 8 raw DTMF digits): the
  stored attribute must be canonical `YYYY-MM-DD`. Since the IVR prompt collects raw digits with
  no separators (e.g. `MMDDYYYY`), set the `date_format` property to that digit order - aligned
  by index with the `identifier` fields only (not the secret field) - and the authenticator
  normalizes into `YYYY-MM-DD` before matching. Leave it unset (or leave that field's entry
  blank) for identifier fields that aren't dates.

**Example (target design)** - DOB + national ID + PIN, no voter ID. Requires the `beyond` fixes
described above; not usable end-to-end today:

| Property | Value |
|---|---|
| `field` | `dob##nationalId##pin` |
| `max_digits` | `8##10##8` |
| `kind` | `identifier##identifier##secret` |
| `maps_to` | `dob##nationalId##password` |
| `prompt_key` | `auth_enter_dob##auth_enter_national_id##auth_enter_pin` (optional; each prompt's text must still be configured via the IVR prompt-override admin interface) |

**Example (works today)** - a single identifying attribute, submitted the way the current Lambda
expects:

| Property | Value |
|---|---|
| `field` | `dob` |
| `max_digits` | `8` |
| `kind` | `identifier` |
| `maps_to` | `username` |

(then a second entry for the PIN: `field=pin`, `max_digits=8`, `kind=secret`,
`maps_to=password` - i.e. `field=dob##pin`, `max_digits=8##8`, `kind=identifier##secret`,
`maps_to=username##password`)

Either way, the authenticator resolves the voter by intersecting candidates matching every
`identifier` field's submitted value, then checking the submitted secret (PIN) against that
candidate set.

---

## Step 1 - Create the Direct Grant Flow

1. In the Keycloak Admin Console, navigate to **Authentication** → **Flows**.
2. Duplicate an existing Direct Grant flow (or create a new one), e.g.
   `ivr direct grant - dob pin`.
3. Remove (or set to **DISABLED**) the default `Username Validation` and `Password Validation`
   steps.
4. Click **Add step**, search for **Multi-Attribute + Password (Direct Grant)**, and add it with
   requirement **REQUIRED**.

## Step 2 - Configure the Fields

1. Click **⚙ Config** next to the new step.
2. Fill in `field`, `max_digits`, `kind`, `maps_to` (and optionally `prompt_key`). Use the
   "works today" example above (`maps_to=username##password`) unless the deployment's `beyond`
   version has already picked up the fixes described in
   [Current IVR Lambda compatibility](#current-ivr-lambda-compatibility).
3. Click **Save**.

## Step 3 - Bind the Flow to the `ivr-voting` Client

1. Navigate to **Clients** → `ivr-voting` → **Advanced** → **Authentication flow overrides**.
2. Set **Direct Grant Flow** to the flow created in Step 1.
3. Click **Save**.

`IvrConfigResourceProvider` (see [Keycloak Configuration](./keycloak-config.md)) automatically
picks up this override when building `/ivr-config` for the `ivr-voting` client - no separate
configuration needed there.

---

## Behavior Summary

| Scenario | Authenticator action |
|---|---|
| `maps_to`/`kind` missing, or their `##`-separated counts don't match | Direct Grant `invalid_grant` error (misconfiguration, fails closed). |
| No `kind=secret` entry, or no `kind=identifier` entries | Direct Grant `invalid_grant` error (misconfiguration, fails closed). |
| No user matches all `identifier` attributes | Direct Grant `invalid_grant` error. |
| Exactly one candidate, correct PIN | Authenticates as that user. |
| Exactly one candidate, wrong PIN | Direct Grant `invalid_grant` error - counted toward that account's Brute Force Detection lockout, same as a standard login. |
| Exactly one candidate, currently locked out by Brute Force Detection | Direct Grant `invalid_grant` error - no PIN check is even attempted. |
| Multiple candidates share the identifying attribute(s), PIN matches exactly one | Authenticates as that user. |
| Multiple candidates match the PIN (or none do) | Direct Grant `invalid_grant` error - see the brute-force note below. |

The error is the same generic `invalid_grant` regardless of cause, matching the web form's
generic-error behavior - a failed attempt never reveals which attribute, or the PIN, was wrong.
Every "no match" outcome takes the same time to respond, including a real password-hash
computation on paths that never found a candidate to check.

> **Note on brute-force protection**, same behavior as the web form: Keycloak's per-account
> brute-force lockout engages once resolution narrows to a single candidate - that account's
> failed PIN attempts get counted the same way a standard login's would. When more than one
> candidate still shares the identifying attribute(s), there is no single account a failed attempt
> can honestly be attributed to, so the counter can't engage for that specific request (a
> locked-out account among several ambiguous candidates still can't have its PIN probed, though -
> it's excluded from consideration before any PIN is checked). Configuring more identifying
> attributes narrows the candidate set before the PIN check, making the single-candidate (fully
> protected) case the common one; keep **Brute Force Detection** enabled at the realm level
> regardless.
