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
            options,
            series,
        }: {
            className?: string
            options?: {labels?: string[]}
            series?: number[]
        }) =>
            react.createElement("div", {
                className,
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

jest.mock("@sequentech/ui-core", () => ({
    formatPercentOne: (value: number) => `${value}%`,
}))

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
        expect(markup).toContain("Non voters")
        expect(markup).toContain('data-series="[100]"')
        expect(markup).not.toContain("No results")
        expect(markup).not.toContain('role="img"')
    })
})
