// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {RootState} from "../store"
import {IBallotStyle, selectBallotStyleByElectionEventId} from "./ballotStylesSlice"

describe("ballot style selectors", () => {
    it("selects the current event even when an earlier event remains cached first", () => {
        const eventA = {
            ballot_publication_id: "publication-a",
            publication_published_at: "2026-08-18T00:00:00Z",
            election_id: "election-a",
            election_event_id: "event-a",
        } as IBallotStyle
        const eventB = {
            ballot_publication_id: "publication-b",
            publication_published_at: "2026-08-18T00:00:00Z",
            election_id: "election-b",
            election_event_id: "event-b",
        } as IBallotStyle
        const state = {
            ballotStyles: {
                [eventA.election_id]: eventA,
                [eventB.election_id]: eventB,
            },
        } as RootState

        expect(selectBallotStyleByElectionEventId("event-b")(state)).toBe(eventB)
        expect(selectBallotStyleByElectionEventId("missing-event")(state)).toBeUndefined()
    })

    it("selects the newest publication snapshot across separately published elections", () => {
        const olderSnapshot = {
            ballot_publication_id: "publication-older",
            publication_published_at: "2026-08-18T00:00:00Z",
            election_id: "election-a",
            election_event_id: "event-a",
        } as IBallotStyle
        const newerSnapshot = {
            ballot_publication_id: "publication-newer",
            publication_published_at: "2026-08-18T01:00:00Z",
            election_id: "election-b",
            election_event_id: "event-a",
        } as IBallotStyle
        const state = {
            ballotStyles: {
                [olderSnapshot.election_id]: olderSnapshot,
                [newerSnapshot.election_id]: newerSnapshot,
            },
        } as RootState

        expect(selectBallotStyleByElectionEventId("event-a")(state)).toBe(newerSnapshot)
    })
})
