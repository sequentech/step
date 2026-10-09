// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {AppDispatch} from "../store/store"
import {
    GetPublishedBallotStylesQuery,
    findPublishedBallotStyle,
    updateBallotStyleAndSelection,
} from "./BallotStyles"

jest.mock("@sequentech/ui-core", () => ({
    isString: (value: unknown) => typeof value === "string",
}))

const ballotStyle = (id: string, publicationId: string) => ({
    id,
    ballot_publication_id: publicationId,
    election_id: "election-a",
    election_event_id: "event-a",
    status: null,
    tenant_id: "tenant-a",
    ballot_eml: "{}",
    ballot_signature: null,
    created_at: "2026-08-18T00:00:00Z",
    area_id: null,
    annotations: null,
    labels: null,
    last_updated_at: "2026-08-18T00:00:00Z",
    deleted_at: null,
})

describe("updateBallotStyleAndSelection", () => {
    it("stores only styles belonging to a published non-deleted publication", () => {
        const dispatch = jest.fn()
        const data = {
            sequent_backend_ballot_publication: [
                {id: "published", published_at: "2026-08-18T01:00:00Z"},
            ],
            sequent_backend_ballot_style: [
                ballotStyle("published-style", "published"),
                ballotStyle("draft-style", "generated-draft"),
            ],
        } as GetPublishedBallotStylesQuery

        updateBallotStyleAndSelection(data, dispatch as unknown as AppDispatch)

        expect(dispatch).toHaveBeenCalledTimes(1)
        expect(dispatch.mock.calls[0][0].payload.id).toBe("published-style")
        expect(dispatch.mock.calls[0][0].payload.publication_published_at).toBe(
            "2026-08-18T01:00:00Z"
        )
    })

    it("stores the newest publication last when active publications overlap an election", () => {
        const dispatch = jest.fn()
        const data = {
            sequent_backend_ballot_publication: [
                {id: "newer-election", published_at: "2026-08-18T01:00:00Z"},
                {id: "older-event", published_at: "2026-08-18T00:00:00Z"},
            ],
            // Deliberately newest-first: GraphQL does not guarantee row order.
            sequent_backend_ballot_style: [
                ballotStyle("newer-style", "newer-election"),
                ballotStyle("older-style", "older-event"),
            ],
        } as GetPublishedBallotStylesQuery

        updateBallotStyleAndSelection(data, dispatch as unknown as AppDispatch)

        expect(dispatch).toHaveBeenCalledTimes(2)
        expect(dispatch.mock.calls[0][0].payload.id).toBe("older-style")
        expect(dispatch.mock.calls[1][0].payload.id).toBe("newer-style")
    })
})

describe("findPublishedBallotStyle", () => {
    const data = {
        sequent_backend_ballot_publication: [
            {id: "published", published_at: "2026-08-18T01:00:00Z"},
        ],
        sequent_backend_ballot_style: [
            {
                ...ballotStyle("published-style", "published"),
                ballot_eml: '{"id":"published-style"}',
            },
            {...ballotStyle("draft-style", "generated-draft"), ballot_eml: '{"id":"draft-style"}'},
            {...ballotStyle("unreadable-style", "published"), ballot_eml: "{"},
        ],
    } as GetPublishedBallotStylesQuery

    it("returns the ballot style with that id from a published publication", () => {
        expect(findPublishedBallotStyle(data, "published-style")).toEqual({id: "published-style"})
    })

    it.each([
        ["of a publication that is not published", "draft-style"],
        ["that is not listed", "unknown-style"],
        ["whose ballot style cannot be read", "unreadable-style"],
    ])("returns null for an id %s", (_, ballotStyleId) => {
        expect(findPublishedBallotStyle(data, ballotStyleId)).toBeNull()
    })

    it("returns null before the ballot styles load or without an id", () => {
        expect(findPublishedBallotStyle(undefined, "published-style")).toBeNull()
        expect(findPublishedBallotStyle(data, undefined)).toBeNull()
    })
})
