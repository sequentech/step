// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import en from "@/translations/en"
import es from "@/translations/es"
import ca from "@/translations/cat"
import eu from "@/translations/eu"
import fr from "@/translations/fr"
import gl from "@/translations/gl"
import nl from "@/translations/nl"
import tl from "@/translations/tl"

type Tree = {[key: string]: string | Tree}

const leaves = (tree: Tree, prefix = ""): Array<[string, string]> =>
    Object.entries(tree).flatMap(([key, value]) =>
        typeof value === "string"
            ? [[`${prefix}${key}`, value] as [string, string]]
            : leaves(value, `${prefix}${key}.`)
    )

const section = (translation: typeof en) =>
    Object.fromEntries(leaves(translation.translations.sentMessages as unknown as Tree))

const variables = (text: string): string[] =>
    Array.from(text.matchAll(/\{\{\s*(\w+)\s*\}\}/g), (match) => match[1]).sort()

// The same as in English: a format name, and words a language shares.
const SAME_EVERYWHERE = ["email.html"]
const SAME_IN: Record<string, string[]> = {
    fr: ["column.message", "content.code"],
    nl: ["content.code"],
    // "Channel" and "Code" are kept as loanwords, as in the rest of the Tagalog texts.
    tl: ["column.channel", "content.code"],
}

const LANGUAGES = {es, ca, eu, fr, gl, nl, tl}

describe("Sent messages translations", () => {
    const english = section(en)

    it.each(Object.entries(LANGUAGES))("%s translates every text", (language, translation) => {
        const texts = section(translation)
        expect(Object.keys(texts).sort()).toEqual(Object.keys(english).sort())
        for (const [key, text] of Object.entries(texts)) {
            expect([key, variables(text)]).toEqual([key, variables(english[key])])
            if (!SAME_EVERYWHERE.includes(key) && !SAME_IN[language]?.includes(key)) {
                expect([key, text]).not.toEqual([key, english[key]])
            }
        }
    })

    it("names no client", () => {
        for (const translation of [en, ...Object.values(LANGUAGES)]) {
            expect(JSON.stringify(section(translation))).not.toMatch(/COMELEC/i)
        }
    })
})
