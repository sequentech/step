---
id: add_new_language
title: Add a New Language
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


# Add a New Language to the System

This guide outlines the steps to add a new language to the system. When adding a new language, replace `[lang_code]` with the new language's ISO 639-1 code (e.g., `fr` for French) and `[LangNameInEnglish]` (e.g., `French`), `[LangNameInNative]` (e.g., `Français`) as appropriate.

## Pluralisation

A string that contains a number needs one entry per plural form of the target language. Do not write one entry
and choose between two keys in the component: `count === 1` is English's rule and it is wrong in most other
languages. French counts zero as singular, and Filipino counts two as singular.

Instead, give the key a `_one` and an `_other` suffix and pass a numeric `count`. i18next asks
`Intl.PluralRules` for the category of `count` in the current language and picks the matching suffix:

```typescript
// translations/en.ts
selectedCandidates_one: "{{count}} candidate selected",
selectedCandidates_other: "{{count}} candidates selected",
```

```tsx
// the component never decides which form to use
t("candidatesList.selectedCandidates", {count: selectedCandidatesCount})
```

Rules to follow:

* **`count` must be a number.** i18next skips pluralisation entirely when `count` is a string, and silently
  falls back to the unsuffixed key. `{count: String(n)}` and `{count: n.toString()}` are bugs.
* **The variable must be called `count`.** No other name triggers plural selection.
* **Use `_one` and `_other`.** `_plural` is the i18next v3 suffix and is never selected by the version in use
  here — an entry under `_plural` is dead code.
* **Declare both suffixes even when the text is identical.** Basque and Tagalog do not inflect a noun after a
  numeral, so their two forms are the same string. They are both still required, because the non-English files
  are typed `TranslationType = typeof englishTranslation` and must carry the same key set as `en.ts`.
* **Add an entry for every category the language reports.** `_one` and `_other` are the English set, not the
  universal one. Check the language you are adding:

  ```js
  new Intl.PluralRules("[lang_code]").resolvedOptions().pluralCategories
  // "ar" -> ["few", "many", "one", "other", "two", "zero"]
  // "sl" -> ["few", "one", "other", "two"]
  // "es" -> ["many", "one", "other"]
  ```

  Arabic needs all six suffixes; Slovenian and Hebrew need `_two`, because both select `two` for a count of 2.
  Spanish, Catalan and French report `many`, but only for exact multiples of a million, so a count of
  candidates never reaches it.
* **`_zero` can also be added deliberately.** Beyond the CLDR category, i18next selects `_zero` whenever
  `count` is exactly 0, in any language — so you can give "none" its own wording even where
  `pluralCategories` does not list `zero`.
* **The runtime must provide `Intl.PluralRules`.** i18next 25 requires it and has no fallback to the old v3
  behaviour; `compatibilityJSON: "v3"` was removed in v24. Every browser we support has it. If a target ever
  lacks it, install the `intl-pluralrules` polyfill — without one, i18next logs an error and degrades to
  English-style `_one`/`_other` for every language, which silently mis-pluralises the rest.

### Ballot validation messages

The warnings a voter sees while marking a ballot come from the Rust checker in `sequent-core` as a message key
plus a `message_map`. Every value in that map is a string, and the map describes the *state* of the selection
(`numSelected`, `min`, `max`) rather than the number the sentence is about.

`getBallotErrorOptions` in `ui-core` bridges the two: it coerces the numeric values and derives `count` per
message key — `max - numSelected` for an undervote, `numSelected - max` for an overvote, and so on. Render these
messages through it:

```tsx
import {getBallotErrorOptions} from "@sequentech/ui-core"

t(error.message || "", getBallotErrorOptions(error.message, error.message_map))
```

`getBallotErrorOptions` copies every entry of `message_map` into the options it returns, coercing numeric
strings as it goes. So if the checker already sends a `count`, the message pluralises with no further work.

Add a derivation to `COUNT_DERIVATIONS` in `packages/ui-core/src/services/ballotErrorMessages.ts` only when the
number the sentence is about is *not* in `message_map` and has to be computed from the state fields — as with
`underVote`, where the sentence needs `max - numSelected`. A derivation, when present, overrides any `count`
that came in the map.

## 1. Admin Portal (`packages/admin-portal`)

### 1.1. Add Main Translation File

