// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IBallotStyle} from "@sequentech/ui-core"
import {getBallotStyleDefaultLanguageCode} from "./defaultLanguageCode"

const ballotStyleWithLanguages = (
    electionLanguage?: string,
    eventLanguage?: string
): IBallotStyle =>
    ({
        election_presentation: electionLanguage
            ? {language_conf: {default_language_code: electionLanguage}}
            : undefined,
        election_event_presentation: eventLanguage
            ? {language_conf: {default_language_code: eventLanguage}}
            : undefined,
    }) as IBallotStyle

describe("getBallotStyleDefaultLanguageCode", () => {
    it("prefers the election default language", () => {
        expect(getBallotStyleDefaultLanguageCode(ballotStyleWithLanguages("fr", "en"))).toBe("fr")
    })

    it("falls back to the election-event default language", () => {
        expect(getBallotStyleDefaultLanguageCode(ballotStyleWithLanguages(undefined, "en"))).toBe(
            "en"
        )
    })

    it("returns undefined when the ballot style has no configured default", () => {
        expect(getBallotStyleDefaultLanguageCode(null)).toBeUndefined()
    })
})
