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

const FULL = "30ffd31442386472fd6dbdb4df3695590f7a414d02ab24995befc5a940b89da1"

describe("Tracking IDs", () => {
    it("groups an ID in fours, whatever its spacing and case", () => {
        expect(groupBallotId(FULL)).toBe(
            "30ff d314 4238 6472 fd6d bdb4 df36 9559 0f7a 414d 02ab 2499 5bef c5a9 40b8 9da1"
        )
        expect(groupBallotId(" 30FF d3 14 ")).toBe("30ff d314")
        expect(groupBallotId("")).toBe("")
    })

    it("takes the first two groups as the short ID", () => {
        expect(shortBallotId(FULL)).toBe("30ff d314")
        expect(shortBallotId("30ff")).toBe("30ff")
    })

    it("splits the full ID into lines", () => {
        const wide = ballotIdLines(FULL, 8)
        expect(wide).toHaveLength(2)
        expect(wide[0]).toEqual(["30ff", "d314", "4238", "6472", "fd6d", "bdb4", "df36", "9559"])
        expect(ballotIdLines(FULL, 4)).toHaveLength(4)
        expect(ballotIdLines("", 4)).toEqual([])
    })

    it("tells a short ID from a full one", () => {
        expect(ballotIdInput("")).toBe(EBallotIdInput.EMPTY)
        expect(ballotIdInput("   ")).toBe(EBallotIdInput.EMPTY)
        expect(ballotIdInput("30FF D314")).toBe(EBallotIdInput.SHORT)
        expect(ballotIdInput(groupBallotId(FULL))).toBe(EBallotIdInput.FULL)
        expect(ballotIdInput("30ff d31")).toBe(EBallotIdInput.INVALID)
        expect(ballotIdInput("30ff d31g")).toBe(EBallotIdInput.INVALID)
        expect(ballotIdInput(`${FULL}00`)).toBe(EBallotIdInput.INVALID)
    })

    it("keeps only hex digits as the voter types, up to a full ID", () => {
        expect(typedBallotId("30-ff:D3 1x4")).toBe("30ff d314")
        expect(typedBallotId(`${FULL}ffff`)).toBe(groupBallotId(FULL))
    })
})
