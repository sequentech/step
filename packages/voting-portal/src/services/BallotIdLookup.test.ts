// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {isBallotIdInput, typedBallotId} from "./BallotIdLookup"

// Stands in for sequent-core, whose own tests cover how typed IDs are read.
jest.mock("@sequentech/ui-core", () => ({
    normalizeTrackingId: (typed: string) => {
        const characters = typed.replace(/[-\s]/g, "").toUpperCase()
        return /^[0-9A-HJKMNP-TV-Z]+$/.test(characters) && [4, 8, 52].includes(characters.length)
            ? (characters.match(/.{1,4}/g) ?? []).join("-")
            : null
    },
}))

const HASH = "0123456789abcdef".repeat(4)

describe("what a voter may type to find a ballot", () => {
    it.each(["", "  ", HASH, HASH.toUpperCase(), "FTBE", "FTBE-MHRX", "ftbemhrx", "ftbe mhrx"])(
        "accepts %p",
        (input) => expect(isBallotIdInput(input)).toBe(true)
    )

    it.each(["0", "01a", "xyz", "FTBE-MHR", "FTBE-MHRXX", "FTBE_MHRX", "FTBE-MHRU", "FTBE:MHRX"])(
        "refuses %p",
        (input) => expect(isBallotIdInput(input)).toBe(false)
    )

    it("writes a Tracking ID the way the screens do and keeps a hash as typed", () => {
        expect(typedBallotId("ftbemhrx")).toBe("FTBE-MHRX")
        expect(typedBallotId(HASH)).toBe(HASH)
    })
})
