// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {describe, expect, it, jest} from "@jest/globals"
import {renderToStaticMarkup} from "react-dom/server"

jest.mock("react-apexcharts", () => {
    const react = jest.requireActual<typeof import("react")>("react")

    return {
        __esModule: true,
        default: ({
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
    }
})

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) =>
            ({
                "resultsPortal.summary.title": "General information",
                "resultsPortal.summary.ariaLabel": "General results information",
                "resultsPortal.summary.election": "Election",
                "resultsPortal.summary.eligibleVoters": "Eligible voters",
                "resultsPortal.summary.totalVotesCounted": "Total votes counted",
                "resultsPortal.summary.participation": "Participation",
                "resultsPortal.summary.totalBlankBallots": "Total blank ballots",
                "resultsPortal.resultsAndParticipation.nonVoters": "Non voters",
            })[key] ?? key,
    }),
}))

jest.mock("@sequentech/ui-core", () => ({
    formatPercentOne: (value: number) => `${value.toFixed(2)}%`,
    isNumber: (value: unknown) => typeof value === "number" && Number.isFinite(value),
}))

jest.mock("@sequentech/ui-essentials", () => ({
    TALLY_RESULTS_PIE_HEIGHT: 170,
    TALLY_RESULTS_PIE_PANEL_WIDTH: 360,
}))

jest.mock("@/services/resultLabels", () => ({
    translatedLabel: () => "Election",
}))

import {ResultsSummary} from "./ResultsSummary"

describe("ResultsSummary", () => {
    it("renders a 100 percent non-voters pie for an empty tally", () => {
        const markup = renderToStaticMarkup(
            <ResultsSummary
                elections={[{id: "election-id", presentation: {en: "Election"}}]}
                resultsElections={[
                    {
                        id: "result-id",
                        election_id: "election-id",
                        name: "Election",
                        elegible_census: 0,
                        total_voters: 0,
                        total_voters_percent: 0,
                    },
                ]}
                locale="en"
            />
        )

        expect(markup).toContain("seq-results-summary__chart")
        expect(markup).toContain("seq-results-summary__pie")
        expect(markup).toContain('data-height="170"')
        expect(markup).toContain("Non voters")
        expect(markup).toContain('data-series="[100]"')
    })

    it("shows the blank ballots column when a row carries a numeric value", () => {
        const markup = renderToStaticMarkup(
            <ResultsSummary
                elections={[{id: "election-id", presentation: {en: "Election"}}]}
                resultsElections={[
                    {
                        id: "result-id",
                        election_id: "election-id",
                        name: "Election",
                        elegible_census: 10,
                        total_voters: 5,
                        total_voters_percent: 50,
                        blank_ballots: 2,
                    },
                ]}
                locale="en"
            />
        )

        expect(markup).toContain("seq-results-summary__blank-ballots-heading")
        expect(markup).toContain("seq-results-summary__blank-ballots-cell")
        expect(markup).toContain("Total blank ballots")
    })

    it("hides the blank ballots column when no row carries a numeric value", () => {
        const markup = renderToStaticMarkup(
            <ResultsSummary
                elections={[{id: "election-id", presentation: {en: "Election"}}]}
                resultsElections={[
                    {
                        id: "result-id",
                        election_id: "election-id",
                        name: "Election",
                        elegible_census: 10,
                        total_voters: 5,
                        total_voters_percent: 50,
                        blank_ballots: null,
                    },
                ]}
                locale="en"
            />
        )

        expect(markup).not.toContain("seq-results-summary__blank-ballots-heading")
        expect(markup).not.toContain("seq-results-summary__blank-ballots-cell")
    })
})
