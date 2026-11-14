---
id: admin_portal_tutorials_custom_css
title: Customize the Voting Portal with CSS
description: "Change the look of the Voting Portal with the Custom CSS field of the election event. Find the class of an element, write a rule, publish and check the result."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Use the **Custom CSS** field to change the look of the Voting Portal: colors, spacing, alignment or
hidden elements. The rules apply to all the elections of the election event. This tutorial shows
how to find the class of an element, write a rule and check the result.

Use CSS only for the look. Do not hide a text that the voter must read to cast a valid vote.

## Before you start

- Have a role that can edit the election event and publish it.
- Know the basics of CSS: a selector and a list of properties.

## 1. Find the Custom CSS field

1. Open the election event.
2. Click the **Data** tab.
3. Open the **Ballot Design** section.
4. Find the **Custom CSS** field.

The field is on the election event only. An election does not have its own **Custom CSS** field.

:::note
The **Custom CSS** field in **Settings** → **Look & Feel** changes the admin portal. It does not change the Voting Portal.
:::

## 2. Find the class of an element

1. Open the preview of the ballot. See [Preview the ballot](../03-procedures/05-publish.md#2-preview-the-ballot).
2. Right-click the element to change.
3. Click **Inspect** in the menu of the browser.
4. Read the `class` attribute of the element.

Use the stable classes. The pages [Voting Portal CSS hooks](../02-reference/09-voting-portal-custom-css.md)
and [Styling Ballot Errors and Warnings with Custom CSS](../02-reference/08-ballot-errors-custom-css.md)
list them.

:::caution
Do not use a class that looks generated, such as `css-1abc2d`. It changes when the software
changes. A rule that uses it can stop working after an update.
:::

## 3. Write the rule and publish it

1. Type the rule in the **Custom CSS** field.
2. Click **Save**.
3. Make a publication. See [Make a publication](../03-procedures/05-publish.md#1-make-a-publication).
4. Open the preview and check the result.
5. Publish the ballot. See [Publish the ballot](../03-procedures/05-publish.md#3-publish-the-ballot).

The Voting Portal reads the CSS from the published ballot. Voters see a change only after you
publish it.

**Expected result:** the preview shows the new look.

## Examples

Hide the help button on the **Vote** screen:

```css
.voting-screen .screen-help-button {
  display: none;
}
```

Center the title of each contest:

```css
.contest-title {
  text-align: center;
}
```

Change the background color of the footer:

```css
.footer-class {
  background-color: #f0f0f0;
}
```

## If there is a problem

| Problem | Action |
| --- | --- |
| The rule has no effect. | Make sure that you clicked **Save** and published the changes. Open the preview again. |
| The rule has no effect after you publish. | Check the class in the inspector. Make the selector more specific, for example `.voting-screen .contest-title`. |
| The rule has no effect on a dialog or a menu. | Dialogs and menus are outside the screen. The Custom CSS does not reach them. See [Voting Portal CSS hooks](../02-reference/09-voting-portal-custom-css.md). |
| The layout breaks. | Remove the rule from the **Custom CSS** field. Click **Save**. Publish the changes again. |

## Related pages

- [Voting Portal CSS hooks](../02-reference/09-voting-portal-custom-css.md)
- [Styling Ballot Errors and Warnings with Custom CSS](../02-reference/08-ballot-errors-custom-css.md)
- [Publish and Manage the Voting Period](../03-procedures/05-publish.md)
