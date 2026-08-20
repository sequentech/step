---
id: user_profile_login_registration_fields
title: Configuring Login and Registration Fields
sidebar_position: 10
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The fields voters see on the **registration** page and on the **login** page are not hardcoded in
the Sequent theme. They are generated from the realm's **User Profile** configuration in Keycloak:
one User Profile attribute produces one form field, and the attribute's *annotations* decide what
that field looks like and how it behaves.

This means the same configuration drives both pages. An attribute annotated as a date picker
renders as a date picker at registration *and* on the login form when that attribute is used for
attribute-based login — there is no second place to configure it.

This page is the reference for the annotations the Sequent theme supports. For the step-by-step of
creating an attribute in the first place, see
[Adding User Attributes to Keycloak](../01-tutorials/99-admin_portal_tutorials_add-user-attributes-to-keycloak.md).

---

## Where the configuration lives

All of it is in the Keycloak Admin Console, per realm:

**Realm settings** → **User profile** → select or create an attribute.

Each attribute has three groups of settings that matter here:

| Setting group | Controls |
|---|---|
| **General** (name, display name, required) | The field's identity, its label, and whether it is mandatory |
| **Annotations** | The input widget: type, placeholder, helper text, length and range limits, option labels |
| **Validations** | Server-side rules, and the option list for `select` / radio / checkbox fields |

