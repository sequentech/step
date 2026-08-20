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

| Setting group | Controls |
|---|---|
| **General** (name, display name, required) | Field identity, label, and whether it is mandatory |
| **Annotations** | Input type, placeholder, helper text, limits, option labels |
| **Validations** | Server-side rules, and the option list for `select` / radio / checkbox fields |

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

Attribute-based login is set up in
[Logging In Without a Username (Attribute + Password)](../01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md).

An attribute in **User attributes to match** that is not declared in User Profile renders as a
plain text field labelled with its raw attribute name, and is always mandatory to match.

> ⚠️ **Match attributes must hold a single value.** Matching compares one value per attribute. A
> `multiselect` or `multiselect-checkboxes` attribute used for matching has everything after its
> first selected value ignored.

---

## Input types

Set the `inputType` annotation (**Input type** in the console).

| `inputType` | Renders as |
|---|---|
| *(not set)* or `text` | Plain text input |
| `textarea` | Multi-line text box |
| `select` | Dropdown with an empty first option |
| `multiselect` | Multiple-selection list |
| `select-radiobuttons` | One radio button per option |
| `multiselect-checkboxes` | One checkbox per option |
| `html5-date` | Native date picker |
| `html5-tel` | Phone-number widget with country selector |
| `html5-email`, `html5-number`, `html5-url`, … | The corresponding native HTML5 input |

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

| Annotation | Effect |
|---|---|
| `inputHelperTextBefore` | Text above the input, under the label |
| `inputHelperTextAfter` | Text below the input |

Both accept literal text or a translation key, and both render on the registration page and the
attribute-based login page.

---

## Text and number settings

| Annotation | Effect |
|---|---|
| `inputTypePlaceholder` | Placeholder text |
| `inputTypePattern` | Regular expression enforced by the browser |
| `inputTypeSize` | Visible width in characters |
| `inputTypeMinlength` / `inputTypeMaxlength` | Minimum / maximum characters |
| `inputTypeMin` / `inputTypeMax` | Minimum / maximum value or date |
| `inputTypeStep` | Step increment for numeric and date inputs |
| `inputTypeCols` / `inputTypeRows` | Text area dimensions |

Date inputs are capped at `9999-12-31` unless `inputTypeMax` sets another maximum. Browser-side
settings are a usability aid; use **Validations** for rules that must be enforced.

---

## Option lists

For `select`, `multiselect`, `select-radiobuttons` and `multiselect-checkboxes`, options come from
a validator:

1. Under **Validations**, **Add validator** → `options`.
2. Add each option value (e.g. `M`, `F`).

The option value is what gets stored. To display something else:

| Annotation | Effect |
|---|---|
| `inputOptionLabels` | Map of option value → label, e.g. `{"M": "Male", "F": "Female"}`. Values may be translation keys. |
| `inputOptionLabelsI18nPrefix` | Each option is translated as `prefix.optionValue` (e.g. `sex.M`) |
| `inputOptionsFromValidation` | Take options from a different named validator |

### Dependent dropdowns

`filterSelectAttribute`, set on the controlling attribute with the dependent field's id as its
value, narrows the dependent dropdown to options whose value contains the selected value, and
preselects the first match. Option values must embed the controlling value (province `08`,
municipalities `08001`, `08002`).

### Enabling and disabling other fields

On radio-button and checkbox attributes:

| Annotation | Effect |
|---|---|
| `disableAttribute` | Ticking the option makes the named field read-only and clears it |
| `disableElement` | Ticking the option disables the named field |

---

## Other annotations

| Annotation | Effect |
|---|---|
| `default` | Initial value. Registration only; ignored on the attribute-based login form. |
| `confirm` | Renders a second input for the same attribute. The value is the confirmation field's label. |
| `html-attribute:<name>` | Sets an arbitrary HTML attribute, e.g. `html-attribute:autocomplete` = `off`. Applies to text and HTML5 inputs only — dropdowns, radio buttons, checkboxes and text areas ignore it. On the attribute-based login page it overrides the theme's own `autocomplete`. |
| `hidden` = `true` | The attribute is not rendered on the registration page. It is still stored and still usable for matching. |

The **Deferred Registration User Creation** form action's **Hidden Profile Attributes** setting
hides a comma-separated list of attributes from the registration form and ignores them if User
Profile marks them required.

---

## Phone number fields

`inputType` = `html5-tel` renders an international phone-number widget: country selector with dial
code, per-country placeholder formatting, and an initial country guessed from the browser
timezone. The submitted value is the full international number, stored under the attribute's own
name.

---

## Required fields

An attribute's **Required field** toggle marks it mandatory. On the registration page a required
attribute gets an asterisk and is enforced by the browser and the server.

On the attribute-based login page, the authenticator's **Honor User Profile required attributes**
setting governs the fields in **User attributes to match**:

| Setting | Behavior |
|---|---|
| **Off** (default) | Every listed attribute is mandatory. No asterisks are shown. |
| **On** | Each field follows its own **Required field** setting. Non-required fields may be left blank, and matching proceeds on the attributes that were filled in. |

An all-blank submission always fails. An attribute not declared in User Profile stays mandatory.

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

| Value | Effect |
|---|---|
| *(not set)* or `EDITABLE` | Prefilled, and the voter may change it |
| `READ_ONLY` | Prefilled and locked; a changed value is rejected |
| `IGNORE` | Never prefilled |

A locked value is still submitted, including for dropdowns, radio buttons and checkboxes.
Password fields, attributes annotated `hidden`, and attributes rendered invisible or uneditable by
`html-attribute:` annotations or inline styles are never prefilled.

On the login page the username field is governed instead by the realm attribute
`loginHintUsernamePolicy` (`EDITABLE` or `READ_ONLY`).

---

## The password and PIN field

The password field is not a User Profile attribute; both pages add it themselves. These
annotations apply to the registration form, where the password box sits underneath a chosen
attribute:

| Annotation | Effect |
|---|---|
| `showPasswordAfterThis` | Position of the password box. Defaults to after `username` (or `email` when used as username). Set `false` on that attribute to suppress it, or `true` on another attribute to move it. |
| `passwordHelperTextBefore` / `passwordHelperTextAfter` | Text above / below the password box |
| `passwordStrengthBar` | Password-strength meter. Registration only. |

### Structured PIN input

Renders the password box as fixed-length digit groups with numeric keyboards, group navigation and
paste handling. It remains the voter's ordinary Keycloak password. Configured with realm
attributes, in the election event's **Keycloak realm attributes** editor:

| Realm attribute | Values | Default |
|---|---|---|
| `credential-input-policy` | `standard` or `structured` | `standard` |
| `credential-input-pattern` | Digit pattern, e.g. `dddd-dddd-dddd-dddd` | `dddd-dddd-dddd-dddd` |
| `credential-input-placeholder` | Character shown for each empty digit, e.g. `#` | `d` |

It applies to the login page, and to the registration form in login mode with a password field.
For the pattern grammar and limits, see
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
