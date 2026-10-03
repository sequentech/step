---
id: voter_accessibility
title: Voter Accessibility
sidebar_position: 13
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The Voting Portal can offer each voter a set of display settings. They are meant for voters
with low vision, dyslexia, vestibular disorders and other conditions that the default
presentation does not suit.

---

## Accessibility settings

When enabled, an **Accessibility** button appears in the Voting Portal header, next to the
language selector. It opens a dialog with four settings:

| Setting | Choices | Effect |
|---|---|---|
| **Text size** | Default, Large, Larger | Text is shown at 100%, 125% or 150%. The page reflows; nothing needs horizontal scrolling. |
| **Contrast** | Default, High contrast | Black text on white, solid borders on controls, underlined links and a thicker focus indicator. Disabled and read-only content is no longer dimmed. |
| **Text spacing** | Default, Wide | Line height 1.5, letter spacing 0.12em, word spacing 0.16em and wider paragraph spacing. |
| **Motion** | Default, Reduced | Animations and transitions are removed. |

A setting applies as soon as it is chosen. **Reset settings** returns to the defaults.

Until the voter chooses, the portal follows the device: a device set to increase contrast gets
**High contrast**, and one set to reduce motion gets **Reduced** motion.

The choices are remembered for the browser session, on that device only. They are stored in a
cookie, never in the voter's account or with the ballot, and the portal works the same in a
private window.

### Enabling the settings

The settings are configured per Election Event, in **Election Event → Data → Ballot Design**,
with the **Voter accessibility settings** select:

| Choice | Wire value | Behavior |
|---|---|---|
| **Hide the accessibility settings** (default) | `disabled` | The header has no Accessibility button. |
| **Offer text size, contrast, spacing and motion settings** | `enabled` | The button is shown on every voter screen. |

The value is stored in the event's presentation as `voter_accessibility_settings_policy`. An
event that does not have the field behaves as `disabled`, so existing events do not change.
Events created from the default template start with `enabled`.

The change reaches voters with the next publication of the event.

### Custom CSS

Each setting other than the default is exposed as an attribute on the page's `<html>` element,
which an event's [custom CSS](./09-voting-portal-custom-css.md) can select on:

| Attribute | Values |
|---|---|
| `data-a11y-text-size` | `large`, `larger` |
| `data-a11y-contrast` | `high` |
| `data-a11y-text-spacing` | `wide` |
| `data-a11y-motion` | `reduced` |

For example, to keep a brand color out of high contrast:

```css
html[data-a11y-contrast="high"] .election-title {
    color: #000;
}
```

Custom CSS that sets font sizes in `px` does not follow the **Text size** setting. Use `rem`
or `em`.
