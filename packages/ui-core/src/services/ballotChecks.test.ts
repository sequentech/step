// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EChecksPeriodStatus, getChecksPeriod, isValidChecksAvailableUntil} from "./ballotChecks"
import {EChecksPeriodPolicy, IElectionEventPresentation} from "../types/ElectionEventPresentation"

const presentation = (receipts: IElectionEventPresentation["receipts"]) =>
    ({receipts}) as IElectionEventPresentation

const UNTIL = "2028-06-07T23:59:00+08:00"

describe("getChecksPeriod", () => {
    it("keeps unlimited checks for events without the setting", () => {
        const now = new Date("2030-01-01T00:00:00Z")
        for (const value of [
            undefined,
            null,
            {} as IElectionEventPresentation,
            presentation(undefined),
            presentation({}),
            presentation({checks_period_policy: EChecksPeriodPolicy.UNLIMITED}),
            presentation({
                checks_period_policy: EChecksPeriodPolicy.UNLIMITED,
                checks_available_until: "2020-01-01T00:00:00Z",
            }),
        ]) {
            expect(getChecksPeriod(value, now)).toEqual({status: EChecksPeriodStatus.UNLIMITED})
        }
    })

    it("is open until the configured instant and ended after it", () => {
        const configured = presentation({
            checks_period_policy: EChecksPeriodPolicy.UNTIL_DATE,
            checks_available_until: UNTIL,
        })
        const until = new Date(UNTIL)

        expect(getChecksPeriod(configured, new Date("2028-05-09T00:00:00Z"))).toEqual({
            status: EChecksPeriodStatus.OPEN,
            until,
        })
        expect(getChecksPeriod(configured, until)).toEqual({
            status: EChecksPeriodStatus.OPEN,
            until,
        })
        expect(getChecksPeriod(configured, new Date(until.getTime() + 1000))).toEqual({
            status: EChecksPeriodStatus.ENDED,
            until,
        })
    })

    it("reports a period without a readable date as invalid", () => {
        for (const checks_available_until of [
            undefined,
            "",
            "soon",
            "2028-06-07",
            "2028-06-07T23:59",
        ]) {
            expect(
                getChecksPeriod(
                    presentation({
                        checks_period_policy: EChecksPeriodPolicy.UNTIL_DATE,
                        checks_available_until,
                    }),
                    new Date()
                )
            ).toEqual({status: EChecksPeriodStatus.INVALID})
        }
    })
})

describe("isValidChecksAvailableUntil", () => {
    it("requires a date and time with an offset", () => {
        expect(isValidChecksAvailableUntil(UNTIL)).toBe(true)
        expect(isValidChecksAvailableUntil("2028-06-07T15:59:00Z")).toBe(true)
        expect(isValidChecksAvailableUntil("2028-06-07T15:59:00.000Z")).toBe(true)
        expect(isValidChecksAvailableUntil("2028-06-07T23:59")).toBe(false)
        expect(isValidChecksAvailableUntil("2028-06-07")).toBe(false)
        expect(isValidChecksAvailableUntil("2028-13-40T23:59:00Z")).toBe(false)
        expect(isValidChecksAvailableUntil("")).toBe(false)
        expect(isValidChecksAvailableUntil(undefined)).toBe(false)
    })
})
