// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    ETranslationScope,
    parseTranslationOverrideKey,
} from "../../../../ui-core/src/services/translationScopes"
import uiCoreCatalan from "../../../../ui-core/src/translations/cat"
import uiCoreSpanish from "../../../../ui-core/src/translations/es"
import votingPortalCatalan from "../../../../voting-portal/src/translations/cat"
import votingPortalSpanish from "../../../../voting-portal/src/translations/es"
import ballotVerifierCatalan from "../../../../ballot-verifier/src/translations/cat"
import ballotVerifierSpanish from "../../../../ballot-verifier/src/translations/es"
import resultsPortalCatalan from "../../../../results-portal/src/translations/cat"
import resultsPortalSpanish from "../../../../results-portal/src/translations/es"
import {TRANSLATION_PRESET_TEMPLATES} from "."

type Bundle = {translations: Record<string, unknown>}

// The bundle each scope's overrides are applied over. Global overrides reach
// every voter-facing portal, and the strings they share come from ui-core.
const BUNDLES: Record<string, Partial<Record<ETranslationScope, Bundle>>> = {
    es: {
        [ETranslationScope.GLOBAL]: uiCoreSpanish,
        [ETranslationScope.VOTING_PORTAL]: votingPortalSpanish,
        [ETranslationScope.BALLOT_VERIFIER]: ballotVerifierSpanish,
        [ETranslationScope.RESULTS_PORTAL]: resultsPortalSpanish,
    },
    cat: {
        [ETranslationScope.GLOBAL]: uiCoreCatalan,
        [ETranslationScope.VOTING_PORTAL]: votingPortalCatalan,
        [ETranslationScope.BALLOT_VERIFIER]: ballotVerifierCatalan,
        [ETranslationScope.RESULTS_PORTAL]: resultsPortalCatalan,
    },
}

const lookup = (bundle: Bundle, key: string): unknown =>
    key
        .split(".")
        .reduce<unknown>(
            (node, part) =>
                typeof node === "object" && node !== null
                    ? (node as Record<string, unknown>)[part]
                    : undefined,
            bundle.translations
        )

// Placeholders and tags are what the code fills in or renders, so a template
// text must carry exactly the ones of the text it replaces.
const markers = (text: string): string[] =>
    (text.match(/\{\{[^}]+\}\}|<\/?[\w-]+\s*\/?>/g) ?? []).map((m) => m.replace(/\s+/g, "")).sort()

const baseText = (language: string, storedKey: string): string | undefined => {
    const {key, scope} = parseTranslationOverrideKey(storedKey)
    const bundle = scope ? BUNDLES[language]?.[scope] : undefined
    const value = bundle ? lookup(bundle, key) : undefined
    return typeof value === "string" ? value : undefined
}

describe.each(Object.values(TRANSLATION_PRESET_TEMPLATES))("$id template", (template) => {
    const entries = Object.entries(template.overrides)

    it("targets a language whose bundles are checked", () => {
        expect(BUNDLES[template.language]).toBeDefined()
    })

    it("only overrides keys the portal of each scope ships", () => {
        const unknown = entries
            .filter(([storedKey]) => baseText(template.language, storedKey) === undefined)
            .map(([storedKey]) => storedKey)

        expect(unknown).toEqual([])
    })

    it("keeps the placeholders and tags of the text it replaces", () => {
        const mismatched = entries
            .filter(([storedKey, text]) => {
                const base = baseText(template.language, storedKey)
                return base !== undefined && markers(text).join() !== markers(base).join()
            })
            .map(([storedKey]) => storedKey)

        expect(mismatched).toEqual([])
    })
})

describe("markers", () => {
    it("tells a dropped placeholder or tag apart, whatever the order", () => {
        expect(markers("Select {{count}} more <b>now</b>")).not.toEqual(
            markers("Select more <b>now</b>")
        )
        expect(markers("<b>{{name}}</b>")).toEqual(markers("{{name}} <b></b>"))
    })
})

describe("baseText", () => {
    it("rejects an unscoped key, an unknown scope and a key the bundle lacks", () => {
        expect(baseText("es", "startScreen.step1Title")).toBeUndefined()
        expect(baseText("es", "adminPortal:startScreen.step1Title")).toBeUndefined()
        expect(baseText("es", "votingPortal:startScreen.noSuchKey")).toBeUndefined()
        expect(baseText("fr", "votingPortal:startScreen.step1Title")).toBeUndefined()
    })

    it("finds a key in the bundle of its scope", () => {
        expect(baseText("es", "votingPortal:startScreen.step1Title")).toEqual(expect.any(String))
    })
})
