// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {isLanguageEnabled, isTranslatablePresentation, translateFromPresentation} from "./translate"

const presentation = {
    i18n: {
        en: {alias: "english-alias", name: "Election", description: null},
        fr: {name: "French election"},
        cat: {name: "Catalan election"},
    },
    language_conf: {
        default_language_code: "en",
    },
}

describe("translateFromPresentation", () => {
    it("normalizes regional user locales to internal language codes", () => {
        expect(translateFromPresentation({presentation}, "name", "en-GB")).toBe("Election")
        expect(translateFromPresentation({presentation}, "name", "EN-GB")).toBe("Election")
        expect(translateFromPresentation({presentation}, "name", "fr-CA")).toBe("French election")
        expect(translateFromPresentation({presentation}, "name", "cat")).toBe("Catalan election")
    })

    it("falls through null, undefined, and empty translations", () => {
        const presentationWithMissingValues = {
            i18n: {
                en: {name: "", description: null, alias: undefined},
                fr: {name: "French fallback", description: "", alias: null},
            },
        }

        expect(
            translateFromPresentation(presentationWithMissingValues, "name", "en", {
                defaultLanguageCode: "fr",
            })
        ).toBe("French fallback")
        expect(
            translateFromPresentation(presentationWithMissingValues, "description", "en", {
                defaultLanguageCode: "fr",
            })
        ).toBeUndefined()
        expect(
            translateFromPresentation(presentationWithMissingValues, "alias", "en", {
                defaultLanguageCode: "fr",
            })
        ).toBeUndefined()
        expect(
            translateFromPresentation(
                {name: "Legacy fallback", presentation: presentationWithMissingValues},
                "name",
                "en"
            )
        ).toBe("Legacy fallback")
    })

    it("uses an explicitly requested default language", () => {
        expect(
            translateFromPresentation({presentation}, "name", "de-DE", {
                defaultLanguageCode: "en-GB",
            })
        ).toBe("Election")
    })

    it("ignores malformed unrelated translations when the requested value is valid", () => {
        const malformedPresentation = ({
            i18n: {
                en: {name: "Election X", sort_hint: 2},
            },
        } as unknown) as Parameters<typeof translateFromPresentation>[0]

        expect(isTranslatablePresentation(malformedPresentation)).toBe(false)
        expect(translateFromPresentation(malformedPresentation, "name", "en")).toBe("Election X")
    })

    it("does not assume that every presentation owns the fallback policy", () => {
        expect(translateFromPresentation(presentation, "name", "de-DE")).toBeUndefined()
    })

    it("preserves user-language alias and name ordering", () => {
        expect(translateFromPresentation({presentation}, "alias", "fr")).toBeUndefined()
        expect(translateFromPresentation({presentation}, "name", "fr")).toBe("French election")
    })

    it("prefers a direct presentation over a nested presentation", () => {
        expect(
            translateFromPresentation(
                {
                    i18n: {en: {name: "Direct election"}},
                    presentation: {i18n: {en: {name: "Nested election"}}},
                },
                "name",
                "en"
            )
        ).toBe("Direct election")
    })

    it("supports receiving the presentation object directly", () => {
        expect(
            translateFromPresentation(presentation, "name", "de-DE", {
                defaultLanguageCode: "en",
            })
        ).toBe("Election")
    })

    it("preserves the entity value fallback used by legacy callers", () => {
        expect(
            translateFromPresentation({name: "Legacy election", presentation}, "name", "de-DE")
        ).toBe("Legacy election")
    })

    it("prefers a default-language translation over an entity legacy value", () => {
        expect(
            translateFromPresentation({name: "Legacy election", presentation}, "name", "de-DE", {
                defaultLanguageCode: "en",
            })
        ).toBe("Election")
    })

    it("returns undefined when neither translation nor legacy value exists", () => {
        expect(
            translateFromPresentation(
                {
                    presentation: {
                        i18n: {fr: {name: "French election"}},
                    },
                },
                "name",
                "de-DE",
                {defaultLanguageCode: "en"}
            )
        ).toBeUndefined()
    })

    it("preserves the legacy value when no presentation translations exist", () => {
        expect(translateFromPresentation({name: "Election"}, "name", "en-GB")).toBe("Election")
    })
})

describe("isLanguageEnabled", () => {
    it("matches enabled internal language codes", () => {
        expect(isLanguageEnabled("fr", ["en", "fr"])).toBe(true)
        expect(isLanguageEnabled("cat", ["cat"])).toBe(true)
    })

    it("normalizes regional user locales to enabled internal language codes", () => {
        expect(isLanguageEnabled("en-GB", ["en"])).toBe(true)
        expect(isLanguageEnabled("EN-gb", ["en"])).toBe(true)
        expect(isLanguageEnabled("fr-CA", ["en", "fr", "es"])).toBe(true)
    })

    it("rejects languages that are not enabled", () => {
        expect(isLanguageEnabled("de-DE", ["en", "fr"])).toBe(false)
        expect(isLanguageEnabled("es", ["en"])).toBe(false)
        expect(isLanguageEnabled("fr-CA", [])).toBe(false)
    })

    it("rejects a missing language or language list", () => {
        expect(isLanguageEnabled(undefined, ["en"])).toBe(false)
        expect(isLanguageEnabled("", ["en"])).toBe(false)
        expect(isLanguageEnabled("en", undefined)).toBe(false)
    })
})
