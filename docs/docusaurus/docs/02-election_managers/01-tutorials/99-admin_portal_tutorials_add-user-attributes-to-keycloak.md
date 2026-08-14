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
- `.voter-attribute-group__grid`: fields within a group.
- `.voter-field`, `[data-field-name="..."]`, `[data-input-type="..."]`, and `[data-required="true"|"false"]`: field wrapper and metadata. Field names are the canonical raw Keycloak names.

Step-owned fields also expose stable names: `enabled`, `area`, `password`, `confirm_password`, and `password_temporary`.

The following example creates responsive two-column groups with bordered cards and lets selected fields span both columns:

```css
.voter-editor .voter-editor__groups {
  gap: 1.25rem;
}

.voter-editor .voter-attribute-group {
  border: 1px solid #d9dce8;
  border-radius: 12px;
  padding: 1rem;
}

.voter-editor .voter-attribute-group__legend {
  color: #17105f;
  padding: 0 0.35rem;
}

.voter-editor .voter-attribute-group__grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.voter-editor .voter-field[data-field-name="email"],
.voter-editor .voter-field[data-input-type="multiselect-checkboxes"] {
  grid-column: 1 / -1;
}

@media (max-width: 700px) {
  .voter-editor .voter-attribute-group__grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
```

Do not target generated MUI or Emotion class names; they are implementation details and can change during upgrades. CSS `order` can change only the visual order, not keyboard or screen-reader order, so configure field order in Keycloak instead.

## Localization Overrides

To customize the display label of a user attribute, add a translation override via the Admin Portal under **Settings** > **Localization** > **Add**.

User attribute keys must use the prefix `usersAndRolesScreen.users.fields.` followed by the attribute name.

**Example:** to display the `personal_administrative_number` attribute as "PAN", add the following to the tenant's localization settings:

Key: `usersAndRolesScreen.users.fields.personal_administrative_number`
Value: `PAN`

This works for any attribute name, including custom ones not present in the default translations.
