// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, render, waitFor, screen} from "@testing-library/react"
import {Provider} from "react-redux"
import {MemoryRouter, Route, Routes, useLocation, useNavigate} from "react-router-dom"
import PublishedBallot from "./PublishedBallot"
import {store, clearVoterSession} from "../store/store"
import {setBallotSelectionVoteChoice} from "../store/ballotSelections/ballotSelectionsSlice"
import {ELECTION_WITH_INVALID} from "../fixtures/election"

jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {DISABLE_AUTH: false}}),
}))
jest.mock("../hooks/useVoterContext", () => ({
    useVoterContext: () => ({data: mockData, loading: !mockData}),
}))

const eml = ELECTION_WITH_INVALID
const record = {
    id: eml.id,
    election_id: eml.election_id,
    election_event_id: eml.election_event_id,
    tenant_id: eml.tenant_id,
    ballot_eml: JSON.stringify(eml),
}
const initialData = () => ({
    sequent_backend_election: [],
    sequent_backend_election_event: [],
    sequent_backend_ballot_style: [record],
})
let mockData: ReturnType<typeof initialData> | undefined = initialData()
beforeEach(() => {
    mockData = initialData()
    store.dispatch(clearVoterSession())
})

const view = () => (
    <Provider store={store}>
        <MemoryRouter
            initialEntries={[
                `/tenant/${eml.tenant_id}/event/${eml.election_event_id}/election/${eml.election_id}`,
            ]}
        >
            <Routes>
                <Route
                    path="/tenant/:tenantId/event/:eventId/election/:electionId"
                    element={<PublishedBallot />}
                >
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
    mockData = {...initialData(), sequent_backend_ballot_style: [{...record}]}
    mounted.rerender(view())
    await waitFor(() => expect(mounted.getByText("Ready")).toBeVisible())
    expect(store.getState().ballotStyles[eml.election_id]).toBe(style)
    expect(store.getState().ballotSelections[eml.election_id]).toEqual(choices)
    expect(choices?.[0].choices.find((choice) => choice.id === candidate.id)?.selected).toBe(0)
    mounted.unmount()
    store.dispatch(clearVoterSession())
})

test("closing voting does not eject an in-progress ballot before server grace validation", async () => {
    store.dispatch(clearVoterSession())
    mockData = {
        sequent_backend_election: [],
        sequent_backend_election_event: [],
        sequent_backend_ballot_style: [record],
    }
    const mounted = render(view())
    await mounted.findByText("Ready")
    const selection = store.getState().ballotSelections[eml.election_id]
    mockData = {
        ...mockData,
        sequent_backend_election_event: [
            {id: eml.election_event_id, status: {voting_status: "CLOSED"}},
        ],
    } as any
    mounted.rerender(view())
    await waitFor(() => expect(mounted.getByText("Ready")).toBeVisible())
    expect(store.getState().ballotSelections[eml.election_id]).toEqual(selection)
    mounted.unmount()
    store.dispatch(clearVoterSession())
})

const eventPath = `/tenant/${eml.tenant_id}/event/${eml.election_event_id}`
function LocationProbe() {
    const location = useLocation()
    const navigate = useNavigate()
    return (
        <>
            <div data-testid="location">{location.pathname + location.search}</div>
            <button onClick={() => navigate(eventPath + "/election/other/vote")}>
                Other election
            </button>
        </>
    )
}
const flowView = () => (
    <Provider store={store}>
        <MemoryRouter
            initialEntries={[`${eventPath}/election/${eml.election_id}/vote?kiosk=true&lang=es`]}
        >
            <LocationProbe />
            <Routes>
                <Route
                    path="/tenant/:tenantId/event/:eventId/election/:electionId"
                    element={<PublishedBallot />}
                >
                    <Route path="vote" element={<div>Voting</div>} />
                    <Route path="start" element={<div>New ballot start</div>} />
                </Route>
            </Routes>
        </MemoryRouter>
    </Provider>
)

test("a replacement publication resets selection and returns to start with kiosk and language parameters", async () => {
    const mounted = render(flowView())
    await screen.findByText("Voting")
    const style = store.getState().ballotStyles[eml.election_id]!
    const contest = eml.contests[0]
    const candidate = contest.candidates[0]
    act(() =>
        store.dispatch(
            setBallotSelectionVoteChoice({
                ballotStyle: style,
                contestId: contest.id,
                voteChoice: {id: candidate.id, selected: 0},
            })
        )
    )
    mockData = {
        ...initialData(),
        sequent_backend_ballot_style: [
            {
                ...record,
                id: "replacement",
                ballot_eml: JSON.stringify({...eml, id: "replacement"}),
            },
        ],
    }
    mounted.rerender(flowView())
    await screen.findByText("New ballot start")
    expect(screen.getByTestId("location")).toHaveTextContent(
        `${eventPath}/election/${eml.election_id}/start?kiosk=true&lang=es`
    )
    expect(store.getState().ballotStyles[eml.election_id]?.id).toBe("replacement")
    const choices = store.getState().ballotSelections[eml.election_id]
    expect(choices?.[0].choices.find((choice) => choice.id === candidate.id)?.selected).toBe(-1)
    mounted.unmount()
})

test("changing elections waits for the new ballot instead of reusing the previous ready state", async () => {
    const mounted = render(flowView())
    await screen.findByText("Voting")
    mockData = undefined
    act(() => screen.getByRole("button", {name: "Other election"}).click())
    expect(screen.queryByText("Voting")).toBeNull()
    expect(screen.getByRole("progressbar")).toBeVisible()
    mockData = {
        ...initialData(),
        sequent_backend_ballot_style: [
            {
                ...record,
                id: "other-style",
                election_id: "other",
                ballot_eml: JSON.stringify({...eml, id: "other-style", election_id: "other"}),
            },
        ],
    }
    mounted.rerender(flowView())
    await screen.findByText("Voting")
    expect(store.getState().ballotStyles.other?.id).toBe("other-style")
    mounted.unmount()
})
