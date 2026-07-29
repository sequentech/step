---
id: election_management_election_event_templates
title: Templates
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


Managing Templates is essential for consistent report generation. Each Report Type is associated with a Template to form a “recipe” used when generating reports.

### Adding a New Template

1. **Select Add** to create a Template.
2. **Fill in the fields**:
   - **Template Alias (Optional)**: Display name shown in the Admin Portal.
   - **Template Name**: Internal name for the template in the Admin Portal.
   - **Template Type**: Category or area that will use this template. (E.g., Ballot Receipt, Statistical Report, etc.)
   - **Email / SMS / Document**: Choose whether this template includes an email/SMS message or attaches a document. Select the appropriate radio button.
3. **Save** the Template.

Once configured, the Template becomes available for its associated Report Types and other system areas.

### Key Points

- **Consistency**: Use predefined or default formats where possible to ensure consistency across reports.
- **Reuse**: A single Template can be applied to multiple Report Types if suitable.
- **Preview**: After saving, preview the Template in context (e.g., generate a sample report) to confirm formatting.
- **Updates**: Editing a Template will affect all future report generations that reference it; consider versioning or alias changes if you need to preserve older formats.
- **Examples**:  
  - Configuring the Ballot Receipt template: select “Ballot Receipt” as Template Type, define alias/name, choose Document radio, set layout/content, then Save. This will be used whenever a Ballot Receipt is generated.

### Tips

- Maintain clear, descriptive Template Names and Aliases so administrators can identify their purpose quickly.
- Document any special placeholders or variables used in templates (e.g., voter name, election date) in a separate reference or within the template description.
- Test email/SMS templates by sending to a test address or number before enabling in production.
- For document templates, ensure any required assets (logos, images) are accessible and correctly referenced.
- If your system supports previewing or templating languages (e.g., handlebars, Liquid), include sample data to verify rendering.

## Voter variables in notification templates

Email and SMS Handlebars templates can use these standard voter variables:

- `user.first_name`
- `user.last_name`
- `user.username`
- `user.email`

Custom Keycloak user attributes are also available. The first value is exposed
as `user.<attribute>`. The complete value list remains available as
`user.attributes.<attribute>`. Standard variables take precedence if a custom
attribute uses the same name. The `attributes` name is reserved for the complete
attribute map. Empty custom value lists are present only under `user.attributes`.

Dot notation works for simple names such as `dateOfBirth`. Use Handlebars
`lookup` for names containing dots or dashes. For example:

```handlebars
{{lookup user "sequent.read-only.mobile-number"}}
{{#each (lookup user.attributes "sequent.read-only.mobile-number")}}{{this}} {{/each}}
```

For example, a `reference` attribute with values `ABC-123` and `legacy-456` can
be rendered as follows:

```handlebars
Primary reference: {{user.reference}}
All references: {{#each user.attributes.reference}}{{this}} {{/each}}
```

### Prefilled voting links

Voting Portal `/login` and `/enroll` links accept up to five prefilled fields
named `login_hint__<field>`. Field names may contain letters, numbers, `.`, `_`,
and `-`; names are limited to 128 characters and values to 255 characters.

Use the `url_encode` helper around every dynamic query value. Keep the parameter
names and URL structure static:

```handlebars
https://vote.example/tenant/TENANT_ID/event/EVENT_ID/login?login_hint__username={{url_encode user.username}}&login_hint__reference={{url_encode user.reference}}
```

```handlebars
https://vote.example/tenant/TENANT_ID/event/EVENT_ID/enroll?login_hint__username={{url_encode user.username}}&login_hint__dateOfBirth={{url_encode user.dateOfBirth}}
```

The Voting Portal removes accepted hint parameters from its visible URL before
redirecting to Keycloak. Invalid, duplicate, or over-limit hint sets are rejected
as a whole.

#### Per-field prefill policy

Each registration field decides how it accepts a prefilled value through the
`loginHintPrefillPolicy` annotation of its Keycloak user profile attribute:

| Policy | Behaviour |
| --- | --- |
| `EDITABLE` | Prefill the field and let the voter change the value. Applied when the annotation is absent. |
| `READ_ONLY` | Prefill the field, render it read-only, and reject the registration if the submitted value was changed. |
| `IGNORE` | Never prefill the field from a voting link. |

Set the annotation in **Realm settings → User profile → Attributes → *(attribute)*
→ Annotations**, for example `loginHintPrefillPolicy` = `READ_ONLY`. An
unrecognised value is treated as `IGNORE`, so a typo never prefills a field.

Credential fields, unmanaged attributes, attributes the voter cannot write, and
attributes rendered as hidden inputs are never prefilled, whatever the policy.

:::warning
Prefilled values are convenience data, not verified identity claims. They never
bypass authentication, registration validation, or required actions. `READ_ONLY`
stops a voter from changing a prefilled field; it does not make the value
trustworthy, because whoever built the link chose it. Do not include passwords,
tokens, secrets, or sensitive attributes that are not approved for browser URLs
and notification delivery. Percent encoding protects URL structure; it does not
provide confidentiality or authenticity.
:::

> **Note:** For further guidance on template fields or syntax, refer to the Reports section of the guide where Template usage in report configuration is detailed.

