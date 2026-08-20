---
id: admin_portal_tutorials_multi_attribute_password_login
title: Logging In Without a Username (Attribute + Password)
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Overview

By default, voters log in with a username and password. Some elections instead identify voters by
attributes they already know - a date of birth, a national ID - without asking them to remember a
separate username. This tutorial explains how to configure the **Multi-Attribute + Password Form**
authenticator so voters log in with one or more configured user attributes plus a password,
instead of a username.

The authenticator finds every user whose configured attribute(s) match the submitted value(s) -
**all** configured attributes must match the same user - then checks the submitted password
against that candidate. Login succeeds only when **exactly one** candidate's password matches.

A single attribute like date of birth is not unique on its own (many voters share a birth date).
This still works: the authenticator collects every user with that birth date as candidates and
relies on the password to uniquely identify one of them. Configuring a second attribute (e.g. also
a national ID) narrows the candidate set before the password check, and is recommended whenever a
second identifying attribute is available.

---

## Prerequisites

- Access to the Keycloak Admin Console.
- The `sequent.message-otp-authenticator.jar` extension deployed in Keycloak's `providers/`
  directory (included in the Sequent Keycloak Docker image by default).
- The user attribute(s) you want to log in with must already exist in the realm's **User
  Profile** - see
  [Adding User Attributes to Keycloak](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md).
  If an attribute is configured there with **Input type** `html5-date` (e.g. a date of birth
  field), the login form automatically renders it as a native date picker too, matching what
  voters already see at registration. The same is true of every other User Profile annotation -
  helper text, placeholders, dropdown options, phone-number widgets - see
  [Configuring Login and Registration Fields](../02-reference/10-user-profile-login-registration-fields.md).
- **Date-valued attributes must be stored as `YYYY-MM-DD`** (e.g. `1990-01-05` for January 5,
  1990) - the same format the browser's native date picker always submits, so no reformatting is
  needed on the browser path.

---

## Step 1 – Create a Browser Flow with the New Authenticator

