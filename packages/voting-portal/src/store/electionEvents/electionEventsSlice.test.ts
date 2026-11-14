// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EElectionEventDelegatedVotingPolicy} from "@sequentech/ui-core"

import reducer, {IElectionEvent, seedElectionEvent, setElectionEvent} from "./electionEventsSlice"

describe("election event storage", () => {
    it("does not replace a full or preview event with a partial config seed", () => {
        const fullEvent: IElectionEvent = {
            id: "event-a",
            name: "Full event name",
            description: "Full event description",
            presentation: {
                delegated_voting_policy: EElectionEventDelegatedVotingPolicy.DISABLED,
                i18n: {en: {name: "Snapshot event name"}},
            },
        }
        const initialState = reducer(undefined, setElectionEvent(fullEvent))
        const seededState = reducer(
            initialState,
            seedElectionEvent({
                id: "event-a",
                presentation: {
                    delegated_voting_policy: EElectionEventDelegatedVotingPolicy.DISABLED,
                    i18n: {en: {name: "Live config name"}},
                },
            })
        )

        expect(seededState["event-a"]).toEqual(fullEvent)
    })

    it("keeps the known presentation when a later record carries none", () => {
        const presentation = {
            delegated_voting_policy: EElectionEventDelegatedVotingPolicy.DISABLED,
            css: "color: red;",
        }
        const seededState = reducer(undefined, seedElectionEvent({id: "event-a", presentation}))
        const state = reducer(
            seededState,
            setElectionEvent({id: "event-a", description: "Published", presentation: null})
        )

        expect(state["event-a"]).toEqual({
            id: "event-a",
            description: "Published",
            presentation,
        })
    })

    it("replaces the presentation when a later record carries one", () => {
        const seededState = reducer(
            undefined,
            seedElectionEvent({
                id: "event-a",
                presentation: {
                    delegated_voting_policy: EElectionEventDelegatedVotingPolicy.DISABLED,
                    css: "color: red;",
                },
            })
        )
        const presentation = {
            delegated_voting_policy: EElectionEventDelegatedVotingPolicy.DISABLED,
            css: "color: blue;",
        }
        const state = reducer(seededState, setElectionEvent({id: "event-a", presentation}))

        expect(state["event-a"]?.presentation).toEqual(presentation)
    })
})
