// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETallyType} from "@/types/ceremonies"
import {offersBallotImagesReport} from "./templateReports"

describe("offersBallotImagesReport", () => {
    const offered = {
        miruExportEnabled: true,
        tallyType: ETallyType.ELECTORAL_RESULTS,
        canGenerateReports: true,
    }

    it("offers the report to users who can generate it", () => {
        expect(offersBallotImagesReport(offered)).toBe(true)
    })

    it("hides the report from users without report-generate", () => {
        expect(offersBallotImagesReport({...offered, canGenerateReports: false})).toBe(false)
    })

    it("hides the report without Miru exports or for an initialization report", () => {
        expect(offersBallotImagesReport({...offered, miruExportEnabled: false})).toBe(false)
        expect(
            offersBallotImagesReport({...offered, tallyType: ETallyType.INITIALIZATION_REPORT})
        ).toBe(false)
    })
})
