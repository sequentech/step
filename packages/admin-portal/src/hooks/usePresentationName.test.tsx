/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {renderHook} from "@testing-library/react"
import {configuredLanguage, useInheritedLanguage, usePresentationName} from "./usePresentationName"
import {useAliasRenderer} from "./useAliasRenderer"

jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../ui-core/src/services/translate")
)

let mockLanguage = "en"
jest.mock("react-i18next", () => ({useTranslation: () => ({i18n: {language: mockLanguage}})}))

const mockRecords: Record<string, unknown> = {}
jest.mock("react-admin", () => ({
    useGetOne: (resource: string, {id}: {id: string}, {enabled}: {enabled: boolean}) => ({
        data: enabled ? mockRecords[`${resource}/${id}`] : undefined,
    }),
}))

const entity = (
    i18n: Record<string, Record<string, string>>,
    defaultLanguage?: string
): {presentation: Record<string, unknown>} => ({
    presentation: {
        i18n,
        ...(defaultLanguage ? {language_conf: {default_language_code: defaultLanguage}} : {}),
    },
})

const render = (language: string) => {
    mockLanguage = language
    const name = renderHook(() => usePresentationName()).result.current
    const alias = renderHook(() => useAliasRenderer()).result.current
    return {name, alias}
}

describe("names in the interface language, the configured default language, then English", () => {
    it("keeps an English-default record unchanged", () => {
        const record = entity({en: {name: "Council"}, es: {name: "Consejo"}}, "en")
        expect(render("en").name(record)).toBe("Council")
        expect(render("en").alias(record)).toBe("Council")
        expect(render("es").name(record)).toBe("Consejo")
        expect(render("es-MX").alias(record)).toBe("Consejo")
    })

    it("shows a Spanish-default record's name to an English interface", () => {
        const record = entity({en: {}, es: {name: "Consejo"}, tl: {name: "Pangalan"}}, "es")
        expect(render("en").name(record)).toBe("Consejo")
        expect(render("en").alias(record)).toBe("Consejo")
        expect(render("tl").alias(record)).toBe("Pangalan")
    })

    it("prefers an existing translation in the interface language over the default", () => {
        const record = entity({en: {name: "Council"}, es: {name: "Consejo"}}, "es")
        expect(render("en-US").name(record)).toBe("Council")
        expect(render("en").alias(record)).toBe("Council")
    })

    it("falls back to the configured default for a missing interface language", () => {
        const record = entity({es: {name: "Consejo"}}, "es")
        expect(render("fr").name(record)).toBe("Consejo")
        expect(render("fr-CA").alias(record)).toBe("Consejo")
    })

    it("still reaches an English name when the default language has none", () => {
        const migrated = entity({en: {name: "Council"}}, "es")
        expect(render("fr").name(migrated)).toBe("Council")
        expect(render("fr").alias(migrated)).toBe("Council")
    })

    it("uses a default the caller passes before the record's own", () => {
        const record = entity({es: {name: "Consejo"}, tl: {name: "Pangalan"}}, "es")
        expect(render("en").alias(record, "tl")).toBe("Pangalan")
        expect(render("en").alias(record.presentation, "es")).toBe("Consejo")
    })

    it("renders '-' and an empty name only when no language has one", () => {
        const record = entity({es: {}}, "es")
        expect(render("en").alias(record)).toBe("-")
        expect(render("en").name(record)).toBe("")
    })
})

describe("configuredLanguage", () => {
    it("reads a record or its presentation", () => {
        expect(configuredLanguage(entity({}, "es"))).toBe("es")
        expect(configuredLanguage(entity({}, "es").presentation)).toBe("es")
        expect(configuredLanguage(entity({}))).toBeUndefined()
        expect(
            configuredLanguage({presentation: {language_conf: {default_language_code: ""}}})
        ).toBeUndefined()
        expect(configuredLanguage(null)).toBeUndefined()
    })
})

describe("useInheritedLanguage", () => {
    beforeEach(() => {
        for (const key of Object.keys(mockRecords)) delete mockRecords[key]
    })

    it("takes the election's default, else the event's", () => {
        mockRecords["sequent_backend_election/e1"] = entity({}, "tl")
        mockRecords["sequent_backend_election_event/ev"] = entity({}, "es")
        const inherited = (record: object) =>
            renderHook(() => useInheritedLanguage(record)).result.current
        expect(inherited({election_id: "e1", election_event_id: "ev"})).toBe("tl")
        expect(inherited({election_event_id: "ev"})).toBe("es")
        expect(inherited({election_id: "missing", election_event_id: "ev"})).toBe("es")
        expect(inherited({})).toBeUndefined()
    })

    it("lets a contest heading show its election's language", () => {
        mockRecords["sequent_backend_election/e1"] = entity({}, "es")
        const contest = {election_id: "e1", ...entity({en: {}, es: {name: "Escaño"}})}
        const inherited = renderHook(() => useInheritedLanguage(contest)).result.current
        expect(render("en").alias(contest)).toBe("-")
        expect(render("en").alias(contest, configuredLanguage(contest) ?? inherited)).toBe("Escaño")
    })
})
