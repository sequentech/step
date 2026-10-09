---
id: election_management_election_event_templates
title: Templates
description: "Managing Templates is essential for consistent report generation. Each Report Type is associated with a Template to form a “recipe” used when generating reports."
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

## Voter variables

Email and SMS templates, and document templates for reports that render one
voter at a time (the voter information letter and the manual verification
report), can use these standard voter variables:

- `user.first_name`
- `user.last_name`
- `user.username`
- `user.email`

Custom Keycloak user attributes are also available. The first value is exposed
as `user.<attribute>`. The complete value list remains available as
`user.attributes.<attribute>`. Standard variables take precedence if a custom
attribute uses the same name. The `attributes` name is reserved for the complete
attribute map. Empty custom value lists are present only under `user.attributes`.

Aggregate reports, such as turnout, activity, tally and results reports, render
many voters at once and do not receive a `user` object.

Secret attributes (`sequent.secret=true`) are the exception: they are available
only when the template declares them in `secret_attribute_names`. See [Secret
Voter Variables](../user-manual/templates/admin_portal_reference_user-manual_templates.md#secret-voter-variables).

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

### Voter information letter variables

Besides `user`, the voter information letter template receives these fields:

| Variable | Content |
| --- | --- |
| `election_event_name` | The election event name in the event's default language. |
| `issue_date` | The generation date, written out in the event's default language. |
| `voter_first_name` | The voter's first name. |
| `voter_last_name` | The voter's last name. |
| `voter_full_name` | First and last name, separated by a space. |
| `username` | The voter's username. |
| `password` | The voter's new credential. When the realm uses a structured credential pattern, it is formatted with that pattern, for example `1234-5678-9012-3456`. |
| `voting_portal_url` | The voting portal login URL for the election event. |
| `logo_url` | The URL of the default logo. |

### Text helpers

These helpers make it easier to print voter data that may be incomplete. They
never fail: a missing value renders as an empty string. Use them as
subexpressions to combine them.

| Helper | Example | Result |
| --- | --- | --- |
| `concat` | `{{concat user.first_name user.middle_names user.last_name}}` | `Jane Doe` when `middle_names` is missing or empty. |
| `concat` with `sep` | `{{concat user.corr_city user.corr_province sep=", "}}` | `Denbigh, ON` |
| `format_pattern` | `{{format_pattern user.corr_postal_code "### ###"}}` | `K0H 1L0` from `K0H1L0`. |
| `upper` | `{{upper user.corr_country_name}}` | `CA` from `Ca`. |

`concat` joins any number of values with a separator, a single space by
default. It trims each value and skips the ones that are missing, null, empty or
blank, so the separator only appears between values that are present. Arrays,
such as `user.attributes.<attribute>`, are flattened. Numbers and quoted literals
are accepted too.

`format_pattern` fills each `#` in the pattern with the next non-space character
of the value. Every other pattern character is copied as is. If the value does
not have exactly as many characters as the pattern has `#`, the trimmed value is
printed unchanged.

For example, a mailing address that skips the parts a voter does not have:

```handlebars
{{concat user.corr_unit user.corr_street_number user.corr_street_number_suffix user.corr_street}}
{{concat user.corr_city user.corr_province (format_pattern user.corr_postal_code "### ###")}}
{{upper user.corr_country_name}}
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

Some attributes are never prefilled, whatever the policy says: credential
fields, unmanaged attributes, attributes the voter cannot write, and attributes
the voter would not be able to see or edit on the form. The last group covers an
attribute annotated `hidden`, one rendered as a hidden or password input, and one
whose annotations set `html-attribute:disabled`, `hidden`, `inert`, `readonly`,
`aria-hidden="true"`, or an inline `html-attribute:style` that hides it
(`display:none`, `visibility:hidden`, `visibility:collapse` or `opacity:0`).

Cosmetic annotations do not affect prefill — `html-attribute:class` and inline
styles that only change appearance leave the field prefillable. When an
attribute is skipped, Keycloak logs the attribute name and the reason at `DEBUG`
level under `sequent.keycloak.voter_enrollment.LoginHintPrefill`; hint values are
never logged.

The policy applies to registration forms once the **Sequent: Login hint
registration prefill** action is part of the registration flow.

#### Locking the username on the login page

The login page is not rendered from the user profile, so it cannot read attribute
annotations. Set the realm attribute `loginHintUsernamePolicy` to `READ_ONLY`
(default `EDITABLE`) to render a prefilled username read-only. A username
restored by *remember me* stays editable, so voters can still sign in as somebody
else.

This is a presentation-only lock: it stops a voter from editing the field by
accident, but Keycloak still authenticates whichever username is submitted. That
is the same guarantee as an unprefilled login page — the password decides which
account is entered. Registration is different: there the value is stored, so
`READ_ONLY` is enforced on the server as well.

:::warning
Prefilled values are convenience data, not verified identity claims. They never
bypass authentication, registration validation, or required actions. `READ_ONLY`
stops a voter from changing a prefilled field; it does not make the value
trustworthy, because whoever built the link chose it. Do not include passwords,
one-time passwords (OTPs), tokens, secrets, government identifiers, or other
inappropriate sensitive values. Percent encoding protects URL structure; it
does not provide confidentiality or authenticity.
:::

> **Note:** For further guidance on template fields or syntax, refer to the Reports section of the guide where Template usage in report configuration is detailed.