Attribute names must match the names used elsewhere (voter import files, the Admin Portal's Voters
tab, and the authenticator's **User attributes to match** list). Renaming an attribute in User
Profile does not rename it in existing user records.

---

## Which pages use which attributes

| Page | Attributes rendered |
|---|---|
| **Registration** (`register.ftl`) | Every attribute in the realm's User Profile that is in scope for registration, in the order User Profile lists them |
| **Login** (`login.ftl`), standard username + password | No User Profile attributes — the username and password fields only |
| **Login**, attribute-based (**Multi-Attribute + Password Form**) | Only the attributes listed in the authenticator's **User attributes to match** config |

Attribute-based login is set up separately — see
[Logging In Without a Username (Attribute + Password)](../01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md).
Once it is set up, the fields it shows are configured entirely through the annotations documented
below.

> An attribute listed in **User attributes to match** that does **not** exist in the realm's User
> Profile still works for matching, but it can only render as a plain text field with its raw
> attribute name as the label. Declare it in User Profile to get a proper label, a proper input
> type, and translations.

---

## Choosing the input type

The `inputType` annotation selects the widget. Set it under **Annotations** with Key `inputType`
(the console labels it **Input type**).

| `inputType` value | Renders as |
|---|---|
| *(not set)* | A plain text input |
| `text` | A plain text input |
| `textarea` | A multi-line text box |
| `select` | A dropdown, with an empty first option |
| `multiselect` | A multiple-selection list |
| `select-radiobuttons` | One radio button per option |
| `multiselect-checkboxes` | One checkbox per option |
| `html5-date` | A native date picker |
| `html5-tel` | A phone-number widget with country selector (see below) |
| `html5-email`, `html5-number`, `html5-url`, `html5-text`, … | The corresponding native HTML5 input |

Any `html5-` prefixed value becomes that HTML5 input type — `html5-number` produces a numeric
input, and so on.

**Date attributes must be stored as `YYYY-MM-DD`** (e.g. `1990-01-05`). This is the format the
native date picker submits, so voter import data must use it too, or matching at login will fail.

---

## Labels and translations

The **Display name** field is the label shown above the input. Two ways to set it:

- **Literal text** — type the label directly, e.g. `Date of birth`. The same text is shown in
  every language.
- **A translation key** — type `${dateOfBirth}`. The theme resolves it against the realm's
  translations, so each language shows its own label.

To provide the translations for a key, go to **Realm settings** → **Localization** → **Realm
overrides** and add the key (`dateOfBirth`) with a value per language.

For option labels inside a `select` / radio / checkbox field, see
[Option lists](#option-lists) below.

---

## Helper text

Two annotations put explanatory text around a field. Both accept literal text or a translation key
(`${...}`), and both render on the registration page and on the attribute-based login page.

| Annotation | Effect |
|---|---|
| `inputHelperTextBefore` | Text shown **above** the input, under the label |
| `inputHelperTextAfter` | Text shown **below** the input |

Typical use: `inputHelperTextBefore` = `Enter it exactly as printed on your ID card`,
`inputHelperTextAfter` = `Format: DD/MM/YYYY`.

Helper text is announced to screen readers when it changes, so it is the right place for
format hints — not the placeholder, which assistive technology may skip.

---

## Text and number field settings

These annotations apply to the plain and HTML5 inputs.

| Annotation | Effect |
|---|---|
| `inputTypePlaceholder` | Placeholder text shown inside an empty input |
| `inputTypePattern` | A regular expression the browser enforces before submitting |
| `inputTypeSize` | The visible width of the input, in characters |
| `inputTypeMinlength` / `inputTypeMaxlength` | Minimum / maximum number of characters |
| `inputTypeMin` / `inputTypeMax` | Minimum / maximum value (numbers) or date (date inputs) |
| `inputTypeStep` | Step increment for numeric and date inputs |

**Date inputs are capped at `9999-12-31` by default.** A native date input otherwise accepts years
of five or more digits, letting a voter enter something like `123456-01-01`. Setting your own
`inputTypeMax` (e.g. `2010-12-31` to require a minimum age) replaces that default cap.

For a text area, use `inputTypeCols` and `inputTypeRows` instead of `inputTypeSize`;
`inputTypeMaxlength` applies there too.

Browser-side settings like `inputTypePattern` are a usability aid, not a security control. Use
**Validations** for rules that must actually hold.

---

## Option lists

For `select`, `multiselect`, `select-radiobuttons` and `multiselect-checkboxes`, the options come
from a validator, not from an annotation:

1. Under **Validations**, click **Add validator** and choose `options`.
2. Add each option value (e.g. `M`, `F`).

The stored value is the option value itself. To show voters something friendlier, add one of:

| Annotation | Effect |
|---|---|
| `inputOptionLabels` | A map of option value → label, e.g. `{"M": "Male", "F": "Female"}`. Values may be translation keys. |
| `inputOptionLabelsI18nPrefix` | A prefix; each option is translated as `prefix.optionValue` (e.g. `sex.M`) |
| `inputOptionsFromValidation` | Take the option list from a *different* named validator instead of the default `options` one |

### Filtering one dropdown by another

`filterSelectAttribute` narrows a second dropdown based on what is chosen in the first. Set it on
the **controlling** attribute, with the **id of the dependent field** as its value — for example,
set `filterSelectAttribute` = `municipality` on a `province` attribute. When the voter picks a
province, only the municipality options whose value contains the selected province value stay
visible, and the first match is preselected.

This works by substring match on the option *values*, so the dependent field's option values must
embed the controlling value (e.g. province `08`, municipalities `08001`, `08002`).

Both the registration page and the attribute-based login page load the script this needs, so a
filtered dropdown behaves the same on either.

### Enabling and disabling other fields

On radio-button and checkbox attributes, two further annotations let one option control another
field:

| Annotation | Effect |
|---|---|
| `disableAttribute` | Ticking the option makes the named field read-only (and clears it); unticking restores it |
| `disableElement` | Ticking the option disables the named field entirely |

---

## Phone number fields

An attribute with `inputType` = `html5-tel` renders as an international phone-number widget: a
country selector with dial code, placeholder formatting for the selected country, and a guess at
the voter's country from their browser timezone.

The submitted value is the full number in international format, stored under the attribute's own
name. This applies to any `html5-tel` attribute, whatever it is called, and works identically on
the registration page and on the attribute-based login page.

---

## Required fields

An attribute's **Required field** toggle in User Profile marks it mandatory. On the registration
page a required attribute gets an asterisk next to its label and is enforced by the browser and
the server.

On the **attribute-based login page** the behavior depends on one authenticator setting:

| **Honor User Profile required attributes** | Behavior of the fields in **User attributes to match** |
|---|---|
| **Off** (default) | Every listed attribute is mandatory. No asterisks are shown, and all of them must match. |
| **On** | Each field follows its own User Profile **Required field** setting — required ones get the asterisk and must match; the rest may be left blank and matching proceeds on the attributes that were filled in. |

The setting is under the authenticator's **⚙ Config**, alongside the attribute list.

> ⚠️ **Turning this on widens who a login attempt can match.** If `dateOfBirth` and `nationalId`
> are both configured and `nationalId` is not required, a voter who fills in only the date of
> birth is matched against everyone born that day, instead of one person. That makes the password
> the only thing narrowing the candidate set. Only enable it when that trade-off is intended, and
> read the DoS and match-policy guidance in the
> [attribute login tutorial](../01-tutorials/101-admin_portal_tutorials_multi-attribute-password-login.md#denial-of-service-considerations).
>
> An all-blank submission always fails, regardless of this setting. An attribute that is not
> declared in User Profile at all stays mandatory.

---

## Hiding a field

| Annotation | Effect |
|---|---|
| `hidden` = `true` | The attribute is not rendered on the **registration** page |

Use this for attributes managed by import or by an administrator rather than filled in by the
voter. A hidden attribute is still stored, and still usable for attribute-based login.

There is a second, authenticator-level way to hide fields: the **Deferred Registration User
Creation** form action's **Hidden Profile Attributes** setting takes a comma-separated list of
attribute names to leave out of the registration form and to ignore even if User Profile marks
them required. Use it when the same User Profile attribute should be visible in some flows and not
others; use the `hidden` annotation when it should never be shown at registration.

> The `hidden` annotation applies to the registration page only. An attribute listed in an
> attribute-based login's **User attributes to match** is always rendered on the login form —
> the voter has to be able to type it for matching to work.

---

## Prefilled values from a login link

When a login link carries values for profile attributes (login hints), two settings decide what
happens to them.

First, the **Deferred Registration User Creation** form action's **Prefill Parameters Policy**
acts as the master switch:

- `IGNORE` (default) — login hints never prefill anything.
- `ACCEPT` — validated hints may prefill fields, subject to the per-attribute policy below.

Then, per attribute, the `loginHintPrefillPolicy` annotation:

| `loginHintPrefillPolicy` | Effect |
|---|---|
| *(not set)* or `EDITABLE` | The field is prefilled and the voter may change the value |
| `READ_ONLY` | The field is prefilled and locked; a submission with a changed value is rejected |
| `IGNORE` | This field is never prefilled from a login hint |

A locked value is still submitted — including for dropdowns, radio buttons and checkboxes, which
submit a hidden copy of it.

Some attributes are never prefillable regardless of these settings: password fields, attributes
annotated `hidden`, and attributes whose `html-attribute:` annotations or inline styles would make
the field invisible or uneditable. This prevents a link from silently setting a value the voter
cannot see or check.

---

## Quick reference

| Goal | Setting |
|---|---|
| Dropdown of fixed choices | `inputType` = `select` + an `options` validator |
| Date of birth picker | `inputType` = `html5-date`, values stored as `YYYY-MM-DD` |
| Phone number with country code | `inputType` = `html5-tel` |
| Format hint under the label | `inputHelperTextBefore` |
| Translated label | Display name = `${myAttribute}` + a Localization override |
| Translated option labels | `inputOptionLabelsI18nPrefix` = `myAttribute` |
| Cap a date at a maximum | `inputTypeMax` = `2010-12-31` |
| Narrow a dropdown by another one | `filterSelectAttribute` = *id of the dependent field* |
| Field stored but never shown at registration | `hidden` = `true` |
| Field locked to a value from a login link | `loginHintPrefillPolicy` = `READ_ONLY` |
| Optional field on attribute login | User Profile **Required field** off + **Honor User Profile required attributes** on |
