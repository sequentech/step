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
  voters already see at registration.
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
4. Click **Save**.

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
