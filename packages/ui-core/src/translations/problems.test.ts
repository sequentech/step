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

type Entry = {lead: string; text: string}
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

describe("problem catalog", () => {
    const raised = raisedIds()
    const english = entries(en.translations.problems.messages)

    it("finds the core's problem names", () => {
        // A guard on the scan itself: an empty set would pass everything below.
        expect(raised.size).toBeGreaterThan(100)
        expect(raised).toContain("import.not-json")
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
        (_, locale) => {
            const own = entries(locale.translations.problems.messages)
            expect([...own.keys()].sort()).toEqual([...english.keys()].sort())
            for (const [id, entry] of own) {
                expect({id, opens: entry.text.startsWith(entry.lead)}).toEqual({
                    id,
                    opens: true,
                })
                expect({id, names: placeholders(entry.text)}).toEqual({
                    id,
                    names: placeholders(english.get(id)!.text),
                })
            }
        }
    )
})
