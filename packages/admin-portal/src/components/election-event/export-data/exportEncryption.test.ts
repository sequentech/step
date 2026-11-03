// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ExportSelection, requiresEncryption} from "./exportEncryption"

const nothing: ExportSelection = {
    bulletinBoard: false,
    activityLogs: false,
    publications: false,
    reports: false,
    applications: false,
}

describe("requiresEncryption", () => {
    it.each(["bulletinBoard", "activityLogs", "publications", "reports", "applications"] as const)(
        "is required when %s is exported",
        (option) => {
            expect(requiresEncryption({...nothing, [option]: true})).toBe(true)
        }
    )

    it("is not required for an export without keys, reports or applications", () => {
        expect(requiresEncryption(nothing)).toBe(false)
    })
})
