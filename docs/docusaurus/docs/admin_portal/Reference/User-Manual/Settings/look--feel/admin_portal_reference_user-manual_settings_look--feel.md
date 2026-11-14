---
id: admin_portal_reference_user_manual_settings_look___feel
title: Look & Feel
description: "The Look & Feel tab sets the logo, the style and the help links of the tenant."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Look & Feel** tab sets the logo, the style and the help links of the tenant.

## Open the tab

1. In the menu on the left, click **Settings**.
2. Click the **Look & Feel** tab.

To save changes, your role needs the **Edit Tenant** permission.

## Fields

| Field | Use |
| --- | --- |
| **Logo URL** | The address of the logo image of the tenant. |
| **Custom CSS** | Style rules that change the look of the portals. |
| **Help Links** | The links of the **Help** menu, in JSON format. If the list is empty, the **Help** menu does not show. |
| **Save** | Saves the changes. |

**Help Links** is a JSON list. Each link has a `title` and a `url`:

```json
[
  {"title": "User guide", "url": "https://example.com/guide"}
]
```

## If there is a problem

| Problem | Action |
| --- | --- |
| "Invalid Help Links format" | The text is not a JSON list. Check the brackets, the quotes and the commas. |
| The **Help** menu does not show. | **Help Links** is empty. Add one or more links and save. |

## Related pages

- [Settings](../admin_portal_reference_user-manual_settings.md)
- [Basic Navigation](../../../basic_navigation.md)
