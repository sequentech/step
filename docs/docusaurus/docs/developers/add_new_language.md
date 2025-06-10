<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

---
id: add_new_language
title: Add a new Language
---

# Add a New Language to the System

This guide outlines the steps to add a new language to the system. When adding a new language, replace `[lang_code]` with the new language's ISO 639-1 code (e.g., `fr` for French) and `[LangNameInEnglish]` (e.g., `French`), `[LangNameInNative]` (e.g., `Français`) as appropriate.

## 1. Admin Portal (`packages/admin-portal`)

### 1.1. Add Main Translation File

1.  **Create the translation file**:
    * Navigate to `packages/admin-portal/src/translations/`.
    * Create a new file named `[lang_code].ts` (e.g., `eu.ts` for Basque). [cite: 684]
    * This file will contain all the translations for the admin portal in the new language. You can use `en.ts` as a template for the structure and keys. [cite: 684]
    * Populate this file with the translations for the new language

### 1.2. Update i18n Service

1.  **Edit `packages/admin-portal/src/services/i18n.ts`**: [cite: 678]
    * **Import the new translation file**:
        Add an import statement for your new translation file at the top.
        ```typescript
        import [lang_code]Translation from "../translations/[lang_code]" // e.g., import basqueTranslation from "../translations/eu" [cite: 678]
        ```
    * **Add to `initializeLanguages` function**:
        In the `initializeLanguages` call, add the new language and its imported translation.
        ```typescript
        initializeLanguages({
            // ... other languages
            [lang_code]: [lang_code]Translation, // e.g., eu: basqueTranslation, [cite: 678]
        })
        ```
    * **Add to `triggerOverrideTranslations` function**:
        Similarly, add the new language to the `overwriteTranslations` call within this function.
        ```typescript
        overwriteTranslations({
            // ... other languages
            [lang_code]: [lang_code]Translation, // e.g., eu: basqueTranslation, [cite: 678, 679]
        })
        ```
    * **Add to `getAllLangs` function**:
        Add the new language code to the array returned by this function.
        ```typescript
        export const getAllLangs = (): Array<string> => ["en", "es", "cat", /*...,*/ "[lang_code]"] // e.g., "eu" [cite: 679]
        ```

### 1.3. Add Language Name to Existing Translations

For each existing language file in `packages/admin-portal/src/translations/` (e.g., `cat.ts`, `en.ts`, `es.ts`, `fr.ts`, `gl.ts`, `nl.ts`, `tl.ts`):
1.  Open the file.
2.  Locate the `language` object within the `translations.common` object.
3.  Add a new key for your language code, with its name translated into the language of that specific file.
    * Example for `cat.ts` (Catalan adding Basque):
        ```typescript
        language: {
            // ... other languages
            eu: "Euskera", // [lang_code]: "[LangNameInBasque]" [cite: 680]
        },
        ```
    * Example for `en.ts` (English adding Basque):
        ```typescript
        language: {
            // ... other languages
            eu: "Euskera", // [lang_code]: "[LangNameInBasque]" [cite: 682]
        },
        ```
    * Repeat this for `es.ts`[cite: 683], `fr.ts`[cite: 1054], `gl.ts`[cite: 1055], `nl.ts`[cite: 1056], and `tl.ts`[cite: 1058].

## 2. Keycloak Extensions (`packages/keycloak-extensions`)

### 2.1. Message OTP Authenticator

1.  **Create message properties file**:
    * Navigate to `packages/keycloak-extensions/message-otp-authenticator/src/main/resources/theme-resources/messages/`.
    * Create a new file named `messages_[lang_code].properties` (e.g., `messages_eu.properties`). [cite: 1059]
    * This file contains translations for the OTP authenticator. Use an existing file like `messages_en.properties`  as a template. [cite: 1059]
2.  **Add license file for properties**:
    * In the same directory, create a corresponding license file: `messages_[lang_code].properties.license` (e.g., `messages_eu.properties.license`).
    * Copy the content from an existing license file (e.g., `messages_en.properties.license`).

### 2.2. Sequent Theme - Admin Portal - Account Messages

1.  **Update existing language property files**:
    * Navigate to `packages/keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.admin-portal/account/messages/`.
    * For each relevant existing `messages_*.properties` file (e.g., `messages_en.properties`[cite: 1078], `messages_tl.properties` [cite: 1119]):
        * Add a line for the new locale:
            ```properties
            locale_[lang_code]=[LangNameInThatLanguage] // e.g., locale_eu=Euskera [cite: 1078, 1119]
            ```
2.  **Create new language properties file**:
    * In the same directory, create `messages_[lang_code].properties` (e.g., `messages_eu.properties`). [cite: 1079]
    * This file contains translations for the account management theme. Use `messages_en.properties` as a template. [cite: 1079]
3.  **Add license file**:
    * In the same directory, create `messages_[lang_code].properties.license` (e.g., `messages_eu.properties.license`).
    * Copy content from an existing license file.

