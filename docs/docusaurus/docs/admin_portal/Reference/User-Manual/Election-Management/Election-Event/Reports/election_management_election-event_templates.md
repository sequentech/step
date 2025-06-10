<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

---
id: election_management_election_event_templates
title: Templates
---

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

> **Note:** For further guidance on template fields or syntax, refer to the Reports section of the guide where Template usage in report configuration is detailed. ```

