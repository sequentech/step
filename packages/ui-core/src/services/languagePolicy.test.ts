/**
 * @jest-environment jsdom
 * @jest-environment-options {"url":"https://voting.example.org/"}
 */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {afterEach, beforeEach, expect, it, jest} from "@jest/globals"
import * as backend from "sequent-core"
import i18n, {
    initializeLanguages,
    normalizeLanguageCode,
    toBCP47,
    getLanguages,
    applyLanguagePolicy,
    applyConfigurationLanguagePolicy,
    overwriteTranslations,
    USER_LANGUAGE_COOKIE_NAME,
} from "./i18n"
import {setCookie} from "../utils/cookies"
import {ELanguageDetectionPolicy} from "../types/LanguageConf"
import {ETranslationScope} from "./translationScopes"

jest.mock("sequent-core", () => ({
    locale_to_internal_language_code_js: jest.fn(),
    iso_639_2t_to_bcp47_js: jest.fn(),
}))

const forcedFrench = {
    language_detection_policy: ELanguageDetectionPolicy.FORCE_DEFAULT,
    default_language_code: "fr",
}

beforeEach(() => {
    jest.spyOn(console, "log").mockImplementation(() => undefined)
    jest.spyOn(console, "info").mockImplementation(() => undefined)
    jest.mocked(backend.locale_to_internal_language_code_js).mockImplementation((value: string) =>
        value.toLowerCase()
    )
    jest.mocked(backend.iso_639_2t_to_bcp47_js).mockImplementation((value: string) => value)
    window.history.replaceState({}, "", "/")
    setCookie(USER_LANGUAGE_COOKIE_NAME, "")
    initializeLanguages({}, "en")
})

afterEach(() => {
    i18n.off("languageChanged")
    jest.restoreAllMocks()
    jest.resetAllMocks()
})

it("normalizes external locales even before WASM is available", () => {
    const unavailable = () => {
        throw new Error("WASM not loaded yet")
    }
    jest.mocked(backend.locale_to_internal_language_code_js).mockImplementation(unavailable)
    expect(normalizeLanguageCode(undefined)).toBeUndefined()
    expect(normalizeLanguageCode("CA-ES")).toBe("cat")
    expect(normalizeLanguageCode("FR-CA")).toBe("fr")
    jest.mocked(backend.iso_639_2t_to_bcp47_js).mockImplementation(unavailable)
    expect(toBCP47("cat")).toBe("cat")
})

it("normalizes language and region casing for the document's screen-reader language", async () => {
    expect(toBCP47("FR-ca")).toBe("fr-CA")
    expect(toBCP47("EN")).toBe("en")
    expect(toBCP47("ZH-Hant-TW")).toBe("zh-hant-tw")
    expect(document.documentElement.lang).toBe("en")
    await i18n.changeLanguage("fr")
    expect(document.documentElement.lang).toBe("fr")
    expect(getLanguages(i18n)).toEqual(expect.arrayContaining(["en", "fr", "cat"]))
})

it("uses configured defaults only when a force policy and a language are both present", () => {
    for (const unforced of [
        undefined,
        {},
        {language_detection_policy: ELanguageDetectionPolicy.BROWSER_DETECT},
        {language_detection_policy: ELanguageDetectionPolicy.FORCE_DEFAULT},
    ]) {
        expect(applyLanguagePolicy(unforced)).toBe(false)
    }
    expect(applyLanguagePolicy(forcedFrench)).toBe(true)
    expect(i18n.language).toBe("fr")
})

it("gives an explicit URL precedence over the saved cookie and forced event language", () => {
    setCookie(USER_LANGUAGE_COOKIE_NAME, "es")
    window.history.replaceState({}, "", "/?lang=en")
    expect(applyConfigurationLanguagePolicy({language_conf: forcedFrench})).toBe(false)
    expect(i18n.language).toBe("en")
})

it("uses the saved user language before the forced event language", () => {
    expect(applyConfigurationLanguagePolicy(undefined)).toBe(false)
    setCookie(USER_LANGUAGE_COOKIE_NAME, "es")
    expect(applyConfigurationLanguagePolicy({language_conf: forcedFrench})).toBe(true)
    expect(i18n.language).toBe("es")
    setCookie(USER_LANGUAGE_COOKIE_NAME, "")
    expect(applyConfigurationLanguagePolicy({language_conf: forcedFrench})).toBe(true)
    expect(i18n.language).toBe("fr")
})

it("applies scoped translations and their default-language policy together", () => {
    const changed = overwriteTranslations(
        {
            i18n: {fr: {"votingPortal:page.title": "Conseil"}},
            language_conf: forcedFrench,
        },
        {scope: ETranslationScope.VOTING_PORTAL}
    )
    expect(changed).toBe(true)
    expect(i18n.language).toBe("fr")
    expect(i18n.t("page.title")).toBe("Conseil")
})

it("initializes browser language detection when no language was supplied", () => {
    initializeLanguages({en: {translations: {ready: "Ready"}}})
    expect(getLanguages(i18n)).toContain("en")
    expect(document.documentElement.lang).not.toBe("")
})
