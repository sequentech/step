// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, render, waitFor} from "@testing-library/react"
import {Provider} from "react-redux"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import PublishedBallot from "./PublishedBallot"
import {store, clearVoterSession} from "../store/store"
import {setBallotSelectionVoteChoice} from "../store/ballotSelections/ballotSelectionsSlice"
import {ELECTION_WITH_INVALID} from "../fixtures/election"

jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {DISABLE_AUTH: false}}),
}))
jest.mock("../hooks/useVoterContext", () => ({
    useVoterContext: () => ({data: mockData, loading: false}),
}))

const eml = ELECTION_WITH_INVALID
const record = {
    id: eml.id,
    election_id: eml.election_id,
    election_event_id: eml.election_event_id,
    tenant_id: eml.tenant_id,
    ballot_eml: JSON.stringify(eml),
}
let mockData = {
    sequent_backend_election: [],
    sequent_backend_election_event: [],
    sequent_backend_ballot_style: [record],
}

const view = () => (
    <Provider store={store}>
        <MemoryRouter initialEntries={[`/election/${eml.election_id}`]}>
            <Routes>
                <Route path="/election/:electionId" element={<PublishedBallot />}>
                    <Route index element={<div>Ready</div>} />
                </Route>
            </Routes>
        </MemoryRouter>
    </Provider>
)

test("reloading the same published style during token refresh preserves the voter's choices", async () => {
    store.dispatch(clearVoterSession())
    const mounted = render(view())
    await mounted.findByText("Ready")
    const style = store.getState().ballotStyles[eml.election_id]!
    const contest = eml.contests[0]
    const candidate = contest.candidates[0]
    act(() => {
        store.dispatch(
            setBallotSelectionVoteChoice({
                ballotStyle: style,
                contestId: contest.id,
                voteChoice: {id: candidate.id, selected: 0},
            })
        )
    })
    const choices = store.getState().ballotSelections[eml.election_id]
    mockData = {...mockData, sequent_backend_ballot_style: [{...record}]}
    mounted.rerender(view())
    await waitFor(() => expect(mounted.getByText("Ready")).toBeVisible())
    expect(store.getState().ballotStyles[eml.election_id]).toBe(style)
    expect(store.getState().ballotSelections[eml.election_id]).toEqual(choices)
    expect(choices?.[0].choices.find((choice) => choice.id === candidate.id)?.selected).toBe(0)
    mounted.unmount()
    store.dispatch(clearVoterSession())
})
