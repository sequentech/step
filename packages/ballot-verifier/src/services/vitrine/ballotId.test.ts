// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    ballotIdInput,
    EBallotIdInput,
    groupBallotId,
    sameBallotId,
    shortBallotId,
    trackingIdDigestHex,
    trackingIdOfDigestHex,
    typedBallotId,
} from "./ballotId"

const FULL = "FTBEMHRX0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789A0"
const GROUPED = "FTBE-MHRX-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ-0123-4567-89A0"

describe("tracking IDs", () => {
    it("groups an ID in fours with hyphens, whatever its spacing and case", () => {
        expect(groupBallotId(FULL)).toBe(GROUPED)
        expect(groupBallotId(" ftbe mh rx ")).toBe("FTBE-MHRX")
        expect(groupBallotId("ftbo-mhrl")).toBe("FTB0-MHR1")
        expect(groupBallotId("")).toBe("")
    })

    it("takes the first two groups as the short ID", () => {
        expect(shortBallotId(FULL)).toBe("FTBE-MHRX")
        expect(shortBallotId("ftbe")).toBe("FTBE")
    })

    it("tells a short ID from a full one", () => {
        expect(ballotIdInput("")).toBe(EBallotIdInput.EMPTY)
        expect(ballotIdInput("ftbe mhrx")).toBe(EBallotIdInput.SHORT)
        expect(ballotIdInput(GROUPED)).toBe(EBallotIdInput.FULL)
        expect(ballotIdInput("FTBE-MHR")).toBe(EBallotIdInput.INVALID)
        expect(ballotIdInput("FTBE-MHRU")).toBe(EBallotIdInput.INVALID)
        expect(ballotIdInput(`${FULL}0`)).toBe(EBallotIdInput.INVALID)
    })

    it("never takes a voter pseudonym for a tracking ID", () => {
        expect(ballotIdInput("FTBE:MHRX")).toBe(EBallotIdInput.INVALID)
    })

    it("keeps only tracking ID characters as the voter types, up to a full ID", () => {
        expect(typedBallotId("ftbe mhrx u")).toBe("FTBE-MHRX")
        expect(typedBallotId(`${FULL}FFFF`)).toBe(GROUPED)
    })

    it("compares IDs whatever their spacing and case", () => {
        expect(sameBallotId("ftbe mhrx", "FTBE-MHRX")).toBe(true)
        expect(sameBallotId("FTBE-MHRX", "FTBE-MHRZ")).toBe(false)
    })

    it("converts a full ID to its digest in hex and back", () => {
        const digest = "c0ffee".repeat(10) + "abcd"
        const id = trackingIdOfDigestHex(digest) as string
        expect(id).toMatch(/^([0-9A-Z]{4}-){12}[0-9A-Z]{4}$/)
        expect(trackingIdDigestHex(id)).toBe(digest)
        expect(trackingIdDigestHex(id.toLowerCase())).toBe(digest)
        expect(trackingIdDigestHex("FTBE-MHRX")).toBeUndefined()
        expect(trackingIdOfDigestHex("abcd")).toBeUndefined()
    })
})
