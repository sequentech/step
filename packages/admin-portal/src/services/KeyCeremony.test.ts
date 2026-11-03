// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    canTrusteeReconfirmCustody,
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
} from "./KeyCeremony"

describe("canTrusteeReconfirmCustody", () => {
    it.each([EStatus.IN_PROGRESS, EStatus.SUCCESS])(
        "allows a trustee with checked keys during %s",
        (executionStatus) => {
            expect(
                canTrusteeReconfirmCustody({
                    executionStatus,
                    trusteeStatus: TStatus.CUSTODY_CONFIRMED,
                    isAutomaticCeremony: false,
                })
            ).toBe(true)
        }
    )

    it.each([EStatus.USER_CONFIGURATION, EStatus.STARTED, EStatus.CANCELLED, EStatus.FAILED])(
        "rejects execution status %s",
        (executionStatus) => {
            expect(
                canTrusteeReconfirmCustody({
                    executionStatus,
                    trusteeStatus: TStatus.CUSTODY_CONFIRMED,
                    isAutomaticCeremony: false,
                })
            ).toBe(false)
        }
    )

    it.each([
        undefined,
        TStatus.WAITING,
        TStatus.SHARES_POSTED,
        TStatus.HALTED,
        TStatus.KEY_GENERATED,
        TStatus.CUSTODY_REQUEST_RETRIEVED,
    ])("rejects trustee status %s", (trusteeStatus) => {
        expect(
            canTrusteeReconfirmCustody({
                executionStatus: EStatus.IN_PROGRESS,
                trusteeStatus,
                isAutomaticCeremony: false,
            })
        ).toBe(false)
    })

    it("rejects automatic ceremonies", () => {
        expect(
            canTrusteeReconfirmCustody({
                executionStatus: EStatus.SUCCESS,
                trusteeStatus: TStatus.CUSTODY_CONFIRMED,
                isAutomaticCeremony: true,
            })
        ).toBe(false)
    })
})
