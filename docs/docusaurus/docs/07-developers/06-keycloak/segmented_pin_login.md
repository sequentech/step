---
id: segmented_pin_login
title: Segmented PIN login
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

The segmented PIN input is an optional presentation for a voter's ordinary
Keycloak password. The other login value remains the username. It does not add
a credential type, change how Keycloak stores or verifies credentials, or add a
date-of-birth field.

## Realm configuration

Set these Keycloak realm attributes on the election event realm:

| Attribute | Values | Default |
| --- | --- | --- |
| `credential-input-policy` | `standard` or `segmented-numeric` | `standard` |
| `credential-segment-layout` | Dash-separated segment sizes such as `4-4-4-4`, `3-3`, or `2-4-2` | `4-4-4-4` |

A layout may contain 1–8 groups. Each group may contain 1–12 digits and the
combined length may not exceed 64 digits. The application rejects invalid
values when realm attributes are saved. If malformed configuration reaches
Keycloak through another route, the theme leaves the standard password field
usable.

The attributes can be included in deployment realm configuration or entered in
the permission-gated **Keycloak realm attributes** JSON editor for the election
event. For example:

```json
{
  "credential-input-policy": "segmented-numeric",
  "credential-segment-layout": "4-4-4-4"
}
```

Removing `credential-input-policy`, or setting it to `standard`, restores the
ordinary password field. The setting is realm-wide; this implementation has no
client-level override.

## Supported login forms

The policy applies to both:

- the normal `sequent.voting-portal` Keycloak login page; and
- the deferred voter-registration form when its authenticator has
  `form-mode=LOGIN` and `password-required=true`.

Actual registration (`form-mode=REGISTRATION`) is unchanged: it continues to
show password confirmation, the strength indicator, and Keycloak's password
creation-policy validation.

The segmented fields accept ASCII digits only. They are presentation-only and
have no form names. On submit, the browser concatenates them into the one
ordinary `password` form value. Authentication failures use the same generic
message for an unknown username, incorrect PIN, disabled voter, lockout, or
expired credential. Operational errors that require a different action, such
as a failed reCAPTCHA challenge, retain their specific message.

## Password policy and provisioning

Configure a numeric-compatible Keycloak password policy for realms that create
PIN credentials, for example `length(16) and maxLength(16)` for `4-4-4-4`.
Password policies are creation-time rules; the deferred LOGIN flow does not
re-apply them after Keycloak has validated an existing credential. Credential
imports that supply password hashes may bypass creation-time policy checks, so
the provisioning process must validate the expected PIN length and format.

Keep Keycloak brute-force protection enabled. The deferred LOGIN flow records
the attempted username so its failures participate in Keycloak's normal
failure counting and lockout behavior.

## Localization overrides

The theme supplies defaults for these message keys:

- `segmentedCredentialLabel`
- `segmentedCredentialHint`
- `segmentedCredentialError`
- `segmentedCredentialGroupLabel`
- `showSegmentedCredential`
- `hideSegmentedCredential`

Override them per locale using **Realm Settings → Localization** in Keycloak.
`segmentedCredentialGroupLabel` receives the one-based group number as `{0}`
and the number of groups as `{1}`. Realm localization overrides take precedence
over the theme defaults.

## Rollout checklist

1. Confirm the event realm uses the `sequent.voting-portal` login theme.
2. Confirm provisioned usernames and numeric PIN passwords match the selected
   layout.
3. Configure a PIN-compatible realm password policy.
4. If using registration-as-login, confirm the deferred authenticator uses
   `form-mode=LOGIN` and `password-required=true`.
5. Enable the two realm attributes and test a successful login, a failed login,
   leading zeroes, paste, show/hide, and lockout before opening voting.
