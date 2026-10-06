// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {describe, expect, it} from "@jest/globals"
import {
    composeTranslationOverrideKey,
    ETranslationScope,
    filterTranslationOverrides,
    parseTranslationOverrideKey,
    updateTranslationOverride,
} from "./translationScopes"

const overrides = {
    en: {
        "global:footer.poweredBy": "Global footer",
        "votingPortal:confirmationScreen.title": "Voting title",
        "ballotVerifier:confirmationScreen.title": "Verifier title",
        "resultsPortal:resultsPortal.pageTitle": "Results title",
        "adminPortal:header.title": "Admin title",
        "confirmationScreen.title": "Legacy title",
        "unknown:confirmationScreen.title": "Unknown prefix",
    },
}

describe("translation override scopes", () => {
    it("strips the caller's scope and applies global overrides", () => {
        expect(filterTranslationOverrides(overrides, ETranslationScope.BALLOT_VERIFIER)).toEqual({
            en: {
                "footer.poweredBy": "Global footer",
                "confirmationScreen.title": "Verifier title",
            },
        })
    })

    it("skips overrides for other known scopes", () => {
        const filtered = filterTranslationOverrides(overrides, ETranslationScope.RESULTS_PORTAL)

        expect(filtered?.en["confirmationScreen.title"]).toBeUndefined()
        expect(filtered?.en["resultsPortal.pageTitle"]).toBe("Results title")
    })

    it("applies unprefixed and unknown-prefix keys only for the declared legacy scope", () => {
        expect(
            filterTranslationOverrides(
                overrides,
                ETranslationScope.VOTING_PORTAL,
                ETranslationScope.VOTING_PORTAL
            )
        ).toEqual({
            en: {
                "footer.poweredBy": "Global footer",
                "confirmationScreen.title": "Voting title",
                "unknown:confirmationScreen.title": "Unknown prefix",
            },
        })

        expect(
            filterTranslationOverrides(
                overrides,
                ETranslationScope.VOTING_PORTAL,
                ETranslationScope.ADMIN_PORTAL
            )?.en["unknown:confirmationScreen.title"]
        ).toBeUndefined()
    })

    it("gives own-scope overrides precedence over legacy and global values", () => {
        const collisions = {
            en: {
                "votingPortal:shared.key": "Own",
                "shared.key": "Legacy",
                "global:shared.key": "Global",
            },
        }

        expect(
            filterTranslationOverrides(
                collisions,
                ETranslationScope.VOTING_PORTAL,
                ETranslationScope.VOTING_PORTAL
            )?.en["shared.key"]
        ).toBe("Own")
    })

    it("is a no-op when translations are absent", () => {
        expect(
            filterTranslationOverrides(undefined, ETranslationScope.RESULTS_PORTAL)
        ).toBeUndefined()
    })

    it("parses known scopes and preserves unknown prefixes as legacy keys", () => {
        expect(parseTranslationOverrideKey("resultsPortal:page.title")).toEqual({
            scope: ETranslationScope.RESULTS_PORTAL,
            key: "page.title",
        })
        expect(parseTranslationOverrideKey("typo:page.title")).toEqual({
            key: "typo:page.title",
        })
        expect(
            composeTranslationOverrideKey("global:page.title", ETranslationScope.ADMIN_PORTAL)
        ).toBe("adminPortal:page.title")
    })

    it("promotes a legacy row without retaining the unprefixed key", () => {
        expect(
            updateTranslationOverride(
                {"page.title": "Legacy", "untouched": "Keep"},
                "page.title",
                ETranslationScope.VOTING_PORTAL,
                "Updated",
                "page.title"
            )
        ).toEqual({
            "votingPortal:page.title": "Updated",
            "untouched": "Keep",
        })
    })

    it("rejects a scope change that would overwrite an existing row", () => {
        const translations = {
            "page.title": "Legacy",
            "resultsPortal:page.title": "Existing results title",
        }

        expect(
            updateTranslationOverride(
                translations,
                "page.title",
                ETranslationScope.RESULTS_PORTAL,
                "Replacement",
                "page.title"
            )
        ).toBeUndefined()
        expect(translations["resultsPortal:page.title"]).toBe("Existing results title")
    })

    it("rejects prefix-only and whitespace-only logical keys", () => {
        const translations = {untouched: "Keep"}

        expect(
            updateTranslationOverride(
                translations,
                "global:",
                ETranslationScope.VOTING_PORTAL,
                "Ignored"
            )
        ).toBeUndefined()
        expect(
            updateTranslationOverride(
                translations,
                "   ",
                ETranslationScope.ADMIN_PORTAL,
                "Ignored"
            )
        ).toBeUndefined()
        expect(translations).toEqual({untouched: "Keep"})
    })
})
