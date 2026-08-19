// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {beforeAll, describe, expect, it, jest} from "@jest/globals"
import i18n, {initializeLanguages, overwriteTranslations} from "./i18n"
import {ETranslationScope} from "./translationScopes"

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
})
