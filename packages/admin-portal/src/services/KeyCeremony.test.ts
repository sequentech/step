// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    canTrusteeRecheckPrivateKey,
    IKeysCeremonyExecutionStatus as EStatus,
    IKeysCeremonyTrusteeStatus as TStatus,
} from "./KeyCeremony"

describe("canTrusteeRecheckPrivateKey", () => {
    it.each([EStatus.IN_PROGRESS, EStatus.SUCCESS])(
        "allows a trustee with checked keys during %s",
        (executionStatus) => {
            expect(
                canTrusteeRecheckPrivateKey({
                    executionStatus,
                    trusteeStatus: TStatus.KEY_CHECKED,
                    isAutomaticCeremony: false,
                })
            ).toBe(true)
        }
    )

    it.each([EStatus.USER_CONFIGURATION, EStatus.STARTED, EStatus.CANCELLED])(
        "rejects execution status %s",
        (executionStatus) => {
            expect(
                canTrusteeRecheckPrivateKey({
                    executionStatus,
                    trusteeStatus: TStatus.KEY_CHECKED,
                    isAutomaticCeremony: false,
                })
            ).toBe(false)
        }
    )

    it.each([undefined, TStatus.WAITING, TStatus.KEY_GENERATED, TStatus.KEY_RETRIEVED])(
        "rejects trustee status %s",
        (trusteeStatus) => {
            expect(
                canTrusteeRecheckPrivateKey({
                    executionStatus: EStatus.IN_PROGRESS,
                    trusteeStatus,
                    isAutomaticCeremony: false,
                })
            ).toBe(false)
        }
    )

    it("rejects automatic ceremonies", () => {
        expect(
            canTrusteeRecheckPrivateKey({
                executionStatus: EStatus.SUCCESS,
                trusteeStatus: TStatus.KEY_CHECKED,
                isAutomaticCeremony: true,
            })
        ).toBe(false)
    })
})
