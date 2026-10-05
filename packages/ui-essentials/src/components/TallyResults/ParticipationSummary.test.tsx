// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderToStaticMarkup} from "react-dom/server"

jest.mock("./ChartPanel", () => {
    const react = jest.requireActual<typeof import("react")>("react")

    return {
        // Labels every slice and tooltip the way the chart library would, through
        // the formatters the component passes.
        Chart: ({
            className,
            height,
            options,
            series = [],
        }: {
            className?: string
            height?: number | string
            options?: {
                labels?: string[]
                dataLabels?: {formatter?: (percentage: number) => string}
                tooltip?: {y?: {formatter?: (value: number) => string}}
            }
            series?: number[]
        }) => {
            const total = series.reduce((sum, value) => sum + value, 0)

            return react.createElement("div", {
                className,
                "data-height": height,
                "data-labels": JSON.stringify(options?.labels ?? []),
                "data-series": JSON.stringify(series),
                "data-slice-labels": series
                    .map((value) => options?.dataLabels?.formatter?.((value / total) * 100))
                    .join("|"),
                "data-tooltip-labels": series
                    .map((value) => options?.tooltip?.y?.formatter?.(value))
                    .join("|"),
            })
        },
        ChartPanel: ({
            children,
            title,
            className,
        }: {
            children: React.ReactNode
            title: string
            className?: string
        }) => react.createElement("section", {className, "aria-label": title}, children),
    }
})

// ui-core's built dist is unavailable when this package's tests run alone, so
// load the canonical channel and number format modules from source instead of
// duplicating them.
jest.mock(
    "@sequentech/ui-core",
    () => ({
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/types/VotingChannel"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/types/ElectionEventPresentation"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/services/numberFormat"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/services/NumberFormatContext"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/services/percentFormatter"
        ),
    }),
    {virtual: true}
)

import {
    ENumberFormatPolicy,
    NumberFormatProvider,
    TallySheetVotingChannel,
    VotingStatusChannel,
    parseParticipationChannel,
} from "@sequentech/ui-core"
import {ParticipationByChannel} from "./ParticipationByChannel"
import {ParticipationSummary, ParticipationSummaryChart} from "./ParticipationSummary"
import type {ResultsParticipationSummary} from "./types"

const NBSP = "\u00a0"

describe("ParticipationSummaryChart", () => {
    it("keeps the chart panel visible when every tally value is zero", () => {
        const result: ResultsParticipationSummary = {
            eligibleCensus: 0,
            totalAuditableVotes: 0,
            totalAuditableVotesPercent: 0,
            totalVotes: 0,
            totalVotesPercent: 0,
            totalValidVotes: 0,
            totalValidVotesPercent: 0,
            totalInvalidVotes: 0,
            totalInvalidVotesPercent: 0,
            explicitInvalidVotes: 0,
            explicitInvalidVotesPercent: 0,
            implicitInvalidVotes: 0,
            implicitInvalidVotesPercent: 0,
            blankVotes: 0,
            blankVotesPercent: 0,
        }

        const markup = renderToStaticMarkup(
            <ParticipationSummaryChart
                result={result}
                chartName="Election - Contest"
                labels={{empty: "No results", nonVoters: "Non voters"}}
            />
        )

        expect(markup).toContain("seq-tally-results-participation-chart")
        expect(markup).toContain("seq-tally-results-participation-chart__chart")
        expect(markup).toContain('data-height="170"')
        expect(markup).toContain("Non voters")
        expect(markup).toContain('data-series="[100]"')
        expect(markup).not.toContain("No results")
        expect(markup).not.toContain('role="img"')
    })
})

