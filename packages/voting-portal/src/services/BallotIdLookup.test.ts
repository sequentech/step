// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {ballotIdLookupPattern, isBallotIdInput, typedBallotId} from "./BallotIdLookup"

// Stands in for sequent-core, whose own tests cover how typed IDs are read.
jest.mock("@sequentech/ui-core", () => ({
    normalizeBallotId: (typed: string) => {
        const characters = typed.replace(/[-\s]/g, "").toUpperCase()
        return /^[0-9A-HJKMNP-TV-Z]{8}$/.test(characters)
            ? `${characters.slice(0, 4)}-${characters.slice(4)}`
            : null
    },
}))

const HASH = "0123456789abcdef".repeat(4)

describe("what a voter may type to find a ballot", () => {
    it.each(["", "  ", HASH, HASH.toUpperCase(), "01ab", "FTBE-MHRX", "ftbemhrx", "ftbe mhrx"])(
        "accepts %p",
        (input) => expect(isBallotIdInput(input)).toBe(true)
    )

    it.each(["0", "01a", "xyz", "FTBE-MHR", "FTBE-MHRXX", "FTBE_MHRX", "FTBE-MHRU"])(
        "refuses %p",
        (input) => expect(isBallotIdInput(input)).toBe(false)
    )

    it("writes a ballot box's Ballot ID the way the ballot box does and keeps a hash as typed", () => {
        expect(typedBallotId("ftbemhrx")).toBe("FTBE-MHRX")
        expect(typedBallotId(HASH)).toBe(HASH)
    })
})

describe("the pattern a cast vote's ballot ID must match", () => {
    it("looks for a ballot box's Ballot ID exactly, however it was typed", () => {
        for (const typed of ["FTBE-MHRX", "ftbe-mhrx", "ftbemhrx"]) {
            expect(ballotIdLookupPattern(typed, false)).toBe("FTBE-MHRX")
            expect(ballotIdLookupPattern(typed, true)).toBe("FTBE-MHRX")
        }
    })

    it("looks for a hash exactly, in lower case", () => {
        expect(ballotIdLookupPattern(HASH.toUpperCase(), false)).toBe(HASH)
    })

    it("takes four characters as a prefix only where telephone voting is on", () => {
        expect(ballotIdLookupPattern("01AB", true)).toBe("01ab%")
        expect(ballotIdLookupPattern("01AB", false)).toBe("01ab")
        expect(ballotIdLookupPattern("01ABCD", true)).toBe("01abcd")
    })

    it.each([undefined, "", "xyz", "01ab%", "FTBE_MHRX", "%"])(
        "matches nothing for %p",
        (ballotId) => expect(ballotIdLookupPattern(ballotId, true)).toBe("")
    )
})
