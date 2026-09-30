// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {MONITORING_DEFAULT_REFRESH_MS, MONITORING_MIN_REFRESH_MS} from "../types"
import {monitoringRefreshMs} from "./refresh"

describe("monitoringRefreshMs", () => {
    it("polls as often as the server counts", () => {
        expect(monitoringRefreshMs(60)).toBe(60_000)
        expect(monitoringRefreshMs(3600)).toBe(3_600_000)
    })

    it("keeps the default when the server reports no interval", () => {
        expect(MONITORING_DEFAULT_REFRESH_MS).toBe(30_000)
        expect(monitoringRefreshMs(undefined)).toBe(MONITORING_DEFAULT_REFRESH_MS)
        expect(monitoringRefreshMs(null)).toBe(MONITORING_DEFAULT_REFRESH_MS)
        expect(monitoringRefreshMs(0)).toBe(MONITORING_DEFAULT_REFRESH_MS)
        expect(monitoringRefreshMs(-30)).toBe(MONITORING_DEFAULT_REFRESH_MS)
        expect(monitoringRefreshMs(Number.NaN)).toBe(MONITORING_DEFAULT_REFRESH_MS)
        expect(monitoringRefreshMs(Number.POSITIVE_INFINITY)).toBe(MONITORING_DEFAULT_REFRESH_MS)
    })

    it("never polls more often than every 5 s", () => {
        expect(MONITORING_MIN_REFRESH_MS).toBe(5_000)
        expect(monitoringRefreshMs(1)).toBe(5_000)
        expect(monitoringRefreshMs(4.9)).toBe(5_000)
        expect(monitoringRefreshMs(5)).toBe(5_000)
    })
})
