// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import type {ApexOptions} from "apexcharts"
import type {Props as ApexChartProps} from "react-apexcharts"
import {VotingStatusChannel} from "@sequentech/ui-core"
import {VotersByChannel} from "./VotersByChannel"
import {VotesPerDay} from "./VotesPerDay"
import {DEFAULT_VOTES_TIME_SELECTION} from "./votesTimeRange"

const mockMarkupLabel = "<b>Online</b> & more"
const escapedLabel = "&lt;b&gt;Online&lt;/b&gt; &amp; more"

const mockChartProps: ApexChartProps[] = []

jest.mock("react-apexcharts", () => ({
    __esModule: true,
    default: (props: ApexChartProps) => {
        mockChartProps.push(props)
        return null
    },
}))

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => (key === "common.channel.online" ? mockMarkupLabel : key),
        i18n: {language: "en", resolvedLanguage: "en"},
    }),
}))

jest.mock("./Charts", () => ({
    __esModule: true,
    default: ({children}: {children: React.ReactNode}) => children,
}))

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual<typeof import("@sequentech/ui-core")>("@sequentech/ui-core"),
    escapeHtml: jest.requireActual<typeof import("@sequentech/ui-core")>(
        "../../../../../ui-core/src/services/stringToHtml"
    ).escapeHtml,
}))

jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        withEscapedChartText: jest.requireActual<typeof import("@sequentech/ui-essentials")>(
            "../../../../../ui-essentials/src/services/chartOptions"
        ).withEscapedChartText,
    }),
    {virtual: true}
)

const expectEscapedChartText = (options: ApexOptions | undefined) => {
    const tooltipY = Array.isArray(options?.tooltip?.y) ? undefined : options?.tooltip?.y

    expect(options?.legend?.formatter?.(mockMarkupLabel, {})).toBe(escapedLabel)
    expect(tooltipY?.title?.formatter?.(mockMarkupLabel)).toBe(escapedLabel)
}

beforeEach(() => {
    mockChartProps.length = 0
})

describe("VotersByChannel", () => {
    it("escapes channel names in the legend and tooltip and keeps the labels raw", () => {
        renderToStaticMarkup(
            React.createElement(VotersByChannel, {
                data: [{channel: VotingStatusChannel.Online, count: 2}],
                width: 300,
                height: 200,
            })
        )

        expect(mockChartProps).toHaveLength(1)
        expect(mockChartProps[0].options?.labels).toEqual([mockMarkupLabel])
        expectEscapedChartText(mockChartProps[0].options)
    })
})

describe("VotesPerDay", () => {
    it("escapes channel names in the legend and tooltip and keeps the series names raw", () => {
        renderToStaticMarkup(
            React.createElement(VotesPerDay, {
                data: [
                    {
                        bucket: "2026-07-31",
                        channel: VotingStatusChannel.Online,
                        day: "2026-07-31",
                        day_count: 3,
                    },
                ],
                width: 300,
                height: 200,
                selection: DEFAULT_VOTES_TIME_SELECTION,
                onSelectionChange: () => undefined,
            })
        )

        expect(mockChartProps).toHaveLength(1)
        expect(mockChartProps[0].series).toEqual([{name: mockMarkupLabel, data: [3]}])
        expect(mockChartProps[0].options?.tooltip?.shared).toBe(true)
        expectEscapedChartText(mockChartProps[0].options)
    })
})
