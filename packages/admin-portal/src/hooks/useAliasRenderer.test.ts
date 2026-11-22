// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {renderAlias, translateWithFallback} from "./useAliasRenderer"

jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../ui-core/src/services/translate")
)
jest.mock("react-i18next", () => ({
    useTranslation: () => ({i18n: {language: "en"}}),
}))

const frenchOnly = {
    id: "election-1",
    presentation: {
        language_conf: {enabled_language_codes: ["fr"], default_language_code: "fr"},
        i18n: {fr: {name: "Élection municipale", alias: "Municipale"}},
    },
}

describe("renderAlias", () => {
    it("uses the UI language when the item has it", () => {
        const item = {
            presentation: {i18n: {es: {alias: "Municipales"}, fr: {alias: "Municipale"}}},
        }
        expect(renderAlias(item, "es")).toBe("Municipales")
    })

    it("falls back to the item's own default language", () => {
        expect(renderAlias(frenchOnly, "es")).toBe("Municipale")
    })

    it("prefers the caller's default language over the item's", () => {
        const item = {
            presentation: {
                language_conf: {default_language_code: "fr"},
                i18n: {fr: {alias: "Municipale"}, ca: {alias: "Municipals"}},
            },
        }
        expect(renderAlias(item, "es", "ca")).toBe("Municipals")
    })

    it("prefers English over an arbitrary available language", () => {
        const item = {presentation: {i18n: {fr: {alias: "Municipale"}, en: {alias: "Municipal"}}}}
        expect(renderAlias(item, "es")).toBe("Municipal")
    })

    it("falls back to any available language without a default", () => {
        const item = {presentation: {i18n: {fr: {name: "Élection municipale"}}}}
        expect(renderAlias(item, "es")).toBe("Élection municipale")
    })

    it("accepts a bare presentation object", () => {
        expect(renderAlias(frenchOnly.presentation, "es")).toBe("Municipale")
    })

    it("normalizes regional UI languages", () => {
        expect(renderAlias(frenchOnly, "fr-CA")).toBe("Municipale")
    })

    it("accepts a JSON-encoded presentation", () => {
        expect(renderAlias(JSON.stringify(frenchOnly.presentation), "es")).toBe("Municipale")
    })

    it("prefers a translation in a fallback language over legacy fields", () => {
        expect(renderAlias({...frenchOnly, name: "Legacy name"}, "es")).toBe("Municipale")
    })

    it("falls back to legacy fields when no language has a value", () => {
        const item = {name: "Legacy name", presentation: {i18n: {fr: {alias: ""}}}}
        expect(renderAlias(item, "es")).toBe("Legacy name")
        expect(renderAlias({alias: "Legacy alias", name: "Legacy name"}, "es")).toBe("Legacy alias")
    })

    it("returns a dash only when no language has a value", () => {
        expect(renderAlias({presentation: {i18n: {fr: {alias: ""}}}}, "es")).toBe("-")
        expect(renderAlias(null, "es")).toBe("-")
        expect(renderAlias("", "es")).toBe("-")
        expect(renderAlias("{not json", "es")).toBe("-")
        expect(renderAlias("plain text", "es")).toBe("-")
    })
})

describe("translateWithFallback", () => {
    it("translates a single key through the same fallback chain", () => {
        expect(translateWithFallback(frenchOnly, ["name"], "es")).toBe("Élection municipale")
    })

    it("returns undefined when nothing matches", () => {
        expect(translateWithFallback({presentation: {i18n: {}}}, ["name"], "es")).toBeUndefined()
    })
})
