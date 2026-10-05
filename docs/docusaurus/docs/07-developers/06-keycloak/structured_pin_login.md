---
id: structured_pin_login
title: Structured PIN login
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

The structured PIN input is an optional presentation for a voter's ordinary
Keycloak password. The other login value remains the username. It does not add
a credential type, change how Keycloak stores or verifies credentials, or add a
date-of-birth field.

## Realm configuration

Set these Keycloak realm attributes on the election event realm:

| Attribute | Values | Default |
| --- | --- | --- |
| `credential-input-policy` | `standard` or `structured` | `standard` |
| `credential-input-pattern` | A structured digit pattern such as `dddd-dddd-dddd-dddd` | `dddd-dddd-dddd-dddd` |
| `credential-input-placeholder` | The character shown for each unassigned digit, such as `#` | `d` |

In the current pattern grammar, each `d` represents one required ASCII digit
and a hyphen separates editable groups. A pattern may contain 1–8 groups, each
group may contain 1–12 `d` tokens, and the combined credential length may not
exceed 64 digits. Other characters and future-reserved operators are rejected.
The placeholder must be one visible non-digit character other than `-` or `*`.

The application validates these values when realm attributes are saved. If
malformed configuration reaches Keycloak through another route, the theme
leaves the ordinary password field usable.

The attributes can be included in deployment realm configuration or entered in
the permission-gated **Keycloak realm attributes** JSON editor for the election
event. For example:

```json
{
  "credential-input-policy": "structured",
  "credential-input-pattern": "dddd-dddd-dddd-dddd",
  "credential-input-placeholder": "#"
}
```

Where the PIN box sits relative to the other fields, and the rest of the
User Profile settings that shape these pages, are described in [Configuring
Login and Registration
Fields](../../02-election_managers/02-reference/10-user-profile-login-registration-fields.md).

Removing `credential-input-policy`, or setting it to `standard`, restores the
ordinary password field. The setting is realm-wide; this implementation has no
client-level override.

Events configured for the earlier prototype must replace
`credential-input-policy=segmented-numeric` with `structured`, replace
`credential-segment-layout` with `credential-input-pattern`, convert the value
from group sizes to digit tokens (for example, `4-4-4-4` becomes
`dddd-dddd-dddd-dddd`), and remove the old layout attribute. There is
intentionally no compatibility alias.

## Input behavior

The pattern renders as one textbox with the visibility button inside its outer
border. Empty positions display the configured placeholder (`d` by default).
Entered positions display `*`, except for the most recently entered digit,
which remains visible for one second. The visibility button reveals or hides
every entered digit.

Left and Right select the previous or next group, while Home and End select the
first or last group. Typing after selecting a group replaces it, and completing
a group advances to the next one. Paste accepts ASCII digits with optional
hyphens or ASCII whitespace. Copy, cut, and drop are disabled so the displayed
mask cannot leak or desynchronize the submitted value.
Rejected paste is left unapplied and announced through the control's polite
status region.
Rejected browser replacement or autofill values use a separate, neutral format
message. The message is shown in the credential error area and announced by its
alert role, so both sighted voters and assistive-technology users receive the
same explanation. Submission remains blocked until the voter makes a valid PIN
edit, import, or deletion; editing only the username cannot submit a stale PIN.

The visible control has no form name. On submit, its digit model is copied into
the one ordinary `password` form value without separators. Incomplete input is
blocked in the browser and focuses the first incomplete group. Invalid input,
leading zeroes, and paste never change the Keycloak credential type.

## Accessibility

The structured control is designed and tested against WCAG 2.1 Level AA for
the component and both supported flows. It retains native textbox and button
semantics, associates the visible PIN label and hint with the textbox, exposes
required and invalid states, and announces group progress and validation
errors. Incomplete submission focuses the first incomplete group. Username and
PIN inputs expose the standard `username` and `current-password` purposes.

Every interaction is keyboard-operable. Tab order is PIN textbox, visibility
button, and then the remaining form controls; Left, Right, Home, End,
Backspace, Delete, typing, paste, reveal, and submission do not require a
pointer or keystroke timing. The field and visibility button have distinct
visible focus states, including in forced-colors mode.

Normal text and icons meet their applicable contrast thresholds. The authored
field border and focus/error states have at least 3:1 contrast against the
field background, while text has at least 4.5:1. The 44 CSS-pixel visibility
target exceeds WCAG 2.1 Level AA and also meets its Level AAA target-size
criterion. The control is tested within a 320 CSS-pixel layout (the WCAG 2.1
reflow width) and with the WCAG text-spacing overrides, without loss of content
or functionality.

## Supported login forms

The policy applies to both:

- the normal `sequent.voting-portal` Keycloak login page; and
- the deferred voter-registration form when its authenticator has
  `form-mode=LOGIN` and `password-required=true`.

Actual registration (`form-mode=REGISTRATION`) is unchanged: it continues to
show password confirmation, the strength indicator, and Keycloak's password
creation-policy validation. A deferred LOGIN form with
`password-required=false` also remains unchanged.

Authentication failures use the same generic message for an unknown username,
incorrect PIN, disabled voter, lockout, or expired credential. Operational
errors that require a different action, such as a failed reCAPTCHA challenge,
retain their specific message.
Early password failures perform Keycloak's configured dummy hash so user lookup,
disabled-user, empty-password, and temporary-lockout paths do not omit the
realm's password-hashing cost.

## Password policy and provisioning

Configure a numeric-compatible Keycloak password policy for realms that create
PIN credentials, for example `length(16) and maxLength(16)` for
`dddd-dddd-dddd-dddd`. Password policies are creation-time rules; the deferred
LOGIN flow does not re-apply them after Keycloak has validated an existing
credential. This applies to every deferred LOGIN form, whether its presentation
is `standard` or `structured`. Credential imports that supply password hashes may bypass
creation-time policy checks, so provisioning must validate the configured
length and format.

Keep Keycloak brute-force protection enabled. The deferred LOGIN flow records
the attempted username so its failures participate in Keycloak's normal
failure counting and lockout behavior.

## Localization overrides

The theme supplies defaults for these message keys:

- `structuredCredentialLabel`
- `structuredCredentialHint`
- `structuredCredentialError`
- `structuredCredentialGroupStatus`
- `structuredCredentialPasteError`
- `structuredCredentialFormatError`
- `showStructuredCredential`
- `hideStructuredCredential`

Override them per locale using **Realm Settings → Localization** in Keycloak.
`structuredCredentialGroupStatus` receives the one-based group number as `{0}`,
the number of groups as `{1}`, entered digits as `{2}`, and group size as `{3}`.
Realm localization overrides take precedence over theme defaults.

## Rollout checklist

1. Confirm the event realm uses the `sequent.voting-portal` login theme.
2. Confirm provisioned usernames and numeric PIN passwords match the selected
   pattern.
3. Configure a PIN-compatible realm password policy.
4. If using registration-as-login, confirm the deferred authenticator uses
   `form-mode=LOGIN` and `password-required=true`.
5. Enable the realm attributes and test keyboard navigation, leading
   zeroes, paste, timed masking, show/hide, generic failures, and lockout before
   opening voting.
