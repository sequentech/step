// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ITallyTrusteeStatus} from "@/types/ceremonies"
import {getTallyTrusteeStatus} from "./tallyCeremonyParticipation"

const execution = {
    status: {
        logs: [],
        trustees: [
            {name: "trustee-1", status: ITallyTrusteeStatus.WAITING},
            {name: "trustee-2", status: ITallyTrusteeStatus.KEY_RESTORED},
        ],
        elections_status: [],
    },
}

describe("getTallyTrusteeStatus", () => {
    it("returns the status for the trustee claim", () => {
        expect(getTallyTrusteeStatus(execution, "trustee-2")).toBe(ITallyTrusteeStatus.KEY_RESTORED)
    })

    it.each(["another-trustee", undefined, null])("returns null for non-participant %s", (name) => {
        expect(getTallyTrusteeStatus(execution, name)).toBeNull()
    })
})
