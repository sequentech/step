// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ITallyExecutionStatus, ITallyTrusteeStatus} from "@/types/ceremonies"
import {isTallyAcceptingTrusteeKeys, isTrusteeInTallyCeremony} from "./tallyCeremonyParticipation"

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

describe("isTrusteeInTallyCeremony", () => {
    it("uses the trustee claim to find a participant", () => {
        expect(isTrusteeInTallyCeremony(execution, "trustee-1")).toBe(true)
    })

    it.each(["another-trustee", undefined, null])("rejects non-participant %s", (trusteeName) => {
        expect(isTrusteeInTallyCeremony(execution, trusteeName)).toBe(false)
    })
})

describe("isTallyAcceptingTrusteeKeys", () => {
    it.each([ITallyExecutionStatus.STARTED, ITallyExecutionStatus.CONNECTED])(
        "accepts keys in %s",
        (status) => {
            expect(isTallyAcceptingTrusteeKeys(status)).toBe(true)
        }
    )

    it.each([
        ITallyExecutionStatus.NOT_STARTED,
        ITallyExecutionStatus.IN_PROGRESS,
        ITallyExecutionStatus.AWAITING_INPUT,
        ITallyExecutionStatus.SUCCESS,
        ITallyExecutionStatus.CANCELLED,
    ])("rejects keys in %s", (status) => {
        expect(isTallyAcceptingTrusteeKeys(status)).toBe(false)
    })
})
