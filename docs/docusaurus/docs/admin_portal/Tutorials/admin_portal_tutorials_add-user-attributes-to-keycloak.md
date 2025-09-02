---
id: admin_portal_tutorials_setting-up-your-first-election
title: Setting Up Your First Election
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The system supports adding additional user attributes that will appear as a new 
field in the user data once they are configured it will appear in the Add or Edit 
action in the admin portal´s Voters tab. This must be configured via Keycloak.

1. Login to Keycloak and select the election event you want to edit.
2. Realm settings > User profile > Create attribute
3. Give an attribute name, like in the first example `sex`.
4. Set display name as `${sex}` if you want override the translations in 
   Localization > Realm overrides.
5. Continue configuring Annotations and other parameters, see below.

For example these attribute types are supported:

#### Sex

To be able to select the sex:

1. In Annotations > Add annotation > set Key `Input type` Value `select`
2. Add Validator > Validator type `options` and add the desired options i.e. M, F

#### Birth date

To show a date input field:

1. In Annotations set Key `Input type` Value `html5-date`
2. Add a validation if desired.

#### Checkboxes (not supported yet)

1. In Annotations > Add annotation set Key `Input type` Value `multiselect-checkboxes`
2. TODO...
