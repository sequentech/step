// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {receiptFileName} from "./confirmation"

describe("receiptFileName", () => {
    it("names the receipt after the groups of the short Ballot ID", () => {
        const id = "30ffd3144238".padEnd(64, "0")
        expect(receiptFileName(id)).toBe("ballot-30ff-d314.vitrine-receipt")
    })

    it("reads a grouped, upper-case ID the same way", () => {
        expect(receiptFileName("30FF D314 4238 6472")).toBe("ballot-30ff-d314.vitrine-receipt")
    })

    it("still names a file when there is no ID", () => {
        expect(receiptFileName("")).toBe("ballot.vitrine-receipt")
    })
})
