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

For the full list of annotations and what each one does to the rendered field, see
[Configuring Login and Registration Fields](../02-reference/10-user-profile-login-registration-fields.md).

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

### Checkboxes

1. In **Annotations** > **Add annotation**, set Key: `Input type`, Value: `multiselect-checkboxes`
2. Add an `inputOptionLabels` annotation containing the option values and labels.

## Ordering and Attribute Groups

The Admin Portal renders supported attributes in the order shown under **Realm settings** > **User profile**. Drag attributes into the required order in Keycloak; no tenant CSS is needed to control the semantic or keyboard order.

To organize related fields:

1. In **Realm settings** > **User profile**, create an Attribute Group.
2. Set the group's name, display header, and optional display description.
3. Assign attributes to the group while editing each attribute.

Only groups containing a field visible to the current administrator are rendered. Consecutive attributes in one group share a section. If the same group occurs in separate parts of the profile, each consecutive run is rendered as a separate section so the configured field order is preserved. Attributes without a group are rendered in an unlabelled section.

Group headings use the Keycloak display header and fall back to the group's name. Values written as Keycloak expressions, such as `${personal_details}`, are displayed as readable labels. Group descriptions are displayed below the heading.

## Customizing the Voter Editor with Tenant CSS

Tenant CSS configured under **Settings** > **Look and Feel** can target the following supported selectors:

- `.voter-editor` and `[data-mode="create"|"edit"]`: editor root and mode.
- `.voter-editor__groups`: collection of attribute sections.
- `.voter-attribute-group` and `[data-group-name="..."]`: one consecutive Attribute Group run. The data value is the raw Keycloak group name.
- `.voter-attribute-group__legend`: group heading.
- `.voter-attribute-group__description`: group description.
- `.voter-attribute-group__grid`: fields within a group.
- `.voter-field`, `[data-field-name="..."]`, `[data-input-type="..."]`, and `[data-required="true"|"false"]`: field wrapper and metadata. Field names are the canonical raw Keycloak names.

Step-owned fields also expose stable names: `enabled`, `area`, `password`, `confirm_password`, and `password_temporary`.

The following complete example creates lightly tinted, bordered cards for named Attribute Groups,
uses a responsive two-column grid, keeps form controls white, and lets the postal address span both
columns. The empty-name exclusion leaves the unlabelled section containing Step-owned fields
unboxed.

```css
.voter-editor .voter-attribute-group[data-group-name]:not([data-group-name=""]) {
  min-width: 0;
  margin: 0 0 24px;
  border: 1px solid rgba(15, 5, 76, 0.16);
  border-radius: 12px;
  padding: 20px 22px 24px;
  background-color: rgba(15, 5, 76, 0.02);
}

.voter-editor .voter-attribute-group__legend {
  padding: 0 8px;
  color: #0f054c;
  font-size: 16px;
  font-weight: 700;
  line-height: 1.4;
}

.voter-editor .voter-attribute-group__description {
  margin-bottom: 24px;
  color: #666670;
}

.voter-editor .voter-attribute-group__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  column-gap: 16px;
  row-gap: 20px;
}

.voter-editor [data-field-name="postal_address"] {
  grid-column: 1 / -1;
}

@media (max-width: 700px) {
  .voter-editor .voter-attribute-group[data-group-name]:not([data-group-name=""]) {
    padding: 16px;
  }

  .voter-editor .voter-attribute-group__grid {
    grid-template-columns: 1fr;
  }
}

.voter-editor .voter-attribute-group[data-group-name]:not([data-group-name=""]) .MuiInputBase-root {
  background-color: #fff;
}
```

Do not target generated MUI or Emotion class names such as `.css-abc123`; they are implementation
details and can change during upgrades. `.MuiInputBase-root` in the example is a stable MUI component
class, scoped beneath the stable voter group selector. CSS `order` can change only the visual order,
not keyboard or screen-reader order, so configure field order in Keycloak instead.

## Hiding an Attribute

An attribute that is carried on the voter but is not meant to be seen or edited can be marked
hidden: in **Annotations** > **Add annotation**, set Key: `hidden`, Value: `true`.

The value must be exactly `true`. This is matched literally, so `TRUE` or a value with spaces around
it does not hide anything — the voter-facing forms read the annotation the same way, and matching it
any more loosely here would hide an attribute from administrators while voters still saw it.

A hidden attribute is left off the voter-facing enrollment and login forms, and off the Admin
Portal's voter list and its create and edit forms — it is not shown as a column, is not offered in
the columns selector or the filters, and is not shown as a field. The Approvals screens are not
affected and still show it.

Hiding an attribute does not remove or alter it. Its value is still stored, still carried through
when a voter is edited, and still included when voters are exported.

Hiding is therefore about keeping a form readable, not about restricting access: the value is still
sent to the browser and still appears in exports. To control who may read or write an attribute, set
its permissions in **Realm settings** > **User profile** instead.

Do not mark an attribute both hidden and required: creating a voter through the Admin Portal would
then be impossible, since the field it insists on is one the form does not show.

Note this is different from setting the `Input type` annotation to `hidden`, which is Keycloak's own
way of rendering an attribute as a hidden input on a form rather than keeping it off the form.

## Protecting a Voter Attribute as Secret

