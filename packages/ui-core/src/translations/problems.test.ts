// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The problem catalog against the core that raises the problems.
 *
 * `sequent-core` names each complaint (`.id("area.duplicate-name")`) and every
 * front end that shows one — the Admin Portal's imports, the Election
 * Architect — looks the sentence up here by that name. A name raised with no
 * entry falls back to the core's English, which is the failure this catches
 * before a Basque administrator does. Read off the Rust source rather than a
 * report because no fixture triggers every check.
 */

import {describe, expect, it} from "@jest/globals"
import {readdirSync, readFileSync} from "node:fs"
import {join} from "node:path"

import cat from "./cat"
import en from "./en"
import es from "./es"
import eu from "./eu"
import fr from "./fr"
import gl from "./gl"
import nl from "./nl"
import tl from "./tl"

const locales = {en, es, cat, eu, fr, gl, nl, tl}

const CORE = join(__dirname, "../../../sequent-core/src/election_config")

/** Every `.id("…")` in the core's production code, tests excluded. */
const raisedIds = (): Set<string> => {
    const ids = new Set<string>()
    for (const file of readdirSync(CORE)) {
        if (!file.endsWith(".rs") || file.endsWith("_tests.rs")) {
            continue
        }
        const source = readFileSync(join(CORE, file), "utf8").split(
            /#\[cfg\(test\)\]\s*mod tests/
        )[0]!
        for (const match of source.matchAll(/\.id\("([^"]+)"\)/g)) {
            ids.add(match[1]!)
        }
    }
    return ids
}

/**
 * One complaint's sentence. A sentence that counts something (`count` in the
 * core's details) may carry i18next's singular form beside it: `text_one`, and a
 * `lead_one` when the lead inflects too. The unsuffixed pair is every other count.
 */
type Entry = {lead: string; text: string; lead_one?: string; text_one?: string}
type Messages = Record<string, Record<string, Entry>>

const entries = (messages: Messages): Map<string, Entry> => {
    const all = new Map<string, Entry>()
    for (const [area, complaints] of Object.entries(messages)) {
        for (const [complaint, entry] of Object.entries(complaints)) {
            all.set(`${area}.${complaint}`, entry)
        }
    }
    return all
}

const placeholders = (text: string): string[] =>
    [...new Set([...text.matchAll(/\{\{(\w+)\}\}/g)].map((match) => match[1]!))].sort()

/**
 * A label, not a sentence: the lead is what a row underlines, and a link somebody
 * has to finish reading before deciding to click is not a link. English is held to
 * the Election Architect's original bound; the other languages say the same label
 * in more letters — "circumscripció", "boto-txartel" — so get a looser one that
 * still refuses a whole clause.
 */
const leadFits = (lead: string, strict: boolean): boolean => {
    const words = lead.split(/\s+/).length
    return (
        lead.trim() !== "" &&
        (strict ? lead.length <= 36 && words <= 5 : lead.length <= 56 && words <= 8)
    )
}

describe("problem catalog", () => {
    const raised = raisedIds()
    const english = entries(en.translations.problems.messages)

    it("finds the core's problem names", () => {
        // A guard on the scan itself: an empty set would pass everything below.
        expect(raised.size).toBeGreaterThan(100)
        expect(raised).toContain("file.not-json")
        expect(raised).toContain("voters.vote-weight-out-of-range")
    })

    it("has an English sentence for every problem the core names", () => {
        const missing = [...raised].filter((id) => !english.has(id))
        expect(missing).toEqual([])
    })

    it("keeps no sentence for a problem the core no longer raises", () => {
        const stale = [...english.keys()].filter((id) => !raised.has(id))
        expect(stale).toEqual([])
    })

    it.each(Object.entries(locales))(
        "%s: every text opens with its lead and names what English names",
        (language, locale) => {
            const strict = language === "en"
            const own = entries(locale.translations.problems.messages)
            expect([...own.keys()].sort()).toEqual([...english.keys()].sort())
            for (const [id, entry] of own) {
                const forms: Array<[string, string]> = [[entry.lead, entry.text]]
                if (entry.text_one !== undefined) {
                    forms.push([entry.lead_one ?? entry.lead, entry.text_one])
                }
                for (const [lead, text] of forms) {
                    expect({id, text, opens: text.startsWith(lead)}).toEqual({
                        id,
                        text,
                        opens: true,
                    })
                    expect({id, lead: leadFits(lead, strict)}).toEqual({id, lead: true})
                    expect({id, text, names: placeholders(text)}).toEqual({
                        id,
                        text,
                        names: placeholders(english.get(id)!.text),
                    })
                }
                expect({id, orphanLead: entry.lead_one !== undefined && !entry.text_one}).toEqual({
                    id,
                    orphanLead: false,
                })
            }
        }
    )

    /*
     * Sentences about a counted thing, and the languages whose words change with
     * that count. "greeting have no words in 'es'" is what the Call Emulator said
     * of one missing prompt. Tagalog marks the plural with "mga", which reads as
     * well left out, so its one sentence serves every count; i18next's Filipino
     * rules would call two "one" in any case. It still carries the `_one` keys,
     * because every locale is typed against English's, and repeats the sentence
     * in them.
     */
    const COUNTED = ["ivr.missing-prompts"]
    const INFLECTING = ["en", "es", "cat", "eu", "fr", "gl", "nl"]

    it.each(INFLECTING)("%s: a counted sentence has its singular form", (language) => {
        const own = entries(
            locales[language as keyof typeof locales].translations.problems.messages
        )
        for (const id of COUNTED) {
            const entry = own.get(id)!
            expect({id, singular: typeof entry.text_one}).toEqual({id, singular: "string"})
            expect({id, differs: entry.text_one !== entry.text}).toEqual({id, differs: true})
        }
    })

    it("tl: a counted sentence reads the same for every count", () => {
        const own = entries(tl.translations.problems.messages)
        for (const id of COUNTED) {
            const entry = own.get(id)!
            expect({id, lead: entry.lead_one, text: entry.text_one}).toEqual({
                id,
                lead: entry.lead,
                text: entry.text,
            })
        }
    })
})
