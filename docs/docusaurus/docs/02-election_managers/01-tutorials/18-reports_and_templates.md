---
id: reports_and_templates
title: Reports and Templates
---

## Overview

To generate a report, you need to select the Election Event in the Admin Portal and add an entry for a specific report type.

## Creating a Report

1. Go to the **REPORTS** tab and click the **Create Report** or **ADD** button.
2. Select the type (see special notes for specific types below).
3. **Optional:** Select Election for reporting only a specific election.
4. **Optional:** Choose a template. You must have created one earlier (see [more about creating templates](../02-reference/user-manual/templates/admin_portal_reference_user-manual_templates.md)). If none is selected, the system's default will be used.
5. Click **Save**.

## Generating Reports

After creating a report entry, you will see it in the list. Click on the actions menu (3 dots on the right side) and choose:

- **Generate:** Create a report based on real data
- **Preview:** Generate an example of that report template based on mock data

## Report Types

### Voters Turnout

**Note:** To display male and female percentages, the sex attribute must have been added. Check out [how to add the sex attribute](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md).

### Other Report Types

*Additional report types will be documented here as they become available.*

## Secret Voter Fields in Reports

Secret voter fields are available only to reports that render one identified voter at a time.
Current examples are voter information letters and manual verification reports. Aggregate reports
such as turnout, election activity, tally, and results do not receive a voter object and cannot use
secret fields.

The assigned template must both reference and declare every secret field it uses. For example:

```json
{
  "secret_attribute_names": ["customerReference"],
  "document": "<p>Reference: {{user.customerReference}}</p>"
}
```

At generation time, Step validates the declaration against the election event's current Keycloak
User Profile and decrypts only the declared names for the report's voter. The operator needs
`voter-secret-attribute-read` in addition to the normal report permission. An undeclared, removed,
or unreadable secret causes generation to fail rather than exposing its stored representation.

See [Secret Voter Variables](../02-reference/user-manual/templates/admin_portal_reference_user-manual_templates.md#secret-voter-variables)
for variable shapes and declaration rules.
