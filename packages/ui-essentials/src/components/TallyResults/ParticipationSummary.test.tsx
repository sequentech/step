// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderToStaticMarkup} from "react-dom/server"

jest.mock("./ChartPanel", () => {
    const react = jest.requireActual<typeof import("react")>("react")

    return {
        Chart: ({
            className,
            height,
            options,
            series,
        }: {
            className?: string
            height?: number | string
            options?: {labels?: string[]}
            series?: number[]
        }) =>
            react.createElement("div", {
                className,
                "data-height": height,
                "data-labels": JSON.stringify(options?.labels ?? []),
                "data-series": JSON.stringify(series ?? []),
            }),
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
// load the canonical channel module from source instead of duplicating its values.
jest.mock(
    "@sequentech/ui-core",
    () => {
        const votingChannels = jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/types/VotingChannel"
        )

        return {
            ...votingChannels,
            formatPercentOne: (value: number) => `${(value * 100).toFixed(1)}%`,
        }
    },
    {virtual: true}
)

import {
    TallySheetVotingChannel,
    VotingStatusChannel,
    parseParticipationChannel,
} from "@sequentech/ui-core"
import {ParticipationByChannel} from "./ParticipationByChannel"
import {ParticipationSummaryChart} from "./ParticipationSummary"
import type {ResultsParticipationSummary} from "./types"

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
})
