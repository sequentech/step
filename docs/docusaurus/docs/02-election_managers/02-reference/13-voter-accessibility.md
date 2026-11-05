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

---

## Audio instructions

When enabled, every voter screen starts with a **Listen to the instructions** button. It
explains, for that screen, what the screen is for and how to use it with a keyboard: how to
move, select, continue and go back. It is meant for blind and low-vision voters, and for
anyone who prefers to listen.

The screens that have instructions are the election list, start, ballot, review, confirmation,
audit, ballot locator and support materials.

The voter can pause, resume and stop the audio. It never starts by itself, because a voter
using a screen reader would hear two voices at once, and it stops when the voter leaves the
screen or changes the language. **Read the instructions** shows the same text on screen, so it
also reaches braille displays.

### Where the audio comes from

1. **A recording you upload.** This is the reliable path, and the one to use for an approved
   script and voice.
2. **The browser's own voice**, reading the screen's instruction text, where there is no
   recording. Browsers do not have a voice for every language: Filipino, Basque and Galician
   voices are missing from several. Where there is none, the voter is offered only the text.

### Enabling audio instructions

In **Election Event → Data → Ballot Design**, with the **Audio instructions** select:

| Choice | Wire value | Behavior |
|---|---|---|
| **No audio instructions** (default) | `disabled` | No button. |
| **Uploaded recordings only** | `recorded` | The button appears only on screens that have a recording. |
| **Uploaded recordings, or the browser's voice where there is none** | `recorded-or-synthesized` | Every screen has the button. |

The value is stored in the event's presentation as `audio_instructions_policy`. An event
without the field behaves as `disabled`. Events created from the default template start with
`recorded-or-synthesized`.

### Uploading a recording

Recordings are [support materials](../03-support-materials.md):

1. In **Election Event → Data → Support Materials**, add a material and upload the audio file
   (MP3 is played by every supported browser).
2. Once the file is an audio file, two more fields appear. In **Audio instructions for
   screen**, choose the screen. In **Language of the recording**, choose the language.
3. Give it a title in each language and save. Leave **Is Hidden** off: a hidden material is
   not sent to voters.

A saved recording is available to voters at once. The policy itself reaches voters with the
next publication of the event.

Upload one file per screen and language. A voter hears the recording for the current screen in
their language; if there is none, the one in the event's default language; if there is none
either, the browser's voice or nothing, depending on the policy. If two files are assigned to
the same screen and language, the first one found is played, so keep one.

Recordings are played whether or not the support materials tab is shown to voters. When it is
shown, they are listed there as well.

### Changing the spoken text

The text the browser reads, and that **Read the instructions** shows, can be replaced per
language in the event's **Localization** tab, with the keys
`audioInstructions.screens.<screen>`, where `<screen>` is `election-chooser`, `start`,
`ballot`, `review`, `confirmation`, `audit`, `ballot-locator` or `support-materials`. Keep it
in step with the recording.

---

## Login pages

A voter reaches the sign-in and code entry pages before the Voting Portal, so both features are
offered there too:

- The **Accessibility** button, with the same four settings. A choice made at sign-in is
  still applied in the Voting Portal, and the other way round, because both read the same
  cookie.
- **Listen to the instructions**, with a text for the sign-in page, the username page and the
  code entry page. The login pages have no recordings: the browser's voice reads the text
  where it has one for the language, and the text can always be read on screen. With the
  **Uploaded recordings only** policy the login pages have no audio control.

The login pages cannot read the event's presentation. They read two realm attributes of the
event's realm instead:

| Realm attribute | Values |
|---|---|
| `voter-accessibility-settings-policy` | `disabled`, `enabled` |
| `audio-instructions-policy` | `disabled`, `recorded`, `recorded-or-synthesized` |

Changing either select in **Ballot Design** and saving writes the matching realm attribute, if
you have permission to edit realm attributes. They can also be set directly in the event's
realm attributes, or in the realm configuration an event is imported with. A realm without
them shows neither control. An event imported or created with the policies already in its
presentation does not get the attributes by itself: set them once, in either way.
