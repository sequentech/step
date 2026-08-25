---
id: user_profile_login_registration_fields
title: Configuring Login and Registration Fields
sidebar_position: 10
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The fields on the registration page and on the attribute-based login page are generated from the
realm's **User Profile** configuration in Keycloak. One User Profile attribute produces one form
field; the attribute's annotations decide how that field is rendered. The same configuration
drives both pages.

To create an attribute, see
[Adding User Attributes to Keycloak](../01-tutorials/99-admin_portal_tutorials_add-user-attributes-to-keycloak.md).

---

## Where the configuration lives

**Realm settings** → **User profile** → select or create an attribute.

Settings that apply to the whole page rather than to one field are **realm attributes**, edited in
the Admin Portal under the election event's **Keycloak realm attributes**. That editor lists the
attributes a realm has, not the ones it supports, so each environment's default event realm
configuration should carry them at their defaults — otherwise an election manager has no way to
discover them:

| Realm attribute | Default |
|---|---|
| `credential-field-position` | `LAST` |
| `credential-input-policy` | `standard` |
| `credential-input-pattern` | `dddd-dddd-dddd-dddd` |
| `credential-input-placeholder` | `d` |
| `login-validation-policy` | `BROWSER` |
| `voter-certificate-policy` | `disabled` |

An attribute left unset behaves as its default, so seeding them changes nothing except what the
editor shows.

| Setting group | Controls |
|---|---|
| **General** (name, display name, required) | Field identity, label, and whether it is mandatory |
| **Annotations** | Input type, placeholder, helper text, limits, option labels |
| **Validations** | Server-side rules, and the option list for choice fields |

Attribute names must match those used in voter import files, the Admin Portal's Voters tab, and
the authenticator's **User attributes to match** list. Renaming an attribute does not rename it in
existing user records.

---

## Which pages use which attributes

| Page | Attributes rendered |
|---|---|
| **Registration** | Every User Profile attribute in scope for registration, in the order User Profile lists them |
| **Login**, standard username + password | None — username and password only |
| **Login**, attribute-based (**Multi-Attribute + Password Form**) | Only the attributes in the authenticator's **User attributes to match** |

Setting up attribute-based login, including its matching, ambiguity and abuse-control settings, is
covered in
[Logging In Without a Username (Attribute + Password)](../01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md).

An attribute in **User attributes to match** that is not declared in User Profile renders as a
plain text field. Its attribute name is used as a message key and falls back to the raw name when
no translation exists. It is always mandatory to match.

Matching compares one value per attribute. On the attribute-based login page, a `multiselect`
declaration therefore renders as a single-selection list and `multiselect-checkboxes` renders as
radio buttons. Registration keeps the configured multi-value control.

---

## Input types

Set the `inputType` annotation (**Input type** in the console).

| `inputType` | Renders as | Where it applies |
|---|---|---|
| *(not set)* or `text` | Plain text input | Both |
| `textarea` | Multi-line text box | Both |
| `select` | Dropdown with an empty first option | Both |
| `multiselect` | Multiple-selection list | Registration only |
| `select-radiobuttons` | One radio button per option | Both |
| `multiselect-checkboxes` | One checkbox per option | Registration only |
| `html5-date` | Native date picker | Both |
| `html5-tel` | Phone-number widget with country selector | Both |
| `html5-email`, `html5-number`, `html5-url`, … | The corresponding native HTML5 input | Both |

Any `html5-` prefixed value becomes that HTML5 input type.

**Date values must be stored as `YYYY-MM-DD`** (e.g. `1990-01-05`). Matching at login fails if
imported data uses another format.

---

## Labels and translations

**Display name** is the label shown above the input:

- **Literal text** (`Date of birth`) — the same in every language.
- **A translation key** (`${dateOfBirth}`) — resolved per language.

Define translation keys under **Realm settings** → **Localization** → **Realm overrides**.

---

## Helper text

| Annotation | Effect | Where it applies |
|---|---|---|
| `inputHelperTextBefore` | Text above the input, under the label | Both |
| `inputHelperTextAfter` | Text below the input | Both |

