// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ENumberFormatPolicy} from "@sequentech/ui-core"
import {getVotersByChannelChartOptions} from "./votersByChannelOptions"

describe("getVotersByChannelChartOptions", () => {
    it("names each slice after its channel and always shows the total", () => {
        const options = getVotersByChannelChartOptions({
            labels: ["Online", "Kiosk"],
            numberFormatPolicy: ENumberFormatPolicy.COMMA_PERIOD,
        })

        expect(options.labels).toEqual(["Online", "Kiosk"])
        expect(options.plotOptions?.pie?.donut?.labels).toMatchObject({
            show: true,
            total: {show: true, showAlways: true},
        })
    })

    it("writes the donut's total, hovered count and slice percentages in the event's number format", () => {
        const options = getVotersByChannelChartOptions({
            labels: ["Online", "Kiosk"],
            numberFormatPolicy: ENumberFormatPolicy.APOSTROPHE_PERIOD,
        })
        const donutLabels = options.plotOptions?.pie?.donut?.labels

        expect(donutLabels?.total?.formatter?.({globals: {seriesTotals: [1200, 34]}})).toBe("1’234")
        expect(donutLabels?.value?.formatter?.("1234567")).toBe("1’234’567")
        expect(options.dataLabels?.formatter?.(97.25)).toBe("97.3%")
    })
})
