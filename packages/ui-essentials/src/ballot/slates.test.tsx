// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderHook} from "@testing-library/react"
import {I18nextProvider} from "react-i18next"
import i18next from "i18next"
import {EMobileCandidateLists} from "@sequentech/ui-core"
import type {ICandidate, IContest, ISlatesConfig} from "@sequentech/ui-core"

import {
    BallotSlatesProvider,
    getDefaultLanguageCode,
    useBallotSlates,
    useCandidateSlateLabel,
} from "./slates"
import type {IBallotStyle} from "./types"

const languages = (code?: string) => ({language_conf: code ? {default_language_code: code} : {}})

const ballotStyle = (election?: string, event?: string): IBallotStyle =>
    ({
        ballot_eml: {
            election_presentation: election === undefined ? undefined : languages(election),
            election_event_presentation: event === undefined ? undefined : languages(event),
        },
    }) as unknown as IBallotStyle

const contest = (id = "council"): IContest => ({id}) as unknown as IContest

const candidate = (id: string, presentation: ICandidate["presentation"] = {}): ICandidate =>
    ({id, presentation}) as unknown as ICandidate

const SLATES: ISlatesConfig = {
    version: 1,
    mobile_candidate_lists: EMobileCandidateLists.COLLAPSED,
    slates: [
        {
            id: "independent-voices",
            name: {en: "Independent Voices", es: "Voces Independientes"},
            members: {council: ["alice"], treasurer: []},
        },
    ],
}

const i18nIn = (language: string) => {
    const i18n = i18next.createInstance()
    void i18n.init({
        lng: language,
        fallbackLng: "en",
        resources: {en: {translation: {slates: {independent: "Independent"}}}},
        interpolation: {escapeValue: false},
    })
    return i18n
}

const label = (
    slates: ISlatesConfig | null,
    inContest: IContest,
    forCandidate: ICandidate,
    {language = "en", style = ballotStyle()}: {language?: string; style?: IBallotStyle} = {}
) =>
    renderHook(() => useCandidateSlateLabel(style, inContest, forCandidate), {
        wrapper: ({children}) => (
            <I18nextProvider i18n={i18nIn(language)}>
                <BallotSlatesProvider slates={slates}>{children}</BallotSlatesProvider>
            </I18nextProvider>
        ),
    }).result.current

describe("the default language of a ballot", () => {
    it("is the election's, then the event's", () => {
        expect(getDefaultLanguageCode(ballotStyle("es", "en"))).toBe("es")
        expect(getDefaultLanguageCode(ballotStyle("", "en"))).toBe("en")
        expect(getDefaultLanguageCode(ballotStyle(undefined, "tl"))).toBe("tl")
    })

    it("is unknown when neither sets one", () => {
        expect(getDefaultLanguageCode(ballotStyle())).toBeUndefined()
        expect(
            getDefaultLanguageCode({ballot_eml: {election_presentation: {}}} as IBallotStyle)
        ).toBeUndefined()
    })
})

describe("the slates a host supplies", () => {
    it("are absent without a provider", () => {
        expect(renderHook(() => useBallotSlates()).result.current).toBeNull()
    })

    it("reach the ballot below the provider", () => {
        const {result} = renderHook(() => useBallotSlates(), {
            wrapper: ({children}) => (
                <BallotSlatesProvider slates={SLATES}>{children}</BallotSlatesProvider>
            ),
        })
        expect(result.current).toBe(SLATES)
    })
})

describe("the slate label of a candidate", () => {
    it("is absent on a ballot without slates", () => {
        expect(label(null, contest(), candidate("alice"))).toBeUndefined()
    })

    it("is absent in a contest where no slate has a candidate", () => {
        expect(label(SLATES, contest("treasurer"), candidate("alice"))).toBeUndefined()
        expect(label(SLATES, contest("auditor"), candidate("alice"))).toBeUndefined()
    })

    it.each([
        ["a write-in line", {is_write_in: true}],
        ["the invalid vote", {is_explicit_invalid: true}],
        ["the blank vote", {is_explicit_blank: true}],
        ["a category header", {is_category_list: true}],
    ])("is absent on %s", (_name, presentation) => {
        expect(
            label(
                SLATES,
                contest(),
                candidate("marker", presentation as ICandidate["presentation"])
            )
        ).toBeUndefined()
    })

    it("is the slate's name in the voter's language", () => {
        expect(label(SLATES, contest(), candidate("alice"))).toBe("Independent Voices")
        expect(label(SLATES, contest(), candidate("alice"), {language: "es"})).toBe(
            "Voces Independientes"
        )
    })

    it("falls back to the ballot's default language", () => {
        expect(
            label(SLATES, contest(), candidate("alice"), {
                language: "fr",
                style: ballotStyle("es"),
            })
        ).toBe("Voces Independientes")
    })

    it("is the independent label for a candidate on no slate", () => {
        expect(label(SLATES, contest(), candidate("bob"))).toBe("Independent")
        expect(label(SLATES, contest(), {id: "carol"} as ICandidate)).toBe("Independent")
    })
})
