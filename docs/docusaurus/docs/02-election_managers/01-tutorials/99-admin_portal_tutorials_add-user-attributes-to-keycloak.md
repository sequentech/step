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

### Checkboxes (Not Supported Yet)

1. In **Annotations** > **Add annotation**, set Key: `Input type`, Value: `multiselect-checkboxes`
2. **TODO:** Implementation pending

## Localization Overrides

To customize the display label of a user attribute, add a translation override via the Admin Portal under **Settings** > **Localization** > **Add**.

User attribute keys must use the prefix `usersAndRolesScreen.users.fields.` followed by the attribute name.

**Example:** to display the `personal_administrative_number` attribute as "PAN", add the following to the tenant's localization settings:

Key: `usersAndRolesScreen.users.fields.personal_administrative_number`
Value: `PAN`

This works for any attribute name, including custom ones not present in the default translations.
