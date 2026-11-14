// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {store, clearVoterSession, rootReducer} from "./store"
import {addCastVotes, CastVoteStatus} from "./castVotes/castVotesSlice"
import {setElection} from "./elections/electionsSlice"
import {seedElectionEvent, setElectionEvent} from "./electionEvents/electionEventsSlice"
import {IElectionEventPresentation} from "@sequentech/ui-core"

const presentationWith = (css: string) => ({css}) as IElectionEventPresentation

test("a voter scope change removes prior eligibility and participation", () => {
    store.dispatch(clearVoterSession())
    const initial = store.getState()
    store.dispatch(
        setElection({
            id: "election",
            tenant_id: "tenant",
            election_event_id: "event",
            image_document_id: "",
            contests: [],
        })
    )
    store.dispatch(
        addCastVotes([
            {
                id: "cast",
                tenant_id: "tenant",
                election_id: "election",
                election_event_id: "event",
                status: CastVoteStatus.VALID,
            },
        ])
    )
    expect(store.getState().castVotes.election).toHaveLength(1)
    store.dispatch(clearVoterSession())
    expect(store.getState()).toEqual(initial)
})

describe("the public event configuration across a voter scope change", () => {
    const publicEvent = {
        id: "event",
        tenant_id: "tenant",
        presentation: presentationWith("color: red;"),
    }

    test("is kept, however many times the scope changes", () => {
        let state = rootReducer(undefined, seedElectionEvent(publicEvent))
        state = rootReducer(state, clearVoterSession())
        expect(state.electionEvent.event).toEqual(publicEvent)
        state = rootReducer(state, clearVoterSession())
        expect(state.electionEvent.event).toEqual(publicEvent)
    })

    test("replaces event data that came from the voter's own session", () => {
        let state = rootReducer(undefined, seedElectionEvent(publicEvent))
        state = rootReducer(
            state,
            setElectionEvent({
                id: "event",
                name: "Private",
                presentation: presentationWith("color: blue;"),
            })
        )
        state = rootReducer(
            state,
            setElectionEvent({id: "other", presentation: presentationWith("color: green;")})
        )
        state = rootReducer(state, clearVoterSession())
        expect(state.electionEvent.event).toEqual(publicEvent)
        expect(state.electionEvent.other).toBeUndefined()
    })

    test("is not invented when none was loaded", () => {
        let state = rootReducer(undefined, setElectionEvent({id: "event", name: "Private"}))
        state = rootReducer(state, clearVoterSession())
        expect(state.electionEvent).toEqual({})
    })
})
