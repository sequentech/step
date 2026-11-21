// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {GetBallotStylesQuery} from "../gql/graphql"
import {EPublishedBallotStyleLookup, findPublishedBallotStyle} from "./BallotStyles"

jest.mock("@sequentech/ui-core", () => ({
    isString: (value: unknown) => typeof value === "string",
}))

const ballotStyle = (id: string, eventId: string, ballotEml: string | null) => ({
    id,
    election_id: "election-a",
    election_event_id: eventId,
    tenant_id: "tenant-a",
    ballot_eml: ballotEml,
})

describe("findPublishedBallotStyle", () => {
    const data = {
        sequent_backend_ballot_style: [
            ballotStyle("served-style", "event-a", '{"id":"served-style"}'),
            ballotStyle("unreadable-style", "event-a", "{"),
            ballotStyle("no-eml-style", "event-a", null),
            ballotStyle("other-event-style", "event-b", '{"id":"other-event-style"}'),
        ],
    } as unknown as GetBallotStylesQuery

    it("finds the ballot style with that id of the election event", () => {
        expect(findPublishedBallotStyle(data, "served-style", "event-a")).toEqual({
            status: EPublishedBallotStyleLookup.FOUND,
            ballotStyle: {id: "served-style"},
        })
    })

    it.each([
        ["that is not listed", "unknown-style"],
        ["that is missing", undefined],
        ["of another election event", "other-event-style"],
    ])("reports an id %s as not published", (_, ballotStyleId) => {
        expect(findPublishedBallotStyle(data, ballotStyleId, "event-a")).toEqual({
            status: EPublishedBallotStyleLookup.NOT_PUBLISHED,
        })
    })

    it.each([
        ["malformed", "unreadable-style"],
        ["absent", "no-eml-style"],
    ])("reports a ballot style whose ballot_eml is %s as unreadable", (_, ballotStyleId) => {
        expect(findPublishedBallotStyle(data, ballotStyleId, "event-a")).toEqual({
            status: EPublishedBallotStyleLookup.UNREADABLE,
        })
    })

    it("reports ballot styles that have not loaded apart from an unpublished one", () => {
        expect(findPublishedBallotStyle(undefined, "served-style", "event-a")).toEqual({
            status: EPublishedBallotStyleLookup.NOT_LOADED,
        })
    })

    it("reports a ballot style as not published without an election event", () => {
        expect(findPublishedBallotStyle(data, "served-style", null)).toEqual({
            status: EPublishedBallotStyleLookup.NOT_PUBLISHED,
        })
    })
})
