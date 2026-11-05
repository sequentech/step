// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {clearBallotSent, markBallotSent, readBallotSent} from "./sentBallotMarker"

const owner = {tenantId: "tenant", eventId: "event", userId: "voter"}

beforeEach(() => sessionStorage.clear())

it("remembers a ballot sent from this tab, for its voter and election only", () => {
    markBallotSent({owner, electionId: "election", trackingId: "FTBE-MHRX"})
    expect(readBallotSent(owner, "election")?.trackingId).toBe("FTBE-MHRX")
    expect(readBallotSent({...owner, userId: "other"}, "election")).toBeUndefined()
    expect(readBallotSent({...owner, eventId: "other"}, "election")).toBeUndefined()
    expect(readBallotSent(owner, "other")).toBeUndefined()
})

it("holds no key and no choices", () => {
    markBallotSent({owner, electionId: "election", trackingId: "FTBE-MHRX"})
    const saved = Object.values(sessionStorage).join()
    expect(Object.keys(JSON.parse(saved)).sort()).toEqual(["electionId", "owner", "trackingId"])
})

it("is cleared for its election, and left alone for another", () => {
    markBallotSent({owner, electionId: "election", trackingId: "FTBE-MHRX"})
    clearBallotSent("other")
    expect(readBallotSent(owner, "election")).toBeDefined()
    clearBallotSent("election")
    expect(readBallotSent(owner, "election")).toBeUndefined()
    markBallotSent({owner, electionId: "election", trackingId: "FTBE-MHRX"})
    clearBallotSent()
    expect(readBallotSent(owner, "election")).toBeUndefined()
})

it("reads nothing from a malformed marker, and survives storage that throws", () => {
    sessionStorage.setItem("sentBallotMarker", "{not json")
    expect(readBallotSent(owner, "election")).toBeUndefined()
    expect(() => clearBallotSent("election")).not.toThrow()
    const failing = jest.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
        throw new Error("denied")
    })
    try {
        expect(() =>
            markBallotSent({owner, electionId: "election", trackingId: "FTBE-MHRX"})
        ).not.toThrow()
    } finally {
        failing.mockRestore()
    }
})
