// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    ballotIdInput,
    ballotIdLines,
    EBallotIdInput,
    groupBallotId,
    shortBallotId,
    typedBallotId,
} from "./ballotId"

const FULL = "FTBEMHRX0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789AB"

describe("Tracking IDs", () => {
    it("groups an ID in fours with hyphens, whatever its spacing and case", () => {
        expect(groupBallotId(FULL)).toBe("FTBE-MHRX-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ-0123-4567-89AB")
        expect(groupBallotId(" ftbe mh rx ")).toBe("FTBE-MHRX")
        expect(groupBallotId("ftbo-mhrl")).toBe("FTB0-MHR1")
        expect(groupBallotId("")).toBe("")
    })

    it("takes the first two groups as the short ID", () => {
        expect(shortBallotId(FULL)).toBe("FTBE-MHRX")
        expect(shortBallotId("ftbe")).toBe("FTBE")
    })

    it("splits the full ID into lines", () => {
        const wide = ballotIdLines(FULL, 8)
        expect(wide).toHaveLength(2)
        expect(wide[0]).toEqual(["FTBE", "MHRX", "0123", "4567", "89AB", "CDEF", "GHJK", "MNPQ"])
        expect(ballotIdLines(FULL, 4)).toHaveLength(4)
        expect(ballotIdLines("", 4)).toEqual([])
    })

    it("tells a short ID from a full one", () => {
        expect(ballotIdInput("")).toBe(EBallotIdInput.EMPTY)
        expect(ballotIdInput("   ")).toBe(EBallotIdInput.EMPTY)
        expect(ballotIdInput("ftbe mhrx")).toBe(EBallotIdInput.SHORT)
        expect(ballotIdInput(groupBallotId(FULL))).toBe(EBallotIdInput.FULL)
        expect(ballotIdInput("FTBE-MHR")).toBe(EBallotIdInput.INVALID)
        expect(ballotIdInput("FTBE-MHRX-1")).toBe(EBallotIdInput.PARTIAL)
        expect(ballotIdInput("FTBE-MHRU")).toBe(EBallotIdInput.INVALID)
        expect(ballotIdInput(`${FULL}0`)).toBe(EBallotIdInput.INVALID)
    })

    it("never takes a voter pseudonym for a Tracking ID", () => {
        expect(ballotIdInput("FTBE:MHRX")).toBe(EBallotIdInput.INVALID)
        expect(ballotIdInput("ftbe:mhrx")).toBe(EBallotIdInput.INVALID)
    })

    it("keeps only Tracking ID characters as the voter types, up to a full ID", () => {
        expect(typedBallotId("ft-be:mh rx")).toBe("FTBE-:MHR-X")
        expect(typedBallotId("ftbe mhrx u")).toBe("FTBE-MHRX")
        expect(typedBallotId(`${FULL}FFFF`)).toBe(groupBallotId(FULL))
    })
})
