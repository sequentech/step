// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The portal's redux store, seen through the shared ballot's ports.
 *
 * The ballot tests mount their own port over a plain record, so none of them runs
 * this adapter's forwarding. Each member is checked here against the slice it
 * forwards to, through a real store, including the re-render a mark must cause.
 */

import {act, render} from "@testing-library/react"
import {combineReducers, configureStore} from "@reduxjs/toolkit"
import {EInvalidVotePolicy, isPreferential, sortCandidatesInContest} from "@sequentech/ui-core"
import {useBallotEngine, useBallotSelection} from "@sequentech/ui-essentials"
import type {BallotEngine, BallotSelectionPort} from "@sequentech/ui-essentials"
import React from "react"
import {Provider} from "react-redux"

import {ELECTION_WITH_INVALID} from "../fixtures/election"
import {SettingsContext} from "../providers/SettingsContextProvider"
import ballotSelectionsReducer from "../store/ballotSelections/ballotSelectionsSlice"
import type {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import extraReducer, {setIsVoted} from "../store/extra/extraSlice"
import {BallotSelectionAdapter} from "./BallotSelectionAdapter"

const ballotEml = structuredClone(ELECTION_WITH_INVALID)
ballotEml.contests[0].presentation = {
    ...ballotEml.contests[0].presentation,
    invalid_vote_policy: EInvalidVotePolicy.ALLOWED,
}
const ballotStyle = {
    id: ballotEml.id,
    election_id: ballotEml.election_id,
    election_event_id: ballotEml.election_event_id,
    tenant_id: ballotEml.tenant_id,
    ballot_eml: ballotEml,
    created_at: "2026-01-01T00:00:00.000Z",
    last_updated_at: "2026-01-01T00:00:00.000Z",
} as IBallotStyle
const contest = ballotEml.contests[0]
const regular = contest.candidates.find(
    (candidate) =>
        !candidate.presentation?.is_explicit_invalid && !candidate.presentation?.is_explicit_blank
)!

const mount = () => {
    const store = configureStore({
        reducer: combineReducers({ballotSelections: ballotSelectionsReducer, extra: extraReducer}),
    })
    const seen: Array<{port: BallotSelectionPort; engine: BallotEngine}> = []
    const Probe = () => {
        seen.push({port: useBallotSelection(), engine: useBallotEngine()})
        return null
    }
    const settings = {globalSettings: {PUBLIC_BUCKET_URL: "https://bucket.example/"}}
    render(
        <Provider store={store}>
            <SettingsContext.Provider value={settings as never}>
                <BallotSelectionAdapter>
                    <Probe />
                </BallotSelectionAdapter>
            </SettingsContext.Provider>
        </Provider>
    )
    const port = () => seen[seen.length - 1].port
    return {store, seen, port, engine: () => seen[seen.length - 1].engine}
}

describe("the portal's selection port", () => {
    it("reads nothing before the ballot is reset, and the bucket as the image base", () => {
        const {port} = mount()
        expect(port().contest(ballotStyle, contest.id)).toBeUndefined()
        expect(port().choice(ballotStyle, contest.id, regular.id)).toBeUndefined()
        expect(port().isVoted(ballotStyle.election_id)).toBeFalsy()
        expect(port().imageBaseUrl).toBe("https://bucket.example/")
    })

    it("forwards every write to the slice and re-renders the ballot with a new port", () => {
        const {port, seen} = mount()

        act(() => port().reset({ballotStyle, force: true}))
        expect(port().contest(ballotStyle, contest.id)?.choices).toHaveLength(
            contest.candidates.length
        )

        const before = port()
        act(() =>
            port().setChoice({
                ballotStyle,
                contestId: contest.id,
                voteChoice: {id: regular.id, selected: 0},
            })
        )
        expect(port()).not.toBe(before)
        expect(seen.length).toBeGreaterThan(2)
        expect(port().choice(ballotStyle, contest.id, regular.id)?.selected).toBe(0)

        act(() => port().setBlank({ballotStyle, contestId: contest.id, candidateId: regular.id}))
        const marks = port().contest(ballotStyle, contest.id)?.choices ?? []
        expect(marks.filter((choice) => choice.selected > -1).map((choice) => choice.id)).toEqual([
            regular.id,
        ])

        act(() => port().setInvalid({ballotStyle, contestId: contest.id, isExplicitInvalid: true}))
        expect(port().contest(ballotStyle, contest.id)?.is_explicit_invalid).toBe(true)
    })

    it("answers whether the election was voted from the store", () => {
        const {port, store} = mount()
        act(() => {
            store.dispatch(setIsVoted(ballotStyle.election_id))
        })
        expect(port().isVoted(ballotStyle.election_id)).toBe(true)
        expect(port().isVoted(undefined)).toBe(false)
    })
})

describe("the portal's engine", () => {
    it("is this portal's build of sequent-core, through ui-core", () => {
        const {engine} = mount()
        expect(engine().isPreferential).toBe(isPreferential)
        expect(engine().sortCandidatesInContest).toBe(sortCandidatesInContest)
    })
})