Both accept literal text or a translation key, and both render on the registration page and the
attribute-based login page.

---

## Text and number settings

| Annotation | Effect | Where it applies |
|---|---|---|
| `inputTypePlaceholder` | Placeholder text | Both |
| `inputTypePattern` | Regular expression enforced by the browser | Both |
| `inputTypeSize` | Visible width in characters | Both |
| `inputTypeMinlength` / `inputTypeMaxlength` | Minimum / maximum characters | Both |
| `inputTypeMin` / `inputTypeMax` | Minimum / maximum value or date | Both |
| `inputTypeStep` | Step increment for numeric and date inputs | Both |
| `inputTypeCols` / `inputTypeRows` | Text area dimensions | Both |

Date inputs are capped at `9999-12-31` unless `inputTypeMax` sets another maximum. Browser-side
settings are a usability aid; use **Validations** for rules that must be enforced.

---

## Option lists

For `select`, `multiselect`, `select-radiobuttons` and `multiselect-checkboxes`, options come from
a validator:

1. Under **Validations**, **Add validator** → `options`.
2. Add each option value (e.g. `M`, `F`).

The option value is what gets stored. To display something else:

| Annotation | Effect | Where it applies |
|---|---|---|
| `inputOptionLabels` | Map of option value → label, e.g. `{"M": "Male", "F": "Female"}`. Values may be translation keys. | Both |
| `inputOptionLabelsI18nPrefix` | Each option is translated as `prefix.optionValue` (e.g. `sex.M`) | Both |
| `inputOptionsFromValidation` | Take options from a different named validator | Both |

### Dependent dropdowns

`filterSelectAttribute`, set on the controlling attribute with the dependent field's id as its
value, narrows the dependent dropdown to options whose value contains the selected value, and
preselects the first match. Option values must embed the controlling value (province `08`,
municipalities `08001`, `08002`). On attribute-based login, the dependent field must also be in
**User attributes to match**.

### Enabling and disabling other fields

On radio-button and checkbox attributes:

| Annotation | Effect | Where it applies |
|---|---|---|
| `disableAttribute` | Ticking the option makes the named field read-only and clears it | Registration only |
| `disableElement` | Ticking the option disables the named field | Registration only |

---

## Other annotations

| Annotation | Effect | Where it applies |
|---|---|---|
| `default` | Initial value. Registration only; ignored on the attribute-based login form. | Registration only |
| `confirm` | Renders a second input for the same attribute. The value is the confirmation field's label. | Registration only |
| `html-attribute:<name>` | Sets an arbitrary HTML attribute, e.g. `html-attribute:autocomplete` = `bday`. Applies to text and HTML5 inputs only — dropdowns, radio buttons, checkboxes and text areas ignore it. | Registration only |
| `hidden` = `true` | The attribute is not rendered on the registration page. It is still stored and still usable for matching. | Registration only |

The **Deferred Registration User Creation** form action's **Hidden Profile Attributes** setting
hides a comma-separated list of attributes from the registration form and ignores them if User
Profile marks them required.

---

## Autofill and the login page

The attribute-based login page sets `autocomplete="off"` on every match field and ignores User
Profile `html-attribute:autocomplete` annotations. Failed match values are also not placed back in
the page. These are deliberate shared-device protections for fields such as date of birth,
national ID and phone number. Browsers may still apply their own autofill heuristics, so
`autocomplete="off"` is a request rather than a privacy guarantee.

---

## Phone number fields

`inputType` = `html5-tel` renders an international phone-number widget: country selector with dial
code, per-country placeholder formatting, and an initial country guessed from the browser
timezone. The submitted value is the full international number, stored under the attribute's own
name. If `inputTypePattern` is configured, the browser checks it against that normalized full
number rather than the national-format value visible beside the separate dial code.

---

## Required fields

An attribute's **Required field** toggle marks it mandatory. On the registration page a required
attribute gets an asterisk and is enforced by the browser and the server.

