// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {receiptFileName} from "./confirmation"

describe("receiptFileName", () => {
    it("names the receipt after the groups of the short Tracking ID", () => {
        const id = "FTBEMHRX4238".padEnd(64, "0")
        expect(receiptFileName(id)).toBe("ballot-FTBE-MHRX.vitrine-receipt")
    })

    it("reads a grouped, upper-case ID the same way", () => {
        expect(receiptFileName("ftbe mhrx 0123 4567")).toBe("ballot-FTBE-MHRX.vitrine-receipt")
    })

    it("still names a file when there is no ID", () => {
        expect(receiptFileName("")).toBe("ballot.vitrine-receipt")
    })
})
