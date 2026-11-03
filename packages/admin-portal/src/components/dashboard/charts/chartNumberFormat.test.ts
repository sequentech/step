// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ENumberFormatPolicy} from "@sequentech/ui-core"
import {getPieChartNumberFormatOptions} from "./chartNumberFormat"

const single = <T>(value: T | T[] | undefined): T | undefined =>
    Array.isArray(value) ? value[0] : value

describe("getPieChartNumberFormatOptions", () => {
    it("writes slice percentages and hovered counts in the event's number format", () => {
        const options = getPieChartNumberFormatOptions(ENumberFormatPolicy.PERIOD_COMMA)

        expect(options.dataLabels?.formatter?.(45.678)).toBe("45,7%")
        expect(single(options.tooltip?.y)?.formatter?.(1234567)).toBe("1.234.567")
    })

    it("keeps one decimal on slice percentages for events without a number format policy", () => {
        const options = getPieChartNumberFormatOptions(ENumberFormatPolicy.COMMA_PERIOD)

        expect(options.dataLabels?.formatter?.(100)).toBe("100.0%")
        expect(options.dataLabels?.formatter?.(0.04)).toBe("0.0%")
        expect(single(options.tooltip?.y)?.formatter?.(1234567)).toBe("1,234,567")
    })
})
