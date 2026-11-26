// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {readFileSync} from "fs"
import {resolve} from "path"
import en from "@/translations/en"
import es from "@/translations/es"
import ca from "@/translations/cat"
import eu from "@/translations/eu"
import fr from "@/translations/fr"
import gl from "@/translations/gl"
import nl from "@/translations/nl"
import tl from "@/translations/tl"
import {EVENT_LOG_STATEMENT_KINDS, TENANT_LOG_STATEMENT_KINDS} from "./logScope"

/** The variants of `StatementType`, the kinds the ledger writes in `statement_kind`. */
const backendKinds = (): string[] => {
    const source = readFileSync(
        resolve(__dirname, "../../../../electoral-log/src/messages/statement.rs"),
        "utf8"
    )
    const body = source.match(/pub enum StatementType \{([\s\S]*?)\n\}/)?.[1] ?? ""
    return body
        .split("\n")
        .map((line) => line.replace(/\/\/.*$/, "").trim())
        .filter(Boolean)
        .map((line) => line.replace(/,$/, ""))
}

const LANGUAGES = {en, es, ca, eu, fr, gl, nl, tl}

const labels = (language: keyof typeof LANGUAGES): Record<string, string> =>
    (
        LANGUAGES[language].translations.logsScreen as unknown as {
            statementKind: Record<string, string>
        }
    ).statementKind

describe("the statement kinds an election event's log is filtered by", () => {
    it("are every kind the backend writes, in the backend's order", () => {
        const kinds = backendKinds()
        expect(kinds.length).toBeGreaterThan(70)
        expect(kinds.every((kind) => /^[A-Z][A-Za-z]+$/.test(kind))).toBe(true)
        expect([...EVENT_LOG_STATEMENT_KINDS]).toEqual(kinds)
    })

    it("include the tenant log's kinds", () => {
        for (const kind of TENANT_LOG_STATEMENT_KINDS) {
            expect(EVENT_LOG_STATEMENT_KINDS).toContain(kind)
        }
    })

    it.each(Object.keys(LANGUAGES) as Array<keyof typeof LANGUAGES>)(
        "each have one distinct, friendly name in %s",
        (language) => {
            const names = labels(language)
            expect(Object.keys(names).sort()).toEqual([...EVENT_LOG_STATEMENT_KINDS].sort())
            for (const kind of EVENT_LOG_STATEMENT_KINDS) {
                expect(names[kind].trim()).not.toBe("")
                // A name, not the wire value.
                expect(names[kind]).not.toBe(kind)
            }
            expect(new Set(Object.values(names)).size).toBe(EVENT_LOG_STATEMENT_KINDS.length)
        }
    )

    it("are translated, not copied from English", () => {
        const english = labels("en")
        for (const language of ["es", "ca", "eu", "fr", "gl", "nl"] as const) {
            const same = EVENT_LOG_STATEMENT_KINDS.filter(
                (kind) => labels(language)[kind] === english[kind]
            )
            expect({language, same}).toEqual({language, same: []})
        }
    })
})
