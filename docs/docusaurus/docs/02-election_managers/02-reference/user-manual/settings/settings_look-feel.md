---
id: settings_look-feel
title: Look & Feel
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The **Look & Feel** tab is in **Settings**. It sets the logo and the style of the admin portal
for your tenant, and the links of the **Help** menu. The tenant administrator uses it.

:::note
These settings change the admin portal. The logo of the voting portal is set for each
election event, on its **Data** tab.
:::

## How to open it

1. In the menu on the left, click **Settings**.
2. Click the **Look & Feel** tab.

## Required permission

To open **Settings**, your account must have the **Edit Tenant** and **View settings**
permissions. The **Save** button shows only to users with the **Edit Tenant** permission.

## Fields

| Field | Meaning | Default |
| --- | --- | --- |
| **Logo URL** | The web address of the image that shows at the top of the admin portal. Leave it empty to show the standard logo. | Empty |
| **Custom CSS** | Style rules (CSS) that the admin portal adds to its pages. Use it to change colours or fonts. | Empty |
| **Help Links** | The list of links of the **Help** menu, in JSON format. See [Help links](#help-links). | Empty list |
| **Save** | Saves the three fields. After you save, the button stays disabled until you change a field. | |

## Help links

**Help Links** is a JSON list. Each link has these keys:

| Key | Required | Meaning |
| --- | --- | --- |
| `title` | Yes | The text of the menu item. |
| `url` | Yes | The web address that opens in a new browser tab. |
| `i18n` | No | A translation of the title for each language, for example `{"es": {"title": "Ayuda"}}`. |

Example:

```json
[
  {"title": "User guide", "url": "https://example.com/guide"},
  {
    "title": "Support",
    "url": "https://example.com/support",
    "i18n": {"es": {"title": "Soporte"}}
  }
]
```

When the list has one or more links, the **Help** item shows in the menu on the left. When the
list is empty, the **Help** item does not show.

## Change the look and feel

1. Open the **Look & Feel** tab.
2. In **Logo URL**, type the web address of the logo image.
3. In **Custom CSS**, type the style rules.
4. In **Help Links**, type the JSON list of links.
5. Click **Save**.

**Expected result:** the admin portal shows the new logo and style. The **Help** menu shows
the links.

:::caution CAUTION
Make sure that **Custom CSS** does not hide buttons or fields. Wrong style rules can make
parts of the admin portal difficult to use.
:::

To remove the custom logo or the custom style, delete the text of the field and click **Save**.

## If there is a problem

| Problem | Action |
| --- | --- |
| The message "Invalid Help Links format" shows. | The text in **Help Links** is not a valid JSON list. Correct the text. The list must start with `[` and end with `]`. |
| The **Help** item does not show in the menu. | Make sure that **Help Links** contains one or more links, then click **Save**. |
| The logo does not show. | Make sure that the web address in **Logo URL** opens the image in a browser. |
| The **Save** button does not show. | Your account does not have the **Edit Tenant** permission. |

**Related procedure:** [Set Up the Tenant](../../../03-procedures/01-tenant.md).
