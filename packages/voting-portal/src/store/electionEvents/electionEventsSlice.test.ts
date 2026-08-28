// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import reducer, {IElectionEvent, seedElectionEvent, setElectionEvent} from "./electionEventsSlice"

describe("election event storage", () => {
    it("does not replace a full or preview event with a partial config seed", () => {
        const fullEvent: IElectionEvent = {
            id: "event-a",
            name: "Full event name",
            description: "Full event description",
            presentation: {i18n: {en: {name: "Snapshot event name"}}},
        }
        const initialState = reducer(undefined, setElectionEvent(fullEvent))
        const seededState = reducer(
            initialState,
            seedElectionEvent({
                id: "event-a",
                presentation: {i18n: {en: {name: "Live config name"}}},
            })
        )

        expect(seededState["event-a"]).toEqual(fullEvent)
    })
})
