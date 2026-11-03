// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {combineReducers, configureStore} from "@reduxjs/toolkit"
import {ThemeProvider} from "@mui/material/styles"
import {render, RenderResult} from "@testing-library/react"
import {theme} from "@sequentech/ui-essentials"
import type {
    BallotSelection,
    ICandidate,
    IContest,
    IDecodedVoteContest,
    ISlatesConfig,
} from "@sequentech/ui-core"
import i18next from "i18next"
import {I18nextProvider} from "react-i18next"
import {Provider} from "react-redux"

import {Question} from "../components/Question/Question"
import {BallotSlatesProvider} from "../providers/BallotSlatesProvider"
import ballotSelectionsReducer from "../store/ballotSelections/ballotSelectionsSlice"
import ballotStylesReducer from "../store/ballotStyles/ballotStylesSlice"
import type {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import extraReducer from "../store/extra/extraSlice"
import englishTranslation from "../translations/en"

const harnessReducer = combineReducers({
    ballotSelections: ballotSelectionsReducer,
    ballotStyles: ballotStylesReducer,
    extra: extraReducer,
})
type HarnessState = ReturnType<typeof harnessReducer>

export const ELECTION_ID = "election-1"
export const CONTEST_ID = "contest-1"

/** i18n with the portal's real English strings. */
export const i18n = i18next.createInstance()
void i18n.init({
    lng: "en",
    fallbackLng: "en",
    resources: {en: {translation: englishTranslation.translations}},
    interpolation: {escapeValue: false},
    showSupportNotice: false,
})

export const aCandidate = (id: string, name: string, over: Partial<ICandidate> = {}): ICandidate =>
    ({
        id,
        contest_id: CONTEST_ID,
        name,
        description: "",
        sort_order: 0,
        presentation: {},
        ...over,
    }) as unknown as ICandidate

export const aContest = (over: Partial<IContest> = {}): IContest =>
    ({
        id: CONTEST_ID,
        election_id: ELECTION_ID,
        name: "President",
        description: "",
        min_votes: 0,
        max_votes: 1,
        winning_candidates_num: 1,
        counting_algorithm: "plurality-at-large",
        candidates: [aCandidate("a", "Alice Okonjo"), aCandidate("b", "Bob Iyer")],
        presentation: {},
        ...over,
    }) as unknown as IContest

export const aBallotStyle = (contest: IContest): IBallotStyle =>
    ({
        id: "style-1",
        election_id: ELECTION_ID,
        election_event_id: "event-1",
        tenant_id: "tenant-1",
        area_id: "area-1",
        created_at: "2027-01-01T00:00:00Z",
        last_updated_at: "2027-01-01T00:00:00Z",
        ballot_eml: {
            id: "style-1",
            tenant_id: "tenant-1",
            election_event_id: "event-1",
            election_id: ELECTION_ID,
            area_id: "area-1",
            contests: [contest],
            election_event_presentation: {},
            election_presentation: {},
        },
    }) as unknown as IBallotStyle

/**
 * The voter's marks on one contest. `selected` is a rank and `-1` means
 * unselected; `0` is the first preference.
 */
export const marks = (
    contest: IContest,
    chosen: Record<string, number> = {},
    over: Partial<IDecodedVoteContest> = {}
): IDecodedVoteContest =>
    ({
        contest_id: contest.id,
        is_explicit_invalid: false,
        choices: contest.candidates.map((candidate) => ({
            id: candidate.id,
            selected: chosen[candidate.id] ?? -1,
            write_in_text: "",
        })),
        invalid_errors: [],
        invalid_alerts: [],
        ...over,
    }) as unknown as IDecodedVoteContest

export interface MountOptions {
    /** What the voter has marked so far. Defaults to nothing marked. */
    selection?: IDecodedVoteContest
    isReview?: boolean
    isDeclineToVote?: boolean
    /** The encoder's errors, which the screens pass down. */
    errors?: BallotSelection
    /** The slates of the ballot. Defaults to an election without slates. */
    slates?: ISlatesConfig
}

/** Mount one contest with a real store, the way the voting and review screens do. */
export const mountContest = (
    contest: IContest,
    {selection, isReview = false, isDeclineToVote, errors = [], slates}: MountOptions = {}
): RenderResult => {
    const ballotStyle = aBallotStyle(contest)
    const store = configureStore({
        reducer: harnessReducer,
        preloadedState: {
            ballotSelections: {[ELECTION_ID]: [selection ?? marks(contest)]},
            ballotStyles: {[ELECTION_ID]: ballotStyle},
        } as unknown as HarnessState,
    })

    return render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <Provider store={store}>
                    <BallotSlatesProvider slates={slates ?? null}>
                        <Question
                            ballotStyle={ballotStyle}
                            question={contest}
                            isReview={isReview}
                            isDeclineToVote={isDeclineToVote}
                            errorSelectionState={errors}
                            setDecodedContests={() => undefined}
                        />
                    </BallotSlatesProvider>
                </Provider>
            </ThemeProvider>
        </I18nextProvider>
    )
}