An election-event attribute can be stored as an encrypted **secret voter attribute**. Use this for
data that administrators or voter-level outputs need, but which must not be exposed in ordinary
voter lists or default exports. An eligible custom secret can also serve as the credential for
the explicitly configured **Multi-Attribute + Password** authenticators.

In **Annotations** > **Add annotation**, set:

| Key | Value |
|---|---|
| `sequent.secret` | `true` |

The value may also be the JSON boolean `true` when the profile is configured through an API. Secret
attributes are supported only in election-event realms.

Before enabling the annotation:

- Restrict the User Profile attribute's view and edit permissions to administrators. Do not use a
  secret attribute in voter registration, identity-attribute login matching, token mappers, reconciliation, filters,
  sorting, uniqueness checks, or voter self-service. Keycloak stores ciphertext, so those features
  cannot use its original value.
  The supported authentication exception is
  [encrypted-attribute credential verification](./101-admin_portal_tutorials_multi-attribute-password-login.md#optional-use-an-encrypted-voter-attribute-as-the-credential):
  the extension decrypts it server-side and verifies the existing password input. This requires
  `SECRET_ATTRIBUTE` policy and the matching master key in Keycloak; the annotation alone does
  not change login behavior.
- Do not configure a Keycloak value validator on the attribute. The
  `person-name-prohibited-characters` validator is the only supported exception because it accepts
  the encrypted envelope. A **Required field** rule is supported.
- Keep each plaintext value at or below 150 bytes. The encrypted envelope must fit the
  255-character Keycloak attribute value column that voter imports and listings use.
- Protect or migrate any existing plaintext values before enabling the annotation. Existing values
  are not encrypted retroactively, and Step refuses to reveal an unencrypted value as a fallback.

Only custom voter attributes can be secret. The following identity and operational attributes
cannot:

- `username`, `email`, `first_name`, `last_name`, `dateOfBirth`, `area-id`, and `tenant-id`
- `authorized-election-ids`, `authorized-to-election-alias`, and `permission_labels`
- `vote-weight`, `voted-channel`, and `disable-comment`
- `sequent.read-only.id-card-number-validated` and `sequent.read-only.mobile-number`

Step reads the secret-attribute configuration through a short cache, so a change to the
annotation can take up to 30 seconds to be reflected in the voter list and editor. If the
configuration is invalid (a forbidden attribute or a value validator on a secret attribute), voter
lists still redact the attribute, but revealing, editing, importing, exporting and voter-level
outputs refuse to use it until the profile is corrected.

Secret and hidden are different classifications. A hidden attribute is omitted from the editor but
its plaintext can still reach the browser and ordinary exports. A secret attribute is encrypted at
rest, removed from ordinary list/filter/export paths, and represented by a masked value in the
voter editor.

Authorized operators may explicitly include decrypted secrets in
[voter CSV and password-protected event exports](./20-admin_portal_tutorials_export-data.md).
Anyone who can reveal or export a secret used for login can authenticate as that voter; grant
secret-read permission accordingly. CSV imports accept plaintext and encrypt it for storage.

Combining `hidden=true` with `sequent.secret=true` keeps the field out of the editor, but it remains
available to authorized secret exports and explicitly declared communication templates.

Users need separate permissions to reveal or modify secret values. See
[Permissions](../02-reference/user-manual/users-and-roles/users-and-roles_permissions.md) and
[Create Voters](./07-admin_portal_tutorials_create-voters.md#secret-voter-fields).

## Limiting the Number of Characters

To bound how much text an attribute accepts, **Add Validator** > **Validator type**: `length`, and
set a minimum, a maximum, or both. Keycloak enforces the bounds when the record is saved.

The Admin Portal states the bounds under the field, so they are known before they are broken, and
checks the value when the field is left. A value that breaks a bound marks the field and says which
bound it broke, and the voter cannot be saved until it is corrected.

Bounds nobody types into are left unstated, to keep the form readable: a maximum in the hundreds,
which Keycloak's own base attributes carry as scaffolding, and a minimum of one, which says only
that the value is present. They are still checked, and still reported when broken.

A field with a maximum also stops accepting characters once it is reached, so the maximum cannot be
exceeded by typing. That count is of the characters as typed, so a value padded with spaces can
stop being accepted a little before the validator would object to it. Note that pasting a longer
value into such a field keeps only what fits, without warning, and that a stored value already
longer than the maximum can only be shortened, never extended — in that case the field reports the
value as too long until it is brought within the bound. A minimum cannot be applied while typing at
all, so it is only checked when the field is left.

By default Keycloak measures the value with leading and trailing spaces removed, and the Admin
Portal measures it the same way. Setting the validator's `trim-disabled` option changes both.

A bound that is somehow reached anyway — a value written by an import, or an attribute the form
does not show — is refused on save and reported naming each field and the bound it broke. Note
that on Datafix election events the save is carried out by a background task, so its refusal is
reported through that task rather than in the form.

## Localization Overrides

To customize the display label of a user attribute, add a translation override via the Admin Portal under **Settings** > **Localization** > **Add**.

User attribute keys must use the prefix `usersAndRolesScreen.users.fields.` followed by the attribute name.

**Example:** to display the `personal_administrative_number` attribute as "PAN", add the following to the tenant's localization settings:

Key: `usersAndRolesScreen.users.fields.personal_administrative_number`
Value: `PAN`

This works for any attribute name, including custom ones not present in the default translations.