1.  **Create the translation file**:
    * Navigate to `packages/admin-portal/src/translations/`.
    * Create a new file named `[lang_code].ts` (e.g., `eu.ts` for Basque).
    * This file will contain all the translations for the admin portal in the new language. You can use `en.ts` as a template for the structure and keys.
    * Populate this file with the translations for the new language

### 1.2. Update i18n Service

1.  **Edit `packages/admin-portal/src/services/i18n.ts`**:
    * **Import the new translation file**:
        Add an import statement for your new translation file at the top.
        ```typescript
        import [lang_code]Translation from "../translations/[lang_code]" // e.g., import basqueTranslation from "../translations/eu"
        ```
    * **Add to `initializeLanguages` function**:
        In the `initializeLanguages` call, add the new language and its imported translation.
        ```typescript
        initializeLanguages({
            // ... other languages
            [lang_code]: [lang_code]Translation, // e.g., eu: basqueTranslation,
        })
        ```
    * **Add to `triggerOverrideTranslations` function**:
        Similarly, add the new language to the `overwriteTranslations` call within this function.
        ```typescript
        overwriteTranslations({
            // ... other languages
            [lang_code]: [lang_code]Translation, // e.g., eu: basqueTranslation,
        })
        ```
    * **Add to `getAllLangs` function**:
        Add the new language code to the array returned by this function.
        ```typescript
        export const getAllLangs = (): Array<string> => ["en", "es", "cat", /*...,*/ "[lang_code]"] // e.g., "eu"
        ```

### 1.3. Add Language Name to Existing Translations

For each existing language file in `packages/admin-portal/src/translations/` (e.g., `cat.ts`, `en.ts`, `es.ts`, `fr.ts`, `gl.ts`, `nl.ts`, `tl.ts`):
1.  Open the file.
2.  Locate the `language` object within the `translations.common` object.
3.  Add a new key for your language code. **The name goes in the language of the file you are editing**, not
    in the language being added — the picker shows every entry at once, and a voter has to recognise their own
    among names they cannot read. Adding Basque:

    ```typescript
    // en.ts
    language: {eu: "Basque"},
    // cat.ts
    language: {eu: "Basc"},
    // es.ts
    language: {eu: "Euskera"},
    ```

    The new file is the one exception, and the one people get wrong: `eu.ts` carries the **endonym**, the name
    the language uses for itself.

    ```typescript
    // eu.ts — "Euskara", not the Spanish exonym "Euskera"
    language: {eu: "Euskara"},
    ```
4.  Repeat for the remaining files: `fr.ts`, `gl.ts`, `nl.ts` and `tl.ts`.

## 2. Keycloak Extensions (`packages/keycloak-extensions`)

### 2.1. Message OTP Authenticator

1.  **Create message properties file**:
    * Navigate to `packages/keycloak-extensions/message-otp-authenticator/src/main/resources/theme-resources/messages/`.
    * Create a new file named `messages_[lang_code].properties` (e.g., `messages_eu.properties`).
    * This file contains translations for the OTP authenticator. Use an existing file like `messages_en.properties`  as a template.
2.  **Add license file for properties**:
    * In the same directory, create a corresponding license file: `messages_[lang_code].properties.license` (e.g., `messages_eu.properties.license`).
    * Copy the content from an existing license file (e.g., `messages_en.properties.license`).

### 2.2. Sequent Theme - Admin Portal - Account Messages

1.  **Update existing language property files**:
    * Navigate to `packages/keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.admin-portal/account/messages/`.
    * For each relevant existing `messages_*.properties` file (e.g., `messages_en.properties`, `messages_tl.properties`):
        * Add a line for the new locale:
            ```properties
            # everything after '=' is the value, so a comment needs its own line
            # e.g. locale_eu=Euskara
            locale_[lang_code]=[LangNameInThatLanguage]
            ```
2.  **Create new language properties file**:
    * In the same directory, create `messages_[lang_code].properties` (e.g., `messages_eu.properties`).
    * This file contains translations for the account management theme. Use `messages_en.properties` as a template.
3.  **Add license file**:
    * In the same directory, create `messages_[lang_code].properties.license` (e.g., `messages_eu.properties.license`).
    * Copy content from an existing license file.

### 2.3. Sequent Theme - Admin Portal - Login Messages

