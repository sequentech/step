// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

jest.mock(
    "@sequentech/ui-core",
    () => ({
        escapeHtml: jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../ui-core/src/services/stringToHtml"
        ).escapeHtml,
    }),
    {virtual: true}
)

import {withEscapedChartText} from "./chartOptions"

const MARKUP_NAME = '<b title="x">Ann & Co</b>'
const ESCAPED_NAME = "&lt;b title=&quot;x&quot;&gt;Ann &amp; Co&lt;/b&gt;"

describe("withEscapedChartText", () => {
    it("escapes legend entries and tooltip series names", () => {
        const options = withEscapedChartText({labels: [MARKUP_NAME]})
        const tooltipY = options.tooltip?.y

        expect(options.legend?.formatter?.(MARKUP_NAME, {})).toBe(ESCAPED_NAME)
        expect(
            Array.isArray(tooltipY) ? undefined : tooltipY?.title?.formatter?.(MARKUP_NAME)
        ).toBe(ESCAPED_NAME)
    })

    it("keeps the labels and the other legend and tooltip settings", () => {
        const valueFormatter = (value: number) => `${value} votes`
        const options = withEscapedChartText({
            labels: [MARKUP_NAME, "Plain name"],
            legend: {position: "right", showForZeroSeries: false},
            tooltip: {shared: true, y: {formatter: valueFormatter}},
        })
        const tooltipY = Array.isArray(options.tooltip?.y) ? undefined : options.tooltip?.y

        expect(options.labels).toEqual([MARKUP_NAME, "Plain name"])
        expect(options.legend).toMatchObject({position: "right", showForZeroSeries: false})
        expect(options.tooltip?.shared).toBe(true)
        expect(tooltipY?.formatter).toBe(valueFormatter)
        expect(options.legend?.formatter?.(MARKUP_NAME, {})).toBe(ESCAPED_NAME)
    })

    it("escapes the series names of every per-series tooltip", () => {
        const options = withEscapedChartText({tooltip: {y: [{}, {title: {}}]}})
        const tooltipY = options.tooltip?.y

        expect(Array.isArray(tooltipY)).toBe(true)
        for (const entry of Array.isArray(tooltipY) ? tooltipY : []) {
            expect(entry.title?.formatter?.(MARKUP_NAME)).toBe(ESCAPED_NAME)
        }
    })
})
