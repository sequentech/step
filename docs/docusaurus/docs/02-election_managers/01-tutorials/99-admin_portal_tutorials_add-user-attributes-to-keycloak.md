---
id: admin_portal_tutorials_add_user_attributes_to_keycloak
title: Adding User Attributes to Keycloak
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Overview

The system supports adding additional user attributes that will appear as new fields in the user data. Once configured, these attributes will appear in the Add or Edit actions in the Admin Portal's Voters tab.

**Note:** This configuration must be done via the Keycloak's admin console.

## Configuration Steps

1. Log in to Keycloak and select the realm of the election event you want to edit.
2. Navigate to **Realm settings** > **User profile** > **Create attribute**.
3. Give an attribute name, such as `sex` in the first example.
4. Set the display name as `${sex}` if you want to override the translations in **Localization** > **Realm overrides**.
5. Continue configuring Annotations and other parameters (see examples below).

## Supported Attribute Types

The following attribute types are supported:

### Sex

To enable sex selection:

1. In **Annotations** > **Add annotation**, set Key: `Input type`, Value: `select`
2. **Add Validator** > **Validator type**: `options` and add the desired options (e.g., M, F)
3. Optionally describe each option, so that whoever edits a voter does not have to interpret the
   stored values. In **Annotations** > **Add annotation**, set Key: `inputOptionLabels` and a JSON
   value mapping each option to its description, such as `{"M": "Male", "F": "Female"}`. A
   description written as `${sex_male}` is resolved through the Admin Portal's own localization
   overrides (see below) and falls back to a readable form of the key when there is no override.
   The `inputOptionLabelsI18nPrefix` annotation is honoured on the voter-facing login forms only;
   the Admin Portal ignores it, because it reads a different set of translations.

The dropdown in the Admin Portal shows the stored option followed by its description, for example
`M - Male`, so the value written to the voter stays visible.

### Birth Date

To show a date input field:

1. In **Annotations**, set Key: `Input type`, Value: `html5-date`
2. Add validation if desired.

### Checkboxes (Not Supported Yet)

1. In **Annotations** > **Add annotation**, set Key: `Input type`, Value: `multiselect-checkboxes`
2. **TODO:** Implementation pending

## Hiding an Attribute

An attribute that is carried on the voter but is not meant to be seen or edited can be marked
hidden: in **Annotations** > **Add annotation**, set Key: `hidden`, Value: `true`.

A hidden attribute is left off the voter-facing enrollment and login forms, and off the Admin
Portal's voter list and its create and edit forms — it is not shown as a column, is not offered in
the columns selector or the filters, and is not shown as a field.

Hiding an attribute does not remove or alter it. Its value is still stored, still carried through
when a voter is edited, and still included when voters are exported.

Do not mark an attribute both hidden and required: creating a voter through the Admin Portal would
then be impossible, since the field it insists on is one the form does not show.

Note this is different from setting the `Input type` annotation to `hidden`, which is Keycloak's own
way of rendering an attribute as a hidden input on a form rather than keeping it off the form.

## Limiting the Number of Characters

To bound how much text an attribute accepts, **Add Validator** > **Validator type**: `length`, and
set a minimum, a maximum, or both. Keycloak enforces the bounds when the record is saved.

The Admin Portal states the bounds under the field, so they are known before they are broken, and
checks the value when the field is left. A value that breaks a bound marks the field and says which
bound it broke, and the voter cannot be saved until it is corrected.

A field with a maximum also stops accepting characters once it is reached, so the maximum cannot be
exceeded by typing. That count is of the characters as typed, so a value padded with spaces can stop
being accepted a little before the validator would object to it. Note that pasting a longer value into such a field keeps only what fits, without
warning, and that a stored value already longer than the maximum can only be shortened, never
extended — in that case the field reports the value as too long until it is brought within the
bound. A minimum cannot be applied while typing at all, so it is only checked when the field is
left.

By default Keycloak measures the value with leading and trailing spaces removed, and the Admin
Portal measures it the same way. Setting the validator's `trim-disabled` option changes both.

A bound that is somehow reached anyway — a value written by an import, or an attribute the form does
not show — is refused on save and reported naming each field and the bound it broke. Note that on
Datafix election events the save is carried out by a background task, so its refusal is reported
through that task rather than in the form.

## Localization Overrides

To customize the display label of a user attribute, add a translation override via the Admin Portal under **Settings** > **Localization** > **Add**.

User attribute keys must use the prefix `usersAndRolesScreen.users.fields.` followed by the attribute name.

**Example:** to display the `personal_administrative_number` attribute as "PAN", add the following to the tenant's localization settings:

Key: `usersAndRolesScreen.users.fields.personal_administrative_number`
Value: `PAN`

This works for any attribute name, including custom ones not present in the default translations.
