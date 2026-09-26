// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {onCLS, onINP, onFCP, onLCP, onTTFB, MetricType} from "web-vitals"
import reportWebVitals from "../reportWebVitals"

jest.mock("web-vitals", () => ({
    onCLS: jest.fn(),
    onINP: jest.fn(),
    onFCP: jest.fn(),
    onLCP: jest.fn(),
    onTTFB: jest.fn(),
}))
afterEach(() => jest.clearAllMocks())

test("performance reporting is opt-in and forwards the browser's measurements unchanged", async () => {
    reportWebVitals()
    await new Promise<void>((resolve) => setTimeout(resolve, 0))
    for (const observer of [onCLS, onINP, onFCP, onLCP, onTTFB])
        expect(observer).not.toHaveBeenCalled()

    const receive = jest.fn()
    const onMetric = (metric: MetricType) => receive(metric)
    reportWebVitals(onMetric)
    await new Promise<void>((resolve) => setTimeout(resolve, 0))
    for (const observer of [onCLS, onINP, onFCP, onLCP, onTTFB])
        expect(observer).toHaveBeenCalledWith(onMetric)
    const layoutShift: MetricType = {
        name: "CLS",
        value: 0.05,
        delta: 0.05,
        rating: "good",
        id: "synthetic-layout-shift",
        navigationType: "navigate",
        entries: [],
    }
    jest.mocked(onCLS).mock.calls[0][0](layoutShift)
    expect(receive).toHaveBeenCalledTimes(1)
    expect(receive).toHaveBeenCalledWith(layoutShift)
})