### 2.3. Sequent Theme - Admin Portal - Login Messages

1.  **Update existing language property files**:
    * Navigate to `packages/keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.admin-portal/login/messages/`.
    * For relevant existing `messages_*.properties` files (e.g., `messages_en.properties`, `messages_gl.properties`, `messages_tl.properties`):
        * Add a line for the new locale:
            ```properties
            locale_[lang_code]=[LangNameInThatLanguage] // e.g., locale_eu=Euskera
            ```
            (This change is shown for `messages_en.properties`, `messages_gl.properties`, `messages_tl.properties` in the diff.)
2.  **Create new language properties file**:
    * In the same directory, create `messages_[lang_code].properties` (e.g., `messages_eu.properties`). [cite: 1120]
    * This file contains translations for the login theme. Use `messages_en.properties`  as a template. [cite: 1120]
3.  **Add license file**:
    * In the same directory, create `messages_[lang_code].properties.license` (e.g., `messages_eu.properties.license`).
    * Copy content from an existing license file.

### 2.4. Update Theme Properties

1.  **Admin Portal Login Theme**:
    * Open `packages/keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.admin-portal/login/theme.properties`.
    * Add the new language code to the `locales` property.
        ```properties
        locales=en,...,[lang_code] // e.g., locales=en,eu
        ```
2.  **Voting Portal Login Theme**:
    * Open `packages/keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.voting-portal/login/theme.properties`. [cite: 1214]
    * Add the new language code to the `locales` property. [cite: 1214]
        ```properties
        locales=en,...,[lang_code] // e.g., locales=en,eu [cite: 1214]
        ```

## 3. UI Core (`packages/ui-core`)

### 3.1. Add Translation File

1.  **Create the translation file**:
    * Navigate to `packages/ui-core/src/translations/`.
    * Create a new file named `[lang_code].ts` (e.g., `eu.ts`). [cite: 1216]
    * This file contains translations shared across UI core components. Use `en.ts` as a template. [cite: 1216]

### 3.2. Update i18n Service

1.  **Edit `packages/ui-core/src/services/i18n.ts`**: [cite: 1215]
    * **Import the new translation file**:
        ```typescript
        import [lang_code]Translation from "../translations/[lang_code]" // e.g., import basqueTranslation from "../translations/eu" [cite: 1215]
        ```
    * **Add to `libTranslations` in `initializeLanguages` function**:
        ```typescript
        const libTranslations: Resource = {
            // ... other languages
            [lang_code]: [lang_code]Translation, // e.g., eu: basqueTranslation, [cite: 1215]
        }
        ```

## 4. UI Essentials (`packages/ui-essentials`)

### 4.1. Add Translation File

1.  **Create the translation file**:
    * Navigate to `packages/ui-essentials/src/translations/`.
    * Create `[lang_code].ts` (e.g., `eu.ts`). [cite: 1238]
    * This file contains translations for essential UI components. Use `en.ts` as a template. [cite: 1238]

### 4.2. Update i18n Service

1.  **Edit `packages/ui-essentials/src/services/i18n.ts`**: [cite: 1237]
    * **Import the new translation file**:
        ```typescript
        import [lang_code]Translation from "../translations/[lang_code]" // e.g., import basqueTranslation from "../translations/eu" [cite: 1237]
        ```
    * **Add to `libTranslations` in `initializeLanguages` function**:
        ```typescript
        const libTranslations: Resource = {
            // ... other languages
            [lang_code]: [lang_code]Translation, // e.g., eu: basqueTranslation, [cite: 1237]
        }
        ```

## 5. Voting Portal (`packages/voting-portal`)

### 5.1. Add Translation File

1.  **Create the translation file**:
    * Navigate to `packages/voting-portal/src/translations/`.
    * Create `[lang_code].ts` (e.g., `eu.ts`). [cite: 1260]
    * This file contains translations for the voting portal. Use `en.ts` as a template. [cite: 1260]

### 5.2. Update i18n Service

1.  **Edit `packages/voting-portal/src/services/i18n.ts`**: [cite: 1259]
    * **Import the new translation file**:
        ```typescript
        import [lang_code]Translation from "../translations/[lang_code]" // e.g., import basqueTranslation from "../translations/eu" [cite: 1259]
        ```
    * **Add to `initializeLanguages` call**:
        Add the new language to the translations object passed to `initializeLanguages`.
        ```typescript
        initializeLanguages(
            {
                // ... other languages
                [lang_code]: [lang_code]Translation, // e.g., eu: basqueTranslation, [cite: 1259]
            },
            language
        )
        ```

---

After completing these steps:
1.  Ensure all newly created `.ts` and `.properties` files are fully translated.
2.  Rebuild and deploy your application.
3.  Thoroughly test all parts of the application in the new language to ensure translations are correctly applied and displayed.