On the attribute-based login page, the authenticator's **Honor User Profile required attributes**
setting governs the fields in **User attributes to match**:

| Setting | Behavior |
|---|---|
| **Off** (default) | Every listed attribute is mandatory |
| **On** | Each field follows its own **Required field** setting. Non-required fields may be left blank, and matching proceeds on the attributes that were filled in |

An all-blank submission always fails. An attribute not declared in User Profile stays mandatory.

Asterisks and the "Required fields" notice appear only when at least one field is optional, since
marking every field would distinguish nothing. When they appear, the password or PIN is marked too.

> ⚠️ Enabling this widens who a login attempt can match: leaving `nationalId` blank matches every
> voter with the submitted `dateOfBirth`, leaving the password as the only narrowing factor. See
> [Denial-of-Service Considerations](../01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md#denial-of-service-considerations).

---

## Prefilled values from a login link

Login hints are governed by two settings. The **Deferred Registration User Creation** form
action's **Prefill Parameters Policy** is the master switch:

- `IGNORE` (default) — login hints never prefill.
- `ACCEPT` — validated hints may prefill, subject to the per-attribute policy.

Per attribute, the `loginHintPrefillPolicy` annotation:

| Value | Effect | Where it applies |
|---|---|---|
| *(not set)* or `EDITABLE` | Prefilled, and the voter may change it | Registration only |
| `READ_ONLY` | Prefilled and locked; a changed value is rejected | Registration only |
| `IGNORE` | Never prefilled | Registration only |

A locked value is still submitted, including for dropdowns, radio buttons and checkboxes.
Password fields, attributes annotated `hidden`, and attributes rendered invisible or uneditable by
`html-attribute:` annotations or inline styles are never prefilled.

On the login page the username field is governed instead by the realm attribute
`loginHintUsernamePolicy` (`EDITABLE` or `READ_ONLY`).

---

## Validation

Annotations and validators are separate mechanisms and are enforced in different places.

| Declared as | What it does | Register form | Login form |
|---|---|---|---|
| Annotation (`inputTypePattern`, `inputTypeMaxlength`, `inputTypeMin`/`Max`) | Rendered as an HTML attribute, checked by the browser before submitting | Applies | Applies |
| Validator (`length`, `pattern`, `email`, `options`, …) | Checked on the server after submitting | Applies, with a message beside the field | **Ignored** |

The attribute-based login page never runs User Profile validators: it matches the submitted values
against voter records and answers with a single generic message. A validator declared on a match
attribute has no effect there — it neither constrains the input nor rejects it.

Only the annotation limits typing. A `length` validator with `max: 9` lets a voter type any number
of characters and rejects the value afterwards; `inputTypeMaxlength` = `9` stops the input at nine.
To limit typing *and* enforce the limit, declare both.

The `options` validator is the exception: both forms read it to build the choices for `select`,
radio and checkbox fields, so it is load-bearing for rendering on both.

### Who reports invalid formats on the login page

| `login-validation-policy` | Effect |
|---|---|
| `BROWSER` (default) | The browser checks the constraint attributes and blocks submission, showing its own message beside the offending field |
| `SERVER_ONLY` | The form carries `novalidate`; submission always reaches the authenticator, which answers with the generic message |

Browser messages come from the browser's language, not the realm's, and only one field is reported
at a time. `SERVER_ONLY` removes them. The annotations stay in place either way: required fields
are still announced by assistive technology, typing limits still apply, and the date picker is
still bounded.

The structured PIN validates its own format in either mode, in the page's language, beside the
field.

---

## The password and PIN field

The password field is not a User Profile attribute; both pages add it themselves. These
annotations apply to the registration form, where the password box sits underneath a chosen
attribute:

| Annotation | Effect | Where it applies |
|---|---|---|
| `showPasswordAfterThis` | Position of the password box. Defaults to after `username` (or `email` when used as username). Set `false` on that attribute to suppress it, or `true` on another attribute to move it. Always wins over `credential-field-position`. | Registration only |
| `passwordHelperTextBefore` / `passwordHelperTextAfter` | Text above / below the password box | Registration only |
| `passwordStrengthBar` | Password-strength meter. Registration only. | Registration only |

### Putting the credential first

By default the password or PIN box sits after the identity fields. Set the realm attribute
`credential-field-position` to `FIRST` to put it above them, with the page's initial focus on it —
useful where a PIN from a voter letter is the primary thing being entered.

| Value | Effect |
|---|---|
| `LAST` (default) | Credential after the fields, focus on the first field |
| `FIRST` | Credential before the fields, focus on the credential |

It applies to the attribute-based login page and to the registration form, not to the ordinary
username and password login page. On the registration form an attribute declaring
`showPasswordAfterThis` keeps its placement and this setting is ignored.

### Structured PIN input

Renders the password box as fixed-length digit groups. It remains the voter's ordinary Keycloak
password, and applies to the login page and to the registration form in login mode with a password
field. Set `credential-input-policy` to `structured` to enable it; the pattern, placeholder, input
behaviour and rollout steps are covered in
[Structured PIN login](../../07-developers/06-keycloak/structured_pin_login.md).

---

## Other features that change these pages

### Login page

| Feature | Configured with | Effect |
|---|---|---|
| [Attribute-based login](../01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md) | **Multi-Attribute + Password Form** authenticator | Replaces the username field with User Profile attribute fields |
| [Structured PIN](../../07-developers/06-keycloak/structured_pin_login.md) | `credential-input-*` realm attributes | Password box becomes fixed-length digit groups |
| Locked username | Realm attribute `loginHintUsernamePolicy` = `READ_ONLY` | Username is prefilled and read-only |
| [Digital certificate login](../../07-developers/06-keycloak/x509_client_cert_architecture.md) | Realm attribute `voter-certificate-policy` = `enabled` | Shows the **digital-certificates** provider button; hidden while `disabled` |
| Other identity providers | Keycloak **Identity providers** | Each enabled provider adds a button below the form |
| [OID4VP (digital wallet)](../../07-developers/06-keycloak/oid4vp_testing_guide.md) | OID4VP identity provider | Uses its own QR / wallet page |
| [IdP-initiated SSO](../../07-developers/06-keycloak/idp_initiated_sso_design_implementation.md) | SAML/OIDC identity provider | Login page is skipped |
| Remember me | Realm setting **Remember me** | Adds the checkbox |
| Forgot password | Realm setting **Forgot password** | Adds the reset link |
| Registration link | Realm setting **User registration** | Adds the register link |
| Login with email | Realm settings **Login with email** / **Email as username** | Changes the username field label |
| reCAPTCHA | Keycloak reCAPTCHA authenticator | Invisible reCAPTCHA on submit |

### Registration page

| Feature | Configured with | Effect |
|---|---|---|
| Login mode | **Deferred Registration User Creation**, **Form Mode** = `LOGIN` | Renders as a login form: no password confirmation, no strength bar, no back-to-login link |
| Password field | Same form action, **Password Required** | Whether a password box is rendered |
| Hidden attributes | Same form action, **Hidden Profile Attributes** | Removes the listed attributes |
| Login-link prefilling | Same form action, **Prefill Parameters Policy** | See [Prefilled values](#prefilled-values-from-a-login-link) |
| Terms and conditions | Keycloak **Terms and Conditions** required action | Adds the acceptance checkbox |
| reCAPTCHA | Keycloak reCAPTCHA authenticator | Adds the reCAPTCHA widget |
| Identity providers | Keycloak **Identity providers** | Provider buttons shown in login mode only |

### Both pages

| Feature | Configured with | Effect |
|---|---|---|
| [Languages](./06-Languages.md) | Realm attributes `language_detection_policy`, `forced_language_code`, and enabled locales | Initial language and the language selector |
| Labels and messages | **Realm settings** → **Localization** → **Realm overrides** | Any label or message, per language |
| Branding | Election event theme settings | Logo, colors and layout |