1. Navigate to **Authentication** → **Flows**.
2. Duplicate an existing browser flow (e.g. the realm's default `browser` flow) or create a new
   one, and give it a descriptive name such as `attribute password login`.
3. Inside the appropriate sub-flow, click **Add step**.
4. Search for **Multi-Attribute + Password Form** and add it.
5. Set the requirement to **ALTERNATIVE** (to offer it alongside other login methods already in
   the flow) or **REQUIRED** (to make it the only way to log in on that flow).

---

## Step 2 – Configure the Attributes to Match

1. Click **⚙ Config** (gear icon) next to the new step.
2. Give the config a descriptive alias, e.g. `attribute-login-dateOfBirth`.
3. Under **User attributes to match**, click **+ Add** and enter each user attribute name to
   require, e.g. `dateOfBirth`, then **+ Add** again for `nationalId` if you want a second
   attribute (all listed attributes must match the same user). These must be existing User
   Profile attribute names (see Prerequisites).
4. Optionally adjust the DoS-mitigation settings below (sensible defaults are pre-filled - see
   [Denial-of-Service Considerations](#denial-of-service-considerations)):
   - **Max candidates per request** (default `10`)
   - **Max failures per attribute-value combination** (default `10`)
   - **Failure window (seconds)** (default `60`)
   - **Max user-store rows per attribute lookup** (default `5000`)
5. Leave **Honor User Profile required attributes** off unless you want individual attributes to
   be optional to match - see
   [Required fields](../02-reference/10-user-profile-login-registration-fields.md#required-fields).
   Off (the default) means every attribute listed above is mandatory.
6. Leave **Multiple-candidate match policy** at its default, `REJECT_AMBIGUOUS`, unless you have
   read and understood [Multiple-Candidate Match Policy](#multiple-candidate-match-policy) below -
   the alternative, `FIRST_MATCH`, is only safe when password uniqueness across every possible
   candidate is guaranteed.
7. Click **Save**.

---

## Step 3 – Bind the Flow to a Client

1. Navigate to **Clients** and select the client this login page applies to (e.g.
   `onsite-voting-portal`).
2. Open the **Advanced** tab → **Authentication flow overrides**.
3. Set **Browser Flow** to the flow you created in Step 1.
4. Click **Save**.

This keeps the change scoped to the client(s) you explicitly bind it to - every other client
keeps using the realm's default browser flow unchanged.

---

## Behavior Summary

| Scenario | Authenticator action |
|---|---|
| A configured attribute field is left blank | Generic "invalid credentials" error (no lookup performed). |
| No user matches all configured attributes | Generic "invalid credentials" error. |
| Exactly one candidate, correct password | Login succeeds. |
| Exactly one candidate, wrong password | Generic "invalid credentials" error - this attempt **is** counted toward that account's Brute Force Detection lockout, same as a standard login. |
| Exactly one candidate, currently locked out by Brute Force Detection | "Account temporarily/permanently disabled" - no password check is even attempted. |
| Multiple candidates share the configured attribute(s), and the password matches exactly one | Login succeeds as that user. |
| Multiple candidates match the password (or none do) | Generic "invalid credentials" error - see the brute-force note below. |

The error is always the same generic message regardless of cause, so a failed attempt never
reveals which attribute, or the password, was wrong. Every "no match" outcome above (blank field,
no candidates, wrong password) takes the same time to respond, including a real password-hash
computation on paths that never actually found a candidate to check - so response time doesn't
reveal whether any account has the submitted attribute value.

> **Note on brute-force protection:** Keycloak's built-in per-account lockout only engages once
> resolution narrows to a single candidate - the same account that ends up locked out is also the
> one whose failed attempts get counted, matching how the standard username/password form behaves.
> When more than one candidate still shares the configured attribute(s), there is no single
> account a failed attempt can honestly be attributed to, so the counter can't engage for that
> specific request (a locked-out account among several ambiguous candidates still can't have its
> password probed, though - it's excluded from consideration before any password is checked).
> Configuring more attributes narrows the candidate set before the password check, making the
> single-candidate (fully protected) case the common one; keep **Brute Force Detection** enabled
> at the realm level regardless.

---

## Denial-of-Service Considerations

Because this authenticator matches on attributes rather than a unique username, a single request
can legitimately resolve to many candidates (e.g. every voter born on a common date) - and each
candidate normally costs one password-hash comparison to rule out. Three independent settings
bound that cost per request, on top of Keycloak's standard Brute Force Detection:

- **Max candidates per request** (`maxCandidates`, default `10`): once the configured attributes
  match more candidates than this, the request fails generically without checking any of their
  passwords. This bounds the worst-case number of password hashes a single request can force,
  regardless of how many voters share the submitted attribute value(s).
- **Max failures per attribute-value combination** / **Failure window (seconds)**
  (`tupleMaxFailures` / `tupleFailureWindowSeconds`, defaults `10` / `60`): failures are also
  counted per distinct combination of submitted attribute values, independent of any single
  account. This closes a gap that per-account Brute Force Detection can't cover on its own: when a
  request matches more than one candidate, Keycloak has no single account to attribute the failure
  to, so its lockout counter never engages for that request - an attacker could otherwise repeat a
  common attribute value (e.g. a shared date of birth) indefinitely at full cost. Once a
  combination's failures reach the configured maximum within the window, further attempts against
  it are rejected without any user lookup at all, until the window elapses or a matching request
  succeeds (which clears the count). This throttle is tracked cluster-wide, so it can't be evaded
  by spreading requests across Keycloak nodes.
- **Max user-store rows per attribute lookup** (`maxAttributeLookupResults`, default `5000`): a
  hard ceiling on how many rows the underlying user-store query may return, applied before any
  candidate is even loaded into memory. This is deliberately much larger than **Max candidates per
  request** - it exists only to stop a truly pathological match (e.g. an attribute value shared by
  most of the realm) from pulling an unbounded number of rows from the database, not to replace the
  tighter candidate cap above. Keep it well above the largest realistic combined match count you'd
  expect a legitimate voter lookup to produce; setting it too low can make configuring a second
  identifying attribute (the recommended lever below) less effective at narrowing large buckets.

Raising **Max candidates per request** trades this protection for looser matching (useful if a
single attribute alone is expected to match unusually large groups); lowering the failure
threshold or widening the window trades convenience for tighter DoS protection. Configuring a
second identifying attribute (see [Step 2](#step-2--configure-the-attributes-to-match)) is
generally the better lever - it keeps genuine candidate sets small without needing to loosen these
guards.

If PINs or passwords used with this authenticator are short (e.g. a numeric PIN), do **not**
compensate for guessability by weakening the password hash algorithm - that only makes offline
cracking easier if hashes ever leak. Use the settings above instead; they bound CPU cost without
touching hash strength.

---

## Multiple-Candidate Match Policy

When more than one candidate shares the configured attribute value(s) (e.g. several voters born on
the same date), **Multiple-candidate match policy** governs how the submitted password picks one:

- **`REJECT_AMBIGUOUS`** (default): checks every candidate's password. Only succeeds if the
  submitted password matches **exactly one** of them; if it matches more than one, the request
  fails generically, the same as if none had matched.
- **`FIRST_MATCH`**: succeeds as soon as **any** candidate's password matches, without checking
  whether another candidate would also have matched. Cheaper (stops at the first hit instead of
  hashing every candidate), but only correct under one condition.

> ⚠️ **Security warning:** `FIRST_MATCH` is only safe to enable when **password uniqueness across
> every candidate a request could ever match is guaranteed** - for example, passwords assigned
> centrally and never reused, with no possibility of two voters who share an attribute value also
> sharing a password. If two candidates in a matched set share the same password, **which one
> authenticates is unspecified** - meaning one voter could end up authenticated as a different
> voter's account. If you cannot guarantee password uniqueness across candidates, leave this at
> `REJECT_AMBIGUOUS`.

Enabling `FIRST_MATCH` does not change anything about the DoS mitigations above - the candidate cap
and per-tuple throttle still apply exactly the same way regardless of match policy.
