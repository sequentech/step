// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {INFORMAL_SUFFIX, splitRegister, withRegister, withRegisterBCP47} from "./registerLocale"
import spanishTranslation from "../translations/es"
import spanishInformalTranslation from "../translations/es-tu"
import catalanTranslation from "../translations/cat"
import catalanInformalTranslation from "../translations/cat-tu"

// mirrors what i18n.ts does once the WASM mapping has run
const normalize = (lang: string) => {
    const {base, informal} = splitRegister(lang.toLowerCase())
    return withRegister(base === "ca" ? "cat" : base, informal)
}
const toTag = (lang: string) => {
    const {base, informal} = splitRegister(lang)
    return withRegisterBCP47(base === "cat" ? "ca" : base, informal)
}

describe("register variant locale codes", () => {
    it("keeps the plain codes formal so existing deployments do not shift register", () => {
        expect(normalize("es")).toBe("es")
        expect(normalize("es-ES")).toBe("es")
        expect(normalize("ca")).toBe("cat")
        expect(normalize("ca-ES")).toBe("cat")
        expect(normalize("cat")).toBe("cat")
    })

    it("resolves the informal variant from both spellings of the marker", () => {
        expect(normalize("es-tu")).toBe("es-tu")
        expect(normalize("cat-tu")).toBe("cat-tu")
        // the private-use tag the DOM carries
        expect(normalize("es-x-tu")).toBe("es-tu")
        expect(normalize("ca-x-tu")).toBe("cat-tu")
        expect(normalize("es-ES-x-tu")).toBe("es-tu")
        expect(normalize("CA-X-TU")).toBe("cat-tu")
    })

    it("never mistakes a region subtag for a register", () => {
        expect(normalize("es-MX")).toBe("es")
        expect(normalize("ca-ES-valencia")).toBe("cat")
        expect(splitRegister("es-MX").informal).toBe(false)
    })

    it("puts the register in a private-use sequence, keeping the tag well-formed", () => {
        expect(toTag("es")).toBe("es")
        expect(toTag("cat")).toBe("ca")
        expect(toTag("es-tu")).toBe(`es-x-${INFORMAL_SUFFIX}`)
        expect(toTag("cat-tu")).toBe(`ca-x-${INFORMAL_SUFFIX}`)
    })

    it("round-trips every internal code through the BCP 47 tag and back", () => {
        for (const code of ["en", "es", "cat", "eu", "gl", "nl", "tl", "fr", "es-tu", "cat-tu"]) {
            expect(normalize(toTag(code))).toBe(code)
        }
    })
})

describe("register variant bundles", () => {
    const flatten = (obj: object, prefix = "", out: Record<string, string> = {}) => {
        for (const [k, v] of Object.entries(obj)) {
            const key = prefix ? `${prefix}.${k}` : k
            if (v && typeof v === "object") flatten(v as object, key, out)
            else if (typeof v === "string") out[key] = v
        }
        return out
    }
    const pairs: Array<[string, object, object]> = [
        ["es", spanishTranslation, spanishInformalTranslation],
        ["cat", catalanTranslation, catalanInformalTranslation],
    ]

    it.each(pairs)("%s variant is a complete bundle, not an overlay", (_lang, base, variant) => {
        // a missing key would drop the voter back into the other register mid-screen
        expect(Object.keys(flatten(variant)).sort()).toEqual(Object.keys(flatten(base)).sort())
    })

    it.each(pairs)("%s variant keeps every interpolation placeholder", (_lang, base, variant) => {
        const b = flatten(base)
        const v = flatten(variant)
        const vars = (s: string) =>
            [...new Set([...s.matchAll(/\{\{\s*([^}]+?)\s*\}\}/g)].map((m) => m[1]))].sort()
        for (const key of Object.keys(b)) {
            expect({key, vars: vars(v[key])}).toEqual({key, vars: vars(b[key])})
        }
    })

    it.each(pairs)(
        "%s variant names itself distinctly in the language picker",
        (_lang, base, variant) => {
            // SettingsLanguages renders t("language", {lng}) per bundle; identical
            // labels would give the operator two indistinguishable entries
            expect(flatten(variant)["translations.language"]).not.toBe(
                flatten(base)["translations.language"]
            )
        }
    )

    it("actually differs where the register differs", () => {
        const b = flatten(spanishTranslation)
        const v = flatten(spanishInformalTranslation)
        const differing = Object.keys(b).filter((k) => b[k] !== v[k])
        // the split is substantial, not a token gesture
        expect(differing.length).toBeGreaterThan(15)
        expect(v["translations.header.session.timeLeft"]).toContain("tu voto")
        expect(b["translations.header.session.timeLeft"]).toContain("su voto")
    })
})
