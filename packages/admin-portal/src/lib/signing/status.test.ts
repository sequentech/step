// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {requestStatusKey, shownRequestStatus} from "./status"
import {SigningRequestStatus} from "./types"

describe("the status a request shows", () => {
    const now = new Date("2028-05-08T12:00:00Z")

    it("is the stored status", () => {
        for (const status of Object.values(SigningRequestStatus)) {
            expect(shownRequestStatus({status, expires_at: null}, now)).toBe(status)
        }
    })

    it("is expired for a waiting request past its expiry, before the job says so", () => {
        const waiting = (expires_at: string) => ({status: SigningRequestStatus.Waiting, expires_at})
        expect(shownRequestStatus(waiting("2028-05-08T11:59:59Z"), now)).toBe(
            SigningRequestStatus.Expired
        )
        expect(shownRequestStatus(waiting("2028-05-08T12:00:01Z"), now)).toBe(
            SigningRequestStatus.Waiting
        )
    })

    it("is labelled by signing.status.<status>, so the panel and the Requests list agree", () => {
        expect(requestStatusKey(SigningRequestStatus.Completed)).toBe("signing.status.completed")
        expect(requestStatusKey(SigningRequestStatus.Executed)).toBe("signing.status.executed")
    })
})
