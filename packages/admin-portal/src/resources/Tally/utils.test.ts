// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    ETallyKeyRestoreEligibility,
    ITallyExecutionStatus,
    ITallyTrusteeStatus,
} from "@/types/ceremonies"
import {getTallyKeyRestoreEligibility} from "./utils"

describe("getTallyKeyRestoreEligibility", () => {
    it.each([ITallyExecutionStatus.STARTED, ITallyExecutionStatus.CONNECTED])(
        "allows a waiting trustee while the tally accepts keys (%s)",
        (executionStatus) => {
            expect(
                getTallyKeyRestoreEligibility(ITallyTrusteeStatus.WAITING, executionStatus)
            ).toBe(ETallyKeyRestoreEligibility.ALLOWED)
        }
    )

    it.each([
        ITallyExecutionStatus.NOT_STARTED,
        ITallyExecutionStatus.IN_PROGRESS,
        ITallyExecutionStatus.SUCCESS,
        ITallyExecutionStatus.CANCELLED,
    ])("denies key restoration while the tally is %s", (executionStatus) => {
        expect(getTallyKeyRestoreEligibility(ITallyTrusteeStatus.WAITING, executionStatus)).toBe(
            ETallyKeyRestoreEligibility.DENIED
        )
    })

    it.each([undefined, null])(
        "denies key restoration while the tally status is unknown (%s)",
        (executionStatus) => {
            expect(
                getTallyKeyRestoreEligibility(ITallyTrusteeStatus.WAITING, executionStatus)
            ).toBe(ETallyKeyRestoreEligibility.DENIED)
        }
    )

    it("denies a trustee whose key is already restored", () => {
        expect(
            getTallyKeyRestoreEligibility(
                ITallyTrusteeStatus.KEY_RESTORED,
                ITallyExecutionStatus.STARTED
            )
        ).toBe(ETallyKeyRestoreEligibility.DENIED)
    })

    it("denies a user who is absent from the tally ceremony", () => {
        expect(getTallyKeyRestoreEligibility(null, ITallyExecutionStatus.STARTED)).toBe(
            ETallyKeyRestoreEligibility.DENIED
        )
    })
})
