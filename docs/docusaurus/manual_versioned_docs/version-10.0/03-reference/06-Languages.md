---
id: languages
title: Languages
---

<!--
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
## Language Determination in the Voting Portal

In the voting portal, the system determines which language to display to voters based on a defined order. It respects user preferences while allowing administrators to enforce language policies when needed.

### Language Determination Priority

The language is determined in the following order of precedence (highest to lowest):

1. **URL Search Parameter (`lang`)** — Highest priority
2. **User Selected Locale in the login flow** — Saved in browser cookie
3. **Language Detection Policy** — Configured in election event Data tab
4. **Browser Settings** — Browser language preference (lowest priority)

### How Each Level Works

#### 1. URL Search Parameter (`lang`)

When a voter accesses the voting portal with a `lang` query parameter, it takes absolute precedence over all other settings. This allows you to direct voters to a specific language version via links.

**Example:**
```
https://myelection.sequent.vote/?lang=es
```

The `lang` parameter is checked during i18n initialization, ensuring the language is applied before any policies are evaluated.

#### 2. User Selected Locale

Once a voter selects a language in the voting portal, their choice is saved in a browser cookie. This cookie is stored for the duration of the session (until the browser is closed). On subsequent visits within the same session, the saved language preference is restored.

This allows voters to:
- Select their preferred language once
- Override the language detection policy for their individual session

#### 3. Language Detection Policy

The Language Detection Policy is configured at the election event level and determines how the system selects a language when no URL parameter or user preference cookie exists.

**Available Policies:**

- **`BROWSER_DETECT`** (default) — The voting portal automatically detects the voter's browser language and displays content in the closest available language
- **`FORCE_DEFAULT`** — All voters are shown the default language specified in the election settings, regardless of their browser language

**When this policy applies:**
- Only when no `lang` URL parameter is present
- Only when the user hasn’t manually changed the language.


### Example Scenarios

**Scenario 1: Multi-language election with browser detection**
- No language policy is set
- Voter 1 with Spanish browser settings sees Spanish immediately
- Voter 2 with English browser settings sees English immediately
- Both voters can manually select a different language and their choice is saved

**Scenario 2: Bilingual election with forced default**
- Language policy is set to `FORCE_DEFAULT` with `default_language_code: es`
- All voters see Spanish, regardless of their browser settings
- Voters can still choose a different language manually via the UI

**Scenario 3: Election with persistent user preference**
- No language policy is set
- Voter visits the portal in Spanish and selects English manually
- English preference is saved to a cookie
- The `lang` URL parameter still overrides this if provided