1.  **Update existing language property files**:
    * Navigate to `packages/keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.admin-portal/login/messages/`.
    * For relevant existing `messages_*.properties` files (e.g., `messages_en.properties`, `messages_gl.properties`, `messages_tl.properties`):
        * Add a line for the new locale:
            ```properties
            # e.g. locale_eu=Euskara
            locale_[lang_code]=[LangNameInThatLanguage]
            ```
            (This change is shown for `messages_en.properties`, `messages_gl.properties`, `messages_tl.properties` in the diff.)
2.  **Create new language properties file**:
    * In the same directory, create `messages_[lang_code].properties` (e.g., `messages_eu.properties`).
    * This file contains translations for the login theme. Use `messages_en.properties`  as a template.
3.  **Add license file**:
    * In the same directory, create `messages_[lang_code].properties.license` (e.g., `messages_eu.properties.license`).
    * Copy content from an existing license file.

### 2.4. Update Theme Properties

1.  **Admin Portal Login Theme**:
    * Open `packages/keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.admin-portal/login/theme.properties`.
    * Add the new language code to the `locales` property.
        ```properties
        # e.g. locales=en,eu
        locales=en,...,[lang_code]
        ```
2.  **Voting Portal Login Theme**:
    * Open `packages/keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.voting-portal/login/theme.properties`.
    * Add the new language code to the `locales` property.
        ```properties
        # e.g. locales=en,eu
        locales=en,...,[lang_code]
        ```

## 3. UI Core (`packages/ui-core`)

### 3.1. Add Translation File

1.  **Create the translation file**:
    * Navigate to `packages/ui-core/src/translations/`.
    * Create a new file named `[lang_code].ts` (e.g., `eu.ts`).
    * This file contains translations shared across UI core components. Use `en.ts` as a template.

### 3.2. Update i18n Service

1.  **Edit `packages/ui-core/src/services/i18n.ts`**:
    * **Import the new translation file**:
        ```typescript
        import [lang_code]Translation from "../translations/[lang_code]" // e.g., import basqueTranslation from "../translations/eu"
        ```
    * **Add to `libTranslations` in `initializeLanguages` function**:
        ```typescript
        const libTranslations: Resource = {
            // ... other languages
            [lang_code]: [lang_code]Translation, // e.g., eu: basqueTranslation,
        }
        ```

## 4. UI Essentials (`packages/ui-essentials`)

### 4.1. Add Translation File

1.  **Create the translation file**:
    * Navigate to `packages/ui-essentials/src/translations/`.
    * Create `[lang_code].ts` (e.g., `eu.ts`).
    * This file contains translations for essential UI components. Use `en.ts` as a template.

### 4.2. Update i18n Service

1.  **Edit `packages/ui-essentials/src/services/i18n.ts`**:
    * **Import the new translation file**:
        ```typescript
        import [lang_code]Translation from "../translations/[lang_code]" // e.g., import basqueTranslation from "../translations/eu"
        ```
    * **Add to `libTranslations` in `initializeLanguages` function**:
        ```typescript
        const libTranslations: Resource = {
            // ... other languages
            [lang_code]: [lang_code]Translation, // e.g., eu: basqueTranslation,
        }
        ```

## 5. Voting Portal (`packages/voting-portal`)

### 5.1. Add Translation File

1.  **Create the translation file**:
    * Navigate to `packages/voting-portal/src/translations/`.
    * Create `[lang_code].ts` (e.g., `eu.ts`).
    * This file contains translations for the voting portal. Use `en.ts` as a template.

### 5.2. Update i18n Service

1.  **Edit `packages/voting-portal/src/services/i18n.ts`**:
    * **Import the new translation file**:
        ```typescript
        import [lang_code]Translation from "../translations/[lang_code]" // e.g., import basqueTranslation from "../translations/eu"
        ```
    * **Add to `initializeLanguages` call**:
        Add the new language to the translations object passed to `initializeLanguages`.
        ```typescript
        initializeLanguages(
            {
                // ... other languages
                [lang_code]: [lang_code]Translation, // e.g., eu: basqueTranslation,
            },
            language
        )
        ```

---

After completing these steps:
1.  Ensure all newly created `.ts` and `.properties` files are fully translated.
2.  Rebuild and deploy your application.
3.  Thoroughly test all parts of the application in the new language to ensure translations are correctly applied and displayed.
