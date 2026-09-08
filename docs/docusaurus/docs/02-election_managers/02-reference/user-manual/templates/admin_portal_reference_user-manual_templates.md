---
id: admin_portal_reference_user_manual_templates
title: Templates
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->



## Overview

Templates allow you to customize HTML documents used in reports and communications. You can add images, links, or modify text to match your organization's branding and requirements.

## Creating a Template 


1. In the Admin Portal's left panel, below the Election Events and below `Settings`, click on `Templates`.
2. Give a template name and alias, then choose the type.
3. In `Choose methods`, enable `Document` and the default HTML document will be shown. You can modify this template that will be rendered to PDF later.
4. **Optional:** If the template type is used for communications to the user, you can modify it by enabling `Email` or `SMS`.
5. After making modifications, click on `Save`.

## Usage

The system will automatically use the created template when generating documents of that type (e.g., during the tally process or when exporting data).

You can also use this template when manually [creating reports](../../../01-tutorials/18-reports_and_templates.md) in the Reports tab.

## Secret Voter Variables

A communication or true per-voter report can use fields configured with `sequent.secret=true`.
Secret fields are never made available to aggregate election, turnout, tally, results, or activity
reports.

Use the normal voter variable paths. For a secret attribute named `customerReference`:

```handlebars
{{user.customerReference}}
{{lookup user.attributes "customerReference"}}
```

The first expression returns the first value. The second addresses the complete array stored for
the attribute.

Every secret field used by an output must also be explicitly declared. For stored report template
configuration, add `secret_attribute_names` beside the existing document/email/SMS configuration:

```json
{
  "secret_attribute_names": ["customerReference"],
  "document": "<p>Reference: {{user.customerReference}}</p>"
}
```

The declaration is an allowlist, not just documentation. At generation time Step confirms that
each name is still configured as secret and decrypts only the declared fields. A dynamic
Handlebars lookup cannot gain access to an undeclared secret.

The Voters-tab communication editor builds this declaration automatically from configured secret
attribute names that appear in the email or SMS content. Report templates configured through JSON
or election-event configuration must carry the declaration themselves.

Running an output with a non-empty declaration requires `voter-secret-attribute-read` in addition
to the normal send or report permission. Treat the rendered message or report as sensitive data
and apply its normal document and delivery access controls.

Email and SMS bodies are not password-encrypted: recipients receive readable messages, including
any declared secret values. File reports follow their configured encryption policy and private
document access controls. Delivery audit records do not contain the rendered subject or body.

### Test-only Console delivery

Explicitly setting `EMAIL_TRANSPORT_NAME=Console` or `SMS_TRANSPORT_NAME=Console` prints the full
rendered message to the Windmill worker console instead of sending it. This includes decrypted
secret attributes, so it can be used to inspect a test login code.

Use synthetic voters and credentials only. Do not enable Console transport in production or send
these logs to an unrestricted destination. Real delivery transports do not log message bodies;
an unknown transport name fails instead of silently falling back to Console.
