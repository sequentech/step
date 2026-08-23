// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {beforeAll, describe, expect, it, jest} from "@jest/globals"
import i18n, {initializeLanguages, overwriteTranslations} from "./i18n"
import {ETranslationScope} from "./translationScopes"
import {ELanguageDetectionPolicy} from "../types/LanguageConf"

jest.mock("sequent-core", () => ({
    iso_639_2t_to_bcp47_js: (language: string) => language,
    locale_to_internal_language_code_js: (language: string) => language,
}))

const resultsOptions = {
    scope: ETranslationScope.RESULTS_PORTAL,
    changeDefaultLanguage: false,
} as const

describe("overwriteTranslations", () => {
    beforeAll(() => {
        initializeLanguages(
            {
                en: {
                    translations: {
                        page: {
                            baseOnly: "Base only",
                            title: "Base title",
                        },
                    },
                },
            },
            "en"
        )
    })

    it("restores the base layer and removes stale keys when replacing a portal config", () => {
        overwriteTranslations(
            {
                i18n: {
                    en: {
                        "resultsPortal:page.baseOnly": "Event A base override",
                        "resultsPortal:page.eventOnly": "Event A only",
                        "resultsPortal:page.title": "Event A title",
                    },
                },
            },
            resultsOptions
        )

        expect(i18n.t("page.baseOnly")).toBe("Event A base override")
        expect(i18n.t("page.eventOnly")).toBe("Event A only")

        overwriteTranslations(
            {
                i18n: {
                    en: {
                        "resultsPortal:page.title": "Event B title",
                    },
                },
            },
            resultsOptions
        )

        expect(i18n.t("page.title")).toBe("Event B title")
        expect(i18n.t("page.baseOnly")).toBe("Base only")
        expect(i18n.exists("page.eventOnly")).toBe(false)
    })

    it("clears the current portal layer when the next config has no translations", () => {
        overwriteTranslations(undefined, resultsOptions)

        expect(i18n.t("page.title")).toBe("Base title")
    })

    it("restores nested base resources when parent and child keys are both overridden", () => {
        overwriteTranslations(
            {
                i18n: {
                    en: {
                        "resultsPortal:page": "Event parent",
                        "resultsPortal:page.title": "Event child",
                    },
                },
            },
            resultsOptions
        )

        overwriteTranslations(undefined, resultsOptions)

        expect(i18n.t("page.title")).toBe("Base title")
        expect(i18n.t("page.baseOnly")).toBe("Base only")
    })

    it("restores nested resources when a child override is applied before its parent", () => {
        overwriteTranslations(
            {
                i18n: {
                    en: {
                        "resultsPortal:page.title": "Event child",
                        "resultsPortal:page": "Event parent",
                    },
                },
            },
            resultsOptions
        )

        overwriteTranslations(undefined, resultsOptions)

        expect(i18n.t("page.title")).toBe("Base title")
        expect(i18n.t("page.baseOnly")).toBe("Base only")
    })

    it("keeps scoped parent/child overlays over legacy writes and reveals the new base on cleanup", () => {
        overwriteTranslations(
            {
                i18n: {
                    en: {
                        "resultsPortal:page": "Scoped parent",
                        "resultsPortal:page.title": "Scoped child",
                    },
                },
            },
            resultsOptions
        )

        overwriteTranslations({i18n: {en: {"page.title": "Legacy title"}}}, false)

        expect(i18n.t("page.title")).toBe("Scoped child")

        overwriteTranslations(undefined, resultsOptions)

        expect(i18n.t("page.title")).toBe("Legacy title")
        expect(i18n.t("page.baseOnly")).toBe("Base only")
    })

    it("preserves newer overlapping scopes when replacing and removing an older layer", () => {
        const baseTitle = i18n.t("page.title")
        const votingOptions = {
            scope: ETranslationScope.VOTING_PORTAL,
            changeDefaultLanguage: false,
        } as const

        overwriteTranslations({i18n: {en: {"resultsPortal:page.title": "Results"}}}, resultsOptions)
        overwriteTranslations({i18n: {en: {"votingPortal:page.title": "Voting"}}}, votingOptions)
        overwriteTranslations(
            {i18n: {en: {"resultsPortal:page.title": "Updated results"}}},
            resultsOptions
        )

        expect(i18n.t("page.title")).toBe("Voting")

        overwriteTranslations(undefined, resultsOptions)
        expect(i18n.t("page.title")).toBe("Voting")

        overwriteTranslations(undefined, votingOptions)
        expect(i18n.t("page.title")).toBe(baseTitle)
    })

    it("preserves the legacy boolean and omitted-argument API", () => {
        const legacyConfig = {
            i18n: {en: {"page.title": "Legacy title"}},
            language_conf: {
                language_detection_policy: ELanguageDetectionPolicy.FORCE_DEFAULT,
                default_language_code: "es",
            },
        }
        const originalDocument = (globalThis as any).document
        ;(globalThis as any).document = {
            cookie: "",
            documentElement: {setAttribute: jest.fn()},
        }

        try {
            expect(overwriteTranslations(legacyConfig, false)).toBe(false)
            expect(i18n.t("page.title")).toBe("Legacy title")
            expect(i18n.language).toBe("en")

            expect(overwriteTranslations(legacyConfig)).toBe(true)
            expect(i18n.language).toBe("es")
        } finally {
            if (originalDocument === undefined) {
                delete (globalThis as any).document
            } else {
                ;(globalThis as any).document = originalDocument
            }
        }
    })
})