describe("ParticipationSummary", () => {
    const result: ResultsParticipationSummary = {
        eligibleCensus: 12000000,
        totalVotes: 8589934,
        totalVotesPercent: 0.715827833,
        totalValidVotes: 8589000,
        totalValidVotesPercent: 0.7157,
        weight: 1234567,
    }

    it("writes counts and percentages in the event's number format", () => {
        const markup = renderToStaticMarkup(
            <NumberFormatProvider policy={ENumberFormatPolicy.PERIOD_COMMA}>
                <ParticipationSummary result={result} chartName="Election - Contest" showWeight />
            </NumberFormatProvider>
        )

        expect(markup).toContain(">12.000.000<")
        expect(markup).toContain(">8.589.934<")
        expect(markup).toContain(">71,58%<")
        expect(markup).toContain(">8.589.000<")
        expect(markup).toContain(">71,57%<")
        expect(markup).toContain(">1.234.567<")
        expect(markup).not.toContain("12000000")
    })

    it("writes comma grouped counts for events without a number format", () => {
        const markup = renderToStaticMarkup(
            <ParticipationSummary result={result} chartName="Election - Contest" showWeight />
        )

        expect(markup).toContain(">12,000,000<")
        expect(markup).toContain(">8,589,934<")
        expect(markup).toContain(">71.58%<")
        expect(markup).toContain(">71.57%<")
        expect(markup).toContain(">1,234,567<")
    })

    it("keeps a dash for figures the tally did not produce", () => {
        const markup = renderToStaticMarkup(
            <NumberFormatProvider policy={ENumberFormatPolicy.PERIOD_COMMA}>
                <ParticipationSummary
                    result={{eligibleCensus: 10}}
                    chartName="Election - Contest"
                />
            </NumberFormatProvider>
        )

        expect(markup).toMatch(/__value-cell[^"]*">-</)
        expect(markup).toMatch(/__percent-cell[^"]*">-</)
        expect(markup).not.toContain("NaN")
    })

    it("labels the participation chart in the event's number format", () => {
        const markup = renderToStaticMarkup(
            <NumberFormatProvider policy={ENumberFormatPolicy.SPACE_COMMA}>
                <ParticipationSummaryChart
                    result={{eligibleCensus: 12000000, totalVotes: 3000000}}
                    chartName="Election - Contest"
                />
            </NumberFormatProvider>
        )

        expect(markup).toContain(
            `data-tooltip-labels="3${NBSP}000${NBSP}000|9${NBSP}000${NBSP}000"`
        )
        expect(markup).toContain('data-slice-labels="25,0%|75,0%"')
    })
})

describe("ParticipationByChannel", () => {
    it("renders non-zero channel totals in canonical order using total vote percentages", () => {
        const markup = renderToStaticMarkup(
            <ParticipationByChannel
                chartName="Election - Contest"
                result={{
                    eligibleCensus: 20,
                    votesByChannel: {
                        [TallySheetVotingChannel.Paper]: 3,
                        [VotingStatusChannel.Online]: 5,
                        [TallySheetVotingChannel.Postal]: 0,
                        [parseParticipationChannel("FUTURE_CHANNEL")]: 2,
                    },
                }}
            />
        )

        const onlineIndex = markup.indexOf(">Online<")
        const paperIndex = markup.indexOf(">Paper<")
        const futureChannelIndex = markup.indexOf(">Future Channel<")

        expect(markup).toContain("Participation by channel")
        expect(onlineIndex).toBeGreaterThan(-1)
        expect(paperIndex).toBeGreaterThan(onlineIndex)
        expect(futureChannelIndex).toBeGreaterThan(paperIndex)
        expect(markup).toContain("50.0%")
        expect(markup).toContain("30.0%")
        expect(markup).toContain("20.0%")
        expect(markup).not.toContain("Postal")
        expect(markup).toContain("seq-tally-results-participation-by-channel-chart")
        expect(markup).toContain('aria-label="Election - Contest"')
        expect(markup).toContain('data-height="170"')
        expect(markup).toContain('data-series="[5,3,2]"')
    })

    it("orders unknown channels deterministically after known channels", () => {
        const markup = renderToStaticMarkup(
            <ParticipationByChannel
                result={{
                    eligibleCensus: 10,
                    votesByChannel: {
                        [parseParticipationChannel("A_B")]: 1,
                        [parseParticipationChannel("AA")]: 1,
                        [VotingStatusChannel.Online]: 1,
                    },
                }}
            />
        )

        const onlineIndex = markup.indexOf(">Online<")
        const aaIndex = markup.indexOf(">Aa<")
        const aBIndex = markup.indexOf(">A B<")

        expect(onlineIndex).toBeGreaterThan(-1)
        expect(aaIndex).toBeGreaterThan(onlineIndex)
        expect(aBIndex).toBeGreaterThan(aaIndex)
    })

    it("uses total channel votes independently of the census", () => {
        const markup = renderToStaticMarkup(
            <ParticipationByChannel
                result={{
                    eligibleCensus: 1,
                    votesByChannel: {
                        [VotingStatusChannel.Online]: 3,
                        [TallySheetVotingChannel.Paper]: 1,
                    },
                }}
            />
        )
        const zeroCensus = renderToStaticMarkup(
            <ParticipationByChannel
                result={{
                    eligibleCensus: 0,
                    votesByChannel: {[VotingStatusChannel.Online]: 1},
                }}
            />
        )

        expect(markup).toContain("75.0%")
        expect(markup).toContain("25.0%")
        expect(zeroCensus).toContain("100.0%")
    })

    it("writes channel totals and shares in the event's number format", () => {
        const markup = renderToStaticMarkup(
            <NumberFormatProvider policy={ENumberFormatPolicy.PERIOD_COMMA}>
                <ParticipationByChannel
                    result={{
                        votesByChannel: {
                            [VotingStatusChannel.Online]: 1500000,
                            [TallySheetVotingChannel.Paper]: 500000,
                        },
                    }}
                />
            </NumberFormatProvider>
        )

        expect(markup).toContain(">1.500.000<")
        expect(markup).toContain(">500.000<")
        expect(markup).toContain(">75,0%<")
        expect(markup).toContain(">25,0%<")
        expect(markup).toContain('data-tooltip-labels="1.500.000|500.000"')
        expect(markup).toContain('data-slice-labels="75,0%|25,0%"')
    })

    it("writes comma grouped channel totals for events without a number format", () => {
        const markup = renderToStaticMarkup(
            <ParticipationByChannel
                result={{
                    votesByChannel: {
                        [VotingStatusChannel.Online]: 1500000,
                        [TallySheetVotingChannel.Paper]: 500000,
                    },
                }}
            />
        )

        expect(markup).toContain(">1,500,000<")
        expect(markup).toContain(">75.0%<")
        expect(markup).toContain('data-tooltip-labels="1,500,000|500,000"')
        expect(markup).toContain('data-slice-labels="75.0%|25.0%"')
    })
})